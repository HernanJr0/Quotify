use tauri::State;

use crate::providers::{ProviderInstallation, ProviderRegistry};
use crate::runtime::RuntimeManager;

/// Startup snapshot retained for consumers that need a stable installation
/// while a usage request is in flight. The list command below performs a
/// fresh scan so the UI can observe installs/removals without restarting.
pub struct ProviderState(pub Vec<ProviderInstallation>);

#[tauri::command]
pub fn list_providers(_state: State<ProviderState>) -> Vec<ProviderInstallation> {
    // Re-scan on demand so a provider installed after startup appears without
    // restarting the app. ProviderRegistry isolates failures per runtime and
    // RuntimeManager's optional WSL scan is timeout-bounded.
    let runtimes = RuntimeManager::discover();
    ProviderRegistry::discover(&runtimes)
}
