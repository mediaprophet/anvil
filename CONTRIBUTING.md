# Contributing

Anvil is a standalone hardware ontology. It sits beside QualiaDB and enables
Etalon. Do not fold it into QualiaDB.

## Ownership

- QualiaDB owns CPU nquin sieves, the `.q42` graph, vault crypto, and
  **manages** the human-centric identifier fabric (did:q42, Sanctuary Mode,
  Human Agency, Personal Autonomy).
- Anvil publishes hardware facts: ontology SuperBlock, GPU/NPU routing,
  Naga/WGSL hints, memory routes, and identifier **slots**.
- Etalon consumes `anvil-core` + `anvil-gpu` for shaders and `deviceUUID`.

## Guardrails

- Human-centric modality: Human Agency, Human Dignity, Personal Autonomy,
  Informatics Fiduciary, Personhood, Civics, Sanctuary Mode. Never
  "sovereign" / "sovereignty".
- "Identity" only for enumerated cryptography-supported identifiers.
- Primitive is an **nquin** (never "qualiaquin").
- Tests use generic placeholders: `0x55`, `0xAA`, `Device_001`, `Node_Alpha`.
