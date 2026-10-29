//! Game storage acquisition/upload with verified readback and a protected tail.
use crate::{DeviceSelector, maintenance::digest, rpc::Client};
use anyhow::{Result, ensure};
use km003c_interface::{GameCommand, GameIo, GameReply};
use std::{io::Write, path::Path, time::Duration};

fn save(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
async fn io(client: &Client, command: GameCommand) -> Result<GameReply> {
    let reply = tokio::time::timeout(
        Duration::from_secs(10),
        client.rpc.send_resp::<GameIo>(&command),
    )
    .await??;
    ensure!(
        reply.code == 0,
        "game operation failed: code {} JEDEC {:06x}",
        reply.code,
        reply.jedec
    );
    Ok(reply)
}
pub async fn status(selector: DeviceSelector) -> Result<()> {
    let c = Client::connect(selector).await?;
    let r = io(
        &c,
        GameCommand::Read {
            address: 0,
            len: 16,
        },
    )
    .await?;
    println!(
        "JEDEC {:06x}, rendered frames {}, last frame {} ms, prefix {:02x?}",
        r.jedec,
        r.frames,
        r.elapsed_ms,
        &r.data[0][..16]
    );
    Ok(())
}
async fn read(c: &Client, address: u32, size: usize) -> Result<Vec<u8>> {
    ensure!(
        size <= 0x400000
            && address
                .checked_add(size as u32)
                .is_some_and(|end| end <= 0x400000),
        "invalid NOR range"
    );
    let mut bytes = Vec::with_capacity(size);
    while bytes.len() < size {
        let n = (size - bytes.len()).min(256);
        let reply = io(
            c,
            GameCommand::Read {
                address: address + bytes.len() as u32,
                len: n as u16,
            },
        )
        .await?;
        ensure!(usize::from(reply.len) == n, "short game storage read");
        bytes.extend(reply.data.iter().flatten().take(n));
        if bytes.len() % 0x40000 == 0 || bytes.len() == size {
            println!("NOR {address:#x}: {}/{}", bytes.len(), size);
        }
    }
    Ok(bytes)
}
pub async fn backup(selector: DeviceSelector, output: &Path, stock_prefix: &Path) -> Result<()> {
    let expected = std::fs::read(stock_prefix)?;
    ensure!(
        expected.len() == 0x300000,
        "stock prefix must be exactly 3 MiB"
    );
    std::fs::create_dir(output)?;
    let c = Client::connect(selector).await?;
    let first = read(&c, 0, 0x400000).await?;
    save(&output.join("external-4MiB.bin"), &first)?;
    let second = read(&c, 0, first.len()).await?;
    ensure!(first == second, "external NOR reads differ");
    ensure!(
        first[..0x300000] == expected,
        "custom NOR read differs from stock firmware read"
    );
    save(&output.join("verified.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "bytes":first.len(),"sha256":digest(&first),"matching_reads":2,"stock_prefix_matches":true,
        "protected_tail_sha256":digest(&first[0x300000..])
    }))?.as_slice())?;
    println!("Full external NOR backup verified against two reads and the stock read");
    Ok(())
}
pub async fn install(
    selector: DeviceSelector,
    data: &Path,
    backup: &Path,
    evidence: &Path,
) -> Result<()> {
    let bytes = std::fs::read(data)?;
    ensure!(
        bytes.len() >= 4096 && bytes.len() <= 0x300000 && &bytes[..8] == b"DMCDOOM\0",
        "invalid raw game pack"
    );
    let original = std::fs::read(backup.join("external-4MiB.bin"))?;
    let verified: serde_json::Value =
        serde_json::from_slice(&std::fs::read(backup.join("verified.json"))?)?;
    ensure!(
        original.len() == 0x400000
            && verified["sha256"] == digest(&original)
            && verified["matching_reads"] == 2
            && verified["stock_prefix_matches"] == true,
        "verified full backup required"
    );
    std::fs::create_dir(evidence)?;
    let c = Client::connect(selector).await?;
    let protected = read(&c, 0x300000, 0x100000).await?;
    ensure!(
        protected == original[0x300000..],
        "protected NOR changed since backup"
    );
    save(&evidence.join("before.json"),serde_json::to_vec_pretty(&serde_json::json!({"data_sha256":digest(&bytes),"bytes":bytes.len(),"backup_sha256":digest(&original),"write_limit":0x300000}))?.as_slice())?;
    for address in (0..bytes.len().div_ceil(4096) * 4096).step_by(4096) {
        io(
            &c,
            GameCommand::Erase {
                address: address as u32,
                unlock: 0x444f4f4d,
            },
        )
        .await?;
        if address % 0x40000 == 0 {
            println!("Erase {address:#x}");
        }
    }
    for (i, chunk) in bytes.chunks(256).enumerate() {
        let mut data = [[0xff; 32]; 8];
        for (k, b) in chunk.iter().enumerate() {
            data[k / 32][k % 32] = *b;
        }
        io(
            &c,
            GameCommand::Write {
                address: (i * 256) as u32,
                len: chunk.len() as u16,
                data,
            },
        )
        .await?;
        if i % 1024 == 0 {
            println!("Program {}/{}", i * 256, bytes.len());
        }
    }
    let actual = read(&c, 0, bytes.len()).await?;
    ensure!(actual == bytes, "game data readback differs");
    save(&evidence.join("game-readback.dat"), &actual)?;
    let after = read(&c, 0x300000, 0x100000).await?;
    ensure!(after == protected, "protected NOR tail changed");
    save(&evidence.join("verified.json"),serde_json::to_vec_pretty(&serde_json::json!({"game_readback_verified":true,"data_sha256":digest(&actual),"protected_tail_unchanged":true,"protected_tail_sha256":digest(&after)}))?.as_slice())?;
    println!("Game data verified; factory/settings tail is unchanged");
    Ok(())
}
pub async fn start(selector: DeviceSelector) -> Result<()> {
    let c = Client::connect(selector).await?;
    io(&c, GameCommand::Start).await?;
    println!("DOOM started");
    Ok(())
}

pub async fn restore(selector: DeviceSelector, backup: &Path, evidence: &Path) -> Result<()> {
    let original = std::fs::read(backup.join("external-4MiB.bin"))?;
    let verified: serde_json::Value =
        serde_json::from_slice(&std::fs::read(backup.join("verified.json"))?)?;
    ensure!(
        original.len() == 0x400000
            && verified["sha256"] == digest(&original)
            && verified["matching_reads"] == 2
            && verified["stock_prefix_matches"] == true,
        "verified full backup required"
    );
    std::fs::create_dir(evidence)?;
    let c = Client::connect(selector).await?;
    io(&c, GameCommand::Pause).await?;
    let tail = read(&c, 0x300000, 0x100000).await?;
    ensure!(
        tail == original[0x300000..],
        "protected tail mismatch; refusing restore"
    );
    for address in (0..0x300000).step_by(4096) {
        io(
            &c,
            GameCommand::Erase {
                address,
                unlock: 0x444f4f4d,
            },
        )
        .await?;
        if address % 0x40000 == 0 {
            println!("Restore erase {address:#x}");
        }
    }
    for (i, chunk) in original[..0x300000].chunks(256).enumerate() {
        if chunk.iter().all(|b| *b == 0xff) {
            continue;
        }
        let mut data = [[0; 32]; 8];
        for (k, b) in chunk.iter().enumerate() {
            data[k / 32][k % 32] = *b;
        }
        io(
            &c,
            GameCommand::Write {
                address: (i * 256) as u32,
                len: 256,
                data,
            },
        )
        .await?;
        if i % 1024 == 0 {
            println!("Restore program {}", i * 256);
        }
    }
    let actual = read(&c, 0, 0x400000).await?;
    ensure!(actual == original, "NOR restoration readback mismatch");
    save(&evidence.join("verified.json"),serde_json::to_vec_pretty(&serde_json::json!({"full_external_restore_verified":true,"sha256":digest(&actual),"protected_tail_unchanged":true}))?.as_slice())?;
    println!("Original external NOR restored and all 4 MiB verified");
    Ok(())
}
