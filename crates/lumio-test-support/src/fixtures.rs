//! Architecture-baseline fixture loader. Corpus unpublished; negative gate only.

use lumio_contract_types::{
    ContractMismatch, architecture_baseline_id, verify_generated_contract_revision_against,
};

/// Rejects a `found` baseline that this workspace does not bind.
pub struct FixtureLoader {
    _private: (),
}

impl Default for FixtureLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl FixtureLoader {
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Rejects `found` when it is not the architecture baseline this workspace binds.
    pub fn load_baseline(&self, found: &'static str) -> Result<(), ContractMismatch> {
        verify_generated_contract_revision_against(found)
    }

    pub fn current_baseline(&self) -> &'static str {
        architecture_baseline_id()
    }
}

/// Named fault-injection plan. Corpus unpublished; fields stay private.
pub struct FaultPlan {
    _private: (),
}

impl Default for FaultPlan {
    fn default() -> Self {
        Self::new()
    }
}

impl FaultPlan {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

/// Boundary panic probe. Corpus unpublished; fields stay private.
pub struct PanicProbe {
    _private: (),
}

impl Default for PanicProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl PanicProbe {
    pub fn new() -> Self {
        Self { _private: () }
    }
}
