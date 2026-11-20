# NodeProfile64 — live host profile

**Status:** N0 layout + N1 live probe + N2 Qualia fail-closed + N3 Etalon `deviceUUID` bind + N4 advertising gate.

`NodeProfile64` is a **64-byte** hardware-facts artefact for *this* machine. It sits beside the seeded ontology SuperBlock (`assets/anvil_ontology.q42`). It contains no Qualia types.

The seeded ontology is a catalogue of SKUs. The profile is a scrape of the box you are on. Advertising a hardware-attested vault requires the profile file on that box (and Qualia fail-closed without it). This crate does not advertise a vault.

## File

| Field | Value |
|---|---|
| Magic | `NP64` |
| Size | 64 bytes |
| Default path | `assets/anvil_node_profile.np64` (machine-local; not a seeded SKU) |
| Writer | `anvil-scraper probe [path]` |

## Layout (little-endian)

| Offset | Type | Field |
|---|---|---|
| 0 | `[u8;4]` | magic `NP64` |
| 4 | `u16` | version `1` |
| 6 | `u8` | `AttestationTier` (0 SiliconRoot … 4 EphemeralHint) |
| 7 | `u8` | flags |
| 8 | `u64` | hardware fingerprint (FNV-1a of the machine-anchor body) |
| 16 | `[u8;16]` | GPU `deviceUUID` (Vulkan `VkPhysicalDeviceIDProperties`); zeros until Etalon binds |
| 32 | `u32` | isolation bits (`ISO_VTD_IOMMU`, `ISO_VTX_EPT`, …) |
| 36 | `u32` | crypto bits |
| 40 | `u32` | entropy bits |
| 44 | `u32` | compute bits |
| 48 | `u16` | vendor id (GPU PCI when probed, else CPU vendor) |
| 50 | `u16` | device id (GPU PCI, else 0) |
| 52 | `u32` | CPUID leaf-1 EAX (family/model/stepping) |
| 56 | `u64` | FNV-1a checksum of bytes `0..56` |

### Flags

| Bit | Name | Meaning |
|---|---|---|
| 0 | `PROFILE_HAS_GPU_UUID` | 16-byte GPU UUID is non-zero |
| 1 | `PROFILE_HAS_IOMMU` | ACPI DMAR/IVRS/IORT or `/sys/class/iommu` (or Apple Silicon DART) |
| 2 | `PROFILE_HAS_VTX` | VT-x / EPT CPUID bit |
| 3 | `PROFILE_SANCTUARY_HW_READY` | Anvil `verify_sanctuary_policy(hardware_isolated)` — **not** Qualia granting Sanctuary |
| 4 | `PROFILE_PCI_IS_GPU` | vendor/device are a display PCI pair |

Ephemeral MAC is omitted. A spoofed MAC must not appear in this artefact.

## Commands

```text
anvil-scraper                  # seeded ontology → assets/anvil_ontology.q42
anvil-scraper ontology [path]
anvil-scraper probe            # live profile → assets/anvil_node_profile.np64
anvil-scraper probe [path]
```

`probe` does not rewrite the ontology.

## What N1 does and does not scrape

Does:

- CPU crypto / entropy / AVX2 / AVX-VNNI (existing `probe_host`)
- CPU vendor + stepping
- IOMMU *table presence* (ACPI or sysfs), not “IOMMU is isolating every device”
- Display PCI vendor/device (Windows `EnumDisplayDevices`, Linux sysfs class `0x03`)

Does not (N1 probe):

- Vulkan `deviceUUID` (zeros until Etalon N3 bind)
- TPM EK quote
- NVMe EUI-64
- Sealed machine anchor (Qualia calibrates and seals)
- Human Agency consent

N1 can therefore write a valid profile on a box that is **not** Sanctuary-ready. That is the fail-closed input Qualia needs.

## N4 — advertising gate (F4 overlay)

`NodeProfile64::supports_attested_vault_advertising` is true only for IOMMU + GPU UUID + tier ≤ DieSerial. Qualia `hardware_attested_vault_copy` and `webizen-host-attestation` emit the phrase “hardware-attested vault” only then. A missing profile, this box without IOMMU, and F4 shipping without Anvil all fail closed (empty copy, `hardware_attested_vault: false`). Anvil still does not advertise a vault; Qualia still requires Human Agency consent to unlock. Workbench `GET /sanctuary/status` should call that crate once the host `vibe` pin can take the path dep.

## N2 — QualiaDB

`qualia-core-db` depends on `anvil-core` with `vault-security`. `identity::anvil_policy` reads this file:

- Missing or invalid file → `AttestationClaimError` (fail closed). Not a silent `false`.
- `grant_sanctuary_mode` still needs hardware isolation **and** Human Agency consent.
- `may_advertise_hardware_attested_vault` is true only when the file parses and carries IOMMU + non-zero GPU UUID + tier ≤ 1.

Default path: `ANVIL_NODE_PROFILE`, else `{QUALIA_STORAGE_PATH}/anvil_node_profile.np64`.

## N3 — Etalon

Etalon path-deps `anvil-gpu`. After Qualia/Anvil `probe_host`, Etalon fills `VkPhysicalDeviceIDProperties::deviceUUID` into the same `AnvilContext` (`etalon_bind_gpu`) and overlays it on this file (`NodeProfile64::bind_gpu_device_uuid`). DXGI LUID and WebGPU adapter.info are not this slot. A missing UUID stays zeros (fail closed). Spoofed MAC is omitted from this artefact; UUID drift after Qualia seals the machine anchor trips Sanctuary hardware readiness.
