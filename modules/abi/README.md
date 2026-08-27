# abi

> NativeCore 的稳定 C ABI 基础：固定宽度类型、版本化结构、Buffer 和 Root API Table。

**优先级**：P0  
**实施阶段**：Architecture Gate / Foundation  
**架构基线**：`LGE-V1.0-2026-08-27`

公共 ABI Schema、结构字段和 Fixture 的规范来源是 `LumioGameEngineArchitecture`；本文只描述本地模块边界。

## 负责范围

- 定义跨语言可表达的固定宽度 POD、布尔/枚举宽度和字节布局约束。
- 提供带 `abi_version`、`struct_size` 和 `capability_bits` 的版本化结构约定。
- 提供调用方可拥有的 Buffer 描述和单一 Root API Table 入口。
- 为 Header 编译、布局检查和入口 smoke test 提供实现承载面。

## 不负责范围

- 不维护公共 Error Code、Capability Bit、ID Registry 或领域 Schema。
- 不生成最终 C# Binding，不传 Rust/C# 容器、对象引用或异常。
- 不决定 Host、World、Session、网络或持久化格式。

## 输入、输出与所有权

调用方提供请求结构和可选输出 Buffer；创建侧负责释放由其创建的资源。Buffer 不足返回所需长度，不隐式扩容。ABI 入口不得跨调用保存调用方裸指针；异步结果必须转为版本化批次后由调用方消费。

## 依赖与约束

`abi` 是 NativeCore 的基础模块，不依赖其他 NativeCore 模块。第三方类型只能在内部 Adapter 转换为 ABI POD。任何布局、对齐、编码或可重入性变更都必须回到架构源并生成新 Baseline。

## 线程、错误与观测

布局描述和版本查询应保持无状态、可重入且不阻塞。未知版本、结构过短、结构过长策略和 Buffer 不足必须映射到 `error` 的稳定类别；panic 不得穿过导出边界。模块只提供诊断字段承载，不拥有日志 Sink。

## 测试与性能

- C Header 编译、结构大小、对齐、字段偏移和字节序检查。
- Root API 版本协商、短结构、未知尾字段和 Buffer 不足 smoke test。
- 记录 ABI 调用批次、复制字节和布局检查耗时；不以逐对象 FFI 作为默认接口。

## 版本演进

可向后兼容的尾部扩展必须由 `struct_size` 保护；破坏布局、宽度或所有权语义时提升 ABI 主版本。模块 README 不记录具体字段数值，具体数值以架构源生成物为准。

## 相关

- [仓库边界与架构契约](../../.spec/knowledge/standards/repository-architecture.md)
- [根 README](../../README.md)
- [架构镜像](../../docs/architecture/LumioGameEngine_Architecture_v1.0.md)
