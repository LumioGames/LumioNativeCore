# 0006 · Capability key 只由生成注册表投影，不保留裸构造器

- 日期:2026-08-29
- 状态:生效

## 背景

D-015 裁决（架构源 ADR-040 §7.1 / ADR-048）把 capability **键空间**定为
`ids/index.json` 独有，架构生成器是唯一发射方，下游只消费投影；裁决前作为
「唯一正确模型」的仓内私有键值表就此变成违规。

本仓 `CapabilityKey` 当时是 `from_local_index(u32)` 公共构造器 + 本地序号，
正是被取代的那种模型：任何调用方都能凭空造一个注册表里没有的键，而且不会有
任何东西报错。

## 决策

`CapabilityKey` **不保留任何裸构造器**——crate 内外都没有。唯一构造路径是
`from_registered(ArchitectureCapabilityKey)`（及按 id 查表的
`from_registry_id`），而 `ArchitectureCapabilityKey` 只在
`cargo xtask gen-contracts` 生成的表里存在。

`gen-contracts` 同时做三方交叉核对：`ids/index.json` 的 `Capability` 命名空间、
镜像 C Header 的 `LUMIO_CAPABILITY_<SCREAMING>` 常量、以及
`LUMIO_CAPABILITY_COUNT` 必须逐值一致，任一不一致直接生成失败。
`LUMIO_CAPABILITY_BITS` 显式排除在键空间外：D-015 只裁了键，掩码还是计数、
以及任何 bit 位指派仍未冻结，`registry::capability_bits()` 保持为空。

## 后果

- 仓内无法表达一个上游没注册的 capability；真有需要必须先在架构源注册，接受
  这条往返延迟。
- 测试只能用已发布的键（如 `Native` / `HybridCLR`），不能再用任意序号构造
  用例，可读性反而变好，但重命名上游 id 会同时改到测试。
- 镜像半新半旧（改了 Header 忘了重跑生成器，或反之）在生成期即失败，不会拖到
  运行期才发现。
