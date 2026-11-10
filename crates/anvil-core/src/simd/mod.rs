//! Compile-time SIMD lowering for Etalon CPU-fallback tiles and calibration.
//!
//! QualiaDB owns nquin sieve execution. These kernels fill GPU/calibrator
//! fallback slots; they are not the QualiaDB CPU graph path.

pub mod scalar;
#[cfg(target_arch = "x86_64")]
pub mod x86;

pub use scalar::ScalarEngine;

/// Vector engine monomorphized at compile time. Inner loops have no dyn dispatch.
pub trait AnvilVectorEngine {
    type VecU8x32: Copy;
    type VecI8x32: Copy;
    type VecI32x8: Copy;

    unsafe fn zero_i32x8() -> Self::VecI32x8;
    unsafe fn load_u8x32(ptr: *const u8) -> Self::VecU8x32;
    unsafe fn load_i8x32(ptr: *const i8) -> Self::VecI8x32;
    unsafe fn dp4a_accumulate(
        acc: Self::VecI32x8,
        a: Self::VecU8x32,
        b: Self::VecI8x32,
    ) -> Self::VecI32x8;
    unsafe fn reduce_sum_i32x8(v: Self::VecI32x8) -> i32;
}

/// Generic tile written once; monomorphized per engine.
#[inline(always)]
pub unsafe fn execute_dot_u8_i8_tile<E: AnvilVectorEngine>(
    u8_ptr: *const u8,
    i8_ptr: *const i8,
    blocks_32b: usize,
) -> i32 {
    let mut acc = unsafe { E::zero_i32x8() };
    let mut i = 0;
    while i < blocks_32b {
        let a = unsafe { E::load_u8x32(u8_ptr.add(i * 32)) };
        let b = unsafe { E::load_i8x32(i8_ptr.add(i * 32)) };
        acc = unsafe { E::dp4a_accumulate(acc, a, b) };
        i += 1;
    }
    unsafe { E::reduce_sum_i32x8(acc) }
}

/// Runtime pick of the host tile. Used by Etalon CPU fallback and calibrator.
pub unsafe fn execute_dot_u8_i8_host(
    u8_ptr: *const u8,
    i8_ptr: *const i8,
    blocks_32b: usize,
) -> i32 {
    #[cfg(target_arch = "x86_64")]
    {
        if x86::detect_avx2() {
            return unsafe { x86::dot_u8_i8_avx2(u8_ptr, i8_ptr, blocks_32b) };
        }
    }
    unsafe { execute_dot_u8_i8_tile::<ScalarEngine>(u8_ptr, i8_ptr, blocks_32b) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_dot_matches_naive() {
        let a = [0x55u8; 32];
        let b = [1i8; 32];
        let got = unsafe { execute_dot_u8_i8_tile::<ScalarEngine>(a.as_ptr(), b.as_ptr(), 1) };
        let naive: i32 = a.iter().zip(b.iter()).map(|(x, y)| (*x as i32) * (*y as i32)).sum();
        assert_eq!(got, naive);
    }
}
