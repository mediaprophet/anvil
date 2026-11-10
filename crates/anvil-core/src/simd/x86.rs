//! x86_64 AVX2 / AVX-VNNI backends for Etalon CPU-fallback tiles.

use core::arch::x86_64::{
    __cpuid_count, __m256i, _mm256_add_epi32, _mm256_castsi256_si128, _mm256_extracti128_si256,
    _mm256_loadu_si256, _mm256_madd_epi16, _mm256_maddubs_epi16, _mm256_set1_epi16,
    _mm256_setzero_si256, _mm_add_epi32, _mm_cvtsi128_si32, _mm_srli_si128,
};

/// CPUID Leaf 7, Subleaf 0, EBX bit 5.
pub fn detect_avx2() -> bool {
    let leaf = __cpuid_count(7, 0);
    (leaf.ebx & (1 << 5)) != 0
}

/// CPUID Leaf 7, Subleaf 1, EAX bit 4 (AVX-VNNI), requires AVX2.
pub fn detect_avx_vnni() -> bool {
    let leaf7_0 = __cpuid_count(7, 0);
    let has_avx2 = (leaf7_0.ebx & (1 << 5)) != 0;
    if has_avx2 && leaf7_0.eax >= 1 {
        let leaf7_1 = __cpuid_count(7, 1);
        return (leaf7_1.eax & (1 << 4)) != 0;
    }
    false
}

#[target_feature(enable = "avx2")]
pub unsafe fn dot_u8_i8_avx2(u8_ptr: *const u8, i8_ptr: *const i8, blocks_32b: usize) -> i32 {
    unsafe {
        let mut acc = _mm256_setzero_si256();
        let ones = _mm256_set1_epi16(1);
        let mut i = 0;
        while i < blocks_32b {
            let a = _mm256_loadu_si256(u8_ptr.add(i * 32) as *const __m256i);
            let b = _mm256_loadu_si256(i8_ptr.add(i * 32) as *const __m256i);
            let prod16 = _mm256_maddubs_epi16(a, b);
            let prod32 = _mm256_madd_epi16(prod16, ones);
            acc = _mm256_add_epi32(acc, prod32);
            i += 1;
        }
        horizontal_sum_i32x8(acc)
    }
}

#[target_feature(enable = "avx2,avxvnni")]
pub unsafe fn dot_u8_i8_avx_vnni(u8_ptr: *const u8, i8_ptr: *const i8, blocks_32b: usize) -> i32 {
    unsafe {
        let mut acc = _mm256_setzero_si256();
        let mut i = 0;
        while i < blocks_32b {
            let a = _mm256_loadu_si256(u8_ptr.add(i * 32) as *const __m256i);
            let b = _mm256_loadu_si256(i8_ptr.add(i * 32) as *const __m256i);
            acc = core::arch::x86_64::_mm256_dpbusd_epi32(acc, a, b);
            i += 1;
        }
        horizontal_sum_i32x8(acc)
    }
}

#[inline(always)]
unsafe fn horizontal_sum_i32x8(acc: __m256i) -> i32 {
    unsafe {
        let sum128 = _mm_add_epi32(_mm256_castsi256_si128(acc), _mm256_extracti128_si256(acc, 1));
        let sum64 = _mm_add_epi32(sum128, _mm_srli_si128(sum128, 8));
        let sum32 = _mm_add_epi32(sum64, _mm_srli_si128(sum64, 4));
        _mm_cvtsi128_si32(sum32)
    }
}
