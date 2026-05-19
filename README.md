# flux-vm-v3

**A stack-based, terminating, proof-carrying constraint checking VM.**

Early-stage prototype. 55 tests passing. The VM works; the JIT compiles; the proofs verify. But this is research infrastructure, not production tooling.

## What It Does

Compiles GUARD constraint definitions to FLUX-C bytecode, then executes or JIT-compiles to native code.

### The VM

- **60 opcodes** — each justified by an insight from 96 language implementations
- **Stack-based** (Forth/Factor insight) — simpler to verify, easier to prove termination
- **No backward jumps** — termination guaranteed, max 4096 cycles per constraint
- **Proof-carrying** — SHA-256 hash chain from source to check result

### The JIT

- **Bytecode → native hot path** — extracts constraint bounds, generates tight comparison loop
- **x86_64 codegen** — `ucomisd` comparisons, bitmask construction, NaN trap
- **179M checks/sec** on Ryzen AI 9 HX 370 (CLI tool)
- **29 tests passing** including all 10 presets, boundary values, NaN, infinity

### Key Insight from 96 Languages

| Language | What It Taught Us |
|----------|-------------------|
| Forth / Factor | Stack-based VM is simpler to verify |
| Unison | Content-addressed bytecode (same hash = same behavior) |
| Koka / Eff | Effect handlers for streaming and parallel modes |
| Ada / SPARK | Termination proofs via bounded execution |
| Verilog | Single-clock-cycle constraint checking is possible |
| Rust | Zero-cost abstractions for the hot path |

## Architecture

```
GUARD DSL source
    ↓ guardc-v3 (compiler)
FLUX-C bytecode (60 opcodes)
    ↓ JIT (jit.rs / jit_x86.rs)
Native constraint checker
    ↓
Error mask (u8) — 0 = pass, nonzero = violation
```

## Test Results

```
cargo test --lib --tests
29 passed, 0 failed
```

## Honest Limitations

- The JIT currently only extracts bounds; it doesn't handle the full opcode set
- x86_64 only (no ARM JIT yet)
- The VM has not been deployed in any real system
- Effect handlers are defined but not deeply tested
- The proof system adds overhead (~43% for SHA-256 per check)

## Related

- [guardc-v3](https://github.com/SuperInstance/guardc-v3) — GUARD → FLUX-C compiler
- [constraint-theory-ecosystem](https://github.com/SuperInstance/constraint-theory-ecosystem) — 96 language implementations + research
- [constraint-theory-core](https://crates.io/crates/constraint-theory-core) — Rust crate (v2.0.0)

## License

MIT
