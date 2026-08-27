//! Codec workspace as a KernelContext-owned ContextResource.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use lumio_kernel::context::{CancelReason, ContextResource, Deadline, QuiesceReport, QuiesceState};
use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};

/// Scratch buffer charged as codec workspace. `destroy` clears it to len 0.
pub struct CodecWorkspace {
    scratch: Mutex<Vec<u8>>,
}

impl CodecWorkspace {
    pub fn new() -> Self {
        Self {
            scratch: Mutex::new(Vec::new()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<u8>> {
        self.scratch.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn reserve(&self, bytes: usize) {
        self.lock().resize(bytes, 0);
    }

    fn len(&self) -> usize {
        self.lock().len()
    }

    fn clear(&self) {
        self.lock().clear();
    }
}

/// Owns a `CodecWorkspace` and rejects use after `destroy`.
pub struct CodecResource {
    workspace: CodecWorkspace,
    destroyed: AtomicBool,
}

impl CodecResource {
    pub fn new() -> Self {
        Self {
            workspace: CodecWorkspace::new(),
            destroyed: AtomicBool::new(false),
        }
    }

    pub fn reserve(&self, bytes: usize) -> KernelResult<()> {
        self.ensure_live()?;
        self.workspace.reserve(bytes);
        Ok(())
    }

    pub fn try_use(&self) -> KernelResult<()> {
        self.ensure_live()?;
        let _ = self.workspace.len();
        Ok(())
    }

    pub fn workspace_len(&self) -> usize {
        self.workspace.len()
    }

    fn ensure_live(&self) -> KernelResult<()> {
        if self.destroyed.load(Ordering::SeqCst) {
            return Err(KernelError::new(
                ErrorCategory::ContextDestroyed,
                ErrorDetail::None,
            ));
        }
        Ok(())
    }
}

impl ContextResource for CodecResource {
    fn name(&self) -> &'static str {
        "codec"
    }

    fn cancel_requested(&self, _reason: CancelReason) {}

    fn quiesce(&self, _deadline: Deadline) -> KernelResult<QuiesceReport> {
        Ok(QuiesceReport {
            state: QuiesceState::Quiesced,
        })
    }

    fn destroy(&self) -> KernelResult<()> {
        self.destroyed.store(true, Ordering::SeqCst);
        self.workspace.clear();
        Ok(())
    }
}
