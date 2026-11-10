//! Install / first-boot synthetic micro-sweep.
//!
//! Buffers are generic `0x55` / `0xAA` patterns. Timing needs `std`;
//! `no_std` records a checksum fingerprint only.

use crate::simd::{execute_dot_u8_i8_tile, ScalarEngine};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CalibrationSample {
    pub payload_bytes: usize,
    pub checksum: u64,
    pub elapsed_ns: u64,
}

/// Run a CPU-fallback tile over `payload`. QualiaDB does not call this for
/// nquin sieves; Etalon uses it for iGPU / dGPU crossover hints.
pub fn calibrate_cpu_tile(payload: &[u8]) -> CalibrationSample {
    let checksum = crate::q42_wire::fnv1a64(payload);
    let blocks = payload.len() / 32;
    let elapsed_ns = {
        #[cfg(feature = "std")]
        {
            let start = std::time::Instant::now();
            let ones = [1i8; 32];
            let mut i = 0;
            while i < blocks {
                let _ = unsafe {
                    execute_dot_u8_i8_tile::<ScalarEngine>(
                        payload.as_ptr().add(i * 32),
                        ones.as_ptr(),
                        1,
                    )
                };
                i += 1;
            }
            start.elapsed().as_nanos() as u64
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = (blocks, execute_dot_u8_i8_tile::<ScalarEngine>);
            0
        }
    };
    CalibrationSample {
        payload_bytes: payload.len(),
        checksum,
        elapsed_ns,
    }
}

/// Fill `out` with repeating `0x55` / `0xAA` lanes. Test and calibrator only.
pub fn fill_synthetic(out: &mut [u8]) {
    let mut i = 0;
    while i < out.len() {
        out[i] = if i % 2 == 0 { 0x55 } else { 0xAA };
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_pattern_is_generic() {
        let mut buf = [0u8; 8];
        fill_synthetic(&mut buf);
        assert_eq!(buf, [0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA]);
        let sample = calibrate_cpu_tile(&buf);
        assert_eq!(sample.payload_bytes, 8);
        assert_ne!(sample.checksum, 0);
    }
}
