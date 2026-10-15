use crate::runtime::{RuntimeInfo, RuntimeManager};

/// Lists every runtime detected on this machine (docs/PLAN.md Fase 3).
///
/// Runs on the blocking thread pool so a slow WSL scan never holds the UI
/// event loop or the async runtime's worker threads.
#[tauri::command]
pub async fn list_runtimes() -> Result<Vec<RuntimeInfo>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        // Discovery is intentionally repeated for this read command. WSL
        // distros can be installed or removed while Quotify is running, and
        // the scanner is bounded by the runtime command timeout.
        RuntimeManager::discover()
            .runtimes()
            .iter()
            .map(|runtime| RuntimeInfo::from(runtime.as_ref()))
            .collect()
    })
    .await
    .map_err(|error| format!("runtime discovery failed: {error}"))
}
