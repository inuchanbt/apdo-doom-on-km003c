// SPDX-License-Identifier: MIT
//! Initial acquisition only: no erase/program/update/reboot/settings/PD command.
use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use km003c_backup::*;
use km003c_lib::{
    DeviceConfig, DeviceSelector, KM003C, auth,
    firmware::{FirmwareUsbMode, find_firmware_device},
};
use std::{fs, path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(about = "Read-only first backup and offline verification; no private baseline required")]
struct Args {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Read identity; select explicitly if more than one device is attached.
    Inspect {
        #[arg(long)]
        serial: Option<String>,
    },
    /// Acquire stock APP recovery regions twice. Only HW2.3 / stock 2.0.7.
    Stock {
        #[arg(long)]
        serial: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// Read loader and APP twice in manually entered native DFU on the recorded port.
    DfuCheck {
        #[arg(long)]
        stock: PathBuf,
    },
    /// Verify local files without opening USB.
    Verify {
        #[arg(long)]
        stock: PathBuf,
        #[arg(long)]
        require_dfu: bool,
    },
}

struct Reader {
    device: KM003C,
    dfu: bool,
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
fn port(selector: DeviceSelector) -> Result<UsbPort> {
    if let DeviceSelector::PortPath { bus_id, port_chain } = selector {
        ensure!(!port_chain.is_empty(), "physical USB port unavailable");
        Ok(UsbPort { bus_id, port_chain })
    } else {
        anyhow::bail!("physical USB port identity is required")
    }
}
fn selector(port: &UsbPort) -> DeviceSelector {
    DeviceSelector::PortPath {
        bus_id: port.bus_id.clone(),
        port_chain: port.port_chain.clone(),
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    match Args::parse().command {
        Command::Inspect { serial } => {
            let selected = serial.map_or(DeviceSelector::First, DeviceSelector::SerialNumber);
            let found = find_firmware_device(&selected).await?;
            println!("USB: {:?}; port {:?}", found.mode, found.port_selector);
            if found.mode == FirmwareUsbMode::Dfu {
                let mut device = KM003C::open_maintenance(selected).await?;
                println!("{:?}", device.get_bootloader_info().await?);
            } else {
                ensure!(
                    found.product_string.as_deref() == Some("POWER-Z KM003C"),
                    "stock firmware required; custom RPC is not a stock backup source"
                );
                let mut device =
                    KM003C::new(DeviceConfig::vendor().skip_reset().select(selected)).await?;
                println!("{:?}", device.get_device_info().await?);
            }
        }
        Command::Stock { serial, output } => {
            let found = find_firmware_device(&DeviceSelector::SerialNumber(serial.clone())).await?;
            ensure!(
                found.mode == FirmwareUsbMode::Application
                    && found.product_string.as_deref() == Some("POWER-Z KM003C"),
                "run under untouched stock application"
            );
            let usb_port = port(found.port_selector.context("physical port not available")?)?;
            let mut device = KM003C::new(
                DeviceConfig::vendor()
                    .skip_reset()
                    .select(selector(&usb_port)),
            )
            .await?;
            let info = device.get_device_info().await?;
            let identity = Identity {
                model: info.model,
                hardware: info.hw_version,
                firmware: info.fw_version,
                firmware_date: info.fw_date,
                serial: info.serial_id,
            };
            identity.supported()?;
            ensure!(
                identity.serial == serial,
                "selected USB serial differs from calibration serial"
            );
            fs::create_dir(&output).context("output directory must be new")?;
            save_json(&output.join("identity.json"), &identity)?;
            let mut reader = Reader { device, dfu: false };
            let size = application_size(&reader.read(0x4420, 64).await?)?;
            let mut regions = Vec::new();
            for (name, address, size) in stock_regions(size) {
                regions.push(acquire(&mut reader, &output, name, address, size).await?);
            }
            save_json(
                &output.join("manifest.json"),
                &Manifest {
                    schema_version: 1,
                    identity,
                    usb_port,
                    regions,
                },
            )?;
            verify_stock(&output)?;
            println!(
                "Stock backup verified. External NOR is the FIRST 3 MiB only. Next: manually enter DFU and run dfu-check."
            );
        }
        Command::DfuCheck { stock } => {
            let manifest = verify_stock(&stock)?;
            let selected = selector(&manifest.usb_port);
            let found = find_firmware_device(&selected).await?;
            ensure!(
                found.mode == FirmwareUsbMode::Dfu,
                "hold Exit while reconnecting HID to enter native DFU manually, using the same USB port"
            );
            let mut device = KM003C::open_maintenance(selected).await?;
            let info = device.get_bootloader_info().await?;
            ensure!(
                info.model == manifest.identity.model
                    && info.hw_version == manifest.identity.hardware,
                "DFU hardware identity differs"
            );
            let output = stock.join("dfu-recovery-check");
            fs::create_dir(&output).context("DFU evidence directory must be new")?;
            let mut reader = Reader { device, dfu: true };
            for (original, name, address) in
                [(LOADER, "loader.bin", 4), (APP, "stock-app.bin", 0x4000)]
            {
                let expected = fs::read(stock.join(original))?;
                acquire(&mut reader, &output, name, address, expected.len()).await?;
                ensure!(
                    fs::read(output.join(name))? == expected,
                    "DFU readback differs from stock {original}"
                );
            }
            save_json(
                &output.join("verified.json"),
                &Qualification {
                    schema_version: 1,
                    stock_manifest_sha256: digest(&fs::read(stock.join("manifest.json"))?),
                    usb_port: manifest.usb_port,
                    loader_matches_stock: true,
                    application_matches_stock: true,
                },
            )?;
            verify_qualified(&stock)?;
            println!(
                "Native DFU reads match stock. Device remains in DFU; reconnect normally to start stock. No device writes performed."
            );
        }
        Command::Verify { stock, require_dfu } => {
            if require_dfu {
                verify_qualified(&stock)?;
            } else {
                verify_stock(&stock)?;
            }
            println!(
                "Offline verification passed (DFU qualification required: {require_dfu}). No USB accessed."
            );
        }
    }
    Ok(())
}
