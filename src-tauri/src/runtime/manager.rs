use super::Runtime;

#[cfg(target_os = "windows")]
use super::{windows::WindowsRuntime, wsl};

#[cfg(target_os = "linux")]
use super::linux::LinuxRuntime;

#[cfg(target_os = "macos")]
use super::macos::MacRuntime;

/// Discovers and holds every `Runtime` available on this machine: the host
/// runtime for whichever OS Quotify itself is running on, plus one
/// `WslRuntime` per detected WSL distro when running on Windows
/// (docs/PLAN.md sections 9 and 16).
pub struct RuntimeManager {
    runtimes: Vec<Box<dyn Runtime>>,
}

impl RuntimeManager {
    pub fn discover() -> Self {
        let mut runtimes: Vec<Box<dyn Runtime>> = Vec::new();

        #[cfg(target_os = "windows")]
        {
            runtimes.push(Box::new(WindowsRuntime::new()));
            runtimes.extend(
                wsl::discover()
                    .into_iter()
                    .map(|runtime| Box::new(runtime) as Box<dyn Runtime>),
            );
        }

        #[cfg(target_os = "linux")]
        runtimes.push(Box::new(LinuxRuntime::new()));

        #[cfg(target_os = "macos")]
        runtimes.push(Box::new(MacRuntime::new()));

        Self { runtimes }
    }

    pub fn runtimes(&self) -> &[Box<dyn Runtime>] {
        &self.runtimes
    }

    pub fn get(&self, id: &str) -> Option<&dyn Runtime> {
        self.runtimes
            .iter()
            .find(|runtime| runtime.id() == id)
            .map(AsRef::as_ref)
    }
}
