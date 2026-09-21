use tauri::State;

use crate::providers::{usage::ProviderUsage, AdapterRegistry, ProviderRegistry};
use crate::runtime::RuntimeManager;
use crate::storage::Storage;

use super::provider_commands::ProviderState;

/// Fetches usage for one detected installation through its provider's
/// adapter (docs/PLAN.md section 17). Claude and Codex resolve to real
/// adapters; Gemini and Grok remain mocked.
#[tauri::command]
pub fn fetch_provider_usage(
    installation_id: String,
    force_refresh: bool,
    provider_state: State<ProviderState>,
    adapters: State<AdapterRegistry>,
    runtimes: State<RuntimeManager>,
    storage: State<Storage>,
) -> Result<ProviderUsage, String> {
    let installation = provider_state
        .0
        .iter()
        .find(|installation| installation.id == installation_id)
        .cloned()
        .or_else(|| {
            RuntimeManager::discover()
                .runtimes()
                .iter()
                .find_map(|runtime| {
                    ProviderRegistry::discover_in_runtime(runtime.as_ref())
                        .into_iter()
                        .find(|installation| installation.id == installation_id)
                })
        })
        .ok_or_else(|| format!("unknown installation: {installation_id}"))?;

    let adapter = adapters
        .get(installation.provider)
        .ok_or_else(|| format!("no adapter registered for {:?}", installation.provider))?;

    let fallback_runtimes;
    let runtime = if let Some(runtime) = runtimes.get(&installation.runtime_id) {
        runtime
    } else {
        fallback_runtimes = RuntimeManager::discover();
        fallback_runtimes
            .get(&installation.runtime_id)
            .ok_or_else(|| {
                format!(
                    "runtime is no longer available: {}",
                    installation.runtime_id
                )
            })?
    };

    match adapter.fetch_usage(&installation, runtime, force_refresh) {
        Ok(usage) => {
            if usage.error.is_none() {
                let _ = storage.save_valid_usage(&usage);
            }
            Ok(usage)
        }
        Err(error) => {
            if let Ok(Some(mut cached)) = storage.last_valid_usage(&installation.id) {
                cached.error = Some(error.to_string());
                return Ok(cached);
            }
            Err(error.to_string())
        }
    }
}
