use crate::providers::{ProviderInstallation, ProviderRegistry};
use crate::runtime::RuntimeManager;

/// Startup snapshot retained for consumers that need a stable installation
/// while a usage request is in flight. The list command below performs a
/// fresh scan so the UI can observe installs/removals without restarting.
pub struct ProviderState(pub Vec<ProviderInstallation>);

/// Runs on the blocking thread pool so a slow WSL scan never holds the UI
/// event loop or the async runtime's worker threads.
#[tauri::command]
pub async fn list_providers() -> Result<Vec<ProviderInstallation>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        // Re-scan on demand so a provider installed after startup appears
        // without restarting the app. ProviderRegistry isolates failures per
        // runtime and RuntimeManager's optional WSL scan is timeout-bounded.
        let runtimes = RuntimeManager::discover();
        ProviderRegistry::discover(&runtimes)
    })
    .await
    .map_err(|error| format!("provider discovery failed: {error}"))
}
