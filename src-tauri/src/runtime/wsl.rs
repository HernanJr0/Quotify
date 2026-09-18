use std::time::Duration;

use super::{command, CommandRequest, CommandResult, Runtime, RuntimeError, RuntimeKind};

/// WSL distro names that exist for Docker Desktop's own integration rather
/// than interactive use, so they aren't useful "environments" to surface.
const IGNORED_DISTROS: &[&str] = &["docker-desktop", "docker-desktop-data"];

const WSL_TIMEOUT: Duration = Duration::from_secs(5);

pub struct WslRuntime {
    id: String,
    name: String,
    distro: String,
}

impl WslRuntime {
    pub fn new(distro: impl Into<String>) -> Self {
        let distro = distro.into();
        Self {
            id: format!("wsl:{distro}"),
            name: format!("WSL {distro}"),
            distro,
        }
    }
}

impl Runtime for WslRuntime {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> RuntimeKind {
        RuntimeKind::Wsl
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&self, request: CommandRequest) -> Result<CommandResult, RuntimeError> {
        let mut args = vec![
            "-d".to_string(),
            self.distro.clone(),
            "--".to_string(),
            request.executable,
        ];
        args.extend(request.args);
        command::run(
            "wsl.exe",
            &args,
            &request.env,
            request.stdin.as_deref(),
            request.stdout_marker.as_deref(),
            request.timeout,
        )
    }

    fn which(&self, binary: &str) -> Result<Option<String>, RuntimeError> {
        let args = self.distro_args(&["which", binary]);
        match command::run("wsl.exe", &args, &[], None, None, Some(WSL_TIMEOUT)) {
            Ok(result) if result.exit_code == 0 => Ok(result
                .stdout
                .lines()
                .next()
                .map(|line| line.trim().to_string())
                .filter(|s| !s.is_empty())),
            Ok(_) => Ok(None),
            Err(RuntimeError::Spawn(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn exists(&self, path: &str) -> Result<bool, RuntimeError> {
        let args = self.distro_args(&["test", "-e", path]);
        let result = command::run("wsl.exe", &args, &[], None, None, Some(WSL_TIMEOUT))?;
        Ok(result.exit_code == 0)
    }

    fn read_file(&self, path: &str) -> Result<String, RuntimeError> {
        let args = self.distro_args(&["cat", path]);
        let result = command::run("wsl.exe", &args, &[], None, None, Some(WSL_TIMEOUT))?;
        if result.exit_code != 0 {
            return Err(RuntimeError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{path} not found in WSL {}", self.distro),
            )));
        }
        Ok(result.stdout)
    }

    fn home_directory(&self) -> Result<String, RuntimeError> {
        let args = self.distro_args(&["printenv", "HOME"]);
        let result = command::run("wsl.exe", &args, &[], None, None, Some(WSL_TIMEOUT))?;
        Ok(result.stdout.trim().to_string())
    }
}

impl WslRuntime {
    /// Prefixes a command with `-d <distro> --`, matching the invocation
    /// shape from docs/PLAN.md section 10 (`wsl.exe -d Ubuntu-24.04 -- ...`).
    fn distro_args(&self, command: &[&str]) -> Vec<String> {
        let mut args = vec!["-d".to_string(), self.distro.clone(), "--".to_string()];
        args.extend(command.iter().map(|s| s.to_string()));
        args
    }
}

/// Discovers WSL distros via `wsl.exe --list --quiet` (docs/PLAN.md section
/// 11). Returns an empty list on any failure (e.g. WSL not installed, or not
/// running on Windows at all) rather than erroring — WSL is optional, and an
/// unavailable or broken distro must never take down discovery.
pub fn discover() -> Vec<WslRuntime> {
    let mut command = std::process::Command::new("wsl.exe");
    command.args(["--list", "--quiet"]);

    // Suppress the console window Windows would otherwise pop up for this
    // child process — see the matching comment in runtime::command::run.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = match command.output() {
        Ok(output) if output.status.success() => output,
        _ => return Vec::new(),
    };

    parse_distro_list(&output.stdout)
        .into_iter()
        .map(WslRuntime::new)
        .collect()
}

/// `wsl.exe` prints its list as UTF-16LE, which shows up as a null byte
/// after every ASCII character when read as UTF-8 — strip those instead of
/// doing a full UTF-16 decode, since distro names are ASCII in practice.
fn parse_distro_list(raw: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(raw)
        .replace('\0', "")
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|name| !name.is_empty())
        .filter(|name| !IGNORED_DISTROS.contains(&name.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16le(s: &str) -> Vec<u8> {
        s.encode_utf16().flat_map(|c| c.to_le_bytes()).collect()
    }

    #[test]
    fn parses_utf16_distro_list() {
        let raw = utf16le("Ubuntu\r\nDebian\r\n");
        assert_eq!(parse_distro_list(&raw), vec!["Ubuntu", "Debian"]);
    }

    #[test]
    fn filters_docker_desktop_distros() {
        let raw = utf16le("Ubuntu\r\ndocker-desktop\r\ndocker-desktop-data\r\n");
        assert_eq!(parse_distro_list(&raw), vec!["Ubuntu"]);
    }

    #[test]
    fn ignores_blank_lines() {
        let raw = utf16le("Ubuntu\r\n\r\n\r\n");
        assert_eq!(parse_distro_list(&raw), vec!["Ubuntu"]);
    }

    #[test]
    fn wsl_runtime_id_and_name_include_distro() {
        let runtime = WslRuntime::new("Ubuntu");
        assert_eq!(runtime.id(), "wsl:Ubuntu");
        assert_eq!(runtime.name(), "WSL Ubuntu");
        assert_eq!(runtime.kind(), RuntimeKind::Wsl);
    }
}
