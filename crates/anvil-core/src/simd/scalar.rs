//! Portable scalar engine. Always available under `#![no_std]`.

use super::AnvilVectorEngine;

pub struct ScalarEngine;

impl AnvilVectorEngine for ScalarEngine {
    type VecU8x32 = [u8; 32];
    type VecI8x32 = [i8; 32];
    type VecI32x8 = [i32; 8];

    #[inline(always)]
    unsafe fn zero_i32x8() -> Self::VecI32x8 {
        [0; 8]
    }

    #[inline(always)]
    unsafe fn load_u8x32(ptr: *const u8) -> Self::VecU8x32 {
        unsafe { core::ptr::read_unaligned(ptr as *const [u8; 32]) }
    }

    #[inline(always)]
    unsafe fn load_i8x32(ptr: *const i8) -> Self::VecI8x32 {
        unsafe { core::ptr::read_unaligned(ptr as *const [i8; 32]) }
    }

    #[inline(always)]
    unsafe fn dp4a_accumulate(
        mut acc: Self::VecI32x8,
        a: Self::VecU8x32,
        b: Self::VecI8x32,
    ) -> Self::VecI32x8 {
        let mut lane = 0;
        while lane < 8 {
            let base = lane * 4;
            let mut s = 0i32;
            let mut k = 0;
            while k < 4 {
                s += (a[base + k] as i32) * (b[base + k] as i32);
                k += 1;
            }
            acc[lane] += s;
            lane += 1;
        }
        acc
    }

    #[inline(always)]
    unsafe fn reduce_sum_i32x8(v: Self::VecI32x8) -> i32 {
        v[0] + v[1] + v[2] + v[3] + v[4] + v[5] + v[6] + v[7]
    }
}
