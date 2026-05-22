use std::os::raw::{c_float, c_int, c_uint8_t};
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
    violations: *mut c_uint8_t,
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
