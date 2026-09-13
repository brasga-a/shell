use std::sync::{Arc, RwLock};

use crate::CompositorSnapshot;

/// Thread-safe application-owned compositor snapshot.
///
/// Event adapters publish complete snapshots here; renderers consume copies of
/// the snapshot and never need to know which compositor produced it.
#[derive(Clone, Debug, Default)]
pub struct CompositorStateStore {
    current: Arc<RwLock<Option<CompositorSnapshot>>>,
}

impl CompositorStateStore {
    pub fn snapshot(&self) -> Option<CompositorSnapshot> {
        self.current
            .read()
            .ok()
            .and_then(|snapshot| snapshot.clone())
    }

    pub fn replace(&self, snapshot: CompositorSnapshot) {
        if let Ok(mut current) = self.current.write() {
            *current = Some(snapshot);
        }
    }
}
