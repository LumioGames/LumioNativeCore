//! Ordered ContextResource registrations. Snapshots clone Arcs so later calls are lock-free.

use std::sync::{Arc, Mutex};

use super::resource::ContextResource;
use crate::error::KernelError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceRegistration {
    pub index: u32,
    pub name: &'static str,
}

pub struct ResourceRegistry {
    items: Mutex<Vec<Arc<dyn ContextResource>>>,
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self {
            items: Mutex::new(Vec::new()),
        }
    }

    pub fn register(
        &self,
        r: Arc<dyn ContextResource>,
    ) -> Result<ResourceRegistration, KernelError> {
        let name = r.name();
        let mut items = self.items.lock().expect("resource registry lock");
        let index = items.len() as u32;
        items.push(r);
        Ok(ResourceRegistration { index, name })
    }

    pub fn snapshot_names(&self) -> Vec<&'static str> {
        self.snapshot().iter().map(|r| r.name()).collect()
    }

    pub fn snapshot(&self) -> Vec<Arc<dyn ContextResource>> {
        self.items.lock().expect("resource registry lock").clone()
    }
}
