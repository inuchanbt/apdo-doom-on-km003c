// SPDX-License-Identifier: MIT
//! Fixed recovery regions and offline verification. No private include_bytes!.
use anyhow::{Context, Result, ensure};
use km003c_lib::{auth::DeviceInfo, firmware::InstalledFirmwareInfo};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path};

pub const LOADER: &str = "loader-00000004-00003fff.bin";
pub const APP: &str = "stock-app.bin";
pub const PREFIX: &str = "external-first-3MiB.bin";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub model: String,
    pub hardware: String,
    pub firmware: String,
    pub firmware_date: String,
    pub serial: String,
}
impl Identity {
    pub fn supported(&self) -> Result<()> {
        ensure!(
            self.model == "KM003C" && self.hardware == "2.3" && self.firmware == "2.0.7",
            "initial acquisition supports only KM003C HW2.3 stock firmware 2.0.7; stop on other versions"
        );
        ensure!(
            !self.serial.is_empty()
                && self.serial.is_ascii()
                && self.serial != "YOUR_DEVICE_SERIAL",
            "missing device serial"
        );
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsbPort {
    pub bus_id: String,
    pub port_chain: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    pub file: String,
    pub address: u32,
    pub bytes: usize,
    pub sha256: String,
    pub matching_reads: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u8,
    pub identity: Identity,
    pub usb_port: UsbPort,
    pub regions: Vec<Region>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Qualification {
    pub schema_version: u8,
    pub stock_manifest_sha256: String,
    pub usb_port: UsbPort,
    pub loader_matches_stock: bool,
    pub application_matches_stock: bool,
}

pub fn digest(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub fn save(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("create {} (outputs must be new)", path.display()))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
pub fn save_json(path: &Path, value: &impl Serialize) -> Result<()> {
    save(path, &serde_json::to_vec_pretty(value)?)
}

pub fn installed(data: &[u8]) -> Result<InstalledFirmwareInfo> {
    ensure!(data.len() >= 0x460, "truncated application");
    let mut identity = DeviceInfo::default();
    identity.parse_firmware_info(&data[0x420..0x460]);
    let info = InstalledFirmwareInfo {
        version: identity.fw_version,
        date: identity.fw_date,
        size: u32::from_le_bytes(data[0x428..0x42c].try_into()?) as usize,
        crc32: u32::from_le_bytes(data[0x42c..0x430].try_into()?),
    };
    info.validate_application(data)?;
    Ok(info)
}
pub fn application_size(metadata: &[u8]) -> Result<usize> {
    ensure!(
        metadata.len() == 64 && metadata[..4] == 0x4000u32.to_le_bytes(),
        "invalid application metadata"
    );
    let size = u32::from_le_bytes(metadata[8..12].try_into()?) as usize;
    ensure!(
        (0x460..=0x6fffe).contains(&size),
        "invalid application size"
    );
    Ok(size)
}
pub fn stock_regions(app_size: usize) -> [(&'static str, u32, usize); 5] {
    [
        (APP, 0x4000, app_size),
        (LOADER, 4, 0x3ffc),
        ("internal-tail-00070000.bin", 0x70000, 0x10000),
        ("calibration-03000c00.bin", 0x03000c00, 0x400),
        (PREFIX, 0x98000000, 0x300000),
    ]
}

// A test transport can exercise acquisition and mismatching second reads.
#[allow(async_fn_in_trait)]
pub trait ReadMemory {
    async fn read(&mut self, address: u32, size: usize) -> Result<Vec<u8>>;
}
pub async fn acquire(
    reader: &mut impl ReadMemory,
    dir: &Path,
    name: &str,
    address: u32,
    size: usize,
) -> Result<Region> {
    let first = reader.read(address, size).await?;
    ensure!(first.len() == size, "short first read: {name}");
    save(&dir.join(name), &first)?;
    let second = reader.read(address, size).await?;
    ensure!(
        first == second,
        "two reads disagree: {name}; incomplete backup must not be used"
    );
    let entry = Region {
        file: name.into(),
        address,
        bytes: size,
        sha256: digest(&first),
        matching_reads: 2,
    };
    save(
        &dir.join(format!("{name}.sha256")),
        format!("{}  {name}\n", entry.sha256).as_bytes(),
    )?;
    save_json(&dir.join(format!("{name}.verified.json")), &entry)?;
    Ok(entry)
}
fn verify_region(
    dir: &Path,
    expected: &Region,
    name: &str,
    address: u32,
    size: usize,
) -> Result<Vec<u8>> {
    // Never follow a manifest-supplied pathname or accept a different range.
    ensure!(
        expected.file == name
            && expected.address == address
            && expected.bytes == size
            && expected.matching_reads == 2,
        "invalid region: {name}"
    );
    let data = fs::read(dir.join(name))?;
    ensure!(
        data.len() == size && digest(&data) == expected.sha256,
        "size/hash mismatch: {name}"
    );
    let sidecar: Region =
        serde_json::from_slice(&fs::read(dir.join(format!("{name}.verified.json")))?)?;
    ensure!(
        sidecar == *expected,
        "verification sidecar mismatch: {name}"
    );
    ensure!(
        fs::read_to_string(dir.join(format!("{name}.sha256")))?.trim()
            == format!("{}  {name}", expected.sha256),
        "SHA sidecar mismatch: {name}"
    );
    Ok(data)
}
pub fn verify_stock(dir: &Path) -> Result<Manifest> {
    let manifest: Manifest = serde_json::from_slice(&fs::read(dir.join("manifest.json"))?)?;
    ensure!(
        manifest.schema_version == 1
            && manifest.regions.len() == 5
            && !manifest.usb_port.port_chain.is_empty(),
        "invalid/incomplete stock manifest"
    );
    manifest.identity.supported()?;
    let app = fs::read(dir.join(APP))?;
    let info = installed(&app)?;
    ensure!(
        info.version == manifest.identity.firmware && info.date == manifest.identity.firmware_date,
        "application identity mismatch"
    );
    for (entry, (name, address, size)) in manifest.regions.iter().zip(stock_regions(app.len())) {
        let data = verify_region(dir, entry, name, address, size)?;
        let mut identity = DeviceInfo::default();
        if name == LOADER {
            identity.parse_device_info(&data[0x41c..0x45c]);
            ensure!(
                identity.model == manifest.identity.model
                    && identity.hw_version == manifest.identity.hardware,
                "loader hardware identity mismatch"
            );
        } else if name == "calibration-03000c00.bin" {
            identity.parse_calibration(&data);
            ensure!(
                identity.serial_id == manifest.identity.serial,
                "calibration serial mismatch"
            );
        }
    }
    Ok(manifest)
}
pub fn verify_qualified(dir: &Path) -> Result<Manifest> {
    let manifest = verify_stock(dir)?;
    let dfu = dir.join("dfu-recovery-check");
    let qualification: Qualification =
        serde_json::from_slice(&fs::read(dfu.join("verified.json"))?)?;
    ensure!(
        qualification.schema_version == 1
            && qualification.usb_port == manifest.usb_port
            && qualification.stock_manifest_sha256 == digest(&fs::read(dir.join("manifest.json"))?)
            && qualification.loader_matches_stock
            && qualification.application_matches_stock,
        "DFU qualification does not match stock backup"
    );
    for (stock_name, dfu_name, address) in
        [(LOADER, "loader.bin", 4), (APP, "stock-app.bin", 0x4000)]
    {
        let original = fs::read(dir.join(stock_name))?;
        let entry: Region =
            serde_json::from_slice(&fs::read(dfu.join(format!("{dfu_name}.verified.json")))?)?;
        ensure!(
            verify_region(&dfu, &entry, dfu_name, address, original.len())? == original,
            "DFU differs from stock: {stock_name}"
        );
    }
    Ok(manifest)
}

// Shared by the independent updater; authentication stays in km003c-lib.
use km003c_lib::{KM003C, auth};
use std::time::Duration;
pub struct Reader {
    pub device: KM003C,
    pub dfu: bool,
}
impl ReadMemory for Reader {
    async fn read(&mut self, address: u32, size: usize) -> Result<Vec<u8>> {
        let mut output = Vec::with_capacity(size);
        while output.len() < size {
            let count = (size - output.len()).min(4096);
            let at = address
                .checked_add(output.len().try_into()?)
                .context("read range overflow")?;
            let data = if self.dfu {
                // Pinned library builds/authenticates requests and decrypts replies.
                self.device
                    .send_raw(&auth::build_memory_read_packet(at, count as u32, 1))
                    .await?;
                let reply = tokio::time::timeout(Duration::from_secs(5), self.device.receive_raw())
                    .await??;
                ensure!(
                    reply.len() == 20
                        && auth::parse_memory_read_confirmation(&reply[4..])
                            == Some((at, count as u32)),
                    "invalid DFU confirmation at {at:#x}"
                );
                let aligned = count.div_ceil(16) * 16;
                let mut encrypted = Vec::new();
                while encrypted.len() < aligned {
                    let transfer =
                        tokio::time::timeout(Duration::from_secs(5), self.device.receive_raw())
                            .await??;
                    ensure!(
                        !transfer.is_empty() && transfer.len() <= aligned - encrypted.len(),
                        "invalid DFU transfer length"
                    );
                    encrypted.extend(transfer);
                }
                ensure!(encrypted.len() == aligned, "unexpected DFU transfer size");
                let plain = auth::decrypt_memory_read_response(&encrypted)
                    .context("invalid DFU payload")?;
                ensure!(plain.len() >= count, "short DFU payload");
                plain[..count].to_vec()
            } else {
                self.device.read_memory_block(at, count as u32).await?
            };
            ensure!(data.len() == count, "short memory read at {at:#x}");
            output.extend(data);
            if output.len() % 0x40000 == 0 || output.len() == size {
                println!("Read {address:#010x}: {}/{}", output.len(), size);
            }
        }
        Ok(output)
    }
}
