//! Browser bootstrap facts for Webizen Studio / Poet.
//!
//! No wasm-bindgen dependency. Host JS fills [`WasmProbe`] after
//! `WebAssembly.validate`, `navigator.gpu.requestAdapter`, and
//! `navigator.ml.createContext`.

#![no_std]

use anvil_core::{AnvilContext, AttestationTier, CAP_WASM_RELAXED_SIMD, CAP_WGSL_DP4A};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WasmProbe {
    pub simd128: bool,
    pub relaxed_simd: bool,
    pub webgpu: bool,
    pub webnn_npu: bool,
    /// Bucketed `adapter.info` is Tier 4 only.
    pub adapter_bucket: [u8; 16],
}

impl WasmProbe {
    pub const fn empty() -> Self {
        Self {
            simd128: false,
            relaxed_simd: false,
            webgpu: false,
            webnn_npu: false,
            adapter_bucket: [0; 16],
        }
    }

    pub fn apply(self, ctx: &mut AnvilContext) {
        if self.relaxed_simd {
            ctx.capabilities.compute |= CAP_WASM_RELAXED_SIMD;
        }
        if self.webgpu {
            ctx.capabilities.compute |= CAP_WGSL_DP4A;
        }
        // Bucketed adapter.info is Tier 4. Never raise attestation from it.
        let _ = self.adapter_bucket;
        let _ = self.simd128;
        let _ = self.webnn_npu;
        let _ = AttestationTier::EphemeralHint;
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;

    #[test]
    fn bucketed_adapter_stays_ephemeral() {
        let mut ctx = AnvilContext::empty();
        let mut probe = WasmProbe::empty();
        probe.webgpu = true;
        probe.adapter_bucket = [0x55; 16];
        probe.apply(&mut ctx);
        assert_eq!(ctx.attestation_tier, AttestationTier::EphemeralHint);
        assert_eq!(ctx.capabilities.compute & CAP_WGSL_DP4A, CAP_WGSL_DP4A);
    }
}
