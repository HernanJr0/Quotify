use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::runtime::{CommandRequest, Runtime};

use super::super::adapter::{status_for_percentage, AdapterError};
use super::super::usage::{
    ProviderDiagnostics, ProviderUsage, UsagePeriod, UsageSource, UsageUnit, UsageWindow,
};
use super::super::{ProviderId, ProviderInstallation, UsageProviderAdapter};

const CLI_TIMEOUT: Duration = Duration::from_secs(20);
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);

/// Real Claude subscription adapter (Fase 8).
///
/// The primary source asks the installed Claude CLI for its experimental
/// `get_usage` control response. That path invokes no model, lets Claude own
/// token refresh, and never exposes credentials to Quotify. Quotify does not
/// call Anthropic's undocumented OAuth endpoint directly.
pub struct ClaudeAdapter {
    state: Mutex<AdapterState>,
}

#[derive(Default)]
struct AdapterState {
    last_latency_ms: Option<u64>,
    last_success: Option<String>,
    last_error: Option<String>,
    cached_usage: HashMap<String, CachedUsage>,
    last_source: Option<UsageSource>,
}

struct CachedUsage {
    value: ProviderUsage,
    fetched_at: Instant,
}

impl ClaudeAdapter {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(AdapterState::default()),
        }
    }

    fn fetch_cli_usage(
        &self,
        installation: &ProviderInstallation,
        runtime: &dyn Runtime,
    ) -> Result<ProviderUsage, AdapterError> {
        const CONTROL_REQUEST: &str =
            "{\"type\":\"control_request\",\"request_id\":\"quotify-usage\",\"request\":{\"subtype\":\"get_usage\"}}\n";

        let executable = resolve_claude_executable(installation, runtime);
        let result = runtime
            .execute(
                CommandRequest::new(executable)
                    .args([
                        "-p",
                        "--input-format",
                        "stream-json",
                        "--output-format",
                        "stream-json",
                        "--verbose",
                        "--strict-mcp-config",
                        "--mcp-config",
                        r#"{"mcpServers":{}}"#,
                        "--settings",
                        r#"{"hooks":{}}"#,
                    ])
                    .stdin(CONTROL_REQUEST)
                    .timeout(CLI_TIMEOUT),
            )
            .map_err(|error| match error {
                crate::runtime::RuntimeError::Timeout(_) => AdapterError::Timeout,
                _ => AdapterError::Unavailable("could not start the Claude CLI usage probe".into()),
            })?;

        let response_line = result
            .stdout
            .lines()
            .find(|line| line.contains("\"control_response\""))
            .ok_or_else(|| {
                AdapterError::Unavailable(
                    "Claude CLI did not return a get_usage control response".into(),
                )
            })?;

        let mut usage = normalize_cli_response(response_line, installation)?;
        usage.account_key = fetch_account_key(installation, runtime);
        Ok(usage)
    }

    fn record_result(&self, started: Instant, result: &Result<ProviderUsage, AdapterError>) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        state.last_latency_ms = Some(started.elapsed().as_millis() as u64);
        match result {
            Ok(_) => {
                state.last_success = Some(chrono::Utc::now().to_rfc3339());
                state.last_error = None;
                state.last_source = result.as_ref().ok().map(|usage| usage.source);
            }
            Err(error) => state.last_error = Some(error.to_string()),
        }
    }

    fn fresh_cached_usage(&self, installation_id: &str) -> Option<ProviderUsage> {
        let state = self.state.lock().ok()?;
        let cached = state.cached_usage.get(installation_id)?;
        (cached.fetched_at.elapsed() <= CACHE_TTL).then(|| cached.value.clone())
    }

    fn stale_cached_usage(
        &self,
        installation_id: &str,
        error: &AdapterError,
    ) -> Option<ProviderUsage> {
        let state = self.state.lock().ok()?;
        let mut usage = state.cached_usage.get(installation_id)?.value.clone();
        usage.error = Some(error.to_string());
        Some(usage)
    }

    fn cache_usage(&self, usage: &ProviderUsage) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.cached_usage.insert(
            usage.installation_id.clone(),
            CachedUsage {
                value: usage.clone(),
                fetched_at: Instant::now(),
            },
        );
    }
}

impl UsageProviderAdapter for ClaudeAdapter {
    fn provider(&self) -> ProviderId {
        ProviderId::Claude
    }

    fn fetch_usage(
        &self,
        installation: &ProviderInstallation,
        runtime: &dyn Runtime,
    ) -> Result<ProviderUsage, AdapterError> {
        if let Some(cached) = self.fresh_cached_usage(&installation.id) {
            return Ok(cached);
        }

        let started = Instant::now();
        let result = self.fetch_cli_usage(installation, runtime);
        self.record_result(started, &result);
        match result {
            Ok(usage) => {
                self.cache_usage(&usage);
                Ok(usage)
            }
            Err(error) => {
                if let Some(stale) = self.stale_cached_usage(&installation.id, &error) {
                    return Ok(stale);
                }

                let account_key = fetch_account_key(installation, runtime);
                if account_key.is_some() {
                    let usage = unavailable_usage(installation, account_key, &error);
                    self.cache_usage(&usage);
                    Ok(usage)
                } else {
                    Err(error)
                }
            }
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
            adapter: "ClaudeAdapter".to_string(),
            usage_source: state.last_source.unwrap_or(UsageSource::Cli),
            last_latency_ms: state.last_latency_ms,
            last_success: state.last_success.clone(),
            last_error: state.last_error.clone(),
        })
    }
}

/// npm's Windows shim (`...\npm\claude`) is discoverable with `where`, but
/// cannot be spawned directly by `std::process::Command`. Recent Claude Code
/// packages also ship a native executable, so prefer it when the runtime says
/// it exists. Other installation layouts and runtimes keep their discovered
/// executable unchanged.
fn resolve_claude_executable(installation: &ProviderInstallation, runtime: &dyn Runtime) -> String {
    let discovered = &installation.executable;
    let Some((npm_dir, _)) = discovered.rsplit_once('\\') else {
        return discovered.clone();
    };
    let native = format!(r"{npm_dir}\node_modules\@anthropic-ai\claude-code\bin\claude.exe");

    if runtime.exists(&native).unwrap_or(false) {
        native
    } else {
        discovered.clone()
    }
}

fn normalize_cli_response(
    response_line: &str,
    installation: &ProviderInstallation,
) -> Result<ProviderUsage, AdapterError> {
    let envelope: Value = serde_json::from_str(response_line)
        .map_err(|_| AdapterError::InvalidResponse("Claude CLI returned invalid JSON".into()))?;
    let control = envelope.get("response").ok_or_else(|| {
        AdapterError::InvalidResponse("Claude CLI response had no control payload".into())
    })?;

    if control.get("subtype").and_then(Value::as_str) == Some("error") {
        return Err(AdapterError::Unavailable(
            "Claude CLI rejected the get_usage control request".into(),
        ));
    }

    let usage = control.get("response").unwrap_or(control);
    if usage.get("rate_limits_available").and_then(Value::as_bool) == Some(false) {
        return Err(AdapterError::Unavailable(
            "Claude plan rate limits are not available for this authentication method".into(),
        ));
    }

    let rate_limits = usage.get("rate_limits").ok_or_else(|| {
        AdapterError::Unavailable("Claude CLI returned no plan rate limits".into())
    })?;

    if let Some(limits) = rate_limits.get("limits").and_then(Value::as_array) {
        let session = cli_limit_window(limits, "session");
        let weekly = cli_limit_window(limits, "weekly_all");
        if let Some((percentage, reset_at)) = session.clone().or_else(|| weekly.clone()) {
            let is_session = session.is_some();
            let mut usage = usage_from_window(
                percentage,
                reset_at,
                if is_session {
                    UsagePeriod::Rolling
                } else {
                    UsagePeriod::Weekly
                },
                if is_session { "5h rolling" } else { "weekly" },
                UsageSource::Cli,
                installation,
            )?;
            if is_session {
                usage.weekly = weekly
                    .and_then(|(percentage, reset_at)| weekly_usage_window(percentage, reset_at));
            }
            return Ok(usage);
        }
    }

    let session = rate_limits.get("five_hour").and_then(cli_window_from_value);
    let weekly = rate_limits.get("seven_day").and_then(cli_window_from_value);
    if let Some((percentage, reset_at)) = session.clone().or_else(|| weekly.clone()) {
        let is_session = session.is_some();
        let mut usage = usage_from_window(
            percentage,
            reset_at,
            if is_session {
                UsagePeriod::Rolling
            } else {
                UsagePeriod::Weekly
            },
            if is_session { "5h rolling" } else { "weekly" },
            UsageSource::Cli,
            installation,
        )?;
        if is_session {
            usage.weekly =
                weekly.and_then(|(percentage, reset_at)| weekly_usage_window(percentage, reset_at));
        }
        return Ok(usage);
    }

    Err(AdapterError::Unavailable(
        "Claude CLI returned no active 5-hour or weekly usage window".into(),
    ))
}

fn cli_limit_window(limits: &[Value], kind: &str) -> Option<(f64, Option<String>)> {
    limits
        .iter()
        .find(|limit| {
            limit.get("kind").and_then(Value::as_str) == Some(kind)
                && limit.get("is_active").and_then(Value::as_bool) != Some(false)
        })
        .and_then(cli_window_from_value)
}

fn cli_window_from_value(window: &Value) -> Option<(f64, Option<String>)> {
    let percentage = window
        .get("percent")
        .or_else(|| window.get("utilization"))
        .or_else(|| window.get("used_percentage"))
        .and_then(Value::as_f64)?;
    let reset_at = window
        .get("resets_at")
        .and_then(Value::as_str)
        .map(str::to_string);
    Some((percentage, reset_at))
}

fn weekly_usage_window(percentage: f64, reset_at: Option<String>) -> Option<UsageWindow> {
    (percentage.is_finite() && percentage >= 0.0).then(|| UsageWindow {
        percentage,
        period: UsagePeriod::Weekly,
        period_description: "weekly".to_string(),
        reset_at,
    })
}

fn usage_from_window(
    percentage: f64,
    reset_at: Option<String>,
    period: UsagePeriod,
    period_description: &str,
    source: UsageSource,
    installation: &ProviderInstallation,
) -> Result<ProviderUsage, AdapterError> {
    if !percentage.is_finite() || percentage < 0.0 {
        return Err(AdapterError::InvalidResponse(
            "usage percentage was outside the supported range".into(),
        ));
    }

    Ok(ProviderUsage {
        provider: ProviderId::Claude,
        installation_id: installation.id.clone(),
        runtime_id: installation.runtime_id.clone(),
        account_key: None,
        status: status_for_percentage(percentage),
        percentage: Some(percentage),
        used: None,
        remaining: Some((100.0 - percentage).max(0.0)),
        limit: Some(100.0),
        unit: Some(UsageUnit::Percentage),
        period: Some(period),
        period_description: Some(period_description.to_string()),
        reset_at,
        weekly: None,
        updated_at: chrono::Utc::now().to_rfc3339(),
        source,
        error: None,
    })
}

fn unavailable_usage(
    installation: &ProviderInstallation,
    account_key: Option<String>,
    error: &AdapterError,
) -> ProviderUsage {
    ProviderUsage {
        provider: ProviderId::Claude,
        installation_id: installation.id.clone(),
        runtime_id: installation.runtime_id.clone(),
        account_key,
        status: super::super::usage::UsageStatus::Unavailable,
        percentage: None,
        used: None,
        remaining: None,
        limit: None,
        unit: None,
        period: None,
        period_description: None,
        reset_at: None,
        weekly: None,
        updated_at: chrono::Utc::now().to_rfc3339(),
        source: UsageSource::Cli,
        error: Some(error.to_string()),
    }
}

/// Claude owns the credential store and exposes the non-secret account
/// identity through local account metadata and its status command. The local
/// metadata contains no token and is preferred because it works when the CLI
/// launcher cannot execute (for example, a Windows CLI found through WSL).
/// Missing or partial identity is deliberately treated as unknown: unknown
/// accounts must not be merged.
fn fetch_account_key(installation: &ProviderInstallation, runtime: &dyn Runtime) -> Option<String> {
    account_key_from_local_state(runtime)
        .or_else(|| account_key_from_cli_status(installation, runtime))
}

fn account_key_from_local_state(runtime: &dyn Runtime) -> Option<String> {
    let home = runtime.home_directory().ok()?;
    let separator = if home.contains('\\') { "\\" } else { "/" };
    let path = format!("{home}{separator}.claude.json");
    let state: Value = serde_json::from_str(&runtime.read_file(&path).ok()?).ok()?;
    account_key_from_local_state_value(&state)
}

fn account_key_from_cli_status(
    installation: &ProviderInstallation,
    runtime: &dyn Runtime,
) -> Option<String> {
    let result = runtime
        .execute(
            CommandRequest::new(resolve_claude_executable(installation, runtime))
                .args(["auth", "status", "--json"])
                .timeout(CLI_TIMEOUT),
        )
        .ok()?;

    if result.exit_code != 0 {
        return None;
    }

    let status = result
        .stdout
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str::<Value>(line).ok())?;
    account_key_from_status(&status)
}

fn account_key_from_local_state_value(state: &Value) -> Option<String> {
    let account = state.get("oauthAccount")?;
    let account_id = account.get("accountUuid").and_then(Value::as_str)?.trim();
    let organization = account
        .get("organizationUuid")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("personal");
    let email = account
        .get("emailAddress")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    if account_id.is_empty() {
        return None;
    }

    Some(super::super::identity::fingerprint(
        ProviderId::Claude,
        &[account_id, organization, &email.to_ascii_lowercase()],
    ))
}

fn account_key_from_status(status: &Value) -> Option<String> {
    let account = status.get("account").unwrap_or(&status);
    if status
        .get("loggedIn")
        .or_else(|| account.get("loggedIn"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        return None;
    }

    let email = account
        .get("email")
        .or_else(|| account.get("emailAddress"))
        .or_else(|| status.get("email"))
        .or_else(|| status.get("emailAddress"))
        .and_then(Value::as_str)?
        .trim();
    if email.is_empty() {
        return None;
    }

    let organization = account
        .get("orgId")
        .or_else(|| account.get("organizationId"))
        .or_else(|| status.get("orgId"))
        .or_else(|| status.get("organizationId"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("personal");

    let auth_method = status
        .get("authMethod")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let api_provider = status
        .get("apiProvider")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let email_key = email.to_ascii_lowercase();

    Some(super::super::identity::fingerprint(
        ProviderId::Claude,
        &[auth_method, api_provider, &email_key, organization],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{CommandResult, RuntimeError, RuntimeKind};

    fn installation() -> ProviderInstallation {
        ProviderInstallation {
            id: "claude:test".into(),
            provider: ProviderId::Claude,
            provider_name: "Claude".into(),
            runtime_id: "test".into(),
            runtime_name: "Test".into(),
            executable: "/usr/bin/claude".into(),
        }
    }

    #[test]
    fn normalizes_cli_control_response_without_invoking_a_model() {
        let response = r#"{"type":"control_response","response":{"subtype":"success","response":{"rate_limits_available":true,"rate_limits":{"limits":[{"kind":"session","percent":27.25,"resets_at":"2026-09-18T18:00:00Z","is_active":true},{"kind":"weekly_all","percent":41.0,"resets_at":"2026-09-22T00:00:00Z","is_active":true}]},"total_cost_usd":0,"model_usage":{}}}}"#;

        let usage = normalize_cli_response(response, &installation()).unwrap();

        assert_eq!(usage.percentage, Some(27.25));
        assert_eq!(usage.period, Some(UsagePeriod::Rolling));
        assert_eq!(
            usage.weekly.as_ref().map(|window| window.percentage),
            Some(41.0)
        );
        assert_eq!(usage.source, UsageSource::Cli);
    }

    #[test]
    fn identifies_an_authenticated_personal_account_without_an_organization_id() {
        let account = serde_json::json!({
            "loggedIn": true,
            "emailAddress": "person@example.test",
            "authMethod": "oauth",
            "apiProvider": "anthropic"
        });

        assert_eq!(
            account_key_from_status(&account),
            account_key_from_status(&account)
        );
        assert!(account_key_from_status(&account).is_some());
    }

    #[test]
    fn identifies_the_same_local_account_despite_a_different_cached_plan() {
        let wsl = serde_json::json!({
            "oauthAccount": {
                "accountUuid": "account-1",
                "organizationUuid": "organization-1",
                "emailAddress": "person@example.test",
                "billingType": "pro"
            }
        });
        let windows = serde_json::json!({
            "oauthAccount": {
                "accountUuid": "account-1",
                "organizationUuid": "organization-1",
                "emailAddress": "person@example.test",
                "billingType": "team"
            }
        });

        assert_eq!(
            account_key_from_local_state_value(&wsl),
            account_key_from_local_state_value(&windows)
        );
    }

    #[test]
    fn cli_response_reports_when_plan_limits_do_not_apply() {
        let response = r#"{"type":"control_response","response":{"subtype":"success","response":{"rate_limits_available":false,"rate_limits":null}}}"#;

        let error = normalize_cli_response(response, &installation()).unwrap_err();
        assert!(matches!(error, AdapterError::Unavailable(_)));
    }

    #[test]
    fn resolves_the_native_claude_executable_beside_the_windows_npm_shim() {
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
                Ok(path.ends_with(r"npm\node_modules\@anthropic-ai\claude-code\bin\claude.exe"))
            }
            fn read_file(&self, _path: &str) -> Result<String, RuntimeError> {
                unimplemented!()
            }
            fn home_directory(&self) -> Result<String, RuntimeError> {
                unimplemented!()
            }
        }

        let mut windows_installation = installation();
        windows_installation.executable = r"C:\Users\Ada\AppData\Roaming\npm\claude".into();

        assert_eq!(
            resolve_claude_executable(&windows_installation, &NativeWindowsRuntime),
            r"C:\Users\Ada\AppData\Roaming\npm\node_modules\@anthropic-ai\claude-code\bin\claude.exe"
        );
        assert_eq!(
            resolve_claude_executable(&installation(), &NativeWindowsRuntime),
            "/usr/bin/claude"
        );
    }
}
