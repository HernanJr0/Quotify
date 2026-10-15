use tauri::{AppHandle, Manager};

use crate::providers::{usage::ProviderUsage, AdapterRegistry, ProviderRegistry};
use crate::runtime::RuntimeManager;
use crate::storage::Storage;

use super::provider_commands::ProviderState;

/// Fetches usage for one detected installation through its provider's
/// adapter (docs/PLAN.md section 17). Claude and Codex resolve to real
/// adapters; Gemini and Grok remain mocked.
///
/// Adapters shell out to the provider CLIs and block for seconds. The work
/// runs on the blocking thread pool: a synchronous command would freeze the
/// UI event loop, and blocking the async runtime's few worker threads would
/// stall every other IPC call (window moves, events) during a refresh.
#[tauri::command]
pub async fn fetch_provider_usage(
    installation_id: String,
    force_refresh: bool,
    app: AppHandle,
) -> Result<ProviderUsage, String> {
    tauri::async_runtime::spawn_blocking(move || {
        fetch_usage_blocking(&app, &installation_id, force_refresh)
    })
    .await
    .map_err(|error| format!("usage task failed: {error}"))?
}

fn fetch_usage_blocking(
    app: &AppHandle,
    installation_id: &str,
    force_refresh: bool,
) -> Result<ProviderUsage, String> {
    let provider_state = app.state::<ProviderState>();
    let adapters = app.state::<AdapterRegistry>();
    let runtimes = app.state::<RuntimeManager>();
    // Storage needs the app data directory, so it is only registered by the
    // setup hook. Treat it as optional rather than failing a request that
    // races app startup.
    let storage = app.try_state::<Storage>();
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
            if let (None, Some(storage)) = (&usage.error, &storage) {
                let _ = storage.save_valid_usage(&usage);
            }
            Ok(usage)
        }
        Err(error) => {
            if let Some(Ok(Some(mut cached))) = storage
                .as_ref()
                .map(|storage| storage.last_valid_usage(&installation.id))
            {
                cached.error = Some(error.to_string());
                return Ok(cached);
            }
            Err(error.to_string())
        }
    }
}
