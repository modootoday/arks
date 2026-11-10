//! L1 in-process lease manager.

use crate::error::LeaseError;
use crate::types::{expiry_from, now_ms, LeaseRecord, ResourceId};
use std::collections::BTreeMap;
use std::time::Duration;

/// In-process lease table: grant / release / expire / inspect / list.
///
/// Resource ids are opaque strings. TTL fencing uses absolute expiry timestamps.
/// This manager does **not** touch Git locks — callers must still use
/// `arks-refs` / `arks-index` lock APIs on Git-compat paths.
#[derive(Debug, Default, Clone)]
pub struct LeaseManager {
    leases: BTreeMap<ResourceId, LeaseRecord>,
}

impl LeaseManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grant a lease on `resource_id` to `holder` for `ttl`.
    ///
    /// Fails if another (non-expired) holder already owns the resource.
    /// Same holder may renew (overwrite expiry).
    pub fn grant(
        &mut self,
        resource_id: impl Into<ResourceId>,
        holder: impl Into<String>,
        ttl: Duration,
    ) -> Result<LeaseRecord, LeaseError> {
        self.grant_at(resource_id, holder, ttl, now_ms())
    }

    /// Grant with an explicit `now_ms` (testable fencing).
    pub fn grant_at(
        &mut self,
        resource_id: impl Into<ResourceId>,
        holder: impl Into<String>,
        ttl: Duration,
        now: u64,
    ) -> Result<LeaseRecord, LeaseError> {
        let resource_id = resource_id.into();
        let holder = holder.into();
        self.expire_due_at(now);
        if let Some(existing) = self.leases.get(&resource_id) {
            if existing.holder != holder {
                return Err(LeaseError::AlreadyHeld {
                    resource_id: resource_id.clone(),
                    holder: existing.holder.clone(),
                });
            }
        }
        let record = LeaseRecord {
            resource_id: resource_id.clone(),
            holder,
            expires_at_ms: expiry_from(now, ttl),
        };
        self.leases.insert(resource_id, record.clone());
        Ok(record)
    }

    /// Release a lease held by `holder`.
    pub fn release(
        &mut self,
        resource_id: impl AsRef<str>,
        holder: impl AsRef<str>,
    ) -> Result<(), LeaseError> {
        let resource_id = resource_id.as_ref();
        let holder = holder.as_ref();
        match self.leases.get(resource_id) {
            Some(rec) if rec.holder == holder => {
                self.leases.remove(resource_id);
                Ok(())
            }
            Some(rec) => Err(LeaseError::NotHeld {
                resource_id: resource_id.to_string(),
                holder: rec.holder.clone(),
            }),
            None => Err(LeaseError::NotFound(resource_id.to_string())),
        }
    }

    /// Drop leases whose expiry is at or before wall clock.
    pub fn expire_due(&mut self) -> Vec<LeaseRecord> {
        self.expire_due_at(now_ms())
    }

    /// Drop leases expired at `now` (testable).
    pub fn expire_due_at(&mut self, now: u64) -> Vec<LeaseRecord> {
        let expired: Vec<ResourceId> = self
            .leases
            .iter()
            .filter(|(_, r)| r.is_expired_at(now))
            .map(|(k, _)| k.clone())
            .collect();
        let mut out = Vec::with_capacity(expired.len());
        for id in expired {
            if let Some(rec) = self.leases.remove(&id) {
                out.push(rec);
            }
        }
        out
    }

    /// Inspect one resource (does not auto-expire; call `expire_due` first if needed).
    pub fn inspect(&self, resource_id: impl AsRef<str>) -> Option<&LeaseRecord> {
        self.leases.get(resource_id.as_ref())
    }

    /// List all current lease records (sorted by resource id via `BTreeMap`).
    pub fn list(&self) -> Vec<&LeaseRecord> {
        self.leases.values().collect()
    }

    /// Replace the entire in-memory table (used by L2 load).
    pub fn replace_all(&mut self, records: Vec<LeaseRecord>) {
        self.leases.clear();
        for rec in records {
            self.leases.insert(rec.resource_id.clone(), rec);
        }
    }

    /// Snapshot all records for L2 save.
    pub fn snapshot(&self) -> Vec<LeaseRecord> {
        self.leases.values().cloned().collect()
    }
}
