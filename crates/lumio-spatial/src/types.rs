//! Vendor-free spatial POD types. Coordinates must be finite; AABBs must be ordered.

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SpatialObjectId(u64);

impl SpatialObjectId {
    pub const fn from_raw(v: u64) -> Self {
        Self(v)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point3 {
    pub fn new(x: f32, y: f32, z: f32) -> KernelResult<Self> {
        let point = Self { x, y, z };
        point.validate()?;
        Ok(point)
    }

    pub fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    pub fn validate(&self) -> KernelResult<()> {
        if !self.is_finite() {
            return Err(KernelError::new(
                ErrorCategory::InvalidArgument,
                ErrorDetail::StaticMessage("non-finite coordinate"),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb3 {
    pub min: Point3,
    pub max: Point3,
}

impl Aabb3 {
    pub fn new(min: Point3, max: Point3) -> KernelResult<Self> {
        let aabb = Self { min, max };
        aabb.validate()?;
        Ok(aabb)
    }

    pub fn validate(&self) -> KernelResult<()> {
        self.min.validate()?;
        self.max.validate()?;
        if self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z {
            return Err(KernelError::new(
                ErrorCategory::InvalidArgument,
                ErrorDetail::StaticMessage("inverted AABB bounds"),
            ));
        }
        Ok(())
    }
}
