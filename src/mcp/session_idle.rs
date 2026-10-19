//! Narrow continuity adapter; freshness capture and builders stay in session.rs.
use super::SessionProvider;
use crate::mcp::idle::Published;
use sha2::{Digest, Sha256};

impl SessionProvider {
    pub(crate) fn idle_publication(&self) -> Published {
        // Ordered maps and bincode provide a bounded digest of the existing snapshot identity.
        let bytes =
            bincode::serialize(&self.snapshot).expect("snapshot serialization is infallible");
        Published {
            fingerprint: Sha256::digest(bytes).into(),
            generation: self.generation,
            diverged: self.known_stale_after_refresh.is_some(),
        }
    }

    pub(crate) fn resume_idle_generation(&mut self, previous: Published) {
        // Prior sticky divergence is cleared by bootstrap. With the same fingerprint it
        // changes the logged transition, but keeps the generation of that snapshot.
        self.generation = previous.generation.saturating_add(u64::from(
            self.idle_publication().fingerprint != previous.fingerprint,
        ));
    }
}
