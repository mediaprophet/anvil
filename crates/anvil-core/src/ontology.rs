//! Seeded baseline hardware ontology (five SKUs).

use crate::q42_wire::AnvilNquin;
use crate::register::{
    ComputeUnitDescriptor, MemoryRoute, CAP_ARM_I8MM, CAP_AVX2, CAP_AVX_VNNI, CAP_HW_RAY_TRACING,
    CAP_NEON_DOTPROD, CAP_VK_COOP_MATRIX, CAP_WGSL_DP4A,
};

pub const BASELINE_COUNT: usize = 6;

/// Intel Core i9-14900HX CPU cluster (`0x8086` family `0xB067`).
pub fn sku_hx_cpu() -> ComputeUnitDescriptor {
    let mut u = ComputeUnitDescriptor::empty();
    u.vendor_id = 0x8086;
    u.device_id = 0xB067;
    u.arch_stepping = 0x0000_0C02;
    u.eu_or_sm_count = 24; // 8P + 16E
    u.simd_width_bits = 256;
    u.max_clock_mhz = 5800;
    u.workgroup_size = 1;
    u.instruction_mask = CAP_AVX2 | CAP_AVX_VNNI;
    u.tdp_base_w = 55;
    u.tdp_turbo_w = 157;
    u.memory_route = MemoryRoute::CpuCacheCoherent;
    u.npu_revision = 30; // GNA 3.0
    u
}

/// Intel UHD 14th Gen Xe-LP 32 EU (`8086:A788`).
pub fn sku_xe_lp_igpu() -> ComputeUnitDescriptor {
    let mut u = ComputeUnitDescriptor::empty();
    u.vendor_id = 0x8086;
    u.device_id = 0xA788;
    u.arch_stepping = 0x0000_0C02;
    u.eu_or_sm_count = 32;
    u.simd_width_bits = 128;
    u.max_clock_mhz = 1650;
    u.workgroup_size = 64;
    u.instruction_mask = CAP_WGSL_DP4A;
    u.media_engines = 2;
    u.subgroup_min = 8;
    u.subgroup_max = 32;
    u.memory_route = MemoryRoute::UnifiedZeroCopyHost;
    u
}

/// NVIDIA RTX A2000 Ampere (`10DE:2531`).
pub fn sku_rtx_a2000() -> ComputeUnitDescriptor {
    let mut u = ComputeUnitDescriptor::empty();
    u.vendor_id = 0x10DE;
    u.device_id = 0x2531;
    u.arch_stepping = 0x0000_00A1;
    u.eu_or_sm_count = 26;
    u.simd_width_bits = 256;
    u.max_clock_mhz = 1200;
    u.workgroup_size = 256;
    u.instruction_mask = CAP_WGSL_DP4A | CAP_VK_COOP_MATRIX | CAP_HW_RAY_TRACING;
    u.subgroup_min = 32;
    u.subgroup_max = 32;
    u.memory_route = MemoryRoute::DiscretePcieDma;
    u
}

/// Intel UHD 620 (`8086:5917`).
pub fn sku_uhd_620() -> ComputeUnitDescriptor {
    let mut u = ComputeUnitDescriptor::empty();
    u.vendor_id = 0x8086;
    u.device_id = 0x5917;
    u.arch_stepping = 0x0000_090E;
    u.eu_or_sm_count = 24;
    u.simd_width_bits = 128;
    u.max_clock_mhz = 1100;
    u.workgroup_size = 64;
    u.instruction_mask = CAP_AVX2;
    u.memory_route = MemoryRoute::UnifiedZeroCopyHost;
    u
}

/// Apple Silicon unified GPU (`106B`).
pub fn sku_apple_unified_gpu() -> ComputeUnitDescriptor {
    let mut u = ComputeUnitDescriptor::empty();
    u.vendor_id = 0x106B;
    u.device_id = 0x0001;
    u.arch_stepping = 0x0000_0001;
    u.eu_or_sm_count = 40;
    u.simd_width_bits = 128;
    u.max_clock_mhz = 1400;
    u.workgroup_size = 256;
    u.instruction_mask = CAP_NEON_DOTPROD | CAP_ARM_I8MM;
    u.memory_route = MemoryRoute::UnifiedZeroCopyHost;
    u.npu_revision = 16; // ANE slot
    u
}

/// Mobile Vulkan GPU + Tensor slot (`13B5` ARM Mali-class).
pub fn sku_tensor_mobile_gpu() -> ComputeUnitDescriptor {
    let mut u = ComputeUnitDescriptor::empty();
    u.vendor_id = 0x13B5;
    u.device_id = 0x0001;
    u.arch_stepping = 0x0000_0009;
    u.eu_or_sm_count = 16;
    u.simd_width_bits = 128;
    u.max_clock_mhz = 900;
    u.workgroup_size = 64;
    u.instruction_mask = CAP_NEON_DOTPROD | CAP_ARM_I8MM | CAP_WGSL_DP4A;
    u.memory_route = MemoryRoute::UnifiedZeroCopyHost;
    u.npu_revision = 2; // Tensor TPU slot
    u
}

/// Six compute nquins covering the five baseline machines (HX CPU + iGPU share a package).
pub fn seeded_ontology() -> [AnvilNquin; BASELINE_COUNT] {
    [
        sku_hx_cpu().to_nquin(),
        sku_xe_lp_igpu().to_nquin(),
        sku_rtx_a2000().to_nquin(),
        sku_uhd_620().to_nquin(),
        sku_apple_unified_gpu().to_nquin(),
        sku_tensor_mobile_gpu().to_nquin(),
    ]
}

pub fn lookup(vendor_id: u16, device_id: u16) -> Option<ComputeUnitDescriptor> {
    for rec in seeded_ontology() {
        if rec.vendor_id == vendor_id && rec.device_id == device_id {
            return Some(ComputeUnitDescriptor::from_nquin(rec));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_count_and_xe_lp_workgroup() {
        assert_eq!(seeded_ontology().len(), BASELINE_COUNT);
        let xe = lookup(0x8086, 0xA788).expect("Device_001");
        assert_eq!(xe.workgroup_size, 64);
        assert_eq!(xe.eu_or_sm_count, 32);
        let dgpu = lookup(0x10DE, 0x2531).expect("Device_001");
        assert_eq!(dgpu.workgroup_size, 256);
        assert_eq!(dgpu.instruction_mask & CAP_HW_RAY_TRACING, CAP_HW_RAY_TRACING);
    }
}
