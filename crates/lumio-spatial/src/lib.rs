//! `lumio-spatial`：Grid / Hash / BVH / 邻域 / 批量距离与碰撞基础 Kernel。
//!
//! 不编译期依赖 lumio-job（作为 operation 经 registry 运行时绑定）；
//! 索引作为 ContextResource 注册进 kernel-context。

#![forbid(unsafe_code)]

mod index;
mod query;
mod types;

pub use index::{GridReferenceIndex, RStarIndexAdapter, SpatialIndexBackend};
pub use query::{AabbQuery, SpatialContext, SpatialHit};
pub use types::{Aabb3, Point3, SpatialObjectId};
