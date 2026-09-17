use crate::runtime::{Runtime, RuntimeManager};

use super::{ProviderId, ProviderInstallation};

pub struct ProviderRegistry;

impl ProviderRegistry {
    /// Detects installed providers across every known runtime
    /// (docs/PLAN.md Fase 4). Does not fetch usage — that's Fase 8+.
    pub fn discover(runtime_manager: &RuntimeManager) -> Vec<ProviderInstallation> {
        Self::discover_in(runtime_manager.runtimes())
    }

    fn discover_in(runtimes: &[Box<dyn Runtime>]) -> Vec<ProviderInstallation> {
        let mut installations = Vec::new();

        for runtime in runtimes {
            for provider in ProviderId::ALL {
                match runtime.which(provider.binary_name()) {
                    Ok(Some(executable)) => installations.push(ProviderInstallation {
                        id: format!("{}:{}", provider.binary_name(), runtime.id()),
                        provider,
                        provider_name: provider.display_name().to_string(),
                        runtime_id: runtime.id().to_string(),
                        runtime_name: runtime.name().to_string(),
                        executable,
                    }),
                    Ok(None) => {}
                    Err(err) => {
                        // A single broken runtime (e.g. an unresponsive WSL
                        // distro) must never take down detection for the
                        // rest (docs/PLAN.md sections 18 and 49).
                        eprintln!(
                            "WARN provider {} detection failed on {}: {err}",
                            provider.display_name(),
                            runtime.name()
                        );
                    }
                }
            }
        }

        installations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{CommandRequest, CommandResult, RuntimeError, RuntimeKind};

    struct FakeRuntime {
        id: String,
        available: Vec<&'static str>,
    }

    impl Runtime for FakeRuntime {
        fn id(&self) -> &str {
            &self.id
        }

        fn kind(&self) -> RuntimeKind {
            RuntimeKind::Linux
        }

        fn name(&self) -> &str {
            &self.id
        }

        fn execute(&self, _request: CommandRequest) -> Result<CommandResult, RuntimeError> {
            unimplemented!("not exercised by these tests")
        }

        fn which(&self, binary: &str) -> Result<Option<String>, RuntimeError> {
            Ok(self
                .available
                .contains(&binary)
                .then(|| format!("/usr/bin/{binary}")))
        }

        fn exists(&self, _path: &str) -> Result<bool, RuntimeError> {
            unimplemented!("not exercised by these tests")
        }

        fn read_file(&self, _path: &str) -> Result<String, RuntimeError> {
            unimplemented!("not exercised by these tests")
        }

        fn home_directory(&self) -> Result<String, RuntimeError> {
            unimplemented!("not exercised by these tests")
        }
    }

    #[test]
    fn discovers_only_available_providers() {
        let runtimes: Vec<Box<dyn Runtime>> = vec![Box::new(FakeRuntime {
            id: "fake".to_string(),
            available: vec!["claude", "codex"],
        })];

        let found: Vec<_> = ProviderRegistry::discover_in(&runtimes)
            .into_iter()
            .map(|installation| installation.provider)
            .collect();

        assert!(found.contains(&ProviderId::Claude));
        assert!(found.contains(&ProviderId::Codex));
        assert!(!found.contains(&ProviderId::Gemini));
        assert!(!found.contains(&ProviderId::Grok));
    }

    #[test]
    fn one_broken_runtime_does_not_block_others() {
        struct BrokenRuntime;
        impl Runtime for BrokenRuntime {
            fn id(&self) -> &str {
                "broken"
            }
            fn kind(&self) -> RuntimeKind {
                RuntimeKind::Wsl
            }
            fn name(&self) -> &str {
                "Broken"
            }
            fn execute(&self, _request: CommandRequest) -> Result<CommandResult, RuntimeError> {
                unimplemented!()
            }
            fn which(&self, _binary: &str) -> Result<Option<String>, RuntimeError> {
                Err(RuntimeError::Timeout(std::time::Duration::from_secs(5)))
            }
            fn exists(&self, _path: &str) -> Result<bool, RuntimeError> {
                unimplemented!()
            }
            fn read_file(&self, _path: &str) -> Result<String, RuntimeError> {
                unimplemented!()
            }
            fn home_directory(&self) -> Result<String, RuntimeError> {
                unimplemented!()
            }
        }

        let runtimes: Vec<Box<dyn Runtime>> = vec![
            Box::new(BrokenRuntime),
            Box::new(FakeRuntime {
                id: "fake".to_string(),
                available: vec!["claude"],
            }),
        ];

        let found = ProviderRegistry::discover_in(&runtimes);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].provider, ProviderId::Claude);
    }
}
