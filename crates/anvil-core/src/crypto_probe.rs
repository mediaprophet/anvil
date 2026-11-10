//! Hardware cryptography, entropy, and isolation probes.
//!
//! These bits are facts for QualiaDB's vault / Sanctuary manager. Anvil does
//! not decide Human Agency policy.

use crate::register::{
    CapabilityRegister, CRYPTO_AES_NI, CRYPTO_SHA_NI, CRYPTO_VAES, CRYPTO_VPCLMULQDQ,
    ENTROPY_RDRAND, ENTROPY_RDSEED, ISO_INTEL_CET, ISO_VTX_EPT,
};

/// Crypto / entropy / isolation bitmask triple returned by [`probe_crypto`].
pub type CryptoIsolationBits = CapabilityRegister;

pub fn probe_crypto() -> CryptoIsolationBits {
    #[cfg(target_arch = "x86_64")]
    {
        probe_x86()
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        CapabilityRegister::empty()
    }
}

#[cfg(target_arch = "x86_64")]
fn probe_x86() -> CapabilityRegister {
    use core::arch::x86_64::__cpuid_count;

    let mut bits = CapabilityRegister::empty();
    let leaf1 = __cpuid_count(1, 0);
    // ECX bit 5 VT-x, bit 25 AES-NI, bit 30 RDRAND
    if leaf1.ecx & (1 << 5) != 0 {
        bits.isolation |= ISO_VTX_EPT;
    }
    if leaf1.ecx & (1 << 25) != 0 {
        bits.crypto |= CRYPTO_AES_NI;
    }
    if leaf1.ecx & (1 << 30) != 0 {
        bits.entropy |= ENTROPY_RDRAND;
    }

    let leaf7 = __cpuid_count(7, 0);
    // EBX bit 18 RDSEED, bit 29 SHA
    if leaf7.ebx & (1 << 18) != 0 {
        bits.entropy |= ENTROPY_RDSEED;
    }
    if leaf7.ebx & (1 << 29) != 0 {
        bits.crypto |= CRYPTO_SHA_NI;
    }
    // ECX bit 9 VAES, bit 10 VPCLMULQDQ, bit 7 CET_SS
    if leaf7.ecx & (1 << 9) != 0 {
        bits.crypto |= CRYPTO_VAES;
    }
    if leaf7.ecx & (1 << 10) != 0 {
        bits.crypto |= CRYPTO_VPCLMULQDQ;
    }
    if leaf7.ecx & (1 << 7) != 0 {
        bits.isolation |= ISO_INTEL_CET;
    }
    bits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_returns_struct() {
        let bits = probe_crypto();
        let _ = bits.crypto | bits.entropy | bits.isolation;
    }
}
