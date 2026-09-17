use tauri::State;

use crate::providers::ProviderInstallation;

/// Providers detected once at startup (docs/PLAN.md Fase 4). Cached instead
/// of re-scanned per request, since each check against a WSL runtime spawns
/// a `wsl.exe` process — slow, and pointless to repeat on every UI open.
pub struct ProviderState(pub Vec<ProviderInstallation>);

#[tauri::command]
pub fn list_providers(state: State<ProviderState>) -> Vec<ProviderInstallation> {
    state.0.clone()
}
