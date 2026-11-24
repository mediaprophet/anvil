//! Live host scrape for [`NodeProfile64`].
//!
//! CPUID / MRS facts stay in [`crate::crypto_probe`]. This module adds
//! std-only IOMMU table presence and display PCI ids. Vulkan `deviceUUID`
//! is left empty until Etalon binds it. MAC addresses are not stored.

use crate::id_fabric::AnvilContext;
use crate::node_profile::NodeProfile64;
use crate::register::{ISO_ARM_SMMU, ISO_VTD_IOMMU};

/// Probe this process's machine and emit a [`NodeProfile64`].
///
/// Does not seal a machine anchor. Does not grant Sanctuary Mode.
pub fn probe_live_profile() -> NodeProfile64 {
    let mut ctx = AnvilContext::probe_host();
    if probe_iommu_present() {
        ctx.capabilities.isolation |= iommu_bit();
    }
    ctx.recompute_anchor();

    let (cpu_vendor, arch_stepping) = crate::crypto_probe::probe_cpu_vendor_stepping();
    let gpu = probe_gpu_pci();
    let (vendor_id, device_id, pci_is_gpu) = match gpu {
        Some((v, d)) => (v, d, true),
        None => (cpu_vendor, 0, false),
    };
    NodeProfile64::from_context(&ctx, vendor_id, device_id, arch_stepping, pci_is_gpu)
}

fn iommu_bit() -> u32 {
    if cfg!(target_arch = "aarch64") {
        ISO_ARM_SMMU
    } else {
        ISO_VTD_IOMMU
    }
}

fn probe_iommu_present() -> bool {
    #[cfg(windows)]
    {
        acpi_table_present(b"DMAR") || acpi_table_present(b"IVRS") || acpi_table_present(b"IORT")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        linux_iommu_present()
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        true
    }
    #[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
    {
        false
    }
    #[cfg(not(any(windows, unix)))]
    {
        false
    }
}

#[cfg(windows)]
fn acpi_table_present(sig: &[u8; 4]) -> bool {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemFirmwareTable(
            provider: u32,
            id: u32,
            buf: *mut u8,
            size: u32,
        ) -> u32;
    }
    let provider = u32::from_be_bytes(*b"ACPI");
    let id = u32::from_be_bytes(*sig);
    let n = unsafe { GetSystemFirmwareTable(provider, id, core::ptr::null_mut(), 0) };
    n > 0
}

#[cfg(all(unix, not(target_os = "macos")))]
fn linux_iommu_present() -> bool {
    use std::path::Path;
    Path::new("/sys/firmware/acpi/tables/DMAR").is_file()
        || Path::new("/sys/firmware/acpi/tables/IVRS").is_file()
        || Path::new("/sys/firmware/acpi/tables/IORT").is_file()
        || Path::new("/sys/class/iommu").read_dir().map(|mut d| d.next().is_some()).unwrap_or(false)
}

fn probe_gpu_pci() -> Option<(u16, u16)> {
    #[cfg(windows)]
    {
        windows_display_pci()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        linux_display_pci()
    }
    #[cfg(not(any(windows, all(unix, not(target_os = "macos")))))]
    {
        None
    }
}

#[cfg(windows)]
fn windows_display_pci() -> Option<(u16, u16)> {
    #[repr(C)]
    struct DisplayDeviceW {
        cb: u32,
        device_name: [u16; 32],
        device_string: [u16; 128],
        state_flags: u32,
        device_id: [u16; 128],
        device_key: [u16; 128],
    }
    #[link(name = "user32")]
    extern "system" {
        fn EnumDisplayDevicesW(
            device: *const u16,
            dev_num: u32,
            display_device: *mut DisplayDeviceW,
            flags: u32,
        ) -> i32;
    }

    let mut best: Option<(u16, u16, i32)> = None;
    let mut i = 0u32;
    loop {
        let mut dd = DisplayDeviceW {
            cb: core::mem::size_of::<DisplayDeviceW>() as u32,
            device_name: [0; 32],
            device_string: [0; 128],
            state_flags: 0,
            device_id: [0; 128],
            device_key: [0; 128],
        };
        let ok = unsafe { EnumDisplayDevicesW(core::ptr::null(), i, &mut dd, 0) };
        if ok == 0 {
            break;
        }
        i += 1;
        if let Some((ven, dev)) = parse_pci_ven_dev(&wchar_ascii(&dd.device_id)) {
            if ven == 0x1414 {
                continue;
            }
            let rank = pci_rank(ven);
            match best {
                Some((_, _, r)) if r <= rank => {}
                _ => best = Some((ven, dev, rank)),
            }
        }
    }
    best.map(|(v, d, _)| (v, d))
}

fn pci_rank(vendor: u16) -> i32 {
    match vendor {
        0x10DE | 0x1002 => 0,
        0x8086 => 1,
        _ => 2,
    }
}

fn parse_pci_ven_dev(id: &str) -> Option<(u16, u16)> {
    let upper = id.to_ascii_uppercase();
    let ven = find_hex_after(&upper, "VEN_")?;
    let dev = find_hex_after(&upper, "DEV_")?;
    Some((ven, dev))
}

fn find_hex_after(hay: &str, key: &str) -> Option<u16> {
    let i = hay.find(key)?;
    let rest = hay.get(i + key.len()..)?;
    let hex: std::string::String = rest.chars().take(4).collect();
    if hex.len() != 4 {
        return None;
    }
    u16::from_str_radix(&hex, 16).ok()
}

#[cfg(windows)]
fn wchar_ascii(w: &[u16]) -> std::string::String {
    w.iter()
        .take_while(|c| **c != 0)
        .filter(|c| **c < 128)
        .map(|c| *c as u8 as char)
        .collect()
}

#[cfg(all(unix, not(target_os = "macos")))]
fn linux_display_pci() -> Option<(u16, u16)> {
    use std::fs;
    let dir = fs::read_dir("/sys/bus/pci/devices").ok()?;
    let mut best: Option<(u16, u16, i32)> = None;
    for ent in dir.flatten() {
        let path = ent.path();
        let Ok(class_s) = fs::read_to_string(path.join("class")) else {
            continue;
        };
        let Ok(class) = u32::from_str_radix(class_s.trim().trim_start_matches("0x"), 16) else {
            continue;
        };
        let class_base = class >> 16;
        if class_base != 0x03 {
            continue;
        }
        let Ok(ven_s) = fs::read_to_string(path.join("vendor")) else {
            continue;
        };
        let Ok(dev_s) = fs::read_to_string(path.join("device")) else {
            continue;
        };
        let Ok(ven) = u16::from_str_radix(ven_s.trim().trim_start_matches("0x"), 16) else {
            continue;
        };
        let Ok(dev) = u16::from_str_radix(dev_s.trim().trim_start_matches("0x"), 16) else {
            continue;
        };
        if ven == 0x1414 {
            continue;
        }
        let rank = pci_rank(ven);
        match best {
            Some((_, _, r)) if r <= rank => {}
            _ => best = Some((ven, dev, rank)),
        }
    }
    best.map(|(v, d, _)| (v, d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pci_id_string() {
        let id = r"PCI\VEN_10DE&DEV_2531&SUBSYS_00000000&REV_A1";
        assert_eq!(parse_pci_ven_dev(id), Some((0x10DE, 0x2531)));
    }

    #[test]
    fn live_profile_parses() {
        let p = probe_live_profile();
        let bytes = p.to_bytes();
        let parsed = NodeProfile64::parse(&bytes).expect("live profile");
        assert_eq!(parsed.magic, crate::node_profile::NODE_PROFILE_MAGIC);
        assert_eq!(parsed.version, crate::node_profile::NODE_PROFILE_VERSION);
    }
}
