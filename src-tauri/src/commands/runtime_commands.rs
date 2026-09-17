use tauri::State;

use crate::runtime::{RuntimeInfo, RuntimeManager};

/// Lists every runtime detected on this machine (docs/PLAN.md Fase 3).
#[tauri::command]
pub fn list_runtimes(runtime_manager: State<RuntimeManager>) -> Vec<RuntimeInfo> {
    runtime_manager
        .runtimes()
        .iter()
        .map(|runtime| RuntimeInfo::from(runtime.as_ref()))
        .collect()
}
