//! Typed kernel lookup by crate-local `OperationId`.
//!
//! Architecture-generated operation numbers are unpublished (B-ABI-004).

use std::collections::HashMap;
use std::sync::Arc;

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError};

use crate::id::OperationId;

pub trait TypedKernel: Send + Sync + 'static {
    fn operation_id(&self) -> OperationId;
}

pub struct OperationRegistry {
    kernels: HashMap<OperationId, Arc<dyn TypedKernel>>,
}

impl OperationRegistry {
    pub fn new() -> Self {
        Self {
            kernels: HashMap::new(),
        }
    }

    pub fn register(&mut self, k: Arc<dyn TypedKernel>) -> Result<(), KernelError> {
        let id = k.operation_id();
        if self.kernels.contains_key(&id) {
            return Err(KernelError::new(
                ErrorCategory::InvalidArgument,
                ErrorDetail::None,
            ));
        }
        self.kernels.insert(id, k);
        Ok(())
    }

    pub fn get(&self, id: OperationId) -> Option<Arc<dyn TypedKernel>> {
        self.kernels.get(&id).cloned()
    }
}
