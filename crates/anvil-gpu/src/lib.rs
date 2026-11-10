//! Etalon GPU handshake.
//!
//! Anvil-gpu does not pull wgpu/naga yet. It turns PCI ids + `deviceUUID`
//! into workgroup / SIMD / memory-route hints Etalon injects into Naga IR.
//! QualiaDB does not route shaders through this crate.

#![forbid(unsafe_op_in_unsafe_fn)]

pub mod adapter_probe;
pub mod specializer;

pub use adapter_probe::{etalon_bind_gpu, AdapterReport};
pub use specializer::{hints_for_pci, SpecializerHints};
