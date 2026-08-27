# Gate-0 `lumio-contract-types` (RM-00002)

Cards in order (TDD): R-00056 T-contract-types-01, R-00069 T-contract-types-02, R-00072 T-contract-types-03, R-00074 T-contract-types-04.

Environment: `$env:PATH = "C:\Users\g923\.cargo\bin;" + $env:PATH` then `cargo +stable-x86_64-pc-windows-gnu ...`.

This crate now ships internal seams and negative gates only. It does **not** claim public ABI complete.

## TDD RED / GREEN

### R-00056 T-contract-types-01 `generated_contract_revision_is_readable`

**RED** (test file only; `generated.rs` / re-exports absent):

```text
cargo +stable-x86_64-pc-windows-gnu test -p lumio-contract-types --test generated_contract_revision_is_readable -- --nocapture
```

```text
   Compiling lumio-contract-types v0.0.0 (...)
error[E0432]: unresolved imports `lumio_contract_types::architecture_baseline_id`, `lumio_contract_types::verify_generated_contract_revision`, `lumio_contract_types::AbiVersion`, `lumio_contract_types::ArchitectureErrorCode`, `lumio_contract_types::ArchitectureOperationId`, `lumio_contract_types::CapabilityBits`, `lumio_contract_types::StructSize`
 --> crates\lumio-contract-types\tests\generated_contract_revision_is_readable.rs:2:5
  |
2 |     architecture_baseline_id, verify_generated_contract_revision, AbiVersion,
  |     ^^^^^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^ no `AbiVersion` in the root
  |     |                         |
  |     |                         no `verify_generated_contract_revision` in the root
  |     no `architecture_baseline_id` in the root
3 |     ArchitectureErrorCode, ArchitectureOperationId, CapabilityBits, StructSize,
  |     ^^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^  ^^^^^^^^^^ no `StructSize` in the root
  |     |                      |                        |
  |     |                      |                        no `CapabilityBits` in the root
  |     |                      no `ArchitectureOperationId` in the root
  |     no `ArchitectureErrorCode` in the root

error: could not compile `lumio-contract-types` (test "generated_contract_revision_is_readable") due to 1 previous error
```

**GREEN** (after `src/generated.rs` + controlled re-export):

```text
cargo +stable-x86_64-pc-windows-gnu test -p lumio-contract-types --test generated_contract_revision_is_readable -- --nocapture
```

```text
   Compiling lumio-contract-types v0.0.0 (...)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.78s
     Running tests\generated_contract_revision_is_readable.rs (...)

running 1 test
test generated_contract_revision_is_readable ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### R-00069 T-contract-types-02 `registry_values_are_unique`

**RED** (`registry` module absent):

```text
cargo +stable-x86_64-pc-windows-gnu test -p lumio-contract-types --test registry_values_are_unique -- --nocapture
```

```text
   Compiling lumio-contract-types v0.0.0 (...)
error[E0432]: unresolved import `lumio_contract_types::registry`
 --> crates\lumio-contract-types\tests\registry_values_are_unique.rs:1:5
  |
1 | use lumio_contract_types::registry;
  |     ^^^^^^^^^^^^^^^^^^^^^^--------
  |                           |
  |                           no `registry` in the root

error: could not compile `lumio-contract-types` (test "registry_values_are_unique") due to 1 previous error
```

**GREEN** (after `src/registry.rs` + `pub mod registry`):

```text
cargo +stable-x86_64-pc-windows-gnu test -p lumio-contract-types --test registry_values_are_unique -- --nocapture
```

```text
   Compiling lumio-contract-types v0.0.0 (...)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.32s
     Running tests\registry_values_are_unique.rs (...)

running 1 test
test registry_values_are_unique ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### R-00072 T-contract-types-03 `generated_layout_matches_manifest`

**RED** (`layout` module absent):

```text
cargo +stable-x86_64-pc-windows-gnu test -p lumio-contract-types --test generated_layout_matches_manifest -- --nocapture
```

```text
   Compiling lumio-contract-types v0.0.0 (...)
error[E0432]: unresolved import `lumio_contract_types::layout`
 --> crates\lumio-contract-types\tests\generated_layout_matches_manifest.rs:1:5
  |
1 | use lumio_contract_types::layout;
  |     ^^^^^^^^^^^^^^^^^^^^^^------
  |                           |
  |                           no `layout` in the root

error: could not compile `lumio-contract-types` (test "generated_layout_matches_manifest") due to 1 previous error
```

**GREEN** (after `src/layout.rs` + `pub mod layout`; empty table, no invented sizes):

```text
cargo +stable-x86_64-pc-windows-gnu test -p lumio-contract-types --test generated_layout_matches_manifest -- --nocapture
```

```text
   Compiling lumio-contract-types v0.0.0 (...)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.65s
     Running tests\generated_layout_matches_manifest.rs (...)

running 1 test
test generated_layout_matches_manifest ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### R-00074 T-contract-types-04 `wrong_baseline_is_rejected`

**RED** (`verify_generated_contract_revision_against` absent):

```text
cargo +stable-x86_64-pc-windows-gnu test -p lumio-contract-types --test wrong_baseline_is_rejected -- --nocapture
```

```text
   Compiling lumio-contract-types v0.0.0 (...)
error[E0432]: unresolved import `lumio_contract_types::verify_generated_contract_revision_against`
 --> crates\lumio-contract-types\tests\wrong_baseline_is_rejected.rs:3:5
  |
3 |     verify_generated_contract_revision_against, ContractMismatch,
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `verify_generated_contract_revision_against` in the root

error: could not compile `lumio-contract-types` (test "wrong_baseline_is_rejected") due to 1 previous error
```

**GREEN** (after drift-gate helper; `lib.rs` remains mods + re-exports):

```text
cargo +stable-x86_64-pc-windows-gnu test -p lumio-contract-types --test wrong_baseline_is_rejected -- --nocapture
```

```text
   Compiling lumio-contract-types v0.0.0 (...)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.62s
     Running tests\wrong_baseline_is_rejected.rs (...)

running 1 test
test wrong_baseline_is_rejected ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Final verification

```text
cargo +stable-x86_64-pc-windows-gnu fmt -p lumio-contract-types
cargo +stable-x86_64-pc-windows-gnu fmt -p lumio-contract-types -- --check
```

Exit 0 (no formatter output).

```text
cargo +stable-x86_64-pc-windows-gnu test -p lumio-contract-types
```

```text
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

test generated_contract_revision_is_readable ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

test generated_layout_matches_manifest ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

test registry_values_are_unique ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

test wrong_baseline_is_rejected ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
cargo +stable-x86_64-pc-windows-gnu xtask check-dep-dag
```

```text
check-dep-dag OK：10 个 crate，依赖方向全部符合白名单。
```

## Files changed

- `crates/lumio-contract-types/src/lib.rs` — mods, controlled re-export only
- `crates/lumio-contract-types/src/generated.rs` — created; opaque newtypes + baseline/revision gate
- `crates/lumio-contract-types/src/registry.rs` — created; empty unique query tables
- `crates/lumio-contract-types/src/layout.rs` — created; `verify_layout()` over empty Header rows
- `crates/lumio-contract-types/tests/generated_contract_revision_is_readable.rs`
- `crates/lumio-contract-types/tests/registry_values_are_unique.rs`
- `crates/lumio-contract-types/tests/generated_layout_matches_manifest.rs`
- `crates/lumio-contract-types/tests/wrong_baseline_is_rejected.rs`
- `GATE0_CONTRACT_TYPES.md` — this report

Not modified: `Cargo.toml`, `Cargo.lock`, other crates, architecture mirrors, docs, Workflow tokens.

## Known gaps (Blocked package / Header)

- Architecture source has published baseline id `LGE-V1.4-2026-08-27` but **not** a Rust/C package or C Header (FOUNDATION-W1 generator is still a draft).
- No handwritten public ErrorCode / Capability bit / Operation ID numeric registries. `ArchitectureErrorCode`, `ArchitectureOperationId`, and `CapabilityBits` are opaque newtypes with no public numeric constants.
- `registry::{error_codes,operation_ids,capability_bits}` return empty unique tables until a generated package exists.
- `layout::verify_layout()` succeeds as “no generated structs to check” (`layout::entries().len() == 0`). It does not invent ABI sizes.
- `architecture-contracts` feature was not added. Feature-gated generated POD (`AbiOpaqueHandle` / Buffer views) is not compiled in.
- Schemas were not copied from `LumioGameEngineArchitecture`.
- `verify_generated_contract_revision_against` takes `&'static str` so `ContractMismatch.found: &'static str` can name the observed id without intern/leak. Call sites with string literals (including the stale-baseline test) compile as `&str`.
- Knowledge / spec-steward skipped: exclusive file set forbade docs and `.spec/` edits. Mechanical Gate-0 seam only.

Status: **DONE_WITH_CONCERNS** (seams + negative gates delivered; public ABI package/header still Blocked)
