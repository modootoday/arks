//! Dual-path writer mode valve (Git-compat default; L3 claimed shape unlocked M23).

use crate::error::LeaseError;
use crate::types::WriterMode;

/// Product writer session: selects Git-compat vs high-concurrency path.
///
/// Holding a lease never skips Git locks on [`WriterMode::GitCompat`].
/// [`WriterMode::HighConcurrency`] selects the opt-in claimed shape
/// (lease + batch-apply + worktree/ref-shard; shared-index N-way OUT) unlocked
/// by M23 clean-room receipts ([[adr:root/20261109194200]]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterSession {
    mode: WriterMode,
}

impl Default for WriterSession {
    fn default() -> Self {
        Self {
            mode: WriterMode::GitCompat,
        }
    }
}

impl WriterSession {
    pub fn new(mode: WriterMode) -> Result<Self, LeaseError> {
        let mut s = Self::default();
        s.set_mode(mode)?;
        Ok(s)
    }

    pub fn mode(&self) -> WriterMode {
        self.mode
    }

    /// Select writer mode. HighConcurrency is opt-in claimed shape (M23 Claimed).
    pub fn set_mode(&mut self, mode: WriterMode) -> Result<(), LeaseError> {
        match mode {
            WriterMode::GitCompat => {
                self.mode = WriterMode::GitCompat;
                Ok(())
            }
            WriterMode::HighConcurrency => {
                // Unlocked after Docker g11l3 dual-path receipts. Feature flag
                // remains available for optional compile-time advertising.
                let _ = cfg!(feature = "high-concurrency");
                self.mode = WriterMode::HighConcurrency;
                Ok(())
            }
        }
    }
}
