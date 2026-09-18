mod adapter;
mod adapter_registry;
mod claude;
mod codex;
mod identity;
mod registry;
pub mod usage;

pub use adapter::UsageProviderAdapter;
pub use adapter_registry::AdapterRegistry;
pub use registry::ProviderRegistry;

/// Mirrors the conceptual `ProviderId` in docs/PLAN.md section 19.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Claude,
    Codex,
    Gemini,
    Grok,
}

impl ProviderId {
    pub const ALL: [ProviderId; 4] = [Self::Claude, Self::Codex, Self::Gemini, Self::Grok];

    /// The executable name each provider's CLI is expected to be on PATH
    /// as, used with `Runtime::which` (docs/PLAN.md section 14).
    pub fn binary_name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
            Self::Grok => "grok",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
            Self::Gemini => "Gemini",
            Self::Grok => "Grok",
        }
    }
}

/// Mirrors the conceptual `ProviderInstallation` in docs/PLAN.md section 15.
/// `version` is intentionally omitted for now — resolving it reliably is
/// provider-specific logic that belongs to the Fase 7 adapter framework,
/// not to detection.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInstallation {
    pub id: String,
    pub provider: ProviderId,
    pub provider_name: String,
    pub runtime_id: String,
    pub runtime_name: String,
    pub executable: String,
}
