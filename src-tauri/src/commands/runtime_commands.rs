use tauri::State;

use crate::runtime::{RuntimeInfo, RuntimeManager};

/// Lists every runtime detected on this machine (docs/PLAN.md Fase 3).
#[tauri::command]
pub fn list_runtimes(_runtime_manager: State<RuntimeManager>) -> Vec<RuntimeInfo> {
    // Discovery is intentionally repeated for this read command. WSL distros
    // can be installed or removed while Quotify is running, and the scanner
    // is bounded by the runtime command timeout. Keep the managed state in
    // the signature for API compatibility with existing Tauri wiring.
    RuntimeManager::discover()
        .runtimes()
        .iter()
        .map(|runtime| RuntimeInfo::from(runtime.as_ref()))
        .collect()
}
