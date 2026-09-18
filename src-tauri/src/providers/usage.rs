// Fase 7 defines the full data model ahead of the real adapters (Fase 8-9)
// and the Diagnostics screen (Fase 12) that will actually construct most of
// these variants and `ProviderDiagnostics`. Remove once every field/variant
// below has a real caller.
#![allow(dead_code)]

use super::ProviderId;

/// Mirrors the conceptual `UsageStatus` in docs/PLAN.md section 19.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageStatus {
    Ok,
    Warning,
    Critical,
    Unknown,
    Unavailable,
    Error,
}

/// Mirrors the conceptual `UsageUnit` in docs/PLAN.md section 19.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageUnit {
    Percentage,
    Tokens,
    Requests,
    Credits,
    Messages,
    Unknown,
}

/// Mirrors the conceptual `UsagePeriod` in docs/PLAN.md section 19.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UsagePeriod {
    Hourly,
    Rolling,
    Daily,
    Weekly,
    Monthly,
    Unknown,
}

/// The four collection strategies from docs/PLAN.md section 22, in
/// preference order, plus `Mock` for the Fase 7 `MockAdapter` — which has
/// no real source since it exists only to validate the adapter contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UsageSource {
    Mock,
    OfficialApi,
    Cli,
    LocalState,
    InternalEndpoint,
}

/// Mirrors the conceptual `ProviderUsage` in docs/PLAN.md section 19.
///
/// Fields are deliberately optional beyond `status`/`updatedAt`/`source`:
/// section 20 forbids inventing a `percentage` when a provider can't
/// actually produce one (e.g. Grok's "23 credits remaining").
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsage {
    pub provider: ProviderId,
    pub installation_id: String,
    pub runtime_id: String,
    /// Opaque, local comparison key. It is present only when the provider
    /// confirmed a stable account/authentication identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_key: Option<String>,
    pub status: UsageStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<UsageUnit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<UsagePeriod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_at: Option<String>,
    pub updated_at: String,
    pub source: UsageSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Mirrors the fields unique to the Diagnostics screen in docs/PLAN.md
/// section 36 that aren't already on `ProviderInstallation` (provider,
/// runtime, executable, version). Not consumed anywhere yet — the
/// Diagnostics screen itself is Fase 12 — but the adapter contract (section
/// 17) needs a concrete return type today.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDiagnostics {
    pub adapter: String,
    pub usage_source: UsageSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_success: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}
