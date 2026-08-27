//! Local borrowed view and owned bounded copy of a diagnostic record.
//!
//! `lumio-kernel` has no `record::RecordPort` yet (public schema pending).
//! Field and payload copies are sized before allocation; no unbounded `String`.

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};

/// Borrowed record view local to this crate (kernel port unpublished).
#[derive(Clone, Copy, Debug)]
pub struct KernelRecordRef<'a> {
    pub fields: &'a [&'a str],
    pub payload: &'a [u8],
}

/// Owned copy whose field count and copied byte length are capped at construction.
#[derive(Debug)]
pub struct OwnedKernelRecord {
    field_bytes: Vec<u8>,
    field_ends: Vec<usize>,
    payload: Vec<u8>,
}

impl OwnedKernelRecord {
    pub fn try_from_ref(
        r: KernelRecordRef<'_>,
        max_fields: usize,
        max_bytes: usize,
    ) -> KernelResult<Self> {
        let field_count = r.fields.len();
        if field_count > max_fields {
            return Err(exceeded(max_fields, field_count));
        }

        let total_bytes = match copied_byte_len(r) {
            Some(n) => n,
            None => return Err(exceeded(max_bytes, usize::MAX)),
        };
        if total_bytes > max_bytes {
            return Err(exceeded(max_bytes, total_bytes));
        }

        let mut field_bytes = Vec::with_capacity(total_bytes.saturating_sub(r.payload.len()));
        let mut field_ends = Vec::with_capacity(field_count);
        for field in r.fields {
            field_bytes.extend_from_slice(field.as_bytes());
            field_ends.push(field_bytes.len());
        }

        let mut payload = Vec::with_capacity(r.payload.len());
        payload.extend_from_slice(r.payload);

        Ok(Self {
            field_bytes,
            field_ends,
            payload,
        })
    }

    pub fn field_count(&self) -> usize {
        self.field_ends.len()
    }

    pub fn byte_len(&self) -> usize {
        self.field_bytes.len() + self.payload.len()
    }
}

fn copied_byte_len(r: KernelRecordRef<'_>) -> Option<usize> {
    let mut total = 0usize;
    for field in r.fields {
        total = total.checked_add(field.len())?;
    }
    total.checked_add(r.payload.len())
}

fn exceeded(limit: usize, requested: usize) -> KernelError {
    KernelError::new(
        ErrorCategory::CapacityExceeded,
        ErrorDetail::LimitExceeded {
            limit: limit as u64,
            requested: requested as u64,
        },
    )
}
