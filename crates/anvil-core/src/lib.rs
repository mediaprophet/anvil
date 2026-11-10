//! Hardware capability ontology, 48-byte nquin wire, and GPU/NPU router.
//!
//! `anvil-core` is `#![no_std]` with zero external runtime dependencies.
//! QualiaDB owns CPU nquin execution. Anvil enables Etalon: adapter truth,
//! shader specialization hints, memory routes, and identifier fabric.

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![allow(clippy::missing_safety_doc)]

#[cfg(any(test, feature = "std"))]
extern crate std;

pub mod calibrate;
pub mod crypto_probe;
pub mod id_fabric;
pub mod ir;
pub mod ontology;
pub mod q42_wire;
pub mod register;
pub mod simd;

pub use calibrate::{calibrate_cpu_tile, CalibrationSample};
pub use crypto_probe::{probe_crypto, CryptoIsolationBits};
pub use id_fabric::{
    verify_sanctuary_policy, AnvilContext, AttestationTier, IdentifierRecord, SanctuaryPolicy,
};
pub use ir::{
    AnvilDispatchTable, AnvilInstruction, AnvilOpcode, POLICY_DETERMINISTIC_FP, POLICY_SANCTUARY,
    POLICY_ZERO_COPY,
};
pub use ontology::{lookup, seeded_ontology, BASELINE_COUNT};
pub use q42_wire::{
    fnv1a64, write_superblock, AnvilNquin, Q42Header, Q42ParseError, Q42_FLAG_MACHINE_STATE,
    Q42_FLAG_ONTOLOGY, Q42_FLAG_SANCTUARY, Q42_HEADER_BYTES, Q42_MAGIC, Q42_SCHEMA_VERSION,
};
pub use register::{
    CapabilityRegister, ComputeUnitDescriptor, MemoryRoute, SystemHardwareMap, CAP_ARM_I8MM,
    CAP_AVX2, CAP_AVX_VNNI, CAP_HW_RAY_TRACING, CAP_NEON_DOTPROD, CAP_VK_COOP_MATRIX, CAP_WGSL_DP4A,
    CAP_WASM_RELAXED_SIMD, CRYPTO_AES_NI, CRYPTO_SHA_NI, CRYPTO_VAES, DOMAIN_COMPUTE, DOMAIN_CRYPTO,
    DOMAIN_ID_FABRIC, DOMAIN_ISOLATION, DOMAIN_MEMORY, ISO_ARM_PAC_MTE, ISO_ARM_SMMU, ISO_INTEL_CET,
    ISO_VTD_IOMMU, ISO_VTX_EPT,
};
pub use simd::{execute_dot_u8_i8_host, execute_dot_u8_i8_tile, AnvilVectorEngine, ScalarEngine};
