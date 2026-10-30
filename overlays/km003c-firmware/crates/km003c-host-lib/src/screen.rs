//! Assemble a coherent, bounded snapshot of LCD output without pausing gameplay.
use crate::rpc::Client;
use anyhow::{Context, Result, ensure};
use km003c_interface::{
    CaptureScreen, ReadScreen,
    preview::{CHUNK, ScreenChunk, ScreenRead, WIDTH},
};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[derive(serde::Serialize)]
pub struct Frame {
    pub snapshot: u32,
    pub game_frame: u32,
    pub width: u16,
    pub height: u16,
    pub transfer_ms: u64,
    pub rgb565: String,
}
async fn read_chunk(client: &Client, snapshot: u32, offset: usize) -> Result<ScreenChunk> {
    let request = ScreenRead {
        snapshot,
        offset: offset as u16,
    };
    tokio::time::timeout(
        Duration::from_secs(3),
        client.rpc.send_resp::<ReadScreen>(&request),
    )
    .await
    .context("screen transfer timed out")?
    .context("screen transfer failed")
}
pub async fn capture(client: &Client) -> Result<Frame> {
    let start = Instant::now();
    let info = tokio::time::timeout(
        Duration::from_secs(3),
        client.rpc.send_resp::<CaptureScreen>(&()),
    )
    .await
    .context("screen capture timed out; install the preview firmware")??;
    ensure!(
        usize::from(info.width) == WIDTH
            && [90, 120].contains(&info.height)
            && usize::from(info.bytes) == WIDTH * usize::from(info.height) * 2
            && info.snapshot != 0,
        "unexpected screen geometry"
    );
    // Six reads plus the independent input heartbeat fit the transport's eight
    // in-flight slots. Queue them together to use the gap between game frames.
    let (a, b, c, d, e, f) = tokio::join!(
        read_chunk(client, info.snapshot, 0),
        read_chunk(client, info.snapshot, CHUNK),
        read_chunk(client, info.snapshot, CHUNK * 2),
        read_chunk(client, info.snapshot, CHUNK * 3),
        read_chunk(client, info.snapshot, CHUNK * 4),
        read_chunk(client, info.snapshot, CHUNK * 5),
    );
    let size = usize::from(info.bytes);
    let mut chunks = vec![a?, b?, c?, d?, e?, f?];
    if size > 6 * CHUNK {
        let (g, h) = tokio::join!(
            read_chunk(client, info.snapshot, CHUNK * 6),
            read_chunk(client, info.snapshot, CHUNK * 7)
        );
        chunks.push(g?);
        chunks.push(h?);
    }
    let mut bytes: Vec<u8> = Vec::with_capacity(size);
    for chunk in chunks {
        let count = usize::from(chunk.len);
        ensure!(
            chunk.valid
                && chunk.snapshot == info.snapshot
                && usize::from(chunk.offset) == bytes.len()
                && count == (size - bytes.len()).min(CHUNK),
            "snapshot replaced or invalid screen chunk"
        );
        bytes.extend(chunk.data.iter().flatten().flatten().take(count));
    }
    ensure!(bytes.len() == size, "incomplete screen snapshot");
    let mut rgb565 = String::with_capacity(size * 2);
    const HEX: &[u8] = b"0123456789abcdef";
    for byte in bytes {
        rgb565.push(HEX[usize::from(byte >> 4)] as char);
        rgb565.push(HEX[usize::from(byte & 15)] as char);
    }
    Ok(Frame {
        snapshot: info.snapshot,
        game_frame: info.game_frame,
        width: info.width,
        height: info.height,
        transfer_ms: start.elapsed().as_millis() as u64,
        rgb565,
    })
}
pub async fn stream(client: &Client, enabled: &AtomicBool) {
    loop {
        if !enabled.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }
        match capture(client).await {
            Ok(screen) => println!("{}", serde_json::json!({"screen":screen})),
            Err(error) => {
                println!("{}", serde_json::json!({"screen_error":error.to_string()}));
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
