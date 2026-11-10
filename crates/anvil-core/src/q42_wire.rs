//! Zero-copy `.q42` SuperBlock header and 48-byte ontology nquin records.

use crate::register::{DOMAIN_COMPUTE, DOMAIN_CRYPTO, DOMAIN_ID_FABRIC, DOMAIN_ISOLATION, DOMAIN_MEMORY};

/// `"Q42\x01"`
pub const Q42_MAGIC: [u8; 4] = [0x51, 0x34, 0x32, 0x01];
pub const Q42_SCHEMA_VERSION: u16 = 0x0001;
pub const Q42_HEADER_BYTES: usize = 64;
pub const Q42_FLAG_ONTOLOGY: u16 = 1 << 0;
pub const Q42_FLAG_MACHINE_STATE: u16 = 1 << 1;
pub const Q42_FLAG_SANCTUARY: u16 = 1 << 2;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Q42ParseError {
    TruncatedHeader,
    BadMagic,
    UnsupportedVersion,
    TruncatedRecords,
    RecordCountOverflow,
    ChecksumMismatch,
}

#[inline]
pub const fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let mut i = 0;
    while i < bytes.len() {
        h ^= bytes[i] as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
        i += 1;
    }
    h
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Q42Header {
    pub magic: [u8; 4],
    pub schema_version: u16,
    pub flags: u16,
    pub record_count: u64,
    pub hardware_fingerprint: u64,
    pub domain_offsets: [u64; 4],
    pub header_checksum: u64,
}

impl Q42Header {
    pub const fn empty_ontology() -> Self {
        Self {
            magic: Q42_MAGIC,
            schema_version: Q42_SCHEMA_VERSION,
            flags: Q42_FLAG_ONTOLOGY,
            record_count: 0,
            hardware_fingerprint: 0,
            domain_offsets: [Q42_HEADER_BYTES as u64; 4],
            header_checksum: 0,
        }
    }

    pub fn write_to(&self, out: &mut [u8; Q42_HEADER_BYTES]) {
        out[0..4].copy_from_slice(&self.magic);
        out[4..6].copy_from_slice(&self.schema_version.to_le_bytes());
        out[6..8].copy_from_slice(&self.flags.to_le_bytes());
        out[8..16].copy_from_slice(&self.record_count.to_le_bytes());
        out[16..24].copy_from_slice(&self.hardware_fingerprint.to_le_bytes());
        for (i, off) in self.domain_offsets.iter().enumerate() {
            let s = 24 + i * 8;
            out[s..s + 8].copy_from_slice(&off.to_le_bytes());
        }
        let checksum = fnv1a64(&out[..56]);
        out[56..64].copy_from_slice(&checksum.to_le_bytes());
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, Q42ParseError> {
        if bytes.len() < Q42_HEADER_BYTES {
            return Err(Q42ParseError::TruncatedHeader);
        }
        if bytes[0..4] != Q42_MAGIC {
            return Err(Q42ParseError::BadMagic);
        }
        let schema_version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if schema_version != Q42_SCHEMA_VERSION {
            return Err(Q42ParseError::UnsupportedVersion);
        }
        let expected = fnv1a64(&bytes[..56]);
        let header_checksum = u64::from_le_bytes(bytes[56..64].try_into().unwrap());
        if expected != header_checksum {
            return Err(Q42ParseError::ChecksumMismatch);
        }
        let mut domain_offsets = [0u64; 4];
        for i in 0..4 {
            let s = 24 + i * 8;
            domain_offsets[i] = u64::from_le_bytes(bytes[s..s + 8].try_into().unwrap());
        }
        Ok(Self {
            magic: Q42_MAGIC,
            schema_version,
            flags: u16::from_le_bytes([bytes[6], bytes[7]]),
            record_count: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
            hardware_fingerprint: u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
            domain_offsets,
            header_checksum,
        })
    }
}

/// Ontology nquin: 8-byte subject, 4-byte domain predicate, 36-byte payload.
#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AnvilNquin {
    pub vendor_id: u16,
    pub device_id: u16,
    pub arch_stepping: u32,
    pub domain: u32,
    pub payload: [u8; 36],
}

impl AnvilNquin {
    pub const fn new(vendor_id: u16, device_id: u16, arch_stepping: u32, domain: u32) -> Self {
        Self {
            vendor_id,
            device_id,
            arch_stepping,
            domain,
            payload: [0; 36],
        }
    }

    pub const fn subject_key(self) -> u64 {
        ((self.vendor_id as u64) << 48)
            | ((self.device_id as u64) << 32)
            | (self.arch_stepping as u64)
    }

    pub fn write_to(self, out: &mut [u8; 48]) {
        out[0..2].copy_from_slice(&self.vendor_id.to_le_bytes());
        out[2..4].copy_from_slice(&self.device_id.to_le_bytes());
        out[4..8].copy_from_slice(&self.arch_stepping.to_le_bytes());
        out[8..12].copy_from_slice(&self.domain.to_le_bytes());
        out[12..48].copy_from_slice(&self.payload);
    }

    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 48 {
            return None;
        }
        let mut payload = [0u8; 36];
        payload.copy_from_slice(&bytes[12..48]);
        Some(Self {
            vendor_id: u16::from_le_bytes([bytes[0], bytes[1]]),
            device_id: u16::from_le_bytes([bytes[2], bytes[3]]),
            arch_stepping: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
            domain: u32::from_le_bytes(bytes[8..12].try_into().ok()?),
            payload,
        })
    }

    pub fn known_domain(self) -> bool {
        matches!(
            self.domain,
            DOMAIN_COMPUTE | DOMAIN_MEMORY | DOMAIN_CRYPTO | DOMAIN_ISOLATION | DOMAIN_ID_FABRIC
        )
    }
}

pub fn records_from_payload(bytes: &[u8]) -> Result<&[u8], Q42ParseError> {
    let header = Q42Header::parse(bytes)?;
    let count = header.record_count as usize;
    let payload_bytes = count
        .checked_mul(48)
        .ok_or(Q42ParseError::RecordCountOverflow)?;
    let end = Q42_HEADER_BYTES
        .checked_add(payload_bytes)
        .ok_or(Q42ParseError::RecordCountOverflow)?;
    if bytes.len() < end {
        return Err(Q42ParseError::TruncatedRecords);
    }
    Ok(&bytes[Q42_HEADER_BYTES..end])
}

pub fn write_superblock(header: &mut Q42Header, records: &[AnvilNquin], out: &mut [u8]) -> Result<usize, Q42ParseError> {
    let need = Q42_HEADER_BYTES
        .checked_add(records.len().checked_mul(48).ok_or(Q42ParseError::RecordCountOverflow)?)
        .ok_or(Q42ParseError::RecordCountOverflow)?;
    if out.len() < need {
        return Err(Q42ParseError::TruncatedRecords);
    }
    header.record_count = records.len() as u64;
    let mut hdr = [0u8; Q42_HEADER_BYTES];
    header.write_to(&mut hdr);
    header.header_checksum = u64::from_le_bytes(hdr[56..64].try_into().unwrap());
    out[..Q42_HEADER_BYTES].copy_from_slice(&hdr);
    for (i, rec) in records.iter().enumerate() {
        let mut rec_bytes = [0u8; 48];
        rec.write_to(&mut rec_bytes);
        let s = Q42_HEADER_BYTES + i * 48;
        out[s..s + 48].copy_from_slice(&rec_bytes);
    }
    Ok(need)
}

const _: () = assert!(core::mem::size_of::<Q42Header>() == 64);
const _: () = assert!(core::mem::size_of::<AnvilNquin>() == 48);
const _: () = assert!(core::mem::align_of::<AnvilNquin>() == 16);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_empty_ontology() {
        let mut header = Q42Header::empty_ontology();
        let mut buf = [0u8; 64];
        let n = write_superblock(&mut header, &[], &mut buf).unwrap();
        assert_eq!(n, 64);
        let parsed = Q42Header::parse(&buf).unwrap();
        assert_eq!(parsed.record_count, 0);
        assert_eq!(parsed.flags, Q42_FLAG_ONTOLOGY);
    }

    #[test]
    fn reject_bad_magic() {
        let mut buf = [0u8; 64];
        buf[0..4].copy_from_slice(&[0x00, 0x11, 0x22, 0x33]);
        assert_eq!(Q42Header::parse(&buf), Err(Q42ParseError::BadMagic));
    }
}
