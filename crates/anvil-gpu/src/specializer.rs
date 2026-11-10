//! Naga/WGSL override constants derived from the ontology.
//!
//! Etalon mutates `naga::Module` pipeline-overridable constants with these
//! values before `create_compute_pipeline`. No naga crate dependency yet.

use anvil_core::{lookup, MemoryRoute, CAP_HW_RAY_TRACING, CAP_VK_COOP_MATRIX, CAP_WGSL_DP4A};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SpecializerHints {
    pub workgroup_size: u16,
    pub simd_width: u16,
    pub use_dp4a: bool,
    pub use_coop_matrix: bool,
    pub use_hw_rt: bool,
    pub memory_route: MemoryRoute,
}

impl SpecializerHints {
    pub const fn fallback() -> Self {
        Self {
            workgroup_size: 64,
            simd_width: 16,
            use_dp4a: false,
            use_coop_matrix: false,
            use_hw_rt: false,
            memory_route: MemoryRoute::CpuCacheCoherent,
        }
    }

    /// WGSL snippet Etalon can splice as override defaults.
    pub fn wgsl_overrides(self) -> impl core::fmt::Display {
        WgslOverrides(self)
    }
}

struct WgslOverrides(SpecializerHints);

impl core::fmt::Display for WgslOverrides {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "override WORKGROUP_SIZE: u32 = {};", self.0.workgroup_size)?;
        writeln!(f, "override SIMD_WIDTH: u32 = {};", self.0.simd_width)?;
        writeln!(f, "override USE_DP4A: bool = {};", self.0.use_dp4a)?;
        writeln!(f, "override USE_COOP_MATRIX: bool = {};", self.0.use_coop_matrix)?;
        writeln!(f, "override USE_HW_RT: bool = {};", self.0.use_hw_rt)
    }
}

pub fn hints_for_pci(vendor_id: u16, device_id: u16) -> SpecializerHints {
    let Some(unit) = lookup(vendor_id, device_id) else {
        return SpecializerHints::fallback();
    };
    SpecializerHints {
        workgroup_size: unit.workgroup_size,
        simd_width: if unit.subgroup_max == 0 {
            16
        } else {
            unit.subgroup_max as u16
        },
        use_dp4a: unit.instruction_mask & CAP_WGSL_DP4A != 0,
        use_coop_matrix: unit.instruction_mask & CAP_VK_COOP_MATRIX != 0,
        use_hw_rt: unit.instruction_mask & CAP_HW_RAY_TRACING != 0,
        memory_route: unit.memory_route,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xe_lp_vs_ampere_workgroups() {
        let igpu = hints_for_pci(0x8086, 0xA788);
        assert_eq!(igpu.workgroup_size, 64);
        assert!(igpu.use_dp4a);
        assert!(!igpu.use_hw_rt);
        assert_eq!(igpu.memory_route, MemoryRoute::UnifiedZeroCopyHost);

        let dgpu = hints_for_pci(0x10DE, 0x2531);
        assert_eq!(dgpu.workgroup_size, 256);
        assert!(dgpu.use_hw_rt);
        assert!(dgpu.use_coop_matrix);
        assert_eq!(dgpu.memory_route, MemoryRoute::DiscretePcieDma);
    }
}
