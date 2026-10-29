//! Read-only recovery acquisition using the pinned native protocol library.
use anyhow::{Result, ensure};
use km003c_lib::{DeviceConfig, DeviceSelector, KM003C};
use std::{io::Write, path::Path};

pub async fn dfu_probe(selector: DeviceSelector, output: &Path) -> Result<()> {
    use km003c_lib::firmware::{FirmwareUsbMode, find_firmware_device};
    let found = find_firmware_device(&selector).await?;
    ensure!(found.mode == FirmwareUsbMode::Dfu, "native DFU required");
    let stable = found.port_selector.unwrap_or(selector);
    let mut device = KM003C::open_maintenance(stable).await?;
    let info = device.get_bootloader_info().await?;
    ensure!(
        info.model == "KM003C" && info.hw_version == "2.3",
        "unexpected identity"
    );
    std::fs::create_dir(output)?;
    save(
        &output.join("device.txt"),
        format!("{info:#?}\n").as_bytes(),
    )?;
    for (name, address, size) in [
        ("loader.bin", 4, 0x3ffc),
        ("application-metadata.bin", 0x4420, 64),
    ] {
        let first = crate::maintenance::read(&mut device, address, size).await?;
        save(&output.join(name), &first)?;
        ensure!(
            first == crate::maintenance::read(&mut device, address, size).await?,
            "DFU reads differ"
        );
        println!(
            "DFU verified {name}: {}",
            crate::maintenance::digest(&first)
        );
    }
    match external_dfu(&mut device, 0x3ff000, 4096).await {
        Ok(first) => {
            save(&output.join("external-factory-sector.bin"), &first)?;
            ensure!(
                first == external_dfu(&mut device, 0x3ff000, 4096).await?,
                "factory reads differ"
            );
            println!("External factory sector verified");
        }
        Err(error) => {
            save(
                &output.join("external-read-unavailable.txt"),
                format!("{error:#}\n").as_bytes(),
            )?;
            println!("External DFU read unavailable: {error}");
        }
    }
    device.jump_to_application().await?;
    println!("Returned to original application without flash writes");
    Ok(())
}

// The documented native storage-read command reuses MemoryRead's authenticated
// descriptor. No settings, write-enable, erase, or programming command is sent.
async fn external_dfu(device: &mut KM003C, offset: u32, size: u32) -> Result<Vec<u8>> {
    use km003c_lib::auth;
    use std::time::Duration;
    let mut request = auth::build_memory_read_packet(offset, size, 1);
    request[0] = 0x4b;
    device.send_raw(&request).await?;
    let confirmation = tokio::time::timeout(Duration::from_secs(3), device.receive_raw()).await??;
    ensure!(
        confirmation.len() == 20,
        "storage read rejected: {:02x?}",
        confirmation
    );
    let (at, n) = auth::parse_memory_read_confirmation(&confirmation[4..])
        .ok_or_else(|| anyhow::anyhow!("invalid storage confirmation"))?;
    ensure!(
        (at == offset || at == 0x98000000 + offset) && n == size,
        "unexpected storage confirmation"
    );
    let mut bytes = Vec::new();
    while bytes.len() < (size as usize).div_ceil(16) * 16 {
        bytes.extend(tokio::time::timeout(Duration::from_secs(3), device.receive_raw()).await??);
    }
    let plain = auth::decrypt_memory_read_response(&bytes)
        .ok_or_else(|| anyhow::anyhow!("invalid storage payload"))?;
    Ok(plain[..size as usize].to_vec())
}

pub async fn read_region(
    selector: DeviceSelector,
    address: u32,
    size: usize,
    output: &Path,
) -> Result<()> {
    ensure!(
        size > 0 && size <= 0x400000 && address.checked_add(size as u32).is_some(),
        "invalid range"
    );
    let mut device = open(selector).await?;
    let first = region(&mut device, address, size).await?;
    save(output, &first)?;
    ensure!(
        first == region(&mut device, address, size).await?,
        "reads differ"
    );
    save(&output.with_extension("verified.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "address":address,"bytes":size,"sha256":crate::maintenance::digest(&first),"matching_reads":2
    }))?.as_slice())?;
    Ok(())
}

fn save(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

async fn open(selector: DeviceSelector) -> Result<KM003C> {
    Ok(KM003C::new(DeviceConfig::vendor().skip_reset().select(selector)).await?)
}
pub async fn enter_loader(selector: DeviceSelector) -> Result<()> {
    let mut device = open(selector.clone()).await?;
    let info = device.get_device_info().await?;
    let expected_serial = option_env!("APDO_DEVICE_SERIAL")
        .filter(|serial| !serial.is_empty() && *serial != "YOUR_DEVICE_SERIAL")
        .ok_or_else(|| {
            anyhow::anyhow!("build with the verified device serial in APDO_DEVICE_SERIAL")
        })?;
    ensure!(
        info.model == "KM003C" && info.hw_version == "2.3" && info.serial_id == expected_serial,
        "unexpected stock unit"
    );
    let _ = device.jump_to_dfu().await;
    drop(device);
    for _ in 0..100 {
        if let Ok(found) = km003c_lib::firmware::find_firmware_device(&selector).await
            && found.mode == km003c_lib::firmware::FirmwareUsbMode::Dfu
        {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    anyhow::bail!("native DFU did not enumerate")
}
pub async fn start(selector: DeviceSelector) -> Result<()> {
    let (mut device, stable) = crate::maintenance::open(selector).await?;
    let _ = device.jump_to_application().await;
    drop(device);
    for _ in 0..100 {
        if let Ok(found) = km003c_lib::firmware::find_firmware_device(&stable).await
            && found.mode == km003c_lib::firmware::FirmwareUsbMode::Application
        {
            let mut device = open(stable).await?;
            let info = device.get_device_info().await?;
            ensure!(
                info.hw_version == "2.3" && info.fw_version == "2.0.7",
                "stock firmware identity mismatch"
            );
            println!("Original HW2.3 firmware 2.0.7 is running");
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    anyhow::bail!("stock application did not enumerate")
}

pub async fn inspect(selector: DeviceSelector) -> Result<()> {
    let mut device = open(selector).await?;
    println!("{:#?}", device.get_device_info().await?);
    Ok(())
}

async fn region(device: &mut KM003C, address: u32, size: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(size);
    while bytes.len() < size {
        let count = (size - bytes.len()).min(4096);
        let block = device
            .read_memory_block(address + bytes.len() as u32, count as u32)
            .await?;
        ensure!(block.len() == count, "short read");
        bytes.extend_from_slice(&block);
        if bytes.len() % 0x40000 == 0 || bytes.len() == size {
            println!("Read {address:#010x}: {}/{} bytes", bytes.len(), size);
        }
    }
    Ok(bytes)
}

pub async fn backup(selector: DeviceSelector, output: &Path) -> Result<()> {
    std::fs::create_dir(output)?;
    let mut device = open(selector).await?;
    let info = device.get_device_info().await?;
    ensure!(
        info.model == "KM003C" && !info.hw_version.is_empty(),
        "unidentified hardware"
    );
    save(
        &output.join("device.txt"),
        format!("{info:#?}\n").as_bytes(),
    )?;
    let metadata = device.read_memory_block(0x4420, 64).await?;
    let size = u32::from_le_bytes(metadata[8..12].try_into()?) as usize;
    ensure!(
        (0x460..=0x6fffe).contains(&size),
        "invalid application length"
    );
    let mut entries = Vec::new();
    for (name, address, size) in [
        ("stock-app.bin", 0x4000, size),
        ("loader-00000004-00003fff.bin", 4, 0x3ffc),
        ("internal-tail-00070000.bin", 0x70000, 0x10000),
        ("calibration-03000c00.bin", 0x03000c00, 0x400),
        ("external-98000000.bin", 0x98000000, 0x400000),
    ] {
        let first = region(&mut device, address, size).await?;
        save(&output.join(name), &first)?;
        let second = region(&mut device, address, size).await?;
        ensure!(first == second, "two reads disagree: {name}");
        let hash = crate::maintenance::digest(&first);
        entries.push(serde_json::json!({"file": name, "address": address, "bytes": size, "sha256": hash, "matching_reads": 2}));
        save(
            &output.join(format!("{name}.sha256")),
            format!("{hash}  {name}\n").as_bytes(),
        )?;
        if name == "stock-app.bin" {
            let mut uncommitted = first.clone();
            uncommitted[0x428..0x430].fill(0xff);
            let encoded = crate::package::package(&uncommitted)?;
            let image = crate::maintenance::parse_image(&encoded)?;
            ensure!(
                image.installed_data()? == first,
                "restore package changes installed application"
            );
            save(&output.join("stock-restore.mencrypt"), &encoded)?;
        }
        save(
            &output.join(format!("{name}.verified.json")),
            serde_json::to_vec_pretty(entries.last().unwrap())?.as_slice(),
        )?;
    }
    save(
        &output.join("manifest.json"),
        serde_json::to_vec_pretty(&entries)?.as_slice(),
    )?;
    println!("All recovery regions read twice and verified; no device writes performed.");
    Ok(())
}
