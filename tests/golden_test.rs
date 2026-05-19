use flux_vm_v3::*;

/// Golden vector tests — deterministic outputs for known inputs

#[test]
fn golden_push_add_halt() {
    let mut vm = FluxVM::new();
    let bc = [
        OpCode::Push as u8, 60, 0, 0, 0,
        OpCode::Push as u8, 40, 0, 0, 0,
        OpCode::Add as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    let r = vm.run().unwrap();
    assert_eq!(r.cycles, 4);
    assert!(r.pass);
    assert_eq!(vm.stack().to_vec(), vec![100]);
}

#[test]
fn golden_range_check_aviation_altitude() {
    let mut vm = FluxVM::new();
    vm.load_constraints(vec![Constraint::new(0, 45000, "altitude")]);
    // 35,000 ft — within range
    let bc = [
        OpCode::Push as u8, 0xD8, 0x88, 0x00, 0x00, // 35000 as i32 LE
        OpCode::RangeCheck as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    let r = vm.run().unwrap();
    assert!(r.pass);
}

#[test]
fn golden_range_check_out_of_bounds() {
    let mut vm = FluxVM::new();
    vm.load_constraints(vec![Constraint::new(0, 100, "test")]);
    let bc = [
        OpCode::Push as u8, 0xFF, 0xFF, 0x00, 0x00, // 65535
        OpCode::RangeCheck as u8,
        OpCode::Halt as u8,
    ];
    vm.load_bytecode(&bc);
    let r = vm.run().unwrap();
    assert!(!r.pass);
}

#[test]
fn golden_provenance_trace() {
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
    // Verify hash is deterministic
    let h1 = vm.provenance_log().hash();
    // Run again
    vm.reset();
    vm.load_bytecode(&bc);
    vm.run().unwrap();
    let h2 = vm.provenance_log().hash();
    assert_eq!(h1, h2, "provenance hash must be deterministic");
}

#[test]
fn golden_proof_determinism() {
    let mut ctx = ProofContext::new();
    ctx.prove_value(42);
    ctx.prove_value(99);
    let h1 = ctx.root_hash().unwrap();
    
    let mut ctx2 = ProofContext::new();
    ctx2.prove_value(42);
    ctx2.prove_value(99);
    let h2 = ctx2.root_hash().unwrap();
    
    assert_eq!(h1, h2, "proof chains must be deterministic");
}

#[test]
fn golden_parallel_batch() {
    let mut batch = ParallelBatch::new();
    batch.add_range(&[10, 20, 30, 40, 50], 0, 100);
    let result = batch.dispatch();
    assert!(result.all_pass());
    assert_eq!(result.total, 5);
    assert_eq!(result.passed, 5);
}

#[test]
fn golden_parallel_mixed() {
    let mut batch = ParallelBatch::new();
    batch.add_range(&[10, 200, 30, 400, 50], 0, 100);
    let result = batch.dispatch();
    assert!(!result.all_pass());
    assert_eq!(result.passed, 3);
    assert_eq!(result.failed, 2);
}
