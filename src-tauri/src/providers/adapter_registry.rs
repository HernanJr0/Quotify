use std::collections::HashMap;

use super::adapter::{MockAdapter, UsageProviderAdapter};
use super::claude::ClaudeAdapter;
use super::codex::CodexAdapter;
use super::ProviderId;

/// Holds one `UsageProviderAdapter` per provider (docs/PLAN.md section 17).
/// Claude and Codex are real as of Fases 8-9. The remaining providers stay
/// mocked until they are replaced independently.
pub struct AdapterRegistry {
    adapters: HashMap<ProviderId, Box<dyn UsageProviderAdapter>>,
}

impl AdapterRegistry {
    pub fn with_defaults() -> Self {
        let mut adapters: HashMap<ProviderId, Box<dyn UsageProviderAdapter>> = HashMap::new();
        adapters.insert(ProviderId::Claude, Box::new(ClaudeAdapter::new()));
        adapters.insert(ProviderId::Codex, Box::new(CodexAdapter::new()));
        for provider in [ProviderId::Gemini, ProviderId::Grok] {
            adapters.insert(provider, Box::new(MockAdapter::new(provider)));
        }
        Self { adapters }
    }

    pub fn get(&self, provider: ProviderId) -> Option<&dyn UsageProviderAdapter> {
        self.adapters.get(&provider).map(AsRef::as_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_provider_has_an_adapter() {
        let registry = AdapterRegistry::with_defaults();
        for provider in ProviderId::ALL {
            assert!(registry.get(provider).is_some());
        }
    }
}
