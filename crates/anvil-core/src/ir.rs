//! 48-byte `AnvilInstruction` matching nquin size and alignment.
//!
//! CPU nquin sieves stay in QualiaDB. These opcodes are the Etalon-facing
//! tile vocabulary (GPU/NPU/calibrator), plus shared vault crypto tags.

/// Bit 0: Sanctuary Mode required for this tile.
pub const POLICY_SANCTUARY: u16 = 1 << 0;
/// Bit 1: route only through unified zero-copy host memory.
pub const POLICY_ZERO_COPY: u16 = 1 << 1;
/// Bit 2: require deterministic floating point.
pub const POLICY_DETERMINISTIC_FP: u16 = 1 << 2;

#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AnvilOpcode {
    OpDot4I8PackedTile = 0x0101,
    OpMatVecQ4KBlock = 0x0102,
    OpNquinSieveSetMatch = 0x0103,
    OpManifold10DProject = 0x0201,
    OpRayMarchStepField = 0x0202,
    OpAesGcmSealBlock = 0x0301,
    OpSha256HashBlock = 0x0302,
}

impl AnvilOpcode {
    #[inline]
    pub const fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x0101 => Some(Self::OpDot4I8PackedTile),
            0x0102 => Some(Self::OpMatVecQ4KBlock),
            0x0103 => Some(Self::OpNquinSieveSetMatch),
            0x0201 => Some(Self::OpManifold10DProject),
            0x0202 => Some(Self::OpRayMarchStepField),
            0x0301 => Some(Self::OpAesGcmSealBlock),
            0x0302 => Some(Self::OpSha256HashBlock),
            _ => None,
        }
    }

    #[inline]
    pub const fn as_u16(self) -> u16 {
        self as u16
    }
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug)]
pub struct AnvilInstruction {
    pub opcode: AnvilOpcode,
    pub policy_flags: u16,
    pub element_count: u32,
    pub src_a_offset: u64,
    pub src_b_offset: u64,
    pub dst_offset: u64,
    pub inline_params: [u8; 16],
}

impl AnvilInstruction {
    pub const fn new(opcode: AnvilOpcode, element_count: u32) -> Self {
        Self {
            opcode,
            policy_flags: 0,
            element_count,
            src_a_offset: 0,
            src_b_offset: 0,
            dst_offset: 0,
            inline_params: [0; 16],
        }
    }

    #[inline]
    pub const fn requires_sanctuary(self) -> bool {
        self.policy_flags & POLICY_SANCTUARY != 0
    }
}

/// Pre-bound tile entry points. CPU sieve pointers remain QualiaDB-owned;
/// Anvil fills GPU/calibrator slots for Etalon.
#[derive(Copy, Clone)]
pub struct AnvilDispatchTable {
    pub dot_u8_i8_tile: Option<unsafe fn(*const u8, *const i8, usize) -> i32>,
    pub workgroup_size: u16,
    pub simd_width: u16,
}

impl AnvilDispatchTable {
    pub const fn empty() -> Self {
        Self {
            dot_u8_i8_tile: None,
            workgroup_size: 64,
            simd_width: 16,
        }
    }
}

const _: () = assert!(core::mem::size_of::<AnvilInstruction>() == 48);
const _: () = assert!(core::mem::align_of::<AnvilInstruction>() == 16);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_is_nquin_sized() {
        assert_eq!(core::mem::size_of::<AnvilInstruction>(), 48);
        assert!(AnvilOpcode::from_u16(0x0202) == Some(AnvilOpcode::OpRayMarchStepField));
        assert!(AnvilOpcode::from_u16(0x00FF).is_none());
    }
}
