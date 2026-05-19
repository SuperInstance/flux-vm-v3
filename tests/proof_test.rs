use flux_vm_v3::proof::ProofContext;

#[test]
fn test_proof_chain_grows() {
    let mut ctx = ProofContext::new();
    assert_eq!(ctx.chain_len(), 0);
    ctx.prove_value(42);
    assert_eq!(ctx.chain_len(), 1);
    ctx.prove_value(99);
    assert_eq!(ctx.chain_len(), 2);
}

#[test]
fn test_proof_range() {
    let mut ctx = ProofContext::new();
    let h1 = ctx.prove_range(50, 0, 100, true);
    let h2 = ctx.prove_range(200, 0, 100, false);
    assert_ne!(h1, h2, "different proofs should hash differently");
}

#[test]
fn test_proof_commit() {
    let mut ctx = ProofContext::new();
    ctx.prove_value(42);
    ctx.prove_value(99);
    let commit = ctx.commit();
    assert_ne!(commit, [0u8; 32]);
    assert!(ctx.chain_len() >= 3); // prove, prove, commit
}

#[test]
fn test_proof_seal() {
    let mut ctx = ProofContext::new();
    ctx.prove_value(42);
    let seal = ctx.seal().unwrap();
    assert_ne!(seal, [0u8; 32]);
    assert!(ctx.is_sealed());
    // Seal again should fail
    assert!(ctx.seal().is_err());
}

#[test]
fn test_proof_verify() {
    let mut ctx = ProofContext::new();
    ctx.prove_value(42);
    let root = ctx.root_hash().unwrap();
    assert!(ctx.verify(&root));
    assert!(!ctx.verify(&[0u8; 32]));
}

#[test]
fn test_proof_reset() {
    let mut ctx = ProofContext::new();
    ctx.prove_value(42);
    ctx.reset();
    assert_eq!(ctx.chain_len(), 0);
    assert!(!ctx.is_sealed());
}
