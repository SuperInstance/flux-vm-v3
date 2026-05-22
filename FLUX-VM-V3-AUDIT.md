# FLUX-VM-V3 Security & Architecture Audit

**Auditor:** Deep-review agent (Kimi Code CLI)  
**Date:** 2026-05-22  
**Commit:** HEAD (post-audit fixes applied)  
**Scope:** Full source review, test execution, security analysis, termination verification

---

## Executive Summary

flux-vm-v3 is a stack-based constraint-checking VM written in Rust. It claims to be *proof-carrying, SIMD-native, terminating, and safety-certifiable*. While the architecture is sound in principle, the implementation contains **multiple critical security and correctness bugs** that make the "safety-certifiable" claim premature. Most issues stem from missing bounds checks, silent failure modes, and integer-overflow panics in the interpreter hot path.

**Code Quality Score: 5 / 10**  
*Reasonable structure, good test coverage, but fundamental safety gaps for a certifiable VM.*

---

## 1. Architecture Summary

| Component | File | Description |
|-----------|------|-------------|
| **VM Core** | `src/vm.rs` | Stack machine (256-slot limit), 60 opcodes, cycle-counting execution loop |
| **Opcodes** | `src/opcode.rs` | Enum + `from_u8` decoder, immediate-byte metadata |
| **Memory** | `src/memory.rs` | 4 KB fixed heap, bounds-checked read/write |
| **Vector/SIMD** | `src/vector.rs` | 4 × 8-lane i8 registers, range-check & reduce ops |
| **JIT** | `src/jit.rs`, `src/jit_x86.rs` | Extracts f64 constraints, generates x86_64 machine code (**never executed**; falls back to Rust loop) |
| **Proof** | `src/proof.rs` | SHA-256 hash chain (`prove_value`, `prove_range`, `commit`, `seal`) |
| **Provenance** | `src/provenance.rs` | 1024-entry ring-buffer audit log |
| **Effects** | `src/effects.rs` | Silent/Log/Halt/Broadcast mode handler |
| **Streaming** | `src/streaming.rs` | Buffered batch range checker |
| **Parallel** | `src/parallel.rs` | Rayon-based parallel constraint dispatch |
| **Check** | `src/check.rs` | Constraint definition + aviation/temperature presets |
| **FFI** | `src/ffi.rs` | **DOES NOT EXIST** — referenced in task but never implemented |

### Execution Pipeline
```
Bytecode → VM::run() → tick() (cycle meter) → execute(op) → Result<VmResult>
                              ↑
                    max_cycles (default 4096)
```

### Termination Model
The VM enforces termination via **per-instruction cycle counting**. `tick()` increments `cycle_count` and returns `Err(CycleLimitExceeded)` when `cycle_count > max_cycles`. The default ceiling is 4,096 cycles. There are **no backward jump opcodes** (`JMP`/`JMP_IF` are forward-only), which removes unbounded loops *at the opcode level*. However, `CallBounded` + `Ret` and `Rollback` can still create recursive/backward control flow, so the cycle counter is the actual termination guarantee.

---

## 2. Test Results

### Before Audit Fixes
```
84 passed; 0 failed; 0 ignored
```

### After Audit Fixes (+ Regression Tests)
```
89 passed; 0 failed; 0 ignored
```

| Suite | Count | Status |
|-------|-------|--------|
| Unit tests (`src/`) | 14 | ✅ Pass |
| Golden tests | 7 | ✅ Pass |
| JIT tests | 15 | ✅ Pass |
| Proof tests | 6 | ✅ Pass |
| Streaming tests | 5 | ✅ Pass |
| Vector tests | 8 | ✅ Pass |
| VM tests | 34 | ✅ Pass |

**Note:** Tests were green before fixes, which means the existing suite did **not** cover edge cases such as `i32::MIN / -1`, negative `BatchCheck` counts, or 8-constraint JIT NaN handling.

---

## 3. Security Findings

### 🔴 Critical

#### C1. Integer Division Overflow (`i32::MIN / -1`) — Panic / Wrong Result
- **File:** `src/vm.rs:256`
- **Bug:** The `Div` opcode uses native Rust `/` on `i32`. `i32::MIN / -1` panics in debug builds and wraps to `i32::MIN` in release builds.
- **Impact:** Untrusted bytecode can crash the VM (debug) or silently produce a mathematically incorrect result (release).
- **Fix Applied:** Added explicit check: `if a == i32::MIN && b == -1 { return Err(FluxError::Overflow); }`

#### C2. Integer Abs Overflow (`abs(i32::MIN)`) — Panic / Wrong Result
- **File:** `src/vm.rs:278`
- **Bug:** The `Abs` opcode calls `v.abs()`. For `i32::MIN` this panics in debug and returns `i32::MIN` (still negative) in release.
- **Impact:** Same as C1 — crash or silent corruption of constraint checking logic.
- **Fix Applied:** Added explicit check: `if v == i32::MIN { return Err(FluxError::Overflow); }`

#### C3. BatchCheck Denial of Service (Negative → usize::MAX)
- **File:** `src/vm.rs:349`
- **Bug:** `BatchCheck` pops a `count` and casts it `as usize`. A value of `-1` becomes `usize::MAX`. The loop then spins for ~2⁶⁴ iterations without consuming any additional cycles.
- **Impact:** Single instruction can hang the host indefinitely, bypassing the cycle limiter.
- **Fix Applied:** Reject negative counts and cap loop to `self.stack.len()`.

#### C4. JIT NaN Mask Shift Overflow (8 Constraints)
- **File:** `src/jit.rs:398`
- **Bug:** `check_rust` computes `(1u8 << self.n) - 1` for NaN. When `n == 8`, shifting a `u8` by 8 is undefined-behaviour-adjacent in Rust (panics in debug, wraps to 0 in release), producing the wrong mask.
- **Impact:** With exactly 8 constraints, a NaN input crashes the JIT in debug builds or returns mask `0xFF` only by accident of wrapping arithmetic in release.
- **Fix Applied:** Replaced with `if self.n >= 8 { 0xFF } else { (1u8 << self.n) - 1 }`.

### 🟠 High

#### H1. Unbounded Call Stack Growth
- **File:** `src/vm.rs:477`
- **Bug:** `CallBounded` pushed return addresses to `call_stack: Vec<usize>` without any size limit. A malicious bytecode could exhaust host memory before hitting the cycle limit (if `max_cycles` is raised).
- **Fix Applied:** Enforced `STACK_LIMIT` (256) on `call_stack`.

#### H2. Unbounded Checkpoint Growth
- **File:** `src/vm.rs:494`
- **Bug:** `Checkpoint` pushed to `checkpoints: Vec<Checkpoint>` without bound. Same OOM vector as H1.
- **Fix Applied:** Enforced `STACK_LIMIT` (256) on `checkpoints`.

#### H3. Memory Bounds-Check Integer Overflow
- **File:** `src/memory.rs:28`, `src/memory.rs:37`
- **Bug:** `write` and `read` compute `offset + len > HEAP_SIZE`. If `offset` and `len` are both large, the sum can wrap around `usize`, passing the check and causing a panic on slice indexing.
- **Impact:** Safe panic (no UB), but a certifiable VM must not panic on untrusted input.
- **Fix Applied:** Used `checked_add` and returned `MemoryExceeded` on overflow.

### 🟡 Medium

#### M1. StreamOpen Silently Swallows Stack Underflow
- **File:** `src/vm.rs:567`
- **Bug:** `let batch_size = self.pop().unwrap_or(64);` on a `FluxResult<i32>` turns `Err(StackUnderflow)` into the default value `64`. The VM silently continues instead of erroring.
- **Fix Applied:** Changed to `self.pop()?` and applied `.max(1)` to ensure valid batch size.

#### M2. Silent Truncation of Bytecode Immediates
- **File:** `src/vm.rs:125–150`
- **Bug:** `read_i32`, `read_u16`, and `read_u8` return `0` when the bytecode is truncated. A malformed program silently executes with zero operands rather than failing.
- **Fix:** Not yet applied (requires refactoring return types to `FluxResult`). Recommended for follow-up.

#### M3. QueryBackward Documentation / Implementation Mismatch
- **File:** `src/vm.rs:390`
- **Bug:** Comment says "return hash at that position", but the code pushes the `depth` index as an `i32` instead of the hash.
- **Fix:** Not yet applied. Either update docs or implement hash retrieval.

### 🟢 Low

#### L1. SnapVerify Is a No-Op Stub
- **File:** `src/vm.rs:561`
- **Bug:** `SnapVerify` unconditionally pushes `1` (pass). For a proof-carrying VM, a verify opcode that always passes undermines the security model.

#### L2. Unnecessary `unsafe impl Sync/Send`
- **File:** `src/jit.rs:324`
- **Bug:** `JitChecker` contains only `Vec<f64>` and `Option<Vec<u8>>`, which are already `Send + Sync`. The manual `unsafe impl` demonstrates a misunderstanding of Rust's auto-trait rules.
- **Fix Applied:** Removed the unnecessary unsafe impls.

#### L3. JIT Generates Dead Code
- **File:** `src/jit.rs:388`, `src/jit_x86.rs`
- **Bug:** `JitChecker::check()` explicitly calls `check_rust()` and **never executes** the generated x86_64 bytes. The entire `jit_x86.rs` module is dead code in practice. The README claims "179M checks/sec" from native execution, but the fallback Rust path is what actually runs.

#### L4. Missing FFI Layer
- **File:** `src/ffi.rs` (expected)
- **Bug:** The task explicitly requested auditing `src/ffi.rs`, but the file does not exist. No C ABI bindings, no `#[no_mangle]` exports, no `unsafe` FFI blocks. This makes the "safety-certifiable with FFI" claim unfounded.

---

## 4. Termination Verification

| Claim | Status | Evidence |
|-------|--------|----------|
| "Every program terminates" | ⚠️ Partial | Cycle limiter (`tick()`) is the sole enforcement. Default `max_cycles = 4096`. |
| "No backward jumps" | ⚠️ Partial | `FwdJump`/`CondJump` are forward-only, but `CallBounded`+`Ret` and `Rollback` enable backward/recursive control flow. |
| "Bounded stack depth" | ✅ Fixed | `STACK_LIMIT = 256` enforces operand stack limit. Call stack & checkpoints now also bounded (fixes H1/H2). |
| "Compiler enforces bounded execution" | ❌ N/A | There is no compiler in this repo; it's a pure runtime VM. The claim in README refers to an external `guardc-v3` compiler. |

**Verdict:** The VM *can* enforce termination **if** `max_cycles` is kept at a reasonable default and the recently applied call-stack/checkpoint limits are in place. However, the `BatchCheck` pre-fix bug (C3) showed that a single opcode could perform unbounded work, bypassing the per-instruction cycle meter. A truly certifiable VM should also enforce a **maximum per-opcode work bound** independent of the global cycle budget.

---

## 5. Specific Bugs & Fixes (File:Line)

| ID | Severity | File:Line | Bug | Fix |
|----|----------|-----------|-----|-----|
| C1 | Critical | `src/vm.rs:256` | `Div` uses unchecked `/` | Added `i32::MIN / -1` overflow guard |
| C2 | Critical | `src/vm.rs:278` | `Abs` uses unchecked `.abs()` | Added `i32::MIN` guard |
| C3 | Critical | `src/vm.rs:349` | `BatchCheck` casts negative `count` to `usize` | Reject negative, cap at `stack.len()` |
| C4 | Critical | `src/jit.rs:398` | `(1u8 << 8)` shift overflow on NaN | Use `if n >= 8 { 0xFF }` guard |
| H1 | High | `src/vm.rs:477` | `CallBounded` unbounded `call_stack` | Enforce `STACK_LIMIT` |
| H2 | High | `src/vm.rs:494` | `Checkpoint` unbounded growth | Enforce `STACK_LIMIT` |
| H3 | High | `src/memory.rs:28` | `offset + len` can overflow `usize` | Use `checked_add` |
| M1 | Medium | `src/vm.rs:567` | `StreamOpen` swallows `Err` with `unwrap_or` | Use `?` propagation |
| L2 | Low | `src/jit.rs:324` | Spurious `unsafe impl Sync/Send` | Removed |

---

## 6. Recommended Next Steps

1. **Bytecode Pre-Validation** — Before `run()`, scan bytecode to ensure all immediate operands are fully contained within the bytecode slice. Replace silent-zero reads with hard errors.
2. **Per-Opcode Work Limits** — Cap the loop iterations inside `BatchCheck`, `StreamBatch`, and any future vector ops to a fixed maximum (e.g., 256 or `STACK_LIMIT`).
3. **FFI Layer** — If external language interop is required, create `src/ffi.rs` using `cbindgen` + `#[no_mangle]` exports with rigorous `unsafe` documentation and Miri testing.
4. **JIT Execution** — Either delete `jit_x86.rs` (it's dead code) or implement proper `mmap(PROT_WRITE | PROT_EXEC)` + function-pointer casting to actually run the generated code. If kept, validate generated bytes with a disassembler before execution.
5. **Miri & Fuzzing** — Run `cargo miri test` to catch any latent UB, and use `cargo fuzz` on the bytecode decoder.
6. **Formal Spec** — A "certifiable" VM needs a formal specification of opcode semantics. The current behavior is implicit in the Rust implementation.
7. **Constant-Time Crypto** — `ProofContext` uses `sha2` crate. Ensure hash operations are not vulnerable to timing side-channels if proof verification is security-critical.

---

## 7. Audit Commit

All fixes above have been applied and committed to the repository.
