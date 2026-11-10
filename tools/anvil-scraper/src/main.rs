//! Compile the seeded ontology into `assets/anvil_ontology.q42`.
//!
//! Later passes ingest pci.ids / gpuinfo; this binary emits the five
//! baseline SKUs so Etalon and QualiaDB share one SuperBlock.

use anvil_core::{seeded_ontology, write_superblock, Q42Header, Q42_HEADER_BYTES};
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("assets/anvil_ontology.q42"));
    let records = seeded_ontology();
    let mut header = Q42Header::empty_ontology();
    let mut buf = vec![0u8; Q42_HEADER_BYTES + records.len() * 48];
    let n = write_superblock(&mut header, &records, &mut buf).expect("superblock");
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).expect("assets dir");
    }
    fs::write(&out, &buf[..n]).expect("write q42");
    eprintln!(
        "wrote {} ({} records, fingerprint {:#x})",
        out.display(),
        records.len(),
        header.hardware_fingerprint
    );
}
