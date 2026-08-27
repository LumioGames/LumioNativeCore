//! Call-scoped Vec scratch charged against a MemoryBudget.

use crate::error::KernelError;

use super::budget::MemoryBudget;

/// Call-local allocations charged together and released on `reset`.
pub struct CallScratch {
    budget: MemoryBudget,
    used: u64,
    chunks: Vec<Vec<u8>>,
}

impl CallScratch {
    pub fn new(budget: MemoryBudget) -> Self {
        Self {
            budget,
            used: 0,
            chunks: Vec::new(),
        }
    }

    pub fn alloc(&mut self, n: usize) -> Result<&mut [u8], KernelError> {
        self.budget.try_reserve(n as u64)?;
        self.chunks.push(vec![0u8; n]);
        self.used += n as u64;
        Ok(self.chunks.last_mut().expect("just pushed"))
    }

    pub fn reset(&mut self) {
        self.chunks.clear();
        self.budget.release(self.used);
        self.used = 0;
    }

    pub fn charged(&self) -> u64 {
        self.used
    }
}
