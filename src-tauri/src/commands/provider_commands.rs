use tauri::State;

use crate::providers::{ProviderInstallation, ProviderRegistry};
use crate::runtime::RuntimeManager;

/// Lists every provider installation detected on this machine
/// (docs/PLAN.md Fase 4).
#[tauri::command]
pub fn list_providers(runtime_manager: State<RuntimeManager>) -> Vec<ProviderInstallation> {
    ProviderRegistry::discover(&runtime_manager)
}
