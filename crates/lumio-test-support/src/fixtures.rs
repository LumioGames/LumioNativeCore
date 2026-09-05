//! Test fixtures. Corpus unpublished; fields stay private.

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
