// SPDX-License-Identifier: MIT
use anyhow::{Context, Result, ensure};
use apdo_protocol as wire;
mod recovery;
mod storage;
use nusb::{
    MaybeFuture,
    io::{EndpointRead, EndpointWrite},
    transfer::{Bulk, In, Out},
};
use serde_json::json;
use std::{
    io::{self, BufRead, Read, Write},
    sync::mpsc,
    time::{Duration, Instant},
};

struct Client {
    reader: EndpointRead<Bulk>,
    writer: EndpointWrite<Bulk>,
    sequence: u8,
}
impl Client {
    fn open() -> Result<Self> {
        Self::open_on(None)
    }
    fn open_on(port: Option<&km003c_backup::UsbPort>) -> Result<Self> {
        let mut found = nusb::list_devices().wait()?.filter(|d| {
            d.vendor_id() == 0x5fc9
                && d.product_id() == 0x63
                && d.product_string() == Some("APDO DOOM")
                && port.is_none_or(|p| d.bus_id() == p.bus_id && d.port_chain() == p.port_chain)
        });
        let device = found
            .next()
            .context("Connect a KM003C running the independent APDO firmware")?;
        ensure!(found.next().is_none(), "Connect only one APDO device");
        let device = device.open().wait()?;
        let interface = device.claim_interface(0).wait()?;
        Ok(Self {
            reader: interface
                .endpoint::<Bulk, In>(0x81)?
                .reader(1024)
                .with_read_timeout(Duration::from_secs(10)),
            writer: interface
                .endpoint::<Bulk, Out>(0x01)?
                .writer(64)
                .with_write_timeout(Duration::from_secs(2)),
            sequence: 0,
        })
    }
    fn call(&mut self, op: u8, a: u32, b: u32) -> Result<(u32, Vec<u8>)> {
        self.exchange(wire::request(op, 0, a, b))
    }
    fn exchange(&mut self, mut req: [u8; 64]) -> Result<(u32, Vec<u8>)> {
        self.sequence = self.sequence.wrapping_add(1);
        req[7] = self.sequence;
        let op = req[5];
        self.writer.write_all(&req)?;
        self.writer.flush()?;
        let mut header = [0; 16];
        self.reader.read_exact(&mut header)?;
        let len = wire::reply_len(&header).context("Invalid APDO reply header")?;
        ensure!(
            header[5] == op && header[7] == self.sequence,
            "APDO reply sequence mismatch"
        );
        let mut data = vec![0; len];
        self.reader.read_exact(&mut data)?;
        ensure!(
            header[6] == wire::OK,
            "Device rejected command {op}: status {}",
            header[6]
        );
        Ok((wire::word(&header, 8), data))
    }
    fn info(&mut self) -> Result<[u32; 8]> {
        let (_, data) = self.call(wire::INFO, 0, 0)?;
        ensure!(data.len() == 32, "Invalid device status");
        Ok(core::array::from_fn(|i| wire::word(&data, i * 4)))
    }
}
fn output(value: serde_json::Value) {
    println!("{value}");
    let _ = io::stdout().flush();
}
fn settings(s: &[u32; 8]) {
    output(json!({"display":{"accepted":true,"brightness":s[3],"rotation":s[4]}}));
    output(
        json!({"pacing":{"supported":true,"smooth":s[5]!=0,"period_us":if s[5]!=0 {4_000_000/35}else{0}}}),
    );
}
fn state(s: &[u32; 8]) {
    output(
        json!({"accepted":true,"running":s[1]!=0,"frames":s[0],"frame_ms":s[2],
    "remote_buttons":s[6],"applied_buttons":s[7]&255,"menu_active":s[7]&256!=0,"boot_splash":s[7]&wire::BOOT_SPLASH_FLAG!=0}),
    );
}
struct Capture {
    id: u32,
    frame: u32,
    bytes: Vec<u8>,
    began: Instant,
}
fn control(mut client: Client, mut preview: bool) -> Result<()> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            match line {
                Ok(line) => {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    let mut buttons = 0;
    let mut last_input = Instant::now();
    let mut last_poll = Instant::now() - Duration::from_secs(1);
    let mut last_capture = Instant::now();
    let mut capture: Option<Capture> = None;
    let s = client.info()?;
    let mut preview_ready = s[1] != 0 || s[7] & wire::BOOT_SPLASH_FLAG != 0;
    settings(&s);
    state(&s);
    loop {
        loop {
            let line = match rx.try_recv() {
                Ok(line) => line,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    client.call(wire::INPUT, 0, 0)?;
                    return Ok(());
                }
            };
            let fields: Vec<_> = line.split_whitespace().collect();
            if let [mask] = fields.as_slice() {
                if let Ok(mask) = mask.parse::<u8>() {
                    buttons = mask;
                    last_input = Instant::now();
                }
            } else if let [setting, value] = fields.as_slice()
                && let Ok(value) = value.parse::<u32>()
            {
                match *setting {
                    "preview" => {
                        preview = value != 0;
                        if !preview {
                            capture = None;
                        }
                    }
                    "brightness" | "rotation" | "pacing" => {
                        let op = match *setting {
                            "brightness" => wire::BRIGHTNESS,
                            "rotation" => wire::ROTATION,
                            _ => wire::PACING,
                        };
                        client.call(op, value, 0)?;
                        settings(&client.info()?);
                    }
                    _ => {}
                }
            }
        }
        if last_poll.elapsed() >= Duration::from_millis(80) {
            let mask = if last_input.elapsed() < Duration::from_millis(500) {
                buttons
            } else {
                0
            };
            client.call(wire::INPUT, mask as u32, 0)?;
            let status = client.info()?;
            preview_ready = status[1] != 0 || status[7] & wire::BOOT_SPLASH_FLAG != 0;
            state(&status);
            if !preview_ready {
                capture = None;
            }
            last_poll = Instant::now();
        }
        if preview
            && preview_ready
            && capture.is_none()
            && last_capture.elapsed() >= Duration::from_millis(650)
        {
            let (id, data) = client.call(wire::SNAPSHOT, 0, 0)?;
            ensure!(data.len() == 4, "Invalid snapshot metadata");
            capture = Some(Capture {
                id,
                frame: wire::word(&data, 0),
                bytes: Vec::with_capacity(wire::PREVIEW_BYTES),
                began: Instant::now(),
            });
            last_capture = Instant::now();
        }
        if let Some(c) = capture.as_mut() {
            let (id, chunk) = client.call(wire::SCREEN, c.bytes.len() as u32, c.id)?;
            let count = (wire::PREVIEW_BYTES - c.bytes.len()).min(1024);
            ensure!(
                id == c.id && chunk.len() == count,
                "Stale or truncated snapshot"
            );
            c.bytes.extend_from_slice(&chunk);
            if c.bytes.len() == wire::PREVIEW_BYTES {
                let hex: String = c.bytes.iter().map(|b| format!("{b:02x}")).collect();
                output(
                    json!({"screen":{"snapshot":c.id,"game_frame":c.frame,"width":120,"height":120,
                    "transfer_ms":c.began.elapsed().as_millis(),"rgb565":hex}}),
                );
                capture = None;
            }
        } else {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        !args.is_empty(),
        "Usage: apdo-host info | game-control [--preview] | loader | pause | play"
    );
    match args[0].as_str() {
        "nor-backup" => {
            ensure!(
                args.len() == 3,
                "Usage: apdo-host nor-backup STOCK_BACKUP NEW_OUTPUT_DIRECTORY"
            );
            return storage::acquire(args[1].as_ref(), args[2].as_ref());
        }
        "install-game" | "restore-nor" => {
            ensure!(
                args.len() == 5 || args.len() == 6 && args[5] == "--dry-run",
                "Usage: apdo-host install-game|restore-nor STOCK_BACKUP NOR_BACKUP PACK NEW_EVIDENCE_DIRECTORY [--dry-run]"
            );
            return storage::install(
                args[1].as_ref(),
                args[2].as_ref(),
                args[3].as_ref(),
                args[4].as_ref(),
                args[0] == "restore-nor",
                args.len() == 6,
            );
        }
        "diagnostics" => {
            ensure!(
                args.len() == 3,
                "Usage: apdo-host diagnostics STOCK_BACKUP NEW_OUTPUT_DIRECTORY"
            );
            return tokio::runtime::Runtime::new()?
                .block_on(recovery::diagnostics(args[1].as_ref(), args[2].as_ref()));
        }
        "package" | "package-installed" => {
            ensure!(
                args.len() == 3,
                "Usage: apdo-host package[-installed] INPUT OUTPUT"
            );
            return recovery::package(
                args[1].as_ref(),
                args[2].as_ref(),
                args[0] == "package-installed",
            );
        }
        "flash" => {
            ensure!(
                args.len() == 4,
                "Usage: apdo-host flash STOCK_BACKUP IMAGE EVIDENCE_DIRECTORY"
            );
            return tokio::runtime::Runtime::new()?.block_on(recovery::flash(
                args[1].as_ref(),
                args[2].as_ref(),
                args[3].as_ref(),
            ));
        }
        "start" => {
            ensure!(
                args.len() == 3,
                "Usage: apdo-host start STOCK_BACKUP VERIFIED_IMAGE"
            );
            return tokio::runtime::Runtime::new()?
                .block_on(recovery::start(args[1].as_ref(), args[2].as_ref()));
        }
        _ => {}
    }
    let mut client = Client::open()?;
    match args[0].as_str() {
        "info" => {
            let s = client.info()?;
            settings(&s);
            state(&s);
        }
        "game-control" => control(client, args.iter().any(|a| a == "--preview"))?,
        "loader" => {
            return tokio::runtime::Runtime::new()?.block_on(recovery::enter_loader(client));
        }
        "pause" => {
            client.call(wire::PAUSE, 0, 0)?;
        }
        "play" => {
            client.call(wire::PLAY, 0, 0)?;
        }
        _ => anyhow::bail!("Unknown command"),
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e:#}");
        std::process::exit(1);
    }
}
