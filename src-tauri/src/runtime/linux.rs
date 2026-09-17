use super::{command, CommandRequest, CommandResult, Runtime, RuntimeError, RuntimeKind};

pub struct LinuxRuntime;

impl LinuxRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl Runtime for LinuxRuntime {
    fn id(&self) -> &str {
        "linux"
    }

    fn kind(&self) -> RuntimeKind {
        RuntimeKind::Linux
    }

    fn name(&self) -> &str {
        "Linux"
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
        command::which("which", binary)
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
