// `AdapterRegistry` only calls `fetch_usage` today. `provider`/`detect` gain
// callers once `ProviderRegistry` is rebuilt on top of adapters and
// `diagnostics` once the Diagnostics screen exists (Fase 8-9 and Fase 12,
// docs/PLAN.md). Remove once every trait method has a real caller.
#![allow(dead_code)]

use crate::runtime::{Runtime, RuntimeError};

use super::usage::{ProviderDiagnostics, ProviderUsage, UsageSource, UsageStatus};
use super::{ProviderId, ProviderInstallation};

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("no usage data available: {0}")]
    Unavailable(String),
    #[error("provider authentication unavailable: {0}")]
    Authentication(String),
    #[error("provider rejected the local authentication; sign in again with the provider CLI")]
    Unauthorized,
    #[error("provider usage endpoint is rate limited; try again later")]
    RateLimited,
    #[error("provider usage request timed out")]
    Timeout,
    #[error("provider usage request failed: {0}")]
    Network(String),
    #[error("provider returned an unsupported usage response: {0}")]
    InvalidResponse(String),
}

/// Mirrors the conceptual `UsageProviderAdapter` in docs/PLAN.md section 17.
///
/// An adapter knows its own provider's quirks (e.g. Claude) but must stay
/// ignorant of Windows/Linux/WSL/macOS specifics — those belong entirely to
/// `Runtime` (section 55).
pub trait UsageProviderAdapter: Send + Sync {
    fn provider(&self) -> ProviderId;

    /// Default mirrors what `ProviderRegistry` already does per provider
    /// (`runtime.which(binary_name)`) — real adapters only need to override
    /// this once detection needs more than a PATH lookup (e.g. version
    /// resolution deferred from `ProviderInstallation`, see providers/mod.rs).
    fn detect(&self, runtime: &dyn Runtime) -> Result<Option<ProviderInstallation>, RuntimeError> {
        let provider = self.provider();
        Ok(runtime
            .which(provider.binary_name())?
            .map(|executable| ProviderInstallation {
                id: format!("{}:{}", provider.binary_name(), runtime.id()),
                provider,
                provider_name: provider.display_name().to_string(),
                runtime_id: runtime.id().to_string(),
                runtime_name: runtime.name().to_string(),
                executable,
            }))
    }

    fn fetch_usage(
        &self,
        installation: &ProviderInstallation,
        runtime: &dyn Runtime,
    ) -> Result<ProviderUsage, AdapterError>;

    fn diagnostics(
        &self,
        installation: &ProviderInstallation,
        runtime: &dyn Runtime,
    ) -> Result<ProviderDiagnostics, AdapterError>;
}

pub(super) fn status_for_percentage(percentage: f64) -> UsageStatus {
    if percentage >= 95.0 {
        UsageStatus::Critical
    } else if percentage >= 70.0 {
        UsageStatus::Warning
    } else {
        UsageStatus::Ok
    }
}

/// First adapter built against the contract above, to validate the
/// architecture before Fase 8 wires up a real provider (docs/PLAN.md Fase
/// 7). Returns fixed numbers per provider — the same illustrative values
/// the frontend previously hardcoded in `ProviderCard` — always tagged
/// `UsageSource::Mock` so callers never mistake it for real data.
pub struct MockAdapter {
    provider: ProviderId,
}

impl MockAdapter {
    pub fn new(provider: ProviderId) -> Self {
        Self { provider }
    }

    fn mock_values(&self) -> (f64, &'static str, &'static str) {
        match self.provider {
            ProviderId::Claude => (71.0, "5h rolling", "14:20"),
            ProviderId::Codex => (43.0, "weekly", "Monday"),
            ProviderId::Gemini => (52.0, "daily", "00:00"),
            ProviderId::Grok => (82.0, "daily", "Monday"),
        }
    }
}

impl UsageProviderAdapter for MockAdapter {
    fn provider(&self) -> ProviderId {
        self.provider
    }

    fn fetch_usage(
        &self,
        installation: &ProviderInstallation,
        _runtime: &dyn Runtime,
    ) -> Result<ProviderUsage, AdapterError> {
        use super::usage::UsagePeriod;

        let (percentage, period_description, reset_at) = self.mock_values();
        let period = match self.provider {
            ProviderId::Claude => UsagePeriod::Rolling,
            ProviderId::Codex => UsagePeriod::Weekly,
            ProviderId::Gemini | ProviderId::Grok => UsagePeriod::Daily,
        };

        Ok(ProviderUsage {
            provider: self.provider,
            installation_id: installation.id.clone(),
            runtime_id: installation.runtime_id.clone(),
            status: status_for_percentage(percentage),
            percentage: Some(percentage),
            used: None,
            remaining: None,
            limit: None,
            unit: Some(super::usage::UsageUnit::Percentage),
            period: Some(period),
            period_description: Some(period_description.to_string()),
            reset_at: Some(reset_at.to_string()),
            updated_at: chrono::Utc::now().to_rfc3339(),
            source: UsageSource::Mock,
            error: None,
        })
    }

    fn diagnostics(
        &self,
        _installation: &ProviderInstallation,
        _runtime: &dyn Runtime,
    ) -> Result<ProviderDiagnostics, AdapterError> {
        Ok(ProviderDiagnostics {
            adapter: "MockAdapter".to_string(),
            usage_source: UsageSource::Mock,
            last_latency_ms: Some(0),
            last_success: Some(chrono::Utc::now().to_rfc3339()),
            last_error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{CommandRequest, CommandResult, RuntimeKind};

    struct FakeRuntime {
        id: String,
        available: Vec<&'static str>,
    }

    impl Runtime for FakeRuntime {
        fn id(&self) -> &str {
            &self.id
        }
        fn kind(&self) -> RuntimeKind {
            RuntimeKind::Linux
        }
        fn name(&self) -> &str {
            &self.id
        }
        fn execute(&self, _request: CommandRequest) -> Result<CommandResult, RuntimeError> {
            unimplemented!("not exercised by these tests")
        }
        fn which(&self, binary: &str) -> Result<Option<String>, RuntimeError> {
            Ok(self
                .available
                .contains(&binary)
                .then(|| format!("/usr/bin/{binary}")))
        }
        fn exists(&self, _path: &str) -> Result<bool, RuntimeError> {
            unimplemented!("not exercised by these tests")
        }
        fn read_file(&self, _path: &str) -> Result<String, RuntimeError> {
            unimplemented!("not exercised by these tests")
        }
        fn home_directory(&self) -> Result<String, RuntimeError> {
            unimplemented!("not exercised by these tests")
        }
    }

    #[test]
    fn default_detect_finds_installation_when_binary_present() {
        let adapter = MockAdapter::new(ProviderId::Claude);
        let runtime = FakeRuntime {
            id: "fake".to_string(),
            available: vec!["claude"],
        };

        let installation = adapter.detect(&runtime).unwrap().unwrap();
        assert_eq!(installation.provider, ProviderId::Claude);
        assert_eq!(installation.runtime_id, "fake");
        assert_eq!(installation.executable, "/usr/bin/claude");
    }

    #[test]
    fn default_detect_returns_none_when_binary_missing() {
        let adapter = MockAdapter::new(ProviderId::Grok);
        let runtime = FakeRuntime {
            id: "fake".to_string(),
            available: vec!["claude"],
        };

        assert!(adapter.detect(&runtime).unwrap().is_none());
    }

    #[test]
    fn fetch_usage_always_tags_source_mock() {
        let adapter = MockAdapter::new(ProviderId::Codex);
        let installation = ProviderInstallation {
            id: "codex:fake".to_string(),
            provider: ProviderId::Codex,
            provider_name: "Codex".to_string(),
            runtime_id: "fake".to_string(),
            runtime_name: "Fake".to_string(),
            executable: "/usr/bin/codex".to_string(),
        };

        let runtime = FakeRuntime {
            id: "fake".to_string(),
            available: vec!["codex"],
        };
        let usage = adapter.fetch_usage(&installation, &runtime).unwrap();
        assert_eq!(usage.source, UsageSource::Mock);
        assert_eq!(usage.percentage, Some(43.0));
        assert_eq!(usage.status, UsageStatus::Ok);
    }

    #[test]
    fn status_reflects_percentage_thresholds() {
        assert_eq!(status_for_percentage(50.0), UsageStatus::Ok);
        assert_eq!(status_for_percentage(71.0), UsageStatus::Warning);
        assert_eq!(status_for_percentage(96.0), UsageStatus::Critical);
    }
}
