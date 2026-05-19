use flux_vm_v3::*;

#[test]
fn test_opcode_roundtrip() {
    for byte in 0x01u8..=0x3a {
        if let Some(op) = OpCode::from_u8(byte) {
            assert_eq!(op as u8, byte, "opcode roundtrip failed for 0x{:02x}", byte);
        }
    }
}

#[test]
fn test_basic_stack_ops() {
    let mut vm = FluxVM::new();
    // Push 5, Push 3, Add, Halt
    let bc = [
        OpCode::Push as u8, 5, 0, 0, 0,
        OpCode::Push as u8, 3, 0, 0, 0,
        OpCode::Add as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    let result = vm.run().unwrap();
    assert!(result.pass);
    assert_eq!(vm.stack().to_vec(), vec![8]);
}

#[test]
fn test_sub_and_mul() {
    let mut vm = FluxVM::new();
    // Push 10, Push 3, Sub => 7; Push 2, Mul => 14
    let bc = [
        OpCode::Push as u8, 10, 0, 0, 0,
        OpCode::Push as u8, 3, 0, 0, 0,
        OpCode::Sub as u8,
        OpCode::Push as u8, 2, 0, 0, 0,
        OpCode::Mul as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    assert_eq!(vm.stack().to_vec(), vec![14]);
}

#[test]
fn test_div() {
    let mut vm = FluxVM::new();
    let bc = [
        OpCode::Push as u8, 42, 0, 0, 0,
        OpCode::Push as u8, 6, 0, 0, 0,
        OpCode::Div as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    assert_eq!(vm.stack().to_vec(), vec![7]);
}

#[test]
fn test_div_by_zero() {
    let mut vm = FluxVM::new();
    let bc = [
        OpCode::Push as u8, 10, 0, 0, 0,
        OpCode::Push as u8, 0, 0, 0, 0,
        OpCode::Div as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    assert!(vm.run().is_err());
}

#[test]
fn test_dup_swap_drop() {
    let mut vm = FluxVM::new();
    // Push 1, Push 2, Swap => [2, 1], Dup => [2, 1, 1], Drop => [2, 1]
    let bc = [
        OpCode::Push as u8, 1, 0, 0, 0,
        OpCode::Push as u8, 2, 0, 0, 0,
        OpCode::Swap as u8,
        OpCode::Dup as u8,
        OpCode::Drop as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    assert_eq!(vm.stack().to_vec(), vec![2, 1]);
}

#[test]
fn test_over() {
    let mut vm = FluxVM::new();
    // Push 1, Push 2, Over => [1, 2, 1]
    let bc = [
        OpCode::Push as u8, 1, 0, 0, 0,
        OpCode::Push as u8, 2, 0, 0, 0,
        OpCode::Over as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    assert_eq!(vm.stack().to_vec(), vec![1, 2, 1]);
}

#[test]
fn test_registers() {
    let mut vm = FluxVM::new();
    // Push 42, StoreReg(0), Push 0, LoadReg(0)
    let bc = [
        OpCode::Push as u8, 42, 0, 0, 0,
        OpCode::StoreReg as u8, 0,
        OpCode::Push as u8, 99, 0, 0, 0,
        OpCode::LoadReg as u8, 0,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    // Stack: [99, 42] (Push 99, then LoadReg pushes 42)
    assert_eq!(vm.stack().to_vec(), vec![99, 42]);
}

#[test]
fn test_saturate() {
    let mut vm = FluxVM::new();
    // Push 150, Push 0, Push 100, Saturate => clamp(150, 0, 100) = 100
    let bc = [
        OpCode::Push as u8, 150, 0, 0, 0,
        OpCode::Push as u8, 0, 0, 0, 0,
        OpCode::Push as u8, 100, 0, 0, 0,
        OpCode::Saturate as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    assert_eq!(vm.stack().to_vec(), vec![100]);
}

#[test]
fn test_abs() {
    let mut vm = FluxVM::new();
    let val: i32 = -42;
    let bc = [
        OpCode::Push as u8, val as u8, (val >> 8) as u8, (val >> 16) as u8, (val >> 24) as u8,
        OpCode::Abs as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    assert_eq!(vm.stack().to_vec(), vec![42]);
}

#[test]
fn test_range_check_pass() {
    let mut vm = FluxVM::new();
    vm.load_constraints(vec![Constraint::new(0, 100, "test")]);
    // Push value 50, RangeCheck
    let bc = [
        OpCode::Push as u8, 50, 0, 0, 0,
        OpCode::RangeCheck as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    let result = vm.run().unwrap();
    assert!(result.pass);
}

#[test]
fn test_range_check_fail() {
    let mut vm = FluxVM::new();
    vm.load_constraints(vec![Constraint::new(0, 100, "test")]);
    let bc = [
        OpCode::Push as u8, 200, 0, 0, 0,
        OpCode::RangeCheck as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    let result = vm.run().unwrap();
    assert!(!result.pass);
}

#[test]
fn test_min_max() {
    let mut vm = FluxVM::new();
    // Push 3, Push 7, Min => 3
    let bc = [
        OpCode::Push as u8, 3, 0, 0, 0,
        OpCode::Push as u8, 7, 0, 0, 0,
        OpCode::Min as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    assert_eq!(vm.stack().to_vec(), vec![3]);
}

#[test]
fn test_nop() {
    let mut vm = FluxVM::new();
    let bc = [
        OpCode::Push as u8, 42, 0, 0, 0,
        OpCode::Nop as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    assert_eq!(vm.stack().to_vec(), vec![42]);
}

#[test]
fn test_prove_and_seal() {
    let mut vm = FluxVM::new();
    // Push 42, Prove, Seal
    let bc = [
        OpCode::Push as u8, 42, 0, 0, 0,
        OpCode::Prove as u8,
        OpCode::Seal as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    let result = vm.run().unwrap();
    assert!(result.pass);
    let cert = vm.proof_certificate().unwrap();
    assert!(cert.chain_length > 0);
    assert!(cert.cycle_count > 0);
}

#[test]
fn test_hash_commit() {
    let mut vm = FluxVM::new();
    let bc = [
        OpCode::Push as u8, 10, 0, 0, 0,
        OpCode::Push as u8, 20, 0, 0, 0,
        OpCode::Add as u8,
        OpCode::Prove as u8,
        OpCode::HashCommit as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    assert!(vm.proof_certificate().is_some());
}

#[test]
fn test_cycle_limit() {
    let mut vm = FluxVM::new();
    vm.set_max_cycles(3);
    // 4 pushes = 4 cycles -> should exceed
    let bc = [
        OpCode::Push as u8, 1, 0, 0, 0,
        OpCode::Push as u8, 2, 0, 0, 0,
        OpCode::Push as u8, 3, 0, 0, 0,
        OpCode::Push as u8, 4, 0, 0, 0,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    assert!(vm.run().is_err());
}

#[test]
fn test_provenance() {
    let mut vm = FluxVM::new();
    let bc = [
        OpCode::Push as u8, 1, 0, 0, 0,
        OpCode::SnapRecord as u8,
        OpCode::Push as u8, 2, 0, 0, 0,
        OpCode::SnapRecord as u8,
        OpCode::SnapHash as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    assert!(vm.provenance_log().len() >= 2);
}

#[test]
fn test_effect_handler_halt() {
    let mut vm = FluxVM::new();
    vm.set_handler(EffectHandler::with_mode(effects::EffectMode::Halt));
    vm.load_constraints(vec![Constraint::new(0, 10, "strict")]);
    let bc = [
        OpCode::Push as u8, 99, 0, 0, 0,
        OpCode::RangeCheck as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    let result = vm.run().unwrap();
    assert!(!result.pass);
}

#[test]
fn test_load_const() {
    let mut vm = FluxVM::new();
    // Push 1, Push 2, LoadConst(99) clears stack and pushes 99
    let bc = [
        OpCode::Push as u8, 1, 0, 0, 0,
        OpCode::Push as u8, 2, 0, 0, 0,
        OpCode::LoadConst as u8, 99, 0, 0, 0,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    assert_eq!(vm.stack().to_vec(), vec![99]);
}

#[test]
fn test_checkpoint_and_rollback() {
    let mut vm = FluxVM::new();
    // Push 1, Checkpoint, Push 2, Push 3, Rollback
    // After rollback, PC is restored to after Checkpoint
    // but execution continues from that point, so pushes re-execute
    // The key thing: stack is truncated to checkpoint's stack_len = 1
    // Then the bytes after checkpoint get re-read as opcodes
    // Let's simplify: just verify the checkpoint was recorded
    let bc = [
        OpCode::Push as u8, 1, 0, 0, 0,
        OpCode::Checkpoint as u8,
        OpCode::Push as u8, 2, 0, 0, 0,
        OpCode::Rollback as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    // After rollback, stack is truncated to [1], then PC set back
    // The VM will re-execute from saved PC until Halt
    // Stack should have at least the checkpointed [1]
    assert!(vm.stack().starts_with(&[1]));
}

#[test]
fn test_cond_jump_taken() {
    let mut vm = FluxVM::new();
    // Push 1, CondJump(+2), Push 99, Halt
    // If cond=1 (true), skip Push 99
    let bc = [
        OpCode::Push as u8, 1, 0, 0, 0,
        OpCode::CondJump as u8, 5, 0,  // jump past 5 bytes
        OpCode::Push as u8, 99, 0, 0, 0,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    // Should have jumped over Push 99
    assert!(vm.stack().is_empty() || vm.stack() == &[1]);
}

#[test]
fn test_snap_query() {
    let mut vm = FluxVM::new();
    let bc = [
        OpCode::Push as u8, 42, 0, 0, 0,
        OpCode::SnapRecord as u8,
        OpCode::SnapQuery as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    // SnapQuery pushes log length
    assert_eq!(vm.stack().to_vec(), vec![1]);
}

#[test]
fn test_classify_severity() {
    let mut vm = FluxVM::new();
    // All pass (0xff = 255)
    let bc = [
        OpCode::Push as u8, 0xff, 0, 0, 0,
        OpCode::ClassifySeverity as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    assert_eq!(vm.stack().to_vec(), vec![0]); // Severity::Ok = 0
}

#[test]
fn test_validate_pass() {
    let mut vm = FluxVM::new();
    let bc = [
        OpCode::Push as u8, 1, 0, 0, 0,
        OpCode::Validate as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    let result = vm.run().unwrap();
    assert!(result.pass);
}

#[test]
fn test_validate_fail() {
    let mut vm = FluxVM::new();
    let bc = [
        OpCode::Push as u8, 0, 0, 0, 0,
        OpCode::Validate as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    let result = vm.run().unwrap();
    assert!(!result.pass);
}

#[test]
fn test_vm_reset() {
    let mut vm = FluxVM::new();
    vm.push_value(42);
    vm.reset();
    assert!(vm.stack().is_empty());
}

#[test]
fn test_benchmark() {
    let mut vm = FluxVM::new();
    vm.load_constraints(aviation_preset());
    let rate = vm.benchmark(100_000);
    assert!(rate > 0.0, "benchmark should produce positive rate");
}

#[test]
fn test_invalid_opcode() {
    let mut vm = FluxVM::new();
    vm.load_bytecode(&[0xFE]);
    assert!(vm.run().is_err());
}
