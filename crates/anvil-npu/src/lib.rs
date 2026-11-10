//! Persistent NPU / WebNN subgraph slots.
//!
//! Standalone NPUs compile on a millisecond graph path. Anvil stores a
//! fixed-shape handle Etalon binds at boot; QualiaDB does not execute these
//! slots as nquin sieves.

#![no_std]

use anvil_core::ComputeUnitDescriptor;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NpuKind {
    IntelGna = 1,
    AppleAne = 2,
    GoogleTensor = 3,
    WebNn = 4,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AnvilGraphSlot {
    pub kind: NpuKind,
    pub revision: u8,
    pub shape_hash: u64,
}

impl AnvilGraphSlot {
    pub const fn from_unit(unit: ComputeUnitDescriptor, shape_hash: u64) -> Option<Self> {
        let kind = match (unit.vendor_id, unit.npu_revision) {
            (0x8086, 30) => NpuKind::IntelGna,
            (0x106B, _) if unit.npu_revision != 0 => NpuKind::AppleAne,
            (0x13B5, _) if unit.npu_revision != 0 => NpuKind::GoogleTensor,
            _ => return None,
        };
        Some(Self {
            kind,
            revision: unit.npu_revision,
            shape_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use anvil_core::lookup;

    #[test]
    fn gna_and_ane_slots() {
        let gna = AnvilGraphSlot::from_unit(lookup(0x8086, 0xB067).unwrap(), 0x55).unwrap();
        assert_eq!(gna.kind, NpuKind::IntelGna);
        let ane = AnvilGraphSlot::from_unit(lookup(0x106B, 0x0001).unwrap(), 0xAA).unwrap();
        assert_eq!(ane.kind, NpuKind::AppleAne);
    }
}
