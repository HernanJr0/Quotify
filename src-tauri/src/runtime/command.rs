use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use super::{CommandResult, RuntimeError};

/// Default collection timeout per docs/PLAN.md section 28.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Runs `executable` directly with `args` — never through a shell, so
/// there's no quoting/escaping/injection surface (docs/PLAN.md section 13).
///
/// stdout/stderr are drained on background threads while we poll for exit,
/// so a chatty child process can't deadlock the timeout loop by filling its
/// pipe buffer before we get around to reading it.
pub fn run(
    executable: &str,
    args: &[String],
    env: &[(String, String)],
    timeout: Option<Duration>,
) -> Result<CommandResult, RuntimeError> {
    let start = Instant::now();
    let mut command = Command::new(executable);
    command
        .args(args)
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Windows spawns a brand new console window for every child console
    // process by default when the parent (this GUI app) has none of its
    // own — CREATE_NO_WINDOW suppresses that. Without this, every `where`/
    // `wsl.exe` invocation flashes a visible terminal window.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command
        .spawn()
        .map_err(RuntimeError::Spawn)?;

    let mut stdout_pipe = child.stdout.take().expect("stdout was piped");
    let mut stderr_pipe = child.stderr.take().expect("stderr was piped");

    let (stdout_tx, stdout_rx) = mpsc::channel();
    let stdout_thread = thread::spawn(move || {
        let mut buf = String::new();
        let _ = stdout_pipe.read_to_string(&mut buf);
        let _ = stdout_tx.send(buf);
    });

    let (stderr_tx, stderr_rx) = mpsc::channel();
    let stderr_thread = thread::spawn(move || {
        let mut buf = String::new();
        let _ = stderr_pipe.read_to_string(&mut buf);
        let _ = stderr_tx.send(buf);
    });

    let timeout = timeout.unwrap_or(DEFAULT_TIMEOUT);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(RuntimeError::Io)? {
            break status;
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            return Err(RuntimeError::Timeout(timeout));
        }
        thread::sleep(Duration::from_millis(20));
    };

    let stdout = stdout_thread
        .join()
        .ok()
        .and_then(|_| stdout_rx.recv().ok())
        .unwrap_or_default();
    let stderr = stderr_thread
        .join()
        .ok()
        .and_then(|_| stderr_rx.recv().ok())
        .unwrap_or_default();

    Ok(CommandResult {
        exit_code: status.code().unwrap_or(-1),
        stdout,
        stderr,
        duration: start.elapsed(),
    })
}

/// Resolves a binary via the OS's own resolver (`where` on Windows, `which`
/// elsewhere) instead of walking PATH ourselves, so we automatically honor
/// whatever a user's package manager set up (docs/PLAN.md section 14).
pub fn which(resolver: &str, binary: &str) -> Result<Option<String>, RuntimeError> {
    match run(
        resolver,
        &[binary.to_string()],
        &[],
        Some(Duration::from_secs(5)),
    ) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_a_real_command() {
        let result = run("hostname", &[], &[], None).expect("hostname should run");
        assert_eq!(result.exit_code, 0);
        assert!(!result.stdout.trim().is_empty());
    }

    #[test]
    fn missing_binary_returns_spawn_error() {
        let err = run("quotify-definitely-not-a-real-binary-xyz", &[], &[], None)
            .expect_err("missing binary should fail to spawn");
        assert!(matches!(err, RuntimeError::Spawn(_)));
    }

    #[test]
    fn timeout_is_enforced() {
        #[cfg(windows)]
        let (bin, args) = ("ping", vec!["-n".to_string(), "6".to_string(), "127.0.0.1".to_string()]);
        #[cfg(not(windows))]
        let (bin, args) = ("sleep", vec!["5".to_string()]);

        let err = run(bin, &args, &[], Some(Duration::from_millis(300)))
            .expect_err("long-running command should time out");
        assert!(matches!(err, RuntimeError::Timeout(_)));
    }

    #[test]
    fn which_resolves_a_real_binary() {
        #[cfg(windows)]
        let resolver = "where";
        #[cfg(not(windows))]
        let resolver = "which";

        let found = which(resolver, "hostname").expect("which should not error");
        assert!(found.is_some());
    }

    #[test]
    fn which_returns_none_for_missing_binary() {
        #[cfg(windows)]
        let resolver = "where";
        #[cfg(not(windows))]
        let resolver = "which";

        let found = which(resolver, "quotify-definitely-not-a-real-binary-xyz")
            .expect("which should not error");
        assert!(found.is_none());
    }
}
