# Anvil

Tier-0 `#![no_std]` hardware capability ontology and GPU/NPU instruction
router. Anvil sits **side-by-side with QualiaDB** and is an **enabler for
Etalon**.

## Ownership

- **QualiaDB** owns the CPU path: 48-byte nquin sieves, `.q42` graph runtime,
  vault crypto dispatch, and host SIMD. It also **manages** the human-centric
  cryptographic identifier fabric (did:q42 binding, Sanctuary Mode, Human
  Agency, Personal Autonomy) for Webizen / Wellfair.
- **Anvil** does not take that path and is not a QualiaDB crate. Folding it
  in would grow the monolith Etalon was extracted to avoid.
- **Anvil** publishes hardware *facts*: `.q42` SuperBlock ontology, GPU/NPU
  adapter truth, Naga/WGSL specialization hints, memory/DMA routes, and
  tiered identifier *slots* (TPM quote hash, GPU `deviceUUID`, NVMe EUI-64,
  timing fingerprint). QualiaDB decides Sanctuary / vault policy over those
  facts. Etalon binds shaders, `deviceUUID`, and zero-copy vs PCIe from them.
- Etalon may depend on QualiaDB for graph/vault/human-centric fabric. GPU
  routing still comes from Anvil so Etalon does not pull the database engine
  for workgroup sizes.

## Crates

- `anvil-core` — `#![no_std]`, zero external runtime dependencies
- `anvil-gpu` — adapter report + Naga/WGSL specialization hints for Etalon
- `anvil-npu` — graph-slot handles for NPU / WebNN
- `anvil-wasm` — browser bootstrap types (relaxed-simd, WebGPU, WebNN)
- `tools/anvil-scraper` — compiles `schemas/hardware_ontology.yaml` into `assets/anvil_ontology.q42`

## Guardrails

- Primitive is an **nquin** (never "qualiaquin")
- Human-rights language: Human Agency, Personal Autonomy, Sanctuary Mode
- "Identity" only for enumerated cryptography-supported identifiers
- Tests use generic placeholders (`0x55`, `0xAA`, `Device_001`, `Node_Alpha`)

## Consumers

```toml
# Etalon (GPU / shader routing)
anvil-core = { git = "https://github.com/mediaprophet/anvil", default-features = false, features = ["std"] }
anvil-gpu  = { git = "https://github.com/mediaprophet/anvil" }

# QualiaDB (identifier-fabric facts; Sanctuary policy stays in QualiaDB)
anvil-core = { git = "https://github.com/mediaprophet/anvil", default-features = false, features = ["vault-security"] }
```
