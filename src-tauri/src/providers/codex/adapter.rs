use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{json, Value};

use crate::runtime::{CommandRequest, Runtime};

use super::super::adapter::{status_for_percentage, AdapterError};
use super::super::usage::{
    ProviderDiagnostics, ProviderUsage, UsagePeriod, UsageSource, UsageUnit,
};
use super::super::{ProviderId, ProviderInstallation, UsageProviderAdapter};

const APP_SERVER_TIMEOUT: Duration = Duration::from_secs(15);
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const RATE_LIMIT_REQUEST_ID: u64 = 2;
const RATE_LIMIT_RESPONSE_MARKER: &str = "\"id\":2";

/// Reads the authenticated ChatGPT quota through Codex's documented
/// `app-server` JSON-RPC interface. The Codex CLI owns credentials and token
/// refresh; Quotify only exchanges local JSONL messages over stdio.
pub struct CodexAdapter {
    state: Mutex<AdapterState>,
}

#[derive(Default)]
struct AdapterState {
    last_latency_ms: Option<u64>,
    last_success: Option<String>,
    last_error: Option<String>,
    cached_usage: Option<CachedUsage>,
}

struct CachedUsage {
    value: ProviderUsage,
    fetched_at: Instant,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitResult {
    rate_limits: Option<RateLimitBucket>,
    #[serde(default)]
    rate_limits_by_limit_id: HashMap<String, RateLimitBucket>,
}

#[derive(Debug, Deserialize)]
struct RateLimitBucket {
    primary: Option<RateLimitWindow>,
    secondary: Option<RateLimitWindow>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitWindow {
    used_percent: f64,
    window_duration_mins: u64,
    resets_at: Option<i64>,
}

impl CodexAdapter {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(AdapterState::default()),
        }
    }

    fn fetch_real_usage(
        &self,
        installation: &ProviderInstallation,
        runtime: &dyn Runtime,
    ) -> Result<ProviderUsage, AdapterError> {
        let executable = resolve_codex_executable(installation, runtime);
        let input = app_server_requests();
        let result = runtime
            .execute(
                CommandRequest::new(executable)
                    .args(["app-server", "--stdio"])
                    .stdin(input)
                    .until_stdout_contains(RATE_LIMIT_RESPONSE_MARKER)
                    .timeout(APP_SERVER_TIMEOUT),
            )
            .map_err(|error| match error {
                crate::runtime::RuntimeError::Timeout(_) => AdapterError::Timeout,
                _ => AdapterError::Unavailable("could not start Codex app-server".into()),
            })?;

        normalize_app_server_output(&result.stdout, installation)
    }

    fn fresh_cached_usage(&self) -> Option<ProviderUsage> {
        let state = self.state.lock().ok()?;
        let cached = state.cached_usage.as_ref()?;
        (cached.fetched_at.elapsed() <= CACHE_TTL).then(|| cached.value.clone())
    }

    fn stale_cached_usage(&self, error: &AdapterError) -> Option<ProviderUsage> {
        let state = self.state.lock().ok()?;
        let mut usage = state.cached_usage.as_ref()?.value.clone();
        usage.error = Some(error.to_string());
        Some(usage)
    }

    fn record_result(&self, started: Instant, result: &Result<ProviderUsage, AdapterError>) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        state.last_latency_ms = Some(started.elapsed().as_millis() as u64);
        match result {
            Ok(usage) => {
                state.last_success = Some(chrono::Utc::now().to_rfc3339());
                state.last_error = None;
                state.cached_usage = Some(CachedUsage {
                    value: usage.clone(),
                    fetched_at: Instant::now(),
                });
            }
            Err(error) => state.last_error = Some(error.to_string()),
        }
    }
}

impl UsageProviderAdapter for CodexAdapter {
    fn provider(&self) -> ProviderId {
        ProviderId::Codex
    }

    fn fetch_usage(
        &self,
        installation: &ProviderInstallation,
        runtime: &dyn Runtime,
    ) -> Result<ProviderUsage, AdapterError> {
        if let Some(cached) = self.fresh_cached_usage() {
            return Ok(cached);
        }

        let started = Instant::now();
        let result = self.fetch_real_usage(installation, runtime);
        self.record_result(started, &result);

        match result {
            Ok(usage) => Ok(usage),
            Err(error) => self.stale_cached_usage(&error).ok_or(error),
        }
    }

    fn diagnostics(
        &self,
        _installation: &ProviderInstallation,
        _runtime: &dyn Runtime,
    ) -> Result<ProviderDiagnostics, AdapterError> {
        let state = self
            .state
            .lock()
            .map_err(|_| AdapterError::Unavailable("diagnostics state is unavailable".into()))?;

        Ok(ProviderDiagnostics {
            adapter: "CodexAdapter".to_string(),
            usage_source: UsageSource::Cli,
            last_latency_ms: state.last_latency_ms,
            last_success: state.last_success.clone(),
            last_error: state.last_error.clone(),
        })
    }
}

fn app_server_requests() -> String {
    let initialize = json!({
        "method": "initialize",
        "id": 1,
        "params": {
            "clientInfo": {
                "name": "quotify",
                "title": "Quotify",
                "version": env!("CARGO_PKG_VERSION"),
            }
        }
    });
    let initialized = json!({ "method": "initialized", "params": {} });
    let rate_limits = json!({
        "method": "account/rateLimits/read",
        "id": RATE_LIMIT_REQUEST_ID,
        "params": {},
    });

    format!("{initialize}\n{initialized}\n{rate_limits}\n")
}

fn normalize_app_server_output(
    stdout: &str,
    installation: &ProviderInstallation,
) -> Result<ProviderUsage, AdapterError> {
    let response = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|message| message.get("id").and_then(Value::as_u64) == Some(RATE_LIMIT_REQUEST_ID))
        .ok_or_else(|| {
            AdapterError::Unavailable("Codex app-server returned no rate-limit response".into())
        })?;

    if response.get("error").is_some() {
        return Err(AdapterError::Unavailable(
            "Codex app-server rejected the rate-limit request; check `codex login`".into(),
        ));
    }

    let result = response.get("result").cloned().ok_or_else(|| {
        AdapterError::InvalidResponse("Codex response had no result payload".into())
    })?;
    let mut result: RateLimitResult = serde_json::from_value(result).map_err(|_| {
        AdapterError::InvalidResponse("Codex returned an unsupported rate-limit schema".into())
    })?;

    let bucket = result
        .rate_limits_by_limit_id
        .remove("codex")
        .or(result.rate_limits)
        .or_else(|| result.rate_limits_by_limit_id.into_values().next())
        .ok_or_else(|| {
            AdapterError::Unavailable(
                "Codex returned no ChatGPT quota; sign in with a ChatGPT account".into(),
            )
        })?;
    let window = bucket
        .primary
        .or(bucket.secondary)
        .ok_or_else(|| AdapterError::Unavailable("Codex returned no active quota window".into()))?;

    usage_from_window(window, installation)
}

fn usage_from_window(
    window: RateLimitWindow,
    installation: &ProviderInstallation,
) -> Result<ProviderUsage, AdapterError> {
    if !window.used_percent.is_finite() || window.used_percent < 0.0 {
        return Err(AdapterError::InvalidResponse(
            "Codex usage percentage was outside the supported range".into(),
        ));
    }

    let (period, period_description) = describe_window(window.window_duration_mins);
    let reset_at = window
        .resets_at
        .and_then(|timestamp| chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0))
        .map(|timestamp| timestamp.to_rfc3339());

    Ok(ProviderUsage {
        provider: ProviderId::Codex,
        installation_id: installation.id.clone(),
        runtime_id: installation.runtime_id.clone(),
        status: status_for_percentage(window.used_percent),
        percentage: Some(window.used_percent),
        used: None,
        remaining: Some((100.0 - window.used_percent).max(0.0)),
        limit: Some(100.0),
        unit: Some(UsageUnit::Percentage),
        period: Some(period),
        period_description: Some(period_description),
        reset_at,
        updated_at: chrono::Utc::now().to_rfc3339(),
        source: UsageSource::Cli,
        error: None,
    })
}

fn describe_window(minutes: u64) -> (UsagePeriod, String) {
    match minutes {
        60 => (UsagePeriod::Hourly, "hourly".into()),
        1_440 => (UsagePeriod::Daily, "daily".into()),
        10_080 => (UsagePeriod::Weekly, "weekly".into()),
        value if value > 0 && value % 60 == 0 => {
            (UsagePeriod::Rolling, format!("{}h rolling", value / 60))
        }
        value => (UsagePeriod::Rolling, format!("{value}m rolling")),
    }
}

/// npm exposes PowerShell/CMD shims on Windows, while Rust intentionally does
/// not invoke shell scripts. Prefer the platform executable bundled in the
/// official npm package when it is present.
fn resolve_codex_executable(installation: &ProviderInstallation, runtime: &dyn Runtime) -> String {
    let discovered = &installation.executable;
    if discovered.to_ascii_lowercase().ends_with(".exe") {
        return discovered.clone();
    }

    let Some((npm_dir, _)) = discovered.rsplit_once('\\') else {
        return discovered.clone();
    };
    let candidates = [
        format!(
            r"{npm_dir}\node_modules\@openai\codex\node_modules\@openai\codex-win32-x64\vendor\x86_64-pc-windows-msvc\codex\codex.exe"
        ),
        format!(
            r"{npm_dir}\node_modules\@openai\codex\node_modules\@openai\codex-win32-arm64\vendor\aarch64-pc-windows-msvc\codex\codex.exe"
        ),
    ];

    candidates
        .into_iter()
        .find(|candidate| runtime.exists(candidate).unwrap_or(false))
        .unwrap_or_else(|| discovered.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{CommandResult, RuntimeError, RuntimeKind};

    fn installation() -> ProviderInstallation {
        ProviderInstallation {
            id: "codex:test".into(),
            provider: ProviderId::Codex,
            provider_name: "Codex".into(),
            runtime_id: "test".into(),
            runtime_name: "Test".into(),
            executable: "/usr/bin/codex".into(),
        }
    }

    #[test]
    fn builds_the_documented_handshake_and_rate_limit_request() {
        let requests: Vec<Value> = app_server_requests()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();

        assert_eq!(requests[0]["method"], "initialize");
        assert_eq!(requests[1]["method"], "initialized");
        assert_eq!(requests[2]["method"], "account/rateLimits/read");
        assert_eq!(requests[2]["id"], RATE_LIMIT_REQUEST_ID);
    }

    #[test]
    fn normalizes_the_live_app_server_shape() {
        let output = r#"{"id":1,"result":{"userAgent":"quotify/test"}}
{"method":"account/rateLimits/updated","params":{}}
{"id":2,"result":{"ordinaryUsageAllowed":true,"rateLimits":{"limitId":"codex","primary":{"usedPercent":37,"windowDurationMins":300,"resetsAt":1789754458},"secondary":{"usedPercent":18,"windowDurationMins":10080,"resetsAt":1789999086},"planType":"plus"},"rateLimitsByLimitId":{"codex":{"primary":{"usedPercent":37,"windowDurationMins":300,"resetsAt":1789754458},"secondary":{"usedPercent":18,"windowDurationMins":10080,"resetsAt":1789999086}}}}}"#;

        let usage = normalize_app_server_output(output, &installation()).unwrap();

        assert_eq!(usage.percentage, Some(37.0));
        assert_eq!(usage.remaining, Some(63.0));
        assert_eq!(usage.period, Some(UsagePeriod::Rolling));
        assert_eq!(usage.period_description.as_deref(), Some("5h rolling"));
        assert_eq!(usage.source, UsageSource::Cli);
        assert!(usage.reset_at.is_some());
    }

    #[test]
    fn falls_back_to_the_weekly_window_when_primary_is_absent() {
        let output = r#"{"id":2,"result":{"rateLimits":{"primary":null,"secondary":{"usedPercent":18,"windowDurationMins":10080,"resetsAt":1789999086}},"rateLimitsByLimitId":{}}}"#;

        let usage = normalize_app_server_output(output, &installation()).unwrap();

        assert_eq!(usage.percentage, Some(18.0));
        assert_eq!(usage.period, Some(UsagePeriod::Weekly));
        assert_eq!(usage.period_description.as_deref(), Some("weekly"));
    }

    #[test]
    fn resolves_the_native_windows_npm_binary() {
        struct NativeWindowsRuntime;

        impl Runtime for NativeWindowsRuntime {
            fn id(&self) -> &str {
                "windows"
            }
            fn kind(&self) -> RuntimeKind {
                RuntimeKind::Windows
            }
            fn name(&self) -> &str {
                "Windows"
            }
            fn execute(&self, _request: CommandRequest) -> Result<CommandResult, RuntimeError> {
                unimplemented!()
            }
            fn which(&self, _binary: &str) -> Result<Option<String>, RuntimeError> {
                Ok(None)
            }
            fn exists(&self, path: &str) -> Result<bool, RuntimeError> {
                Ok(path.contains("codex-win32-x64") && path.ends_with("codex.exe"))
            }
            fn read_file(&self, _path: &str) -> Result<String, RuntimeError> {
                unimplemented!()
            }
            fn home_directory(&self) -> Result<String, RuntimeError> {
                unimplemented!()
            }
        }

        let mut windows_installation = installation();
        windows_installation.executable = r"C:\Users\Ada\AppData\Roaming\npm\codex.cmd".into();
        let resolved = resolve_codex_executable(&windows_installation, &NativeWindowsRuntime);

        assert!(resolved.contains(r"@openai\codex-win32-x64"));
        assert!(resolved.ends_with(r"codex\codex.exe"));
    }

    #[test]
    #[ignore = "requires an installed and authenticated Codex CLI"]
    fn live_smoke_test_reads_the_authenticated_accounts_quota() {
        let runtimes = crate::runtime::RuntimeManager::discover();
        let (runtime, executable) = runtimes
            .runtimes()
            .iter()
            .find_map(|runtime| {
                runtime
                    .which("codex")
                    .ok()
                    .flatten()
                    .map(|executable| (runtime.as_ref(), executable))
            })
            .expect("an installed Codex CLI is required");
        let live_installation = ProviderInstallation {
            id: format!("codex:{}", runtime.id()),
            provider: ProviderId::Codex,
            provider_name: "Codex".into(),
            runtime_id: runtime.id().into(),
            runtime_name: runtime.name().into(),
            executable,
        };

        let usage = CodexAdapter::new()
            .fetch_real_usage(&live_installation, runtime)
            .expect("authenticated Codex quota should be available");

        assert_eq!(usage.provider, ProviderId::Codex);
        assert!(usage.percentage.is_some());
        assert!(usage.reset_at.is_some());
    }
}
