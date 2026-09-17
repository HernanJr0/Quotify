use super::{command, CommandRequest, CommandResult, Runtime, RuntimeError, RuntimeKind};

/// GUI-launched macOS apps often inherit a minimal PATH that skips
/// Homebrew, so `which` falls back to its two conventional install
/// locations (docs/PLAN.md section 10).
const HOMEBREW_BIN_DIRS: &[&str] = &["/opt/homebrew/bin", "/usr/local/bin"];

pub struct MacRuntime;

impl MacRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl Runtime for MacRuntime {
    fn id(&self) -> &str {
        "macos"
    }

    fn kind(&self) -> RuntimeKind {
        RuntimeKind::Macos
    }

    fn name(&self) -> &str {
        "macOS"
    }

    fn execute(&self, request: CommandRequest) -> Result<CommandResult, RuntimeError> {
        command::run(
            &request.executable,
            &request.args,
            &request.env,
            request.timeout,
        )
    }

    fn which(&self, binary: &str) -> Result<Option<String>, RuntimeError> {
        if let Some(found) = command::which("which", binary)? {
            return Ok(Some(found));
        }

        for dir in HOMEBREW_BIN_DIRS {
            let candidate = std::path::Path::new(dir).join(binary);
            if candidate.exists() {
                return Ok(Some(candidate.to_string_lossy().into_owned()));
            }
        }

        Ok(None)
    }

    fn exists(&self, path: &str) -> Result<bool, RuntimeError> {
        Ok(std::path::Path::new(path).exists())
    }

    fn read_file(&self, path: &str) -> Result<String, RuntimeError> {
        std::fs::read_to_string(path).map_err(RuntimeError::Io)
    }

    fn home_directory(&self) -> Result<String, RuntimeError> {
        std::env::var("HOME").map_err(|_| {
            RuntimeError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "HOME is not set",
            ))
        })
    }
}
