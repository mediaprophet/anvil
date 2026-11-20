//! Compile the seeded ontology, or probe this machine into NodeProfile64.
//!
//! Default (and `ontology`) still emits the five baseline SKUs so Etalon and
//! QualiaDB share one SuperBlock. `probe` writes a live 64-byte host profile
//! beside that artefact. Probe does not overwrite the ontology.

use anvil_core::{
    probe_live_profile, seeded_ontology, write_superblock, NodeProfile64, Q42Header,
    NODE_PROFILE_BYTES, Q42_HEADER_BYTES,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None => write_ontology(default_ontology()),
        Some("probe") => {
            let out = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(default_profile);
            write_probe(out);
        }
        Some("ontology") => {
            let out = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(default_ontology);
            write_ontology(out);
        }
        Some(path) => write_ontology(PathBuf::from(path)),
    }
}

fn default_ontology() -> PathBuf {
    PathBuf::from("assets/anvil_ontology.q42")
}

fn default_profile() -> PathBuf {
    PathBuf::from("assets/anvil_node_profile.np64")
}

fn write_ontology(out: PathBuf) {
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

fn write_probe(out: PathBuf) {
    let profile = probe_live_profile();
    let bytes = profile.to_bytes();
    if bytes.len() != NODE_PROFILE_BYTES {
        eprintln!("internal error: profile size");
        process::exit(1);
    }
    match NodeProfile64::parse(&bytes) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("internal error: profile did not parse: {e:?}");
            process::exit(1);
        }
    }
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).expect("assets dir");
        }
    }
    fs::write(&out, bytes).expect("write np64");
    eprintln!(
        "wrote {} (tier {}, flags {:#04x}, pci {:04x}:{:04x}, fingerprint {:#x}, iommu {}, gpu_uuid {})",
        out.display(),
        profile.attestation_tier,
        profile.flags,
        profile.vendor_id,
        profile.device_id,
        profile.hardware_fingerprint,
        profile.has_flag(anvil_core::PROFILE_HAS_IOMMU),
        profile.has_flag(anvil_core::PROFILE_HAS_GPU_UUID),
    );
    eprintln!(
        "seeded ontology unchanged; GPU UUID stays zeros until Etalon N3 bind; Qualia fail-closed without this file"
    );
}
