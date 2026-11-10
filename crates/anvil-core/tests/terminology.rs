//! Guardrails: no restricted civic terms, nquin naming, generic test bytes.

const LIB: &str = include_str!("../src/lib.rs");
const ID: &str = include_str!("../src/id_fabric.rs");
const IR: &str = include_str!("../src/ir.rs");
const CAL: &str = include_str!("../src/calibrate.rs");

fn forbidden(hay: &str) {
    let lower = hay.to_ascii_lowercase();
    assert!(!lower.contains("sovereign"), "human-centric modality");
    assert!(!lower.contains("qualiaquin"), "primitive is nquin");
}

#[test]
fn core_sources_keep_guardrails() {
    for src in [LIB, ID, IR, CAL] {
        forbidden(src);
    }
    assert!(CAL.contains("0x55"));
    assert!(CAL.contains("0xAA"));
    assert!(ID.contains("Human Agency") || ID.contains("Sanctuary"));
}

#[test]
fn identifier_word_is_enumerated_only() {
    let lower = ID.to_ascii_lowercase();
    // "identity" is reserved; this module talks about identifier slots.
    assert!(!lower.contains("identity"));
    assert!(lower.contains("identifier"));
}
