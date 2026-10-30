//! Frame timings; optional motion test renews and then releases a leased turn.
use crate::{DeviceSelector, rpc::Client};
use anyhow::{Context, Result, ensure};
use km003c_interface::{GameInputRequest, GameInputState, GetDoomTiming, doom_timing::DoomTiming};
use std::time::{Duration, Instant};

pub async fn sample(selector: DeviceSelector, count: u16, turn_right: bool) -> Result<()> {
    ensure!((1..=1000).contains(&count), "samples must be 1..1000");
    let client = Client::connect(selector).await?;
    // Cold-start qualification blocks the game task for several seconds.
    // Do not start a three-second input RPC until that startup is complete.
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let ready = tokio::time::timeout(
                Duration::from_secs(12),
                client.rpc.send_resp::<GetDoomTiming>(&()),
            )
            .await??;
            if ready.frame != 0 {
                return Ok::<(), anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .context("game did not finish starting")??;
    let collect = collect(&client, count, if turn_right { 20 } else { 100 });
    let result = if turn_right {
        let turn = async {
            loop {
                crate::controller::exchange(
                    &client,
                    GameInputRequest::Set(GameInputState {
                        buttons: 8,
                        lease_ms: 750,
                    }),
                )
                .await?;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            #[allow(unreachable_code)]
            Ok::<(), anyhow::Error>(())
        };
        let result = tokio::select! {
            result = collect => result,
            result = turn => result.and_then(|()| anyhow::bail!("motion task ended")),
        };
        let release = crate::controller::exchange(
            &client,
            GameInputRequest::Set(GameInputState {
                buttons: 0,
                lease_ms: 0,
            }),
        )
        .await;
        result.and_then(|samples| {
            release?;
            Ok(samples)
        })
    } else {
        collect.await
    }?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
async fn collect(client: &Client, count: u16, interval: u64) -> Result<Vec<DoomTiming>> {
    let start = Instant::now();
    let mut samples: Vec<DoomTiming> = Vec::new();
    while samples.len() < usize::from(count) {
        ensure!(
            start.elapsed() < Duration::from_secs(120.max(u64::from(count) * 2)),
            "game timing samples timed out"
        );
        let sample = tokio::time::timeout(
            Duration::from_secs(12), // A cold boot first qualifies the entire game pack.
            client.rpc.send_resp::<GetDoomTiming>(&()),
        )
        .await??;
        if sample.frame != 0 && samples.last().is_none_or(|last| last.frame != sample.frame) {
            samples.push(sample);
        }
        tokio::time::sleep(Duration::from_millis(interval)).await;
    }
    Ok(samples)
}
