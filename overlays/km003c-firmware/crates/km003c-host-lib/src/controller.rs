//! Leased game input; no flash or loader operation is available on this path.
use crate::{DeviceSelector, rpc::Client};
use anyhow::{Context, Result, ensure};
use km003c_interface::{ControlDisplay, display_control::DisplayRequest};
use km003c_interface::{GameInput, GameInputReply, GameInputRequest, GameInputState};
use std::{
    collections::VecDeque,
    io::BufRead,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub async fn exchange(client: &Client, request: GameInputRequest) -> Result<GameInputReply> {
    tokio::time::timeout(
        Duration::from_secs(3),
        client.rpc.send_resp::<GameInput>(&request),
    )
    .await
    .context("game input timed out; install the PC-control firmware")?
    .context("game input endpoint unavailable; install the PC-control firmware")
}
pub async fn input(selector: DeviceSelector, buttons: u8, lease_ms: u16) -> Result<GameInputReply> {
    ensure!(
        buttons == 0 || (100..=1000).contains(&lease_ms),
        "lease must be 100..1000 ms"
    );
    let client = Client::connect(selector).await?;
    exchange(
        &client,
        GameInputRequest::Set(GameInputState { buttons, lease_ms }),
    )
    .await
}
pub async fn status(selector: DeviceSelector) -> Result<GameInputReply> {
    let client = Client::connect(selector).await?;
    exchange(&client, GameInputRequest::Status).await
}

struct InputQueue {
    latest: u8,
    pending: VecDeque<(Instant, u8)>,
    last_update: Instant,
    ended: bool,
}
impl InputQueue {
    fn new(now: Instant) -> Self {
        Self {
            latest: 0,
            pending: VecDeque::new(),
            last_update: now,
            ended: false,
        }
    }
    fn push(&mut self, buttons: u8, now: Instant) {
        if buttons != self.latest {
            if self.pending.len() >= 8 {
                self.pending.clear();
            }
            self.pending.push_back((now, buttons));
            self.latest = buttons;
        }
        self.last_update = now;
    }
    fn next(&mut self, now: Instant) -> u8 {
        if self.ended || now.duration_since(self.last_update) >= Duration::from_millis(500) {
            self.pending.clear();
            self.latest = 0;
            return 0;
        }
        while self
            .pending
            .front()
            .is_some_and(|(time, _)| now.duration_since(*time) >= Duration::from_millis(500))
        {
            self.pending.pop_front();
        }
        self.pending
            .pop_front()
            .map_or(self.latest, |(_, buttons)| buttons)
    }
}

/// Lines of decimal masks on stdin; one JSON status per USB acknowledgement.
/// A queue preserves short taps between rendered frames. Heartbeats expire too.
pub async fn control_stdin(selector: DeviceSelector, preview: bool) -> Result<()> {
    let client = Client::connect(selector).await?;
    let state = Arc::new(Mutex::new(InputQueue::new(Instant::now())));
    let reader_state = state.clone();
    let preview_enabled = Arc::new(AtomicBool::new(preview));
    let reader_preview = preview_enabled.clone();
    let display_queue = Arc::new(Mutex::new(VecDeque::from([DisplayRequest::Status])));
    let reader_display = display_queue.clone();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else {
                break;
            };
            if line.trim() == "preview 0" || line.trim() == "preview 1" {
                reader_preview.store(line.trim() == "preview 1", Ordering::Relaxed);
                continue;
            }
            let command = line.trim().split_once(' ').and_then(|(kind, value)| {
                let value = value.parse::<u8>().ok()?;
                let request = match kind {
                    "brightness" => DisplayRequest::Brightness(value),
                    "rotation" => DisplayRequest::Rotation(value),
                    _ => return None,
                };
                request.valid().then_some(request)
            });
            if let Some(request) = command {
                let mut queue = reader_display.lock().unwrap();
                if queue.len() < 8 {
                    queue.push_back(request);
                }
                continue;
            }
            let Ok(buttons) = line.trim().parse::<u8>() else {
                break;
            };
            reader_state.lock().unwrap().push(buttons, Instant::now());
        }
        reader_state.lock().unwrap().ended = true;
    });
    let interrupt = tokio::signal::ctrl_c();
    tokio::pin!(interrupt);
    let control = async {
        loop {
            let (buttons, ended) = {
                let mut input = state.lock().unwrap();
                (input.next(Instant::now()), input.ended)
            };
            let request = GameInputRequest::Set(GameInputState {
                buttons,
                lease_ms: 750,
            });
            let reply = tokio::select! {
                result = exchange(&client, request) => result?,
                _ = &mut interrupt => break,
            };
            println!("{}", serde_json::to_string(&reply)?);
            let display_request = display_queue.lock().unwrap().pop_front();
            if let Some(request) = display_request {
                match tokio::time::timeout(
                    Duration::from_secs(2),
                    client.rpc.send_resp::<ControlDisplay>(&request),
                )
                .await
                {
                    Ok(Ok(reply)) => println!("{}", serde_json::json!({"display":reply})),
                    _ => println!(
                        "{}",
                        serde_json::json!({"display_error":"LCD settings unavailable; install the display-control firmware"})
                    ),
                }
            }
            if ended {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        Ok(())
    };
    let result: Result<()> = if preview {
        tokio::select! {
            result = control => result,
            _ = crate::screen::stream(&client, &preview_enabled) => Ok(()),
        }
    } else {
        control.await
    };
    let _ = exchange(
        &client,
        GameInputRequest::Set(GameInputState {
            buttons: 0,
            lease_ms: 0,
        }),
    )
    .await;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_press_and_release_survive_between_frames() {
        let now = Instant::now();
        let mut queue = InputQueue::new(now);
        queue.push(1, now);
        queue.push(0, now + Duration::from_millis(20));
        assert_eq!(queue.next(now + Duration::from_millis(170)), 1);
        assert_eq!(queue.next(now + Duration::from_millis(340)), 0);
    }
    #[test]
    fn silence_and_eof_clear_queued_keys() {
        let now = Instant::now();
        let mut queue = InputQueue::new(now);
        queue.push(2, now);
        assert_eq!(queue.next(now + Duration::from_millis(500)), 0);
        queue.push(1, now + Duration::from_millis(600));
        queue.ended = true;
        assert_eq!(queue.next(now + Duration::from_millis(610)), 0);
    }
}
