use std::os::raw::{c_float, c_int, c_uchar};
use std::slice;

/// C FFI: Check a batch of room latents against neural bounds.
///
/// Called from Python via ctypes:
///   flux_check_batch(latents_ptr, n_rooms, latent_dim,
///                    min_bound, max_bound, max_l2, max_var,
///                    violations_ptr)
///
/// Parameters:
///   latents:     (n_rooms × latent_dim) float32 array
///   n_rooms:     number of rooms
///   latent_dim:  dimensionality of each latent vector
///   min_bound:   minimum allowed value per dimension
///   max_bound:   maximum allowed value per dimension
///   max_l2:      maximum allowed L2 norm per room
///   max_var:     maximum allowed variance per room
///   violations:  output buffer (n_rooms bytes, 0=pass, 1=fail)
///
/// Returns:
///   0 on success, nonzero on error
#[no_mangle]
pub extern "C" fn flux_check_batch(
    latents: *const c_float,
    n_rooms: c_int,
    latent_dim: c_int,
    min_bound: c_float,
    max_bound: c_float,
    max_l2: c_float,
    max_var: c_float,
    violations: *mut c_uchar,
) -> c_int {
    if latents.is_null() || violations.is_null() {
        return -1;
    }

    let n = n_rooms as usize;
    let d = latent_dim as usize;
    let lats = unsafe { slice::from_raw_parts(latents, n * d) };
    let vio = unsafe { slice::from_raw_parts_mut(violations, n) };

    for i in 0..n {
        let room = &lats[i * d..(i + 1) * d];
        let mut violated = false;

        // 1. Bounds check
        for &val in room {
            if val < min_bound || val > max_bound {
                violated = true;
                break;
            }
        }

        // 2. L2 norm check (only if not already violated)
        if !violated {
            let l2 = room.iter().map(|v| v * v).sum::<f32>().sqrt();
            if l2 > max_l2 {
                violated = true;
            }
        }

        // 3. Variance check (only if not already violated)
        if !violated && d > 1 {
            let mean = room.iter().sum::<f32>() / d as f32;
            let var = room.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / d as f32;
            if var > max_var {
                violated = true;
            }
        }

        vio[i] = if violated { 1 } else { 0 };
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flux_check_batch_all_pass() {
        let latents = vec![
            0.0f32, 0.0f32, 0.0f32, 0.0f32,
            0.0f32, 0.0f32, 0.0f32, 0.0f32,
        ];
        let mut violations = vec![0u8; 2];

        let ret = flux_check_batch(
            latents.as_ptr(),
            2,
            4,
            -10.0,
            10.0,
            100.0,
            10.0,
            violations.as_mut_ptr(),
        );

        assert_eq!(ret, 0);
        assert_eq!(violations, vec![0, 0]);
    }

    #[test]
    fn test_flux_check_batch_bounds_violation() {
        let latents = vec![
            0.0f32, 0.0f32, 0.0f32, 0.0f32,
            20.0f32, 0.0f32, 0.0f32, 0.0f32,  // room 1 violates bounds
        ];
        let mut violations = vec![0u8; 2];

        let ret = flux_check_batch(
            latents.as_ptr(),
            2,
            4,
            -10.0,
            10.0,
            100.0,
            10.0,
            violations.as_mut_ptr(),
        );

        assert_eq!(ret, 0);
        assert_eq!(violations, vec![0, 1]);
    }

    #[test]
    fn test_flux_check_batch_l2_violation() {
        let latents = vec![
            0.0f32, 0.0f32, 0.0f32, 0.0f32,
            10.0f32, 10.0f32, 10.0f32, 10.0f32,  // L2 ≈ 20
        ];
        let mut violations = vec![0u8; 2];

        let ret = flux_check_batch(
            latents.as_ptr(),
            2,
            4,
            -50.0,
            50.0,
            15.0,  // max L2 = 15
            100.0,
            violations.as_mut_ptr(),
        );

        assert_eq!(ret, 0);
        assert_eq!(violations, vec![0, 1]);
    }
}

// ════════════════════════════════════════════════════════════
// VM Lifecycle FFI — Path B: Full VM Integration
// ════════════════════════════════════════════════════════════

use std::os::raw::{c_void, c_uint, c_ulonglong};
use crate::vm::{FluxVM, VmResult};
use crate::check::Constraint;

/// Opaque handle to a FluxVM instance.
pub struct FluxVmHandle {
    vm: FluxVM,
}

/// Create a new FLUX VM instance.
///
/// Returns an opaque pointer suitable for passing to other flux_vm_* functions.
/// The caller must call flux_vm_free() to destroy the instance.
#[no_mangle]
pub extern "C" fn flux_vm_new() -> *mut c_void {
    let handle = Box::new(FluxVmHandle { vm: FluxVM::new() });
    Box::into_raw(handle) as *mut c_void
}

/// Destroy a FLUX VM instance created by flux_vm_new().
#[no_mangle]
pub extern "C" fn flux_vm_free(vm: *mut c_void) {
    if !vm.is_null() {
        unsafe {
            let _ = Box::from_raw(vm as *mut FluxVmHandle);
        }
    }
}

/// Load bytecode into the VM.
///
/// Parameters:
///   vm:     opaque VM handle
///   data:   bytecode buffer
///   len:    length of bytecode in bytes
///
/// Returns 0 on success, -1 on null pointer, -2 on VM error.
#[no_mangle]
pub extern "C" fn flux_vm_load_bytecode(
    vm: *mut c_void,
    data: *const c_uchar,
    len: c_uint,
) -> c_int {
    if vm.is_null() || data.is_null() {
        return -1;
    }
    let handle = unsafe { &mut *(vm as *mut FluxVmHandle) };
    let bc = unsafe { std::slice::from_raw_parts(data, len as usize) };
    handle.vm.load_bytecode(bc);
    0
}

/// Load a single scalar constraint [lo, hi] into the VM.
///
/// The VM stores this as the first constraint used by RangeCheck / BatchCheck.
#[no_mangle]
pub extern "C" fn flux_vm_load_constraint(
    vm: *mut c_void,
    lo: c_int,
    hi: c_int,
) -> c_int {
    if vm.is_null() {
        return -1;
    }
    let handle = unsafe { &mut *(vm as *mut FluxVmHandle) };
    handle.vm.load_constraints(vec![Constraint::new(lo, hi, "ffi")]);
    0
}

/// Push a single i32 value onto the VM stack.
///
/// Used to pre-load room latent values before calling run().
#[no_mangle]
pub extern "C" fn flux_vm_push_value(vm: *mut c_void, value: c_int) -> c_int {
    if vm.is_null() {
        return -1;
    }
    let handle = unsafe { &mut *(vm as *mut FluxVmHandle) };
    handle.vm.push_value(value);
    0
}

/// Execute the loaded bytecode.
///
/// Returns:
///   1  — program halted with pass=true
///   0  — program halted with pass=false (constraint violation)
///  -1  — null pointer
///  -2  — VM execution error (cycle limit, invalid opcode, etc.)
#[no_mangle]
pub extern "C" fn flux_vm_run(vm: *mut c_void) -> c_int {
    if vm.is_null() {
        return -1;
    }
    let handle = unsafe { &mut *(vm as *mut FluxVmHandle) };
    match handle.vm.run() {
        Ok(result) => {
            if result.pass {
                1
            } else {
                0
            }
        }
        Err(_) => -2,
    }
}

/// Get detailed result after run().
///
/// Writes cycle count and pass flag to out-pointers.
/// Returns 0 on success, -1 on null.
#[no_mangle]
pub extern "C" fn flux_vm_get_result(
    vm: *mut c_void,
    out_cycles: *mut c_ulonglong,
    out_pass: *mut c_int,
) -> c_int {
    if vm.is_null() || out_cycles.is_null() || out_pass.is_null() {
        return -1;
    }
    let handle = unsafe { &mut *(vm as *mut FluxVmHandle) };
    let result = handle.vm.result();
    unsafe {
        *out_cycles = result.cycles;
        *out_pass = if result.pass { 1 } else { 0 };
    }
    0
}

/// Get the proof certificate root hash.
///
/// Writes up to 32 bytes of the SHA-256 root hash into out_buf.
/// Returns number of bytes written (32), or -1 on null / no certificate.
#[no_mangle]
pub extern "C" fn flux_vm_get_proof(
    vm: *mut c_void,
    out_buf: *mut c_uchar,
    buf_len: c_uint,
) -> c_int {
    if vm.is_null() || out_buf.is_null() || buf_len < 32 {
        return -1;
    }
    let handle = unsafe { &mut *(vm as *mut FluxVmHandle) };
    if let Some(cert) = handle.vm.proof_certificate() {
        let hash = cert.root_hash;
        unsafe {
            std::ptr::copy_nonoverlapping(hash.as_ptr(), out_buf, 32);
        }
        32
    } else {
        -1
    }
}

/// Get the number of provenance log entries.
#[no_mangle]
pub extern "C" fn flux_vm_get_provenance_len(vm: *mut c_void) -> c_int {
    if vm.is_null() {
        return -1;
    }
    let handle = unsafe { &mut *(vm as *mut FluxVmHandle) };
    handle.vm.provenance_log().len() as c_int
}

/// Reset the VM to initial state (clear stack, registers, bytecode, etc.).
#[no_mangle]
pub extern "C" fn flux_vm_reset(vm: *mut c_void) -> c_int {
    if vm.is_null() {
        return -1;
    }
    let handle = unsafe { &mut *(vm as *mut FluxVmHandle) };
    handle.vm.reset();
    0
}

// ── FFI Tests ──

#[cfg(test)]
mod vm_ffi_tests {
    use super::*;

    #[test]
    fn test_vm_lifecycle() {
        let vm = flux_vm_new();
        assert!(!vm.is_null());

        // Load a simple program: Push 42, Halt
        let bc = vec![0x01, 42, 0, 0, 0, 0x29]; // Push 42, Halt
        let ret = flux_vm_load_bytecode(vm, bc.as_ptr(), bc.len() as c_uint);
        assert_eq!(ret, 0);

        let run_ret = flux_vm_run(vm);
        assert_eq!(run_ret, 1); // pass=true

        let mut cycles: c_ulonglong = 0;
        let mut pass: c_int = 0;
        let ret = flux_vm_get_result(vm, &mut cycles, &mut pass);
        assert_eq!(ret, 0);
        assert_eq!(pass, 1);
        assert!(cycles > 0);

        flux_vm_reset(vm);
        flux_vm_free(vm);
    }

    #[test]
    fn vm_null_safety() {
        assert!(flux_vm_new().is_null() == false);
        assert_eq!(flux_vm_run(std::ptr::null_mut()), -1);
        assert_eq!(flux_vm_reset(std::ptr::null_mut()), -1);
    }
}
