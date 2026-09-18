use tauri::State;

use crate::providers::{usage::ProviderUsage, AdapterRegistry};
use crate::runtime::RuntimeManager;

use super::provider_commands::ProviderState;

/// Fetches usage for one detected installation through its provider's
/// adapter (docs/PLAN.md section 17). Claude and Codex resolve to real
/// adapters; Gemini and Grok remain mocked.
#[tauri::command]
pub fn fetch_provider_usage(
    installation_id: String,
    provider_state: State<ProviderState>,
    adapters: State<AdapterRegistry>,
    runtimes: State<RuntimeManager>,
) -> Result<ProviderUsage, String> {
    let installation = provider_state
        .0
        .iter()
        .find(|installation| installation.id == installation_id)
        .ok_or_else(|| format!("unknown installation: {installation_id}"))?;

    let adapter = adapters
        .get(installation.provider)
        .ok_or_else(|| format!("no adapter registered for {:?}", installation.provider))?;

    let runtime = runtimes.get(&installation.runtime_id).ok_or_else(|| {
        format!(
            "runtime is no longer available: {}",
            installation.runtime_id
        )
    })?;

    adapter
        .fetch_usage(installation, runtime)
        .map_err(|err| err.to_string())
}
