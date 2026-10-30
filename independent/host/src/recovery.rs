// SPDX-License-Identifier: MIT
//! Offline packaging and runtime-qualified native loader access.
//! No private loader/serial is compiled into this executable.
use aes::{
    Aes128,
    cipher::{BlockCipherEncrypt, KeyInit},
};
use anyhow::{Context, Result, ensure};
use km003c_backup::{self as backup, ReadMemory};
use km003c_lib::{
    DeviceSelector, KM003C,
    auth::MEMORY_READ_KEY,
    firmware::{FirmwareImage, FirmwareUsbMode, find_firmware_device},
};
use std::{fs, path::Path};

pub fn package(input: &Path, output: &Path, installed: bool) -> Result<()> {
    let mut bytes = fs::read(input)?;
    ensure!(
        (0x460..=0x6fffe).contains(&bytes.len()),
        "Invalid application size"
    );
    if installed {
        backup::installed(&bytes)?;
        bytes[0x428..0x430].fill(255);
    }
    ensure!(
        bytes[0x420..0x424] == 0x4000u32.to_le_bytes(),
        "Application base must be 0x4000"
    );
    ensure!(
        bytes[0x428..0x430] == [255; 8],
        "Expected uncommitted size/CRC fields"
    );
    let stack = u32::from_le_bytes(bytes[..4].try_into()?);
    let entry = u32::from_le_bytes(bytes[4..8].try_into()?);
    ensure!(
        stack % 8 == 0 && (0x20000000..=0x2001fff0).contains(&stack),
        "Invalid initial stack"
    );
    ensure!(
        entry & 1 == 1 && (0x4460..0x4000 + bytes.len() as u32).contains(&(entry & !1)),
        "Invalid reset entry"
    );
    let mut fields = vec![
        "Firmware file".to_owned(),
        "ChargerLab".into(),
        "V1.0".into(),
    ];
    for offset in [0x430, 0x43c, 0x448] {
        let field = bytes[offset..offset + 12]
            .split(|b| *b == 0)
            .next()
            .unwrap();
        ensure!(
            !field.is_empty() && field.len() < 12 && field.is_ascii(),
            "Invalid metadata string"
        );
        fields.push(std::str::from_utf8(field)?.into());
    }
    ensure!(
        fields[3] == "KM003C" && fields[4].as_bytes()[0].is_ascii_digit(),
        "Unexpected application identity"
    );
    fields.extend([
        format!("{:08X}", crc32fast::hash(&bytes)),
        bytes.len().to_string(),
        String::new(),
        "APDO application".into(),
    ]);
    let mut container = Vec::new();
    container.extend_from_slice(&10u32.to_be_bytes());
    for field in fields {
        let encoded: Vec<_> = field.encode_utf16().flat_map(u16::to_be_bytes).collect();
        container.extend_from_slice(&(encoded.len() as u32).to_be_bytes());
        container.extend(encoded);
    }
    let mut encrypted = bytes.clone();
    encrypted.resize(bytes.len().div_ceil(16) * 16, 255);
    let cipher = Aes128::new_from_slice(&*MEMORY_READ_KEY)?;
    for chunk in encrypted.as_chunks_mut::<16>().0 {
        cipher.encrypt_block(chunk.into());
    }
    container.extend_from_slice(&(encrypted.len() as u32).to_be_bytes());
    container.extend(encrypted);
    let image = FirmwareImage::parse(&container)?;
    image.validate_for_update()?;
    if installed {
        ensure!(
            image.installed_data()? == fs::read(input)?,
            "Restore package changed application bytes"
        );
    }
    backup::save(output, &container)?;
    println!("Created {} (no USB access)", output.display());
    Ok(())
}
fn selector(m: &backup::Manifest) -> DeviceSelector {
    DeviceSelector::PortPath {
        bus_id: m.usb_port.bus_id.clone(),
        port_chain: m.usb_port.port_chain.clone(),
    }
}
async fn open(stock: &Path) -> Result<(backup::Reader, Vec<u8>)> {
    let manifest = backup::verify_qualified(stock)?;
    let selector = selector(&manifest);
    let found = find_firmware_device(&selector).await?;
    ensure!(
        found.mode == FirmwareUsbMode::Dfu,
        "Manually enter native DFU on the port recorded in the backup"
    );
    let mut device = KM003C::open_maintenance(selector).await?;
    let info = device.get_bootloader_info().await?;
    ensure!(
        info.model == "KM003C" && info.hw_version == "2.3",
        "Unsupported device hardware"
    );
    let baseline = fs::read(stock.join(backup::LOADER))?;
    let mut reader = backup::Reader { device, dfu: true };
    ensure!(
        reader.read(4, 0x3ffc).await? == baseline,
        "Loader differs from this device's qualified backup"
    );
    Ok((reader, baseline))
}
pub async fn flash(stock: &Path, package: &Path, evidence: &Path) -> Result<()> {
    let encoded = fs::read(package)?;
    let image = FirmwareImage::parse(&encoded)?;
    image.validate_for_update()?;
    let (mut reader, baseline) = open(stock).await?;
    fs::create_dir(evidence).context("Evidence directory must be new and its parent must exist")?;
    backup::save(&evidence.join("loader-before.bin"), &baseline)?;
    backup::save_json(
        &evidence.join("before.json"),
        &serde_json::json!({
        "image_sha256":backup::digest(&encoded),"application_bytes":image.data().len(),
        "loader_sha256":backup::digest(&baseline)}),
    )?;
    // Programming errors are never retried: preserve DFU for diagnosis/recovery.
    reader
        .device
        .update_firmware(&image, |p| {
            eprintln!("{:?}: {}/{}", p.stage, p.bytes_written, p.total_bytes)
        })
        .await?;
    let app = reader.read(0x4000, image.data().len()).await?;
    backup::save(&evidence.join("application-readback.bin"), &app)?;
    let after = reader.read(4, 0x3ffc).await?;
    backup::save(&evidence.join("loader-after.bin"), &after)?;
    ensure!(after == baseline, "Loader changed");
    ensure!(
        app == image.installed_data()?,
        "Full application readback differs"
    );
    backup::save_json(
        &evidence.join("verified.json"),
        &serde_json::json!({
        "loader_unchanged":true,"readback_verified":true,"application_sha256":backup::digest(&app)}),
    )?;
    println!("Full readback verified; device remains in native DFU");
    Ok(())
}
pub async fn start(stock: &Path, package: &Path) -> Result<()> {
    let image = FirmwareImage::parse(&fs::read(package)?)?;
    image.validate_for_update()?;
    let (mut reader, _) = open(stock).await?;
    ensure!(
        reader.read(0x4000, image.data().len()).await? == image.installed_data()?,
        "Installed image differs; staying in DFU"
    );
    // A re-enumeration may terminate the final write; readiness is checked by the caller.
    let selected = backup::verify_qualified(stock)?.usb_port;
    let result = reader.device.jump_to_application().await;
    drop(reader);
    let expected_product = if image.metadata().version.contains("-apdo") {
        "APDO DOOM"
    } else {
        "POWER-Z KM003C"
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while std::time::Instant::now() < deadline {
        use nusb::MaybeFuture;
        let found = nusb::list_devices().wait()?.any(|d| {
            d.bus_id() == selected.bus_id
                && d.port_chain() == selected.port_chain
                && d.product_string() == Some(expected_product)
        });
        if found && expected_product == "POWER-Z KM003C" {
            let selected = DeviceSelector::PortPath {
                bus_id: selected.bus_id.clone(),
                port_chain: selected.port_chain.clone(),
            };
            let mut app = KM003C::new(
                km003c_lib::DeviceConfig::vendor()
                    .skip_reset()
                    .select(selected),
            )
            .await?;
            let identity = app.get_device_info().await?;
            ensure!(
                identity.model == "KM003C"
                    && identity.hw_version == "2.3"
                    && identity.fw_version == image.metadata().version,
                "Started stock application identity differs"
            );
            println!("Original application connected and identity verified");
            return Ok(());
        }
        if found && let Ok(mut client) = crate::Client::open() {
            let status = client.info()?;
            println!(
                "Independent application connected: running={}, frames={}",
                status[1] != 0,
                status[0]
            );
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    anyhow::bail!(
        "Independent application did not answer on the selected port; start write result: {result:?}"
    )
}

pub async fn enter_loader(mut client: crate::Client) -> Result<()> {
    use nusb::MaybeFuture;
    let mut devices = nusb::list_devices().wait()?.filter(|d| {
        d.vendor_id() == 0x5fc9 && d.product_id() == 0x63 && d.product_string() == Some("APDO DOOM")
    });
    let info = devices
        .next()
        .context("Independent application is not connected")?;
    ensure!(devices.next().is_none(), "Connect only one APDO device");
    let selected = DeviceSelector::PortPath {
        bus_id: info.bus_id().to_owned(),
        port_chain: info.port_chain().to_vec(),
    };
    let result = client
        .call(apdo_protocol::INPUT, 0, 0)
        .and_then(|_| client.call(apdo_protocol::LOADER, 0, 0));
    drop(client);
    for _ in 0..25 {
        if let Ok(found) = find_firmware_device(&selected).await
            && found.mode == FirmwareUsbMode::Dfu
        {
            println!("Native DFU confirmed on the same physical port");
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    anyhow::bail!("Native DFU was not confirmed after loader request: {result:?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packaging_checks_vectors_and_reproduces_committed_bytes() {
        let root = std::env::temp_dir().join(format!("apdo-package-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let mut app = vec![0; 0x500];
        app[..4].copy_from_slice(&0x2001fff0u32.to_le_bytes());
        app[4..8].copy_from_slice(&0x4461u32.to_le_bytes());
        app[0x420..0x424].copy_from_slice(&0x4000u32.to_le_bytes());
        app[0x428..0x430].fill(255);
        for (offset, data) in [
            (0x430, &b"KM003C"[..]),
            (0x43c, &b"2.0.7-apdo1"[..]),
            (0x448, &b"2026.10.31"[..]),
        ] {
            app[offset..offset + data.len()].copy_from_slice(data);
        }
        let raw = root.join("raw.bin");
        let pkg = root.join("image.mencrypt");
        let restore = root.join("restore.mencrypt");
        fs::write(&raw, &app).unwrap();
        package(&raw, &pkg, false).unwrap();
        let installed = FirmwareImage::parse(&fs::read(&pkg).unwrap())
            .unwrap()
            .installed_data()
            .unwrap();
        fs::write(&raw, &installed).unwrap();
        package(&raw, &restore, true).unwrap();
        assert_eq!(
            FirmwareImage::parse(&fs::read(restore).unwrap())
                .unwrap()
                .installed_data()
                .unwrap(),
            installed
        );
        app[4] = 0;
        fs::write(&raw, app).unwrap();
        assert!(package(&raw, &root.join("bad"), false).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}

pub async fn diagnostics(stock: &Path, out: &Path) -> Result<()> {
    let (mut reader, _) = open(stock).await?;
    let bytes = reader.read(0x200f0000, 4096).await?;
    fs::create_dir(out).context("Diagnostic directory must be new")?;
    backup::save(&out.join("retention.bin"), &bytes)?;
    let text = String::from_utf8_lossy(bytes[0xe00..0xf00].split(|b| *b == 0).next().unwrap());
    backup::save_json(
        &out.join("status.json"),
        &serde_json::json!({
        "stage":u32::from_le_bytes(bytes[0xfc4..0xfc8].try_into()?),
        "reason":u32::from_le_bytes(bytes[0xfcc..0xfd0].try_into()?),"retained_text":text}),
    )?;
    println!("Saved read-only startup diagnostics");
    Ok(())
}
