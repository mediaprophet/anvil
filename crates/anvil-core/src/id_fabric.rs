//! Tiered hardware identifier *slots*.
//!
//! Anvil records what the silicon can attest. QualiaDB / Webizen **manage**
//! the human-centric fabric: did:q42 binding, Sanctuary Mode, Human Agency,
//! and Personal Autonomy. Anvil does not unlock a vault.

use crate::ir::AnvilDispatchTable;
use crate::register::CapabilityRegister;
use crate::simd::{execute_dot_u8_i8_host, ScalarEngine};

/// Cryptographic attestability / spoofability class of a hardware identifier.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttestationTier {
    /// Non-exportable silicon root (TPM EK, Secure Enclave, StrongBox).
    SiliconRoot = 0,
    /// Fused die / bus serial (GPU deviceUUID, NVMe EUI-64, CPUID stepping).
    DieSerial = 1,
    /// Empirical timing fingerprint from calibration.
    TimingPuf = 2,
    /// Mutable board tables (SMBIOS / DMI).
    BoardTable = 3,
    /// Ephemeral OS / network hints (active MAC, hostname).
    EphemeralHint = 4,
}

impl AttestationTier {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::SiliconRoot),
            1 => Some(Self::DieSerial),
            2 => Some(Self::TimingPuf),
            3 => Some(Self::BoardTable),
            4 => Some(Self::EphemeralHint),
            _ => None,
        }
    }
}

/// Kind of enumerated cryptography-supported identifier in a slot.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IdentifierKind {
    TpmEkHash = 0,
    GpuDeviceUuid = 1,
    NvmeEui64 = 2,
    CpuStepping = 3,
    TimingFingerprint = 4,
    SmbiosUuid = 5,
    ActiveMac = 6,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IdentifierRecord {
    pub tier: AttestationTier,
    pub kind: IdentifierKind,
    pub bytes: [u8; 16],
}

impl IdentifierRecord {
    pub const fn empty(tier: AttestationTier, kind: IdentifierKind) -> Self {
        Self {
            tier,
            kind,
            bytes: [0; 16],
        }
    }

    pub const fn is_empty(self) -> bool {
        let mut i = 0;
        while i < 16 {
            if self.bytes[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }
}

/// Hardware-side Sanctuary *readiness*. QualiaDB applies Human Agency policy.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SanctuaryPolicy {
    pub require_iommu: bool,
    pub min_tier: AttestationTier,
}

impl SanctuaryPolicy {
    pub const fn hardware_isolated() -> Self {
        Self {
            require_iommu: true,
            min_tier: AttestationTier::DieSerial,
        }
    }
}

/// Shared handshake context. QualiaDB fills CPU / crypto / isolation;
/// Etalon appends GPU `deviceUUID` and memory route.
#[repr(C, align(64))]
#[derive(Copy, Clone)]
pub struct AnvilContext {
    pub hardware_fingerprint: u64,
    pub attestation_tier: AttestationTier,
    /// True when hardware facts satisfy [`SanctuaryPolicy::hardware_isolated`].
    /// QualiaDB still decides whether Sanctuary Mode is granted.
    pub sanctuary_verified: bool,
    pub capabilities: CapabilityRegister,
    pub p_core_mask: u64,
    pub e_core_mask: u64,
    pub igpu_zero_copy_cutoff_bytes: usize,
    pub dgpu_pcie_cutoff_bytes: usize,
    pub dispatch: AnvilDispatchTable,
    pub machine_anchor_nquin: [u8; 48],
    pub gpu_device_uuid: [u8; 16],
    pub sealed_anchor: [u8; 48],
    pub ephemeral_mac: [u8; 6],
    pub has_gpu_uuid: bool,
    pub has_sealed_anchor: bool,
}

impl AnvilContext {
    pub const fn empty() -> Self {
        Self {
            hardware_fingerprint: 0,
            attestation_tier: AttestationTier::EphemeralHint,
            sanctuary_verified: false,
            capabilities: CapabilityRegister::empty(),
            p_core_mask: 0,
            e_core_mask: 0,
            igpu_zero_copy_cutoff_bytes: usize::MAX,
            dgpu_pcie_cutoff_bytes: usize::MAX,
            dispatch: AnvilDispatchTable::empty(),
            machine_anchor_nquin: [0; 48],
            gpu_device_uuid: [0; 16],
            sealed_anchor: [0; 48],
            ephemeral_mac: [0; 6],
            has_gpu_uuid: false,
            has_sealed_anchor: false,
        }
    }

    /// Host probe QualiaDB runs on boot. Does not bind GPU.
    pub fn probe_host() -> Self {
        let mut ctx = Self::empty();
        ctx.capabilities = crate::crypto_probe::probe_crypto();
        #[cfg(target_arch = "x86_64")]
        {
            if crate::simd::x86::detect_avx2() {
                ctx.capabilities.compute |= crate::register::CAP_AVX2;
            }
            if crate::simd::x86::detect_avx_vnni() {
                ctx.capabilities.compute |= crate::register::CAP_AVX_VNNI;
            }
        }
        ctx.dispatch.dot_u8_i8_tile = Some(execute_dot_u8_i8_host);
        let _ = ScalarEngine;
        ctx.recompute_anchor();
        ctx.attestation_tier = if ctx.capabilities.entropy != 0 {
            AttestationTier::DieSerial
        } else {
            AttestationTier::EphemeralHint
        };
        ctx
    }

    /// Etalon fills the Tier-1 GPU die UUID into the same context.
    pub fn bind_gpu_device_uuid(&mut self, uuid: [u8; 16]) {
        self.gpu_device_uuid = uuid;
        self.has_gpu_uuid = uuid.iter().any(|b| *b != 0);
        self.recompute_anchor();
        if self.has_gpu_uuid && self.attestation_tier > AttestationTier::DieSerial {
            self.attestation_tier = AttestationTier::DieSerial;
        }
    }

    /// Ephemeral MAC (Tier 4). Must not affect Sanctuary hardware readiness.
    pub fn set_ephemeral_mac(&mut self, mac: [u8; 6]) {
        self.ephemeral_mac = mac;
    }

    /// QualiaDB seals the composite Tier 0–2 anchor after first calibration.
    pub fn seal_machine_anchor(&mut self) {
        self.sealed_anchor = self.machine_anchor_nquin;
        self.has_sealed_anchor = true;
        self.sanctuary_verified = verify_sanctuary_policy(self, SanctuaryPolicy::hardware_isolated());
    }

    fn recompute_anchor(&mut self) {
        let mut nquin = [0u8; 48];
        nquin[8] = self.attestation_tier as u8;
        nquin[9] = u8::from(self.has_gpu_uuid);
        nquin[10..14].copy_from_slice(&self.capabilities.crypto.to_le_bytes());
        nquin[14..18].copy_from_slice(&self.capabilities.isolation.to_le_bytes());
        nquin[18..22].copy_from_slice(&self.capabilities.entropy.to_le_bytes());
        nquin[22..26].copy_from_slice(&self.capabilities.compute.to_le_bytes());
        nquin[32..48].copy_from_slice(&self.gpu_device_uuid);
        self.hardware_fingerprint = crate::q42_wire::fnv1a64(&nquin[8..]);
        nquin[0..8].copy_from_slice(&self.hardware_fingerprint.to_le_bytes());
        self.machine_anchor_nquin = nquin;
    }
}

/// Hardware readiness for Sanctuary Mode. QualiaDB calls this, then applies
/// Human Agency policy before unlocking a vault.
pub fn verify_sanctuary_policy(ctx: &AnvilContext, policy: SanctuaryPolicy) -> bool {
    if policy.require_iommu && !ctx.capabilities.has_iommu() {
        return false;
    }
    if ctx.attestation_tier > policy.min_tier {
        return false;
    }
    if ctx.has_sealed_anchor && ctx.machine_anchor_nquin != ctx.sealed_anchor {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::register::ISO_VTD_IOMMU;

    #[test]
    fn spoofed_mac_does_not_trip_sanctuary() {
        let mut ctx = AnvilContext::empty();
        ctx.capabilities.isolation = ISO_VTD_IOMMU;
        ctx.attestation_tier = AttestationTier::DieSerial;
        ctx.bind_gpu_device_uuid([0x11; 16]);
        ctx.seal_machine_anchor();
        assert!(ctx.sanctuary_verified);

        ctx.set_ephemeral_mac([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        assert!(verify_sanctuary_policy(
            &ctx,
            SanctuaryPolicy::hardware_isolated()
        ));
    }

    #[test]
    fn device_uuid_drift_trips_sanctuary() {
        let mut ctx = AnvilContext::empty();
        ctx.capabilities.isolation = ISO_VTD_IOMMU;
        ctx.attestation_tier = AttestationTier::DieSerial;
        ctx.bind_gpu_device_uuid([0xAA; 16]);
        ctx.seal_machine_anchor();
        ctx.bind_gpu_device_uuid([0x55; 16]);
        assert!(!verify_sanctuary_policy(
            &ctx,
            SanctuaryPolicy::hardware_isolated()
        ));
    }

    #[test]
    fn missing_iommu_rejects_hardware_isolated() {
        let mut ctx = AnvilContext::empty();
        ctx.attestation_tier = AttestationTier::DieSerial;
        ctx.bind_gpu_device_uuid([0xAA; 16]);
        ctx.seal_machine_anchor();
        assert!(!verify_sanctuary_policy(
            &ctx,
            SanctuaryPolicy::hardware_isolated()
        ));
    }
}
