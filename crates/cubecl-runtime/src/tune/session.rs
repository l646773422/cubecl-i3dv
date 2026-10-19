//! Opt-in SDK tuning state. Owned by the compute client, never a static tuner.
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::any::{Any, TypeId};
use hashbrown::HashMap;

/// Host-controlled persistence. Missing, corrupt or unwritable caches are optional.
/// Implementations must bound reads/writes and serialize concurrent publication.
pub trait AutotuneStore: core::fmt::Debug + Send + Sync {
    fn read(&self, namespace: &str) -> Option<Vec<u8>>;
    fn write(&self, namespace: &str, bytes: &[u8]);
}

/// Device-local state; no GPU references and no dependency-global configuration changes.
#[derive(Default)]
pub struct SessionTuning {
    states: spin::Mutex<HashMap<(TypeId, &'static str), Box<dyn Any + Send + Sync>>>,
    pub(crate) store: Option<Arc<dyn AutotuneStore>>,
}

impl core::fmt::Debug for SessionTuning {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SessionTuning")
            .field("tables", &self.states.lock().len())
            .field("persistent", &self.store.is_some())
            .finish()
    }
}

impl SessionTuning {
    pub fn new(store: Option<Arc<dyn AutotuneStore>>) -> Self {
        Self {
            states: Default::default(),
            store,
        }
    }

    pub(crate) fn state<T: Default + Send + Sync + 'static>(&self, name: &'static str) -> Arc<T> {
        let mut states = self.states.lock();
        states
            .entry((TypeId::of::<T>(), name))
            .or_insert_with(|| Box::new(Arc::new(T::default())))
            .downcast_ref::<Arc<T>>()
            .expect("type ID and stored state must agree")
            .clone()
    }

    pub fn table_count(&self) -> usize {
        self.states.lock().len()
    }
}
