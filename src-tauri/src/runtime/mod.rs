// Fase 2 builds this layer ahead of Fase 3 (runtime discovery UI) and
// Fase 4 (provider detection), which are what actually call into it.
// Remove once every method below has a real caller.
#![allow(dead_code)]

mod command;
mod manager;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
mod wsl;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

pub use manager::RuntimeManager;

use std::time::Duration;

/// Mirrors the conceptual `Runtime.type` in docs/PLAN.md section 9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeKind {
    Windows,
    Linux,
    Macos,
    Wsl,
}

/// Serializable summary of a `Runtime`, for exposing discovery results to
/// the frontend (docs/PLAN.md Fase 3 — "Mostrar ambientes na UI").
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub id: String,
    pub kind: RuntimeKind,
    pub name: String,
}

impl From<&dyn Runtime> for RuntimeInfo {
    fn from(runtime: &dyn Runtime) -> Self {
        Self {
            id: runtime.id().to_string(),
            kind: runtime.kind(),
            name: runtime.name().to_string(),
        }
    }
}

/// Mirrors the conceptual `CommandRequest` in docs/PLAN.md section 12.
#[derive(Debug, Clone, Default)]
pub struct CommandRequest {
    pub executable: String,
    pub args: Vec<String>,
    pub timeout: Option<Duration>,
    pub env: Vec<(String, String)>,
}

impl CommandRequest {
    pub fn new(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
            ..Default::default()
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// Mirrors the conceptual `CommandResult` in docs/PLAN.md section 12.
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("command timed out after {0:?}")]
    Timeout(Duration),
    #[error("failed to spawn command: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("io error: {0}")]
    Io(#[source] std::io::Error),
}

/// Mirrors the conceptual `Runtime` interface in docs/PLAN.md section 9.
///
/// Providers must never execute OS commands directly and must never branch
/// on `if windows / else linux / else wsl` themselves (section 55) — that
/// decision belongs entirely to the concrete implementations below.
pub trait Runtime: Send + Sync {
    fn id(&self) -> &str;
    fn kind(&self) -> RuntimeKind;
    fn name(&self) -> &str;

    fn execute(&self, request: CommandRequest) -> Result<CommandResult, RuntimeError>;
    fn which(&self, binary: &str) -> Result<Option<String>, RuntimeError>;
    fn exists(&self, path: &str) -> Result<bool, RuntimeError>;
    fn read_file(&self, path: &str) -> Result<String, RuntimeError>;
    fn home_directory(&self) -> Result<String, RuntimeError>;
}
