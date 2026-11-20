//! Live host profile: 64-byte [`NodeProfile64`] artefact.
//!
//! This is hardware *facts* for one machine, written beside
//! `anvil_ontology.q42`. It carries no Qualia types. QualiaDB later reads
//! the file for Sanctuary fail-closed; Etalon later binds `deviceUUID`.
//! Anvil does not grant Sanctuary Mode and does not advertise a vault.

use crate::id_fabric::{verify_sanctuary_policy, AnvilContext, AttestationTier, SanctuaryPolicy};
use crate::q42_wire::fnv1a64;
use crate::register::ISO_VTX_EPT;

/// `"NP64"`
pub const NODE_PROFILE_MAGIC: [u8; 4] = *b"NP64";
pub const NODE_PROFILE_VERSION: u16 = 1;
pub const NODE_PROFILE_BYTES: usize = 64;

pub const PROFILE_HAS_GPU_UUID: u8 = 1 << 0;
pub const PROFILE_HAS_IOMMU: u8 = 1 << 1;
pub const PROFILE_HAS_VTX: u8 = 1 << 2;
/// Anvil hardware-readiness only ([`SanctuaryPolicy::hardware_isolated`]).
pub const PROFILE_SANCTUARY_HW_READY: u8 = 1 << 3;
/// `vendor_id`/`device_id` are a probed GPU PCI pair (not CPU vendor fallback).
pub const PROFILE_PCI_IS_GPU: u8 = 1 << 4;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NodeProfileError {
    Truncated,
    BadMagic,
    UnsupportedVersion,
    ChecksumMismatch,
    BadTier,
}

/// Why a parsed profile must not be advertised as a hardware-attested vault.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AdvertisingDeny {
    MissingIommu,
    MissingGpuUuid,
    TierTooWeak,
}

/// 64-byte live host profile (design name NodeProfile64).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NodeProfile64 {
    pub magic: [u8; 4],
    pub version: u16,
    pub attestation_tier: u8,
    pub flags: u8,
    pub hardware_fingerprint: u64,
    pub gpu_device_uuid: [u8; 16],
    pub isolation: u32,
    pub crypto: u32,
    pub entropy: u32,
    pub compute: u32,
    pub vendor_id: u16,
    pub device_id: u16,
    pub arch_stepping: u32,
    pub checksum: u64,
}

const _: () = assert!(core::mem::size_of::<NodeProfile64>() == NODE_PROFILE_BYTES);

impl NodeProfile64 {
    pub const fn empty() -> Self {
        Self {
            magic: NODE_PROFILE_MAGIC,
            version: NODE_PROFILE_VERSION,
            attestation_tier: AttestationTier::EphemeralHint as u8,
            flags: 0,
            hardware_fingerprint: 0,
            gpu_device_uuid: [0; 16],
            isolation: 0,
            crypto: 0,
            entropy: 0,
            compute: 0,
            vendor_id: 0,
            device_id: 0,
            arch_stepping: 0,
            checksum: 0,
        }
    }

    pub fn tier(self) -> Option<AttestationTier> {
        AttestationTier::from_u8(self.attestation_tier)
    }

    pub const fn has_flag(self, bit: u8) -> bool {
        self.flags & bit != 0
    }

    /// Why N4 product copy must omit “hardware-attested vault”.
    ///
    /// Hardware facts only. Qualia still requires Human Agency consent before
    /// unlocking a vault. A missing file is a Qualia/host I/O deny, not this.
    pub const fn attested_vault_advertising_deny(self) -> Option<AdvertisingDeny> {
        if self.attestation_tier > AttestationTier::DieSerial as u8 {
            return Some(AdvertisingDeny::TierTooWeak);
        }
        if !self.has_flag(PROFILE_HAS_IOMMU) {
            return Some(AdvertisingDeny::MissingIommu);
        }
        if !self.has_flag(PROFILE_HAS_GPU_UUID) {
            return Some(AdvertisingDeny::MissingGpuUuid);
        }
        None
    }

    /// N4: IOMMU + GPU UUID + tier ≤ DieSerial. Does not grant Sanctuary.
    pub const fn supports_attested_vault_advertising(self) -> bool {
        self.attested_vault_advertising_deny().is_none()
    }

    pub fn from_context(
        ctx: &AnvilContext,
        vendor_id: u16,
        device_id: u16,
        arch_stepping: u32,
        pci_is_gpu: bool,
    ) -> Self {
        let mut p = Self::empty();
        p.attestation_tier = ctx.attestation_tier as u8;
        p.hardware_fingerprint = ctx.hardware_fingerprint;
        p.gpu_device_uuid = ctx.gpu_device_uuid;
        p.isolation = ctx.capabilities.isolation;
        p.crypto = ctx.capabilities.crypto;
        p.entropy = ctx.capabilities.entropy;
        p.compute = ctx.capabilities.compute;
        p.vendor_id = vendor_id;
        p.device_id = device_id;
        p.arch_stepping = arch_stepping;
        if ctx.has_gpu_uuid {
            p.flags |= PROFILE_HAS_GPU_UUID;
        }
        if ctx.capabilities.has_iommu() {
            p.flags |= PROFILE_HAS_IOMMU;
        }
        if ctx.capabilities.isolation & ISO_VTX_EPT != 0 {
            p.flags |= PROFILE_HAS_VTX;
        }
        if pci_is_gpu {
            p.flags |= PROFILE_PCI_IS_GPU;
        }
        if verify_sanctuary_policy(ctx, SanctuaryPolicy::hardware_isolated()) {
            p.flags |= PROFILE_SANCTUARY_HW_READY;
        }
        let mut bytes = [0u8; NODE_PROFILE_BYTES];
        p.write_body(&mut bytes);
        p.checksum = fnv1a64(&bytes[..56]);
        p
    }

    /// Overlay Etalon's live GPU die UUID (N3). Recomputes the checksum.
    ///
    /// A non-zero UUID sets [`PROFILE_HAS_GPU_UUID`] and may raise the
    /// recorded tier to [`AttestationTier::DieSerial`]. Zeros clear the
    /// flag. Does not seal and does not grant Sanctuary Mode.
    pub fn bind_gpu_device_uuid(&mut self, uuid: [u8; 16]) {
        self.gpu_device_uuid = uuid;
        let mut nonzero = false;
        let mut i = 0;
        while i < 16 {
            if uuid[i] != 0 {
                nonzero = true;
                break;
            }
            i += 1;
        }
        if nonzero {
            self.flags |= PROFILE_HAS_GPU_UUID;
            if self.attestation_tier > AttestationTier::DieSerial as u8 {
                self.attestation_tier = AttestationTier::DieSerial as u8;
            }
        } else {
            self.flags &= !PROFILE_HAS_GPU_UUID;
        }
        let mut bytes = [0u8; NODE_PROFILE_BYTES];
        self.write_body(&mut bytes);
        self.checksum = fnv1a64(&bytes[..56]);
    }

    /// Overlay facts onto a context. Recomputes the machine-anchor nquin.
    /// Does not seal; QualiaDB seals after calibration.
    pub fn apply_to_context(self, ctx: &mut AnvilContext) {
        ctx.capabilities.isolation = self.isolation;
        ctx.capabilities.crypto = self.crypto;
        ctx.capabilities.entropy = self.entropy;
        ctx.capabilities.compute = self.compute;
        if let Some(tier) = self.tier() {
            ctx.attestation_tier = tier;
        }
        if self.has_flag(PROFILE_HAS_GPU_UUID) {
            ctx.bind_gpu_device_uuid(self.gpu_device_uuid);
        } else {
            ctx.recompute_anchor();
        }
    }

    fn write_body(self, out: &mut [u8; NODE_PROFILE_BYTES]) {
        out[0..4].copy_from_slice(&self.magic);
        out[4..6].copy_from_slice(&self.version.to_le_bytes());
        out[6] = self.attestation_tier;
        out[7] = self.flags;
        out[8..16].copy_from_slice(&self.hardware_fingerprint.to_le_bytes());
        out[16..32].copy_from_slice(&self.gpu_device_uuid);
        out[32..36].copy_from_slice(&self.isolation.to_le_bytes());
        out[36..40].copy_from_slice(&self.crypto.to_le_bytes());
        out[40..44].copy_from_slice(&self.entropy.to_le_bytes());
        out[44..48].copy_from_slice(&self.compute.to_le_bytes());
        out[48..50].copy_from_slice(&self.vendor_id.to_le_bytes());
        out[50..52].copy_from_slice(&self.device_id.to_le_bytes());
        out[52..56].copy_from_slice(&self.arch_stepping.to_le_bytes());
        out[56..64].copy_from_slice(&self.checksum.to_le_bytes());
    }

    pub fn write_to(self, out: &mut [u8; NODE_PROFILE_BYTES]) {
        let mut body = self;
        body.checksum = 0;
        body.write_body(out);
        let sum = fnv1a64(&out[..56]);
        out[56..64].copy_from_slice(&sum.to_le_bytes());
    }

    pub fn to_bytes(self) -> [u8; NODE_PROFILE_BYTES] {
        let mut out = [0u8; NODE_PROFILE_BYTES];
        self.write_to(&mut out);
        out
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, NodeProfileError> {
        if bytes.len() < NODE_PROFILE_BYTES {
            return Err(NodeProfileError::Truncated);
        }
        if bytes[0..4] != NODE_PROFILE_MAGIC {
            return Err(NodeProfileError::BadMagic);
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != NODE_PROFILE_VERSION {
            return Err(NodeProfileError::UnsupportedVersion);
        }
        let expected = fnv1a64(&bytes[..56]);
        let checksum = u64::from_le_bytes(bytes[56..64].try_into().unwrap());
        if expected != checksum {
            return Err(NodeProfileError::ChecksumMismatch);
        }
        let attestation_tier = bytes[6];
        if AttestationTier::from_u8(attestation_tier).is_none() {
            return Err(NodeProfileError::BadTier);
        }
        let mut gpu_device_uuid = [0u8; 16];
        gpu_device_uuid.copy_from_slice(&bytes[16..32]);
        Ok(Self {
            magic: NODE_PROFILE_MAGIC,
            version,
            attestation_tier,
            flags: bytes[7],
            hardware_fingerprint: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
            gpu_device_uuid,
            isolation: u32::from_le_bytes(bytes[32..36].try_into().unwrap()),
            crypto: u32::from_le_bytes(bytes[36..40].try_into().unwrap()),
            entropy: u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
            compute: u32::from_le_bytes(bytes[44..48].try_into().unwrap()),
            vendor_id: u16::from_le_bytes([bytes[48], bytes[49]]),
            device_id: u16::from_le_bytes([bytes[50], bytes[51]]),
            arch_stepping: u32::from_le_bytes(bytes[52..56].try_into().unwrap()),
            checksum,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::register::ISO_VTD_IOMMU;

    #[test]
    fn round_trip_empty() {
        let p = NodeProfile64::empty();
        let bytes = p.to_bytes();
        let parsed = NodeProfile64::parse(&bytes).unwrap();
        assert_eq!(parsed.magic, NODE_PROFILE_MAGIC);
        assert_eq!(parsed.version, NODE_PROFILE_VERSION);
        assert_eq!(parsed.attestation_tier, AttestationTier::EphemeralHint as u8);
    }

    #[test]
    fn from_context_sets_iommu_flag() {
        let mut ctx = AnvilContext::empty();
        ctx.capabilities.isolation = ISO_VTD_IOMMU | ISO_VTX_EPT;
        ctx.attestation_tier = AttestationTier::DieSerial;
        ctx.bind_gpu_device_uuid([0xAA; 16]);
        let p = NodeProfile64::from_context(&ctx, 0x10DE, 0x2531, 0x0000_00A1, true);
        assert!(p.has_flag(PROFILE_HAS_GPU_UUID));
        assert!(p.has_flag(PROFILE_HAS_IOMMU));
        assert!(p.has_flag(PROFILE_HAS_VTX));
        assert!(p.has_flag(PROFILE_PCI_IS_GPU));
        assert_eq!(p.vendor_id, 0x10DE);
        assert_eq!(p.device_id, 0x2531);
        let parsed = NodeProfile64::parse(&p.to_bytes()).unwrap();
        assert_eq!(parsed.gpu_device_uuid, [0xAA; 16]);
    }

    #[test]
    fn reject_bad_magic() {
        let mut bytes = NodeProfile64::empty().to_bytes();
        bytes[0] = 0x00;
        assert_eq!(
            NodeProfile64::parse(&bytes),
            Err(NodeProfileError::BadMagic)
        );
    }

    #[test]
    fn apply_round_trip_gpu_uuid() {
        let mut ctx = AnvilContext::empty();
        ctx.capabilities.isolation = ISO_VTD_IOMMU;
        ctx.attestation_tier = AttestationTier::DieSerial;
        ctx.bind_gpu_device_uuid([0x55; 16]);
        let p = NodeProfile64::from_context(&ctx, 0x8086, 0xA788, 0x0C02, true);
        let mut back = AnvilContext::empty();
        p.apply_to_context(&mut back);
        assert!(back.has_gpu_uuid);
        assert_eq!(back.gpu_device_uuid, [0x55; 16]);
        assert_eq!(back.capabilities.isolation, ISO_VTD_IOMMU);
    }

    #[test]
    fn bind_gpu_uuid_sets_flag_and_checksum() {
        let mut p = NodeProfile64::from_context(
            &AnvilContext::empty(),
            0x10DE,
            0x2D58,
            0,
            true,
        );
        assert!(!p.has_flag(PROFILE_HAS_GPU_UUID));
        p.bind_gpu_device_uuid([0xAA; 16]);
        assert!(p.has_flag(PROFILE_HAS_GPU_UUID));
        assert_eq!(p.gpu_device_uuid, [0xAA; 16]);
        let parsed = NodeProfile64::parse(&p.to_bytes()).unwrap();
        assert_eq!(parsed.gpu_device_uuid, [0xAA; 16]);
        assert!(parsed.has_flag(PROFILE_HAS_GPU_UUID));
    }

    #[test]
    fn bind_zero_uuid_clears_flag() {
        let mut p = NodeProfile64::empty();
        p.bind_gpu_device_uuid([0x55; 16]);
        p.bind_gpu_device_uuid([0; 16]);
        assert!(!p.has_flag(PROFILE_HAS_GPU_UUID));
        assert_eq!(p.gpu_device_uuid, [0; 16]);
        NodeProfile64::parse(&p.to_bytes()).unwrap();
    }

    #[test]
    fn n4_isolated_profile_may_advertise() {
        let mut ctx = AnvilContext::empty();
        ctx.capabilities.isolation = ISO_VTD_IOMMU;
        ctx.attestation_tier = AttestationTier::DieSerial;
        ctx.bind_gpu_device_uuid([0xAA; 16]);
        let p = NodeProfile64::from_context(&ctx, 0x10DE, 0x2531, 0, true);
        assert!(p.supports_attested_vault_advertising());
        assert_eq!(p.attested_vault_advertising_deny(), None);
    }

    #[test]
    fn n4_missing_iommu_must_not_advertise() {
        let mut ctx = AnvilContext::empty();
        ctx.attestation_tier = AttestationTier::DieSerial;
        ctx.bind_gpu_device_uuid([0xAA; 16]);
        let p = NodeProfile64::from_context(&ctx, 0x10DE, 0x2D58, 0, true);
        assert!(!p.supports_attested_vault_advertising());
        assert_eq!(
            p.attested_vault_advertising_deny(),
            Some(AdvertisingDeny::MissingIommu)
        );
    }

    #[test]
    fn n4_missing_uuid_must_not_advertise() {
        let mut ctx = AnvilContext::empty();
        ctx.capabilities.isolation = ISO_VTD_IOMMU;
        ctx.attestation_tier = AttestationTier::DieSerial;
        let p = NodeProfile64::from_context(&ctx, 0x10DE, 0x2531, 0, true);
        assert!(!p.supports_attested_vault_advertising());
        assert_eq!(
            p.attested_vault_advertising_deny(),
            Some(AdvertisingDeny::MissingGpuUuid)
        );
    }

    #[test]
    fn n4_weak_tier_must_not_advertise() {
        let mut ctx = AnvilContext::empty();
        ctx.capabilities.isolation = ISO_VTD_IOMMU;
        ctx.attestation_tier = AttestationTier::DieSerial;
        ctx.bind_gpu_device_uuid([0xAA; 16]);
        let mut p = NodeProfile64::from_context(&ctx, 0x10DE, 0x2531, 0, true);
        p.attestation_tier = AttestationTier::TimingPuf as u8;
        let p = NodeProfile64::parse(&p.to_bytes()).unwrap();
        assert!(!p.supports_attested_vault_advertising());
        assert_eq!(
            p.attested_vault_advertising_deny(),
            Some(AdvertisingDeny::TierTooWeak)
        );
    }
}
