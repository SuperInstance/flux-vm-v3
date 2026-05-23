# Flux VM v3 — Developer Guide

> Extending the 60-opcode constraint-checking VM.

---

## Architecture

The VM is **stack-based**, **deterministic**, and **terminating**:

| Property | Guarantee |
|----------|-----------|
| Max cycles | 4096 per constraint |
| Determinism | Same bytecode → same result, always |
| Proof | SHA-256 hash chain from source to result |

```
GUARD source → FLUX-C bytecode → JIT / interpreter → Error mask (u8) → Proof certificate
```

## The 60 Opcodes

### Stack manipulation
- `PUSH`, `POP`, `DUP`, `SWAP`

### Arithmetic
- `ADD`, `SUB`, `MUL`, `DIV`

### Comparison
- `LT`, `GT`, `EQ`, `LTE`, `GTE`

### Bounds checking
- `CHECK_LO`, `CHECK_HI`, `CHECK_RANGE`

### Bitwise
- `AND`, `OR`, `XOR`, `NOT`

### Control
- `JMP`, `JMP_IF`, `HALT`

### Effects
- `EMIT`, `STREAM`, `PARALLEL`

### Proof
- `HASH`, `MERKLE`, `CERTIFY`

### Safety
- `NAN_CHECK`

### Why no backward jumps?

Backward jumps enable loops. Loops can run forever. A constraint checker that runs forever is wrong by definition. Every FLUX-C program has bounded execution length — the compiler enforces this.

## Adding a New Opcode

### 1. Define the opcode

In `src/opcodes.rs`:

```rust
pub enum Opcode {
    // ... existing opcodes ...
    MY_NEW_OP,  // <-- add
}
```

### 2. Implement execution

In `src/vm.rs`, add to the `execute()` match:

```rust
Opcode::MY_NEW_OP => {
    let a = self.stack.pop()?;
    let b = self.stack.pop()?;
    self.stack.push(a.wrapping_add(b));
    self.cycles += 1;
}
```

### 3. Add tests

In `tests/vm_tests.rs`:

```rust
#[test]
fn test_my_new_op() {
    let mut vm = VM::new();
    vm.load_bytecode(vec![
        Opcode::PUSH as u8, 5,
        Opcode::PUSH as u8, 3,
        Opcode::MY_NEW_OP as u8,
        Opcode::HALT as u8,
    ]);
    let result = vm.execute();
    assert_eq!(result.error_mask(), 0);  // no errors
    assert_eq!(vm.stack.pop().unwrap(), 8);
}
```

### 4. Update docs

Add to README.md opcode table and this DEVELOPER.md.

## FFI for Python

The C FFI allows Python to call the VM via `ctypes`:

```rust
// src/ffi.rs
#[no_mangle]
pub extern "C" fn flux_check_batch(
    bytecode: *const u8,
    bytecode_len: usize,
    inputs: *const f64,
    input_len: usize,
    results: *mut u8,
    result_len: usize,
) -> i32 {
    // ... implementation ...
}
```

### Python usage

```python
import ctypes

lib = ctypes.CDLL("./target/release/libflux_vm.so")

# flux_check_batch(bytecode, bytecode_len, inputs, input_len, results, result_len)
lib.flux_check_batch.argtypes = [
    ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t,
    ctypes.POINTER(ctypes.c_double), ctypes.c_size_t,
    ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t,
]
lib.flux_check_batch.restype = ctypes.c_int32

# Call
bytecode = [...]  # your compiled FLUX-C
inputs = [3000.0]  # value to check
results = (ctypes.c_uint8 * 1)()
lib.flux_check_batch(
    (ctypes.c_uint8 * len(bytecode))(*bytecode),
    len(bytecode),
    (ctypes.c_double * len(inputs))(*inputs),
    len(inputs),
    results,
    1,
)
print(f"Error mask: {results[0]:08b}")  # 0 = pass
```

## Writing a Custom Preset

Presets are constraint definitions for specific domains:

```rust
// src/presets.rs
pub fn automotive_can() -> ConstraintSet {
    ConstraintSet::new()
        .add("rpm", Constraint::range(0.0, 8000.0))
        .add("temp", Constraint::range(-40.0, 150.0))
        .add("pressure", Constraint::range(0.0, 500.0))
}
```

Register in `src/presets.rs` and add CLI command in `src/cli.rs`.

## Benchmarking

```bash
# Individual preset
cargo run --release -- bench --preset automotive_can --iterations 1000000

# All presets
cargo test --release bench
```

Expected: 179M checks/sec on modern x86_64.

## Common Pitfalls

1. **Cycle limit** — VM halts at 4096 cycles. If your constraint is complex, split it into multiple checks.
2. **NaN propagation** — Use `NAN_CHECK` after any division or sqrt.
3. **Stack underflow** — VM returns error mask bit 7 for stack underflow. Ensure your bytecode balances pushes and pops.
4. **Determinism** — Don't use `std::time` or randomness in constraints. The VM must produce identical results for identical inputs.

## Proof Certificates

The VM generates a SHA-256 hash chain:

```
source_code_hash → bytecode_hash → input_hash → result_hash
```

This creates a tamper-evident audit trail. To verify:

```rust
let cert = vm.certificate();
assert!(cert.verify(source_code, input, result));
```

## File Layout

```
flux-vm-v3/
├── src/
│   ├── lib.rs          # Public API
│   ├── vm.rs           # Stack machine, execute loop
│   ├── opcodes.rs      # Opcode enum
│   ├── compiler.rs     # Source → bytecode
│   ├── ffi.rs          # C FFI for Python
│   ├── presets.rs      # Industry constraint sets
│   └── proof.rs        # SHA-256 certificate chain
├── tests/
│   └── vm_tests.rs     # Opcode + integration tests
├── benches/
│   └── throughput.rs   # 179M checks/sec benchmark
└── README.md           # User-facing docs
```

## Quick Reference

| Task | File | Command |
|------|------|---------|
| Add opcode | `src/opcodes.rs`, `src/vm.rs` | `cargo test` |
| Add preset | `src/presets.rs`, `src/cli.rs` | `cargo run -- preset` |
| FFI binding | `src/ffi.rs` | `cargo build --release` |
| Benchmark | `benches/throughput.rs` | `cargo bench` |
| Test all | `tests/` | `cargo test --release` |

---

*Last updated: 2026-05-23*
