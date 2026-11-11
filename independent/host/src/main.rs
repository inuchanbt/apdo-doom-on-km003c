// SPDX-License-Identifier: MIT
use anyhow::{Result, ensure};
use apdo_protocol as wire;
mod recovery;
mod storage;
use serde_json::json;
use std::{
    io::{self, BufRead, Write},
    sync::mpsc,
    time::{Duration, Instant},
};

mod usb;
use usb::Client;

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
        "target-hardware" if args.len() == 1 => {
            println!("{}", km003c_backup::TARGET_HW);
            return Ok(());
        }
        #[cfg(feature = "hw21-trial")]
        "exit-test" => {
            ensure!(
                args.len() == 3,
                "Usage: apdo-host exit-test STOCK_BACKUP DOOM_IMAGE"
            );
            return tokio::runtime::Runtime::new()?
                .block_on(recovery::exit_test(args[1].as_ref(), args[2].as_ref()));
        }
        "audit-image" => {
            ensure!(
                args.len() == 3,
                "Usage: apdo-host audit-image PACKAGE VERIFIED_READBACK"
            );
            return recovery::audit(args[1].as_ref(), args[2].as_ref());
        }
        "verify-nor" => {
            ensure!(
                args.len() == 4,
                "Usage: apdo-host verify-nor STOCK_BACKUP NOR_BACKUP PACK"
            );
            return storage::verify_restore(args[1].as_ref(), args[2].as_ref(), args[3].as_ref());
        }
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
        "check-app" | "start" => {
            ensure!(
                args.len() == 3,
                "Usage: apdo-host check-app|start STOCK_BACKUP VERIFIED_IMAGE"
            );
            let runtime = tokio::runtime::Runtime::new()?;
            let (stock, image) = (args[1].as_ref(), args[2].as_ref());
            return if args[0] == "check-app" {
                runtime.block_on(recovery::check_application(stock, image))
            } else {
                runtime.block_on(recovery::start(stock, image))
            };
        }
        _ => {}
    }
    let mut client =
        if matches!(args[0].as_str(), "loader" | "play" | "check-game") && args.len() == 2 {
            let stock = km003c_backup::verify_qualified(args[1].as_ref())?;
            Client::open_on(Some(&stock.usb_port))?
        } else {
            Client::open()?
        };
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
        #[cfg(feature = "hw21-trial")]
        "check-game" if args.len() == 2 => {
            let initial = client.info()?[0];
            let deadline = Instant::now() + Duration::from_secs(15);
            loop {
                let status = client.info()?;
                if status[1] != 0 && status[0] != initial && status[0] > 0 {
                    state(&status);
                    break;
                }
                ensure!(
                    Instant::now() < deadline,
                    "DOOM frame counter did not advance"
                );
                std::thread::sleep(Duration::from_millis(200));
            }
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
