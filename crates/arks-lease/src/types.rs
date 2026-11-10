//! Lease record types and writer mode.

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Opaque resource identifier (ref namespace, path set, or explicit id).
pub type ResourceId = String;

/// Dual-path writer mode ([[sot:root/lease-concurrency-intent]] Dual path).
///
/// `GitCompat` is the **default**: leases pre-coordinate agents; Git locks
/// (`*.lock`, `index.lock`) still apply. `HighConcurrency` is **opt-in** claimed
/// shape (lease + batch-apply + worktree/ref-shard) unlocked by M23
/// ([[adr:root/20261109194200]]); shared-index N-way remains OUT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriterMode {
    /// Default interchange path: honor Git locks; leases are additive.
    #[default]
    GitCompat,
    /// Opt-in high-concurrency product mode (L3 claimed shape).
    HighConcurrency,
}

/// One active lease grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseRecord {
    pub resource_id: ResourceId,
    pub holder: String,
    /// Absolute expiry as Unix epoch milliseconds (fencing).
    pub expires_at_ms: u64,
}

impl LeaseRecord {
    pub fn is_expired_at(&self, now_ms: u64) -> bool {
        now_ms >= self.expires_at_ms
    }
}

/// Convert [`SystemTime`] to Unix epoch milliseconds.
pub fn system_time_to_ms(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

/// Current wall-clock Unix epoch milliseconds.
pub fn now_ms() -> u64 {
    system_time_to_ms(SystemTime::now())
}

/// Add a [`Duration`] TTL to a base epoch-ms timestamp.
pub fn expiry_from(now: u64, ttl: Duration) -> u64 {
    now.saturating_add(ttl.as_millis() as u64)
}
