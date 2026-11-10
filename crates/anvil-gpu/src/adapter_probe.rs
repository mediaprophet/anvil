//! Adapter facts Etalon fills into [`anvil_core::AnvilContext`].

use anvil_core::{lookup, AnvilContext, MemoryRoute};

/// wgpu/Vulkan adapter snapshot without a wgpu dependency.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AdapterReport {
    pub vendor_id: u16,
    pub device_id: u16,
    /// Vulkan `VkPhysicalDeviceIDProperties::deviceUUID` (Tier 1 slot).
    pub device_uuid: [u8; 16],
    pub high_performance: bool,
    pub unified_zero_copy: bool,
}

impl AdapterReport {
    pub const fn new(vendor_id: u16, device_id: u16) -> Self {
        Self {
            vendor_id,
            device_id,
            device_uuid: [0; 16],
            high_performance: false,
            unified_zero_copy: false,
        }
    }

    pub fn memory_route(self) -> MemoryRoute {
        if let Some(unit) = lookup(self.vendor_id, self.device_id) {
            return unit.memory_route;
        }
        if self.unified_zero_copy {
            MemoryRoute::UnifiedZeroCopyHost
        } else if self.high_performance {
            MemoryRoute::DiscretePcieDma
        } else {
            MemoryRoute::CpuCacheCoherent
        }
    }
}

/// Etalon appends GPU die UUID and cutoff hints onto a QualiaDB-probed context.
pub fn etalon_bind_gpu(ctx: &mut AnvilContext, report: &AdapterReport) {
    ctx.bind_gpu_device_uuid(report.device_uuid);
    if let Some(unit) = lookup(report.vendor_id, report.device_id) {
        ctx.dispatch.workgroup_size = unit.workgroup_size;
        ctx.dispatch.simd_width = unit.simd_width_bits / 8;
        match unit.memory_route {
            MemoryRoute::UnifiedZeroCopyHost => {
                ctx.igpu_zero_copy_cutoff_bytes = unit.crossover_threshold_bytes as usize;
            }
            MemoryRoute::DiscretePcieDma => {
                ctx.dgpu_pcie_cutoff_bytes = unit.crossover_threshold_bytes as usize;
            }
            MemoryRoute::CpuCacheCoherent | MemoryRoute::DedicatedAccelerator => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_core::{verify_sanctuary_policy, SanctuaryPolicy, ISO_VTD_IOMMU};

    #[test]
    fn handshake_qualidb_then_etalon() {
        let mut ctx = AnvilContext::empty();
        ctx.capabilities.isolation = ISO_VTD_IOMMU;
        ctx.attestation_tier = anvil_core::AttestationTier::DieSerial;

        let mut report = AdapterReport::new(0x8086, 0xA788);
        report.device_uuid = [0xAA; 16];
        report.unified_zero_copy = true;
        etalon_bind_gpu(&mut ctx, &report);
        ctx.seal_machine_anchor();

        assert!(ctx.has_gpu_uuid);
        assert_eq!(ctx.dispatch.workgroup_size, 64);
        assert_eq!(report.memory_route(), MemoryRoute::UnifiedZeroCopyHost);
        assert!(verify_sanctuary_policy(
            &ctx,
            SanctuaryPolicy::hardware_isolated()
        ));
    }
}
