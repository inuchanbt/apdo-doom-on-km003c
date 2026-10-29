// SPDX-License-Identifier: MIT
//! Synthetic fixtures only: no hardware, firmware dump or private baseline.
use anyhow::Result;
use km003c_backup::*;
use std::{fs, path::Path};

struct Memory {
    data: Vec<u8>,
    reads: usize,
    disagree: bool,
    short: bool,
}
impl ReadMemory for Memory {
    async fn read(&mut self, _: u32, _: usize) -> Result<Vec<u8>> {
        self.reads += 1;
        let mut data = self.data.clone();
        if self.disagree && self.reads == 2 {
            data[0] ^= 1;
        }
        if self.short {
            data.pop();
        }
        Ok(data)
    }
}
fn memory(data: Vec<u8>) -> Memory {
    Memory {
        data,
        reads: 0,
        disagree: false,
        short: false,
    }
}
fn field(data: &mut [u8], offset: usize, value: &[u8]) {
    data[offset..offset + value.len()].copy_from_slice(value);
    data[offset + value.len()] = 0;
}
fn application() -> Vec<u8> {
    let mut app = vec![0xff; 0x480];
    app[..4].copy_from_slice(&0x2001fff0u32.to_le_bytes());
    app[4..8].copy_from_slice(&0x4461u32.to_le_bytes());
    app[0x420..0x424].copy_from_slice(&0x4000u32.to_le_bytes());
    field(&mut app, 0x430, b"KM003C");
    field(&mut app, 0x43c, b"2.0.7");
    field(&mut app, 0x448, b"2026.1.1");
    let crc = crc32fast::hash(&app);
    let size = app.len() as u32;
    app[0x428..0x42c].copy_from_slice(&size.to_le_bytes());
    app[0x42c..0x430].copy_from_slice(&crc.to_le_bytes());
    installed(&app).unwrap();
    app
}
async fn fixture(dir: &Path) -> Manifest {
    let app = application();
    let mut regions = Vec::new();
    for (name, address, size) in stock_regions(app.len()) {
        let mut bytes = vec![0x5a; size];
        match name {
            APP => bytes = app.clone(),
            LOADER => {
                field(&mut bytes, 0x41c + 0x10, b"KM003C");
                field(&mut bytes, 0x41c + 0x1c, b"2.3");
            }
            "calibration-03000c00.bin" => field(&mut bytes, 0, b"123456"),
            _ => {}
        }
        regions.push(
            acquire(&mut memory(bytes), dir, name, address, size)
                .await
                .unwrap(),
        );
    }
    let manifest = Manifest {
        schema_version: 1,
        identity: Identity {
            model: "KM003C".into(),
            hardware: "2.3".into(),
            firmware: "2.0.7".into(),
            firmware_date: "2026.1.1".into(),
            serial: "123456".into(),
        },
        usb_port: UsbPort {
            bus_id: "synthetic".into(),
            port_chain: vec![1, 2],
        },
        regions,
    };
    save_json(&dir.join("manifest.json"), &manifest).unwrap();
    manifest
}
fn update_manifest(dir: &Path, manifest: &Manifest) {
    fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec_pretty(manifest).unwrap(),
    )
    .unwrap();
}
fn rehash(dir: &Path, entry: &mut Region, bytes: &[u8]) {
    entry.sha256 = digest(bytes);
    fs::write(dir.join(&entry.file), bytes).unwrap();
    fs::write(
        dir.join(format!("{}.verified.json", entry.file)),
        serde_json::to_vec_pretty(entry).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join(format!("{}.sha256", entry.file)),
        format!("{}  {}\n", entry.sha256, entry.file),
    )
    .unwrap();
}
async fn qualify(dir: &Path, manifest: &Manifest) {
    let dfu = dir.join("dfu-recovery-check");
    fs::create_dir(&dfu).unwrap();
    for (stock, name, address) in [(LOADER, "loader.bin", 4), (APP, "stock-app.bin", 0x4000)] {
        let bytes = fs::read(dir.join(stock)).unwrap();
        let size = bytes.len();
        acquire(&mut memory(bytes), &dfu, name, address, size)
            .await
            .unwrap();
    }
    save_json(
        &dfu.join("verified.json"),
        &Qualification {
            schema_version: 1,
            stock_manifest_sha256: digest(&fs::read(dir.join("manifest.json")).unwrap()),
            usb_port: manifest.usb_port.clone(),
            loader_matches_stock: true,
            application_matches_stock: true,
        },
    )
    .unwrap();
}
#[tokio::test]
async fn stock_and_dfu_evidence_verify_offline() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = fixture(dir.path()).await;
    verify_stock(dir.path()).unwrap();
    assert!(verify_qualified(dir.path()).is_err());
    qualify(dir.path(), &manifest).await;
    verify_qualified(dir.path()).unwrap();
}
#[tokio::test]
async fn mismatching_second_read_never_gets_a_success_record() {
    let dir = tempfile::tempdir().unwrap();
    let mut reader = memory(vec![1; 64]);
    reader.disagree = true;
    assert!(
        acquire(&mut reader, dir.path(), "test.bin", 4, 64)
            .await
            .is_err()
    );
    assert!(dir.path().join("test.bin").exists());
    assert!(!dir.path().join("test.bin.verified.json").exists());
    assert!(!dir.path().join("test.bin.sha256").exists());
}
#[tokio::test]
async fn short_reads_and_overwrites_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut reader = memory(vec![1; 64]);
    reader.short = true;
    assert!(
        acquire(&mut reader, dir.path(), "test.bin", 4, 64)
            .await
            .is_err()
    );
    assert!(!dir.path().join("test.bin").exists());
    let mut reader = memory(vec![1; 64]);
    acquire(&mut reader, dir.path(), "test.bin", 4, 64)
        .await
        .unwrap();
    assert!(
        acquire(&mut reader, dir.path(), "test.bin", 4, 64)
            .await
            .is_err()
    );
    assert_eq!(fs::read(dir.path().join("test.bin")).unwrap(), vec![1; 64]);
}
#[tokio::test]
async fn crc_failure_is_rejected_even_with_updated_sha_records() {
    let dir = tempfile::tempdir().unwrap();
    let mut manifest = fixture(dir.path()).await;
    let mut app = fs::read(dir.path().join(APP)).unwrap();
    app[0x470] ^= 1;
    rehash(dir.path(), &mut manifest.regions[0], &app);
    update_manifest(dir.path(), &manifest);
    assert!(
        verify_stock(dir.path())
            .unwrap_err()
            .to_string()
            .contains("CRC")
    );
}
#[tokio::test]
async fn wrong_region_paths_ranges_and_missing_sidecars_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = fixture(dir.path()).await;
    for index in 0..4 {
        let mut changed = manifest.clone();
        match index {
            0 => changed.regions[1].file = "../outside.bin".into(),
            1 => changed.regions[1].address = 0,
            2 => changed.regions[1].matching_reads = 1,
            _ => changed.regions.pop().map(|_| ()).unwrap(),
        }
        update_manifest(dir.path(), &changed);
        assert!(verify_stock(dir.path()).is_err());
    }
    update_manifest(dir.path(), &manifest);
    fs::remove_file(dir.path().join(format!("{PREFIX}.sha256"))).unwrap();
    assert!(verify_stock(dir.path()).is_err());
}
#[tokio::test]
async fn unsupported_hardware_and_wrong_calibration_serial_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = fixture(dir.path()).await;
    for (hardware, serial) in [("2.1", "123456"), ("2.3", "654321")] {
        let mut changed = manifest.clone();
        changed.identity.hardware = hardware.into();
        changed.identity.serial = serial.into();
        update_manifest(dir.path(), &changed);
        assert!(verify_stock(dir.path()).is_err());
    }
}
#[tokio::test]
async fn dfu_must_match_both_original_regions_and_manifest() {
    for stock_name in [LOADER, APP] {
        let dir = tempfile::tempdir().unwrap();
        let manifest = fixture(dir.path()).await;
        qualify(dir.path(), &manifest).await;
        let dfu = dir.path().join("dfu-recovery-check");
        let name = if stock_name == LOADER {
            "loader.bin"
        } else {
            APP
        };
        let mut entry: Region =
            serde_json::from_slice(&fs::read(dfu.join(format!("{name}.verified.json"))).unwrap())
                .unwrap();
        let mut bytes = fs::read(dfu.join(name)).unwrap();
        bytes[0x470] ^= 1;
        rehash(&dfu, &mut entry, &bytes);
        assert!(verify_qualified(dir.path()).is_err());
    }
    let dir = tempfile::tempdir().unwrap();
    let manifest = fixture(dir.path()).await;
    qualify(dir.path(), &manifest).await;
    let mut bytes = fs::read(dir.path().join("manifest.json")).unwrap();
    bytes.push(b' '); // Still valid JSON; qualification must bind the exact manifest.
    fs::write(dir.path().join("manifest.json"), bytes).unwrap();
    assert!(verify_qualified(dir.path()).is_err());
}
#[test]
fn malformed_application_metadata_is_rejected() {
    assert!(application_size(&[]).is_err());
    assert!(application_size(&[0; 64]).is_err());
    let app = application();
    assert_eq!(application_size(&app[0x420..0x460]).unwrap(), app.len());
    let mut corrupt = app.clone();
    corrupt[..4].fill(0);
    assert!(installed(&corrupt).is_err());
}
#[test]
fn executable_exposes_only_read_only_commands() {
    let binary = env!("CARGO_BIN_EXE_km003c-backup");
    let help = std::process::Command::new(binary)
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    for name in ["inspect", "stock", "dfu-check", "verify"] {
        assert!(help.contains(name));
    }
    for name in ["flash", "erase", "reboot", "write", "start"] {
        assert!(
            !std::process::Command::new(binary)
                .arg(name)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
}
