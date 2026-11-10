//! Capability bitmasks, compute-unit descriptors, and the system hardware map.

/// Domain `0x01` — compute and matrix topology.
pub const DOMAIN_COMPUTE: u32 = 0x01;
/// Domain `0x02` — memory and DMA interconnect.
pub const DOMAIN_MEMORY: u32 = 0x02;
/// Domain `0x03` — cryptography and entropy.
pub const DOMAIN_CRYPTO: u32 = 0x03;
/// Domain `0x04` — virtualization and isolation.
pub const DOMAIN_ISOLATION: u32 = 0x04;
/// Domain `0x05` — tiered identifier fabric (hardware slots; QualiaDB manages policy).
pub const DOMAIN_ID_FABRIC: u32 = 0x05;

pub const CAP_AVX2: u32 = 1 << 0;
pub const CAP_AVX_VNNI: u32 = 1 << 1;
pub const CAP_NEON_DOTPROD: u32 = 1 << 2;
pub const CAP_ARM_I8MM: u32 = 1 << 3;
pub const CAP_WGSL_DP4A: u32 = 1 << 4;
pub const CAP_VK_COOP_MATRIX: u32 = 1 << 5;
pub const CAP_HW_RAY_TRACING: u32 = 1 << 6;
pub const CAP_WASM_RELAXED_SIMD: u32 = 1 << 7;

pub const CRYPTO_VAES: u32 = 1 << 0;
pub const CRYPTO_VPCLMULQDQ: u32 = 1 << 1;
pub const CRYPTO_SHA_NI: u32 = 1 << 2;
pub const CRYPTO_AES_NI: u32 = 1 << 3;
pub const CRYPTO_ARM_PMULL: u32 = 1 << 4;
pub const CRYPTO_ARM_SHA2: u32 = 1 << 5;
pub const CRYPTO_ARM_SHA3: u32 = 1 << 6;
pub const CRYPTO_KEY_LOCKER: u32 = 1 << 7;

pub const ENTROPY_RDSEED: u32 = 1 << 0;
pub const ENTROPY_RDRAND: u32 = 1 << 1;
pub const ENTROPY_ARM_RNDR: u32 = 1 << 2;
pub const ENTROPY_TPM2: u32 = 1 << 3;
pub const ENTROPY_SECURE_ENCLAVE: u32 = 1 << 4;
pub const ENTROPY_TITAN_M2: u32 = 1 << 5;

pub const ISO_VTX_EPT: u32 = 1 << 0;
pub const ISO_VTD_IOMMU: u32 = 1 << 1;
pub const ISO_ARM_EL2_PKVM: u32 = 1 << 2;
pub const ISO_ARM_SMMU: u32 = 1 << 3;
pub const ISO_INTEL_CET: u32 = 1 << 4;
pub const ISO_ARM_PAC_MTE: u32 = 1 << 5;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MemoryRoute {
    CpuCacheCoherent = 0,
    UnifiedZeroCopyHost = 1,
    DiscretePcieDma = 2,
    DedicatedAccelerator = 3,
}

impl MemoryRoute {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::CpuCacheCoherent),
            1 => Some(Self::UnifiedZeroCopyHost),
            2 => Some(Self::DiscretePcieDma),
            3 => Some(Self::DedicatedAccelerator),
            _ => None,
        }
    }
}

/// Packed compute facts from a Domain `0x01` nquin payload (36 bytes).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ComputeUnitDescriptor {
    pub vendor_id: u16,
    pub device_id: u16,
    pub arch_stepping: u32,
    pub eu_or_sm_count: u16,
    pub simd_width_bits: u16,
    pub max_clock_mhz: u16,
    pub workgroup_size: u16,
    pub instruction_mask: u32,
    pub tdp_base_w: u16,
    pub tdp_turbo_w: u16,
    pub int8_gops: u32,
    pub crossover_threshold_bytes: u64,
    pub subgroup_min: u8,
    pub subgroup_max: u8,
    pub media_engines: u8,
    pub npu_revision: u8,
    pub memory_route: MemoryRoute,
}

impl ComputeUnitDescriptor {
    pub const fn empty() -> Self {
        Self {
            vendor_id: 0,
            device_id: 0,
            arch_stepping: 0,
            eu_or_sm_count: 0,
            simd_width_bits: 128,
            max_clock_mhz: 0,
            workgroup_size: 64,
            instruction_mask: 0,
            tdp_base_w: 0,
            tdp_turbo_w: 0,
            int8_gops: 0,
            crossover_threshold_bytes: 0,
            subgroup_min: 8,
            subgroup_max: 32,
            media_engines: 0,
            npu_revision: 0,
            memory_route: MemoryRoute::CpuCacheCoherent,
        }
    }

    pub fn pack_payload(self) -> [u8; 36] {
        let mut p = [0u8; 36];
        p[0..2].copy_from_slice(&self.eu_or_sm_count.to_le_bytes());
        p[2..4].copy_from_slice(&self.simd_width_bits.to_le_bytes());
        p[4..6].copy_from_slice(&self.max_clock_mhz.to_le_bytes());
        p[6..8].copy_from_slice(&self.workgroup_size.to_le_bytes());
        p[8..12].copy_from_slice(&self.instruction_mask.to_le_bytes());
        p[12..14].copy_from_slice(&self.tdp_base_w.to_le_bytes());
        p[14..16].copy_from_slice(&self.tdp_turbo_w.to_le_bytes());
        p[16..20].copy_from_slice(&self.int8_gops.to_le_bytes());
        p[20..28].copy_from_slice(&self.crossover_threshold_bytes.to_le_bytes());
        p[28] = self.subgroup_min;
        p[29] = self.subgroup_max;
        p[30] = self.media_engines;
        p[31] = self.npu_revision;
        p[32] = self.memory_route as u8;
        p
    }

    pub fn from_nquin(nquin: crate::q42_wire::AnvilNquin) -> Self {
        let p = nquin.payload;
        Self {
            vendor_id: nquin.vendor_id,
            device_id: nquin.device_id,
            arch_stepping: nquin.arch_stepping,
            eu_or_sm_count: u16::from_le_bytes([p[0], p[1]]),
            simd_width_bits: u16::from_le_bytes([p[2], p[3]]),
            max_clock_mhz: u16::from_le_bytes([p[4], p[5]]),
            workgroup_size: u16::from_le_bytes([p[6], p[7]]),
            instruction_mask: u32::from_le_bytes([p[8], p[9], p[10], p[11]]),
            tdp_base_w: u16::from_le_bytes([p[12], p[13]]),
            tdp_turbo_w: u16::from_le_bytes([p[14], p[15]]),
            int8_gops: u32::from_le_bytes([p[16], p[17], p[18], p[19]]),
            crossover_threshold_bytes: u64::from_le_bytes([
                p[20], p[21], p[22], p[23], p[24], p[25], p[26], p[27],
            ]),
            subgroup_min: p[28],
            subgroup_max: p[29],
            media_engines: p[30],
            npu_revision: p[31],
            memory_route: MemoryRoute::from_u8(p[32]).unwrap_or(MemoryRoute::CpuCacheCoherent),
        }
    }

    pub fn to_nquin(self) -> crate::q42_wire::AnvilNquin {
        let mut n = crate::q42_wire::AnvilNquin::new(
            self.vendor_id,
            self.device_id,
            self.arch_stepping,
            DOMAIN_COMPUTE,
        );
        n.payload = self.pack_payload();
        n
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct CapabilityRegister {
    pub compute: u32,
    pub crypto: u32,
    pub entropy: u32,
    pub isolation: u32,
}

impl CapabilityRegister {
    pub const fn empty() -> Self {
        Self {
            compute: 0,
            crypto: 0,
            entropy: 0,
            isolation: 0,
        }
    }

    #[inline]
    pub const fn has_compute(self, bit: u32) -> bool {
        self.compute & bit != 0
    }

    #[inline]
    pub const fn has_iommu(self) -> bool {
        self.isolation & ISO_VTD_IOMMU != 0 || self.isolation & ISO_ARM_SMMU != 0
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SystemHardwareMap {
    pub cpu: ComputeUnitDescriptor,
    pub igpu: ComputeUnitDescriptor,
    pub dgpu: ComputeUnitDescriptor,
    pub cache_line_bytes: u16,
    pub has_igpu: bool,
    pub has_dgpu: bool,
}

impl SystemHardwareMap {
    pub const fn empty() -> Self {
        Self {
            cpu: ComputeUnitDescriptor::empty(),
            igpu: ComputeUnitDescriptor::empty(),
            dgpu: ComputeUnitDescriptor::empty(),
            cache_line_bytes: 64,
            has_igpu: false,
            has_dgpu: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_payload_round_trip() {
        let mut unit = ComputeUnitDescriptor::empty();
        unit.vendor_id = 0x8086;
        unit.device_id = 0xA788;
        unit.eu_or_sm_count = 32;
        unit.workgroup_size = 64;
        unit.instruction_mask = CAP_WGSL_DP4A;
        unit.memory_route = MemoryRoute::UnifiedZeroCopyHost;
        let parsed = ComputeUnitDescriptor::from_nquin(unit.to_nquin());
        assert_eq!(parsed.eu_or_sm_count, 32);
        assert_eq!(parsed.workgroup_size, 64);
        assert_eq!(parsed.memory_route, MemoryRoute::UnifiedZeroCopyHost);
        assert_eq!(parsed.instruction_mask & CAP_WGSL_DP4A, CAP_WGSL_DP4A);
    }
}
