use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use super::{CommandResult, RuntimeError};

/// Default collection timeout per docs/PLAN.md section 28.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(unix)]
const SIGKILL: i32 = 9;

#[cfg(unix)]
unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

/// Stops a command and waits for its process tree to be reaped.
///
/// On Unix, commands are started in their own process group so a launcher
/// (for example `sh`, node, or a provider CLI) cannot leave descendants
/// holding stdout/stderr open after the parent is killed. Windows does not
/// expose an equivalent process-group operation in `std`, so the direct
/// child is still killed and waited for there.
fn terminate_child(child: &mut Child) -> std::io::Result<ExitStatus> {
    #[cfg(unix)]
    {
        let pid = child.id();
        if pid <= i32::MAX as u32 {
            // SAFETY: `pid` is the process group leader created below and the
            // signal is the non-catchable termination signal. A failed group
            // kill is followed by the direct-child fallback below.
            let _ = unsafe { kill(-(pid as i32), SIGKILL) };
        }
    }

    let _ = child.kill();
    child.wait()
}

fn join_output_threads(
    stdout_thread: thread::JoinHandle<()>,
    stdout_rx: mpsc::Receiver<String>,
    stderr_thread: thread::JoinHandle<()>,
    stderr_rx: mpsc::Receiver<String>,
) -> (String, String) {
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    (
        stdout_rx.recv().unwrap_or_default(),
        stderr_rx.recv().unwrap_or_default(),
    )
}

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
    stdin: Option<&str>,
    stdout_marker: Option<&str>,
    timeout: Option<Duration>,
) -> Result<CommandResult, RuntimeError> {
    let start = Instant::now();
    let mut command = Command::new(executable);
    command
        .args(args)
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Keep the command and any provider-launched descendants in one group so
    // timeout/marker cleanup does not leave a child holding our output pipes.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

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

    let mut child = command.spawn().map_err(RuntimeError::Spawn)?;

    let mut stdout_pipe = child.stdout.take().expect("stdout was piped");
    let mut stderr_pipe = child.stderr.take().expect("stderr was piped");

    let (stdout_tx, stdout_rx) = mpsc::channel();
    let (marker_tx, marker_rx) = mpsc::channel();
    let marker = stdout_marker.map(str::to_string);
    let stdout_thread = thread::spawn(move || {
        let mut reader = BufReader::new(&mut stdout_pipe);
        let mut output = String::new();
        let mut line = String::new();
        let mut marker_reported = false;

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    output.push_str(&line);
                    if !marker_reported
                        && marker
                            .as_deref()
                            .is_some_and(|needle| output.contains(needle))
                    {
                        marker_reported = true;
                        let _ = marker_tx.send(());
                    }
                }
            }
        }
        let _ = stdout_tx.send(output);
    });

    let (stderr_tx, stderr_rx) = mpsc::channel();
    let stderr_thread = thread::spawn(move || {
        let mut buf = String::new();
        let _ = stderr_pipe.read_to_string(&mut buf);
        let _ = stderr_tx.send(buf);
    });

    // Write stdin only after the output drainers are running. Keeping the
    // write in its own thread means a child that does not read stdin cannot
    // bypass the command timeout by blocking this thread before polling
    // starts. The pipe stays open for marker-based protocols until cleanup.
    let (stdin_done_tx, stdin_done_rx) = mpsc::channel();
    let (stdin_release_tx, stdin_release_rx) = mpsc::channel();
    let stdin_thread = stdin.map(str::to_owned).and_then(|input| {
        child.stdin.take().map(|mut child_stdin| {
            let keep_open = stdout_marker.is_some();
            thread::spawn(move || {
                let result = child_stdin.write_all(input.as_bytes());
                let _ = stdin_done_tx.send(
                    result
                        .as_ref()
                        .map(|_| ())
                        .map_err(|err| std::io::Error::new(err.kind(), err.to_string())),
                );
                if keep_open && result.is_ok() {
                    let _ = stdin_release_rx.recv();
                }
            })
        })
    });

    let release_stdin = || {
        let _ = stdin_release_tx.send(());
    };

    let finish = |status: ExitStatus| {
        release_stdin();
        if let Some(stdin_thread) = stdin_thread {
            let _ = stdin_thread.join();
        }
        let (stdout, stderr) =
            join_output_threads(stdout_thread, stdout_rx, stderr_thread, stderr_rx);
        (status, stdout, stderr)
    };

    let timeout = timeout.unwrap_or(DEFAULT_TIMEOUT);
    let status = loop {
        if stdout_marker.is_some() && marker_rx.try_recv().is_ok() {
            let status = terminate_child(&mut child).map_err(RuntimeError::Io)?;
            break finish(status);
        }
        match child.try_wait() {
            Ok(Some(status)) => break finish(status),
            Ok(None) => {}
            Err(err) => {
                if let Ok(status) = terminate_child(&mut child) {
                    let _ = finish(status);
                }
                return Err(RuntimeError::Io(err));
            }
        }
        if let Ok(Err(err)) = stdin_done_rx.try_recv() {
            if let Ok(status) = terminate_child(&mut child) {
                let _ = finish(status);
            }
            return Err(RuntimeError::Io(err));
        }
        if start.elapsed() >= timeout {
            if let Ok(status) = terminate_child(&mut child) {
                let _ = finish(status);
            }
            return Err(RuntimeError::Timeout(timeout));
        }
        thread::sleep(Duration::from_millis(20));
    };

    Ok(CommandResult {
        exit_code: status.0.code().unwrap_or(-1),
        stdout: status.1,
        stderr: status.2,
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
        None,
        None,
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
        let result = run("hostname", &[], &[], None, None, None).expect("hostname should run");
        assert_eq!(result.exit_code, 0);
        assert!(!result.stdout.trim().is_empty());
    }

    #[test]
    fn missing_binary_returns_spawn_error() {
        let err = run(
            "quotify-definitely-not-a-real-binary-xyz",
            &[],
            &[],
            None,
            None,
            None,
        )
        .expect_err("missing binary should fail to spawn");
        assert!(matches!(err, RuntimeError::Spawn(_)));
    }

    #[test]
    fn timeout_is_enforced() {
        #[cfg(windows)]
        let (bin, args) = (
            "ping",
            vec!["-n".to_string(), "6".to_string(), "127.0.0.1".to_string()],
        );
        #[cfg(not(windows))]
        let (bin, args) = ("sleep", vec!["5".to_string()]);

        let err = run(
            bin,
            &args,
            &[],
            None,
            None,
            Some(Duration::from_millis(300)),
        )
        .expect_err("long-running command should time out");
        assert!(matches!(err, RuntimeError::Timeout(_)));
    }

    #[cfg(unix)]
    #[test]
    fn timeout_reaps_descendants_that_inherit_output_pipes() {
        let started = Instant::now();
        let err = run(
            "sh",
            &["-c".to_string(), "sleep 5".to_string()],
            &[],
            None,
            None,
            Some(Duration::from_millis(300)),
        )
        .expect_err("launcher should time out");

        assert!(matches!(err, RuntimeError::Timeout(_)));
        assert!(started.elapsed() < Duration::from_secs(2));
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

    #[test]
    fn writes_configured_stdin_and_closes_the_pipe() {
        #[cfg(windows)]
        let (bin, args) = ("findstr", vec!["hello".to_string()]);
        #[cfg(not(windows))]
        let (bin, args) = ("cat", Vec::new());

        let result =
            run(bin, &args, &[], Some("hello\n"), None, None).expect("stdin should be piped");
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello"));
    }

    #[cfg(not(windows))]
    #[test]
    fn collects_from_a_long_lived_protocol_until_the_marker_arrives() {
        let result = run(
            "cat",
            &[],
            &[],
            Some("protocol response marker\n"),
            Some("response marker"),
            Some(Duration::from_secs(2)),
        )
        .expect("marker should terminate collection");

        assert!(result.stdout.contains("protocol response marker"));
    }
}
