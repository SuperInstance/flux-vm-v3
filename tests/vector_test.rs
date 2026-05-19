use flux_vm_v3::vector::VectorUnit;
use flux_vm_v3::error::FluxError;

#[test]
fn test_vector_load_store() {
    let mut vu = VectorUnit::new();
    let data = [1, 2, 3, 4, 5, 6, 7, 8];
    vu.load(0, data).unwrap();
    assert_eq!(vu.store(0).unwrap(), data);
}

#[test]
fn test_vector_range_check_all_pass() {
    let mut vu = VectorUnit::new();
    vu.load(0, [10, 20, 30, 40, 50, 60, 70, 80]).unwrap();
    let mask = vu.range_check(0, 0, 100).unwrap();
    assert_eq!(mask, 0xff);
}

#[test]
fn test_vector_range_check_partial() {
    let mut vu = VectorUnit::new();
    vu.load(0, [5, 15, 25, 35, 45, 55, 65, 75]).unwrap();
    // lo=10, hi=50 => lanes 0 (5<10) and 6,7 fail
    let mask = vu.range_check(0, 10, 50).unwrap();
    // lanes: 5(F), 15(T), 25(T), 35(T), 45(T), 55(F), 65(F), 75(F)
    // mask: 0b01111010 = 0x7a... wait let me recalculate
    // bit 0: 5 -> fail (0)
    // bit 1: 15 -> pass (1)
    // bit 2: 25 -> pass (1)
    // bit 3: 35 -> pass (1)
    // bit 4: 45 -> pass (1)
    // bit 5: 55 -> fail (0)
    // bit 6: 65 -> fail (0)
    // bit 7: 75 -> fail (0)
    assert_eq!(mask, 0b00011110);
}

#[test]
fn test_vector_reduce() {
    let mut vu = VectorUnit::new();
    vu.load(0, [1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
    assert_eq!(vu.reduce(0).unwrap(), 36);
}

#[test]
fn test_vector_gather() {
    let mut vu = VectorUnit::new();
    vu.load(0, [10, 20, 30, 40, 50, 60, 70, 80]).unwrap();
    vu.gather(1, 0, [3, 2, 1, 0, 7, 6, 5, 4]).unwrap();
    let result = vu.store(1).unwrap();
    assert_eq!(result, [40, 30, 20, 10, 80, 70, 60, 50]);
}

#[test]
fn test_vector_invalid_reg() {
    let vu = VectorUnit::new();
    assert!(matches!(vu.store(4), Err(FluxError::InvalidRegister(4))));
    assert!(matches!(vu.store(5), Err(FluxError::InvalidRegister(5))));
}

#[test]
fn test_mask_merge() {
    let m1 = 0b11111111u8;
    let m2 = 0b11110000u8;
    let m3 = 0b11001100u8;
    let merged = VectorUnit::mask_merge(&[m1, m2, m3]);
    assert_eq!(merged, 0b11000000);
}

#[test]
fn test_batch_range_check() {
    let mut vu = VectorUnit::new();
    vu.load(0, [10, 20, 30, 40, 50, 60, 70, 80]).unwrap();
    vu.load(1, [10, 20, 30, 40, 50, 60, 70, 80]).unwrap();
    vu.load(2, [10, 20, 30, 40, 50, 60, 70, 80]).unwrap();
    vu.load(3, [10, 20, 30, 40, 50, 60, 70, 80]).unwrap();
    let mask = vu.batch_range_check(0, 100).unwrap();
    assert_eq!(mask, 0xff);
}
