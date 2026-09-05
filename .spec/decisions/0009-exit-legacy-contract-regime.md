# 0009 · 退出旧合同制：本仓只留纯 Rust 内核 crate，不再持有跨语言合同

- 日期:2026-09-05
- 状态:生效

## 背景

2026-08 的规矩是「架构仓发布 Baseline 合同（`LGE-V1.4-2026-08-27`，含 C Header 与 Root ABI
bundle），实现仓复印一份、对着复印件写代码、CI 校对复印件」。本仓据此建了三层东西：
字节级镜像目录与 `.baseline.sha256`、`lumio-contract-types` 的布局 golden 与 digest 漂移门、
`lumio-native-ffi` 按旧头文件拼的 provider 表，以及内核里由镜像投影出来的错误码数值与
capability 注册表键。

2026-09 初架构仓按 ADR-059 转入 Living Architecture：合同、`packages/`、校验器与 mirror
全部删除，唯一 ABI 真值改为 `engine/abi/native-abi.json`，SDK（`engine/native/modules/sdk-native`）
以 Cargo 路径依赖直接把本仓 crate 源码编进它自己的动态库。于是这三层的对端全部消失：
镜像钉的是已删除的对象，provider 表等的是已退役的 CoreEngine 来取，内核里的数值来自
一个不再发布的注册表。`architecture.md` §7 第 4 条把「活动源码和 CI 不再依赖 CoreEngine、
Baselines 或 contract mirror」定为迁移完成条件，本仓此前不满足。

Owner 2026-09-05 裁决（`.spec/reviews/2026-09-05-engine-repos-progress-assessment.md`
§6 D1 / D5）：三层全清，一张卡做完，不留「先兼容、以后再清」的中间态。

## 决策

本仓退出旧合同制，变成**只剩纯 Rust 内核 crate**的形态：

1. **不再持有合同复印件与校对。** 删除 `docs/architecture/` 整目录与 `.baseline.sha256`、
   `.gitattributes` 的镜像字节权威行、CI `readme` job 的全部基线字符串与 sha256 断言。
   README 按 Living Architecture 重写；CI 只保留结构性检查。
2. **不再有 C 导出面。** `lumio-native-ffi` 整 crate 删除（D5）。跨语言边界与其唯一真值
   `native-abi.json` 都在架构仓，插头代码也在那边；本仓保留一个 C 导出面只会是第二份、
   且没有装载路径的副本。`xtask dump-symbols` 随之退役，改为
   `assert-no-native-artifacts`：断言 workspace 内不存在 `cdylib` / `staticlib` 目标。
3. **不再有旧合同的门与生成链。** `lumio-contract-types` 整 crate 删除——它承载的全部内容
   （生成适配层、布局 golden、digest 漂移门、注册表投影）都绑定在已退役的 bundle 上，
   删干净后不剩任何本仓内部需要的类型。`xtask` 的 `gen-contracts` / `check-baseline`
   与 `baseline.rs` / `contracts.rs` 一并删除。
4. **内核不再持有跨边界数值。** 删除 `lumio_kernel::error::to_architecture_error_code`
   与 1044–1053 一切数值；`ErrorCategory` 保留为**内部**枚举。跨边界状态码归架构仓插头
   对 `native-abi.json` 决定，本仓不为内核错误码申请状态码。
5. **capability 键改为本仓内部的不透明数值。** 删除 `CapabilityKey::from_registered` /
   `from_registry_id` / `as_registry_numeric` 这条注册表投影，改为 `from_raw(u32)` / `raw()`。

第 5 条在「改为本仓内部常量」与「整块删除」之间取前者，理由是：被裁掉的只是**键的来源**
（旧注册表），而 `StaticCapabilities` 的有序唯一集合与 `CapabilitySource::require` 是内核自己的
门控逻辑，删掉会连带删除本卡没有授权删除的内核功能，并让 `ErrorCategory::CapabilityUnavailable`
失去唯一产出点。同时**不在本仓重建一张键名表**：旧键名（`Native` / `HybridCLR` / `Voxel*`）
是架构与 Voxel 语义，本仓按定位不拥有它们，重建即违反「NativeCore 不拥有领域语义」。
因此键退化为调用方（架构仓 SDK）自己定义的不透明 `u32`，内核只做集合成员判定。

## 后果

- 被取代的历史条目（原文不改写）：
  - **ADR 0001** 的「`abi` 拆 `contract-types` 叶子 + `native-core-ffi` 门面」分层与
    「NativeCore 提供 provider API Table」结论作废——两层都不存在了。其「跨仓 Root 符号
    不由本仓导出」的结论以更强的形式保留：本仓根本不导出任何 C 符号。
  - **ADR 0006** 的「`CapabilityKey` 不保留任何裸构造器、只由生成注册表投影」作废——
    生成注册表已不存在，裸构造器恢复。
  - **ADR 0008** 中「`lumio-native-ffi` 按 `native-abi.json` 根表组装并填充 `timer_*` 槽」
    的实现归属已由其 2026-09-03 修订记录移交架构仓；本 ADR 只是删掉那个空壳 crate。
- 本仓稳定边界从「版本化 C ABI」变为「crate 的公开 Rust API」。改公开 API 即改 SDK 的
  编译输入，必须在架构仓 `engine/native` 复跑 `cargo build -p lumio-engine-native`
  与 `cargo test -p lumio-engine-native`。
- 失去了「仓内自证 ABI 布局/数值没漂」的能力。这不是回退：那些断言现在只有对着
  `native-abi.json` 做才有意义，而 `native-abi.json` 与插头都在架构仓，重复一份即是
  被 ADR-059 废止的复印件。
- 内核错误类别到跨边界状态码的映射当前**无人承载**：架构仓插头只在需要时逐个映射。
  内核函数真正进 ABI 时另开卡，届时状态码在 `native-abi.json` 一侧新增，不回流本仓。
