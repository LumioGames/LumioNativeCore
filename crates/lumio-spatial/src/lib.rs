//! `lumio-spatial`：Grid / Hash / BVH / 邻域 / 批量距离与碰撞基础 Kernel。
//!
//! 不编译期依赖 lumio-job（作为 operation 经 registry 运行时绑定）；
//! 索引作为 ContextResource 注册进 kernel-context。
//! 当前为脚手架，公共 API 面为空。

#![forbid(unsafe_code)]
