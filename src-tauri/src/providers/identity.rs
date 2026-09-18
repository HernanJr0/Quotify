use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::ProviderId;

/// Creates a process-local comparison key without sending account identifiers
/// to the frontend. A key is only produced after an adapter has received a
/// stable identity from the provider's own CLI protocol.
pub fn fingerprint(provider: ProviderId, fields: &[&str]) -> String {
    let mut hasher = DefaultHasher::new();
    provider.hash(&mut hasher);
    for field in fields {
        field.hash(&mut hasher);
    }
    format!("v1:{:016x}", hasher.finish())
}
