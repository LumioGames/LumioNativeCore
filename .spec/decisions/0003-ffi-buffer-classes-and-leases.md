# 0003 · FFI Buffer 三分类 + 异步租约，按 allocator provenance 定释放方

- 日期:2026-08-27
- 状态:生效

## 背景

架构 Review `ARCH-P0-003`："转移所有权"无 allocator 来源与终态回收协议，
且根 README「创建侧释放」与 job README「提交方转移输入所有权」直接矛盾——
按现文档实现必然出现 UAF、双重释放或泄漏。

## 决策

Buffer 分为 `BorrowedCallBuffer`（仅同步调用）、`CallerOutputBuffer`（仅本次调用写入、
不足返回所需长度）、`NativeOwnedBufferHandle`（唯一可进异步 Job 的形态，经 release API 回收）；
`SharedReadOnlyBufferHandle` V1 不做。NativeCore 永不释放调用方内存，调用方永不直接释放
Native 内存。Job 自 submit 起持有输入/输出租约，Cancel/Timeout 不释放租约，
直到真实终态 + reap；每条终态路径的回收责任见
[`ffi-buffer-ownership.md`](../../docs/specs/ffi-buffer-ownership.md) 的矩阵。

## 后果

- 借用字节进异步必须复制，接受一次拷贝成本；零拷贝路径要求调用方预先申请 Native Buffer。
- 每个 ABI 参数要带 provenance/valid-until/release 标注，Header 生成输入变重（待架构源 Schema）。
- 根 README 与 memory/job README 的所有权表述需按契约 §6 修正（任务卡在途）。
