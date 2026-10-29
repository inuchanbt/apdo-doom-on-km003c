//! Small RGB565 shadow of pixels sent to the LCD, with an immutable snapshot.
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

pub const WIDTH: usize = 120;
pub const HEIGHT: usize = 90;
pub const BYTES: usize = WIDTH * HEIGHT * 2;
pub const CHUNK: usize = 4096;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Schema)]
pub struct ScreenInfo {
    pub snapshot: u32,
    pub game_frame: u32,
    pub width: u16,
    pub height: u16,
    pub bytes: u16,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Schema)]
pub struct ScreenRead {
    pub snapshot: u32,
    pub offset: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct ScreenChunk {
    pub valid: bool,
    pub snapshot: u32,
    pub offset: u16,
    pub len: u16,
    pub data: [[[u8; 32]; 16]; 8],
}

pub struct ScreenBuffers {
    live: [u8; BYTES],
    frozen: [u8; BYTES],
    snapshot: u32,
    game_frame: u32,
}
impl Default for ScreenBuffers {
    fn default() -> Self {
        Self::new()
    }
}
impl ScreenBuffers {
    pub const fn new() -> Self {
        Self {
            live: [0; BYTES],
            frozen: [0; BYTES],
            snapshot: 0,
            game_frame: 0,
        }
    }
    /// Match the LCD's 320x240 -> 240x180 mapping, then sample every other pixel.
    pub fn column(&mut self, x: usize, y: usize, pixels: &[u16]) {
        if x >= 320 || x % 4 == 3 || y >= 240 || pixels.len() > 240 - y {
            return;
        }
        let native_x = x - x / 4;
        if !native_x.is_multiple_of(2) {
            return;
        }
        for (i, color) in pixels.iter().enumerate() {
            let source_y = y + i;
            let native_y = source_y - source_y / 4;
            if source_y % 4 == 3 || !native_y.is_multiple_of(2) {
                continue;
            }
            let at = ((native_y / 2) * WIDTH + native_x / 2) * 2;
            self.live[at..at + 2].copy_from_slice(&color.to_le_bytes());
        }
    }
    pub fn finished(&mut self, frame: u32) {
        self.game_frame = frame;
    }
    pub fn capture(&mut self) -> ScreenInfo {
        self.frozen.copy_from_slice(&self.live);
        self.snapshot = self.snapshot.wrapping_add(1).max(1);
        ScreenInfo {
            snapshot: self.snapshot,
            game_frame: self.game_frame,
            width: WIDTH as u16,
            height: HEIGHT as u16,
            bytes: BYTES as u16,
        }
    }
    pub fn read(&self, request: ScreenRead) -> ScreenChunk {
        let mut reply = ScreenChunk {
            valid: false,
            snapshot: self.snapshot,
            offset: request.offset,
            len: 0,
            data: [[[0; 32]; 16]; 8],
        };
        let offset = usize::from(request.offset);
        if self.snapshot == 0 || request.snapshot != self.snapshot || offset >= BYTES {
            return reply;
        }
        let len = (BYTES - offset).min(CHUNK);
        for (i, byte) in self.frozen[offset..offset + len].iter().enumerate() {
            reply.data[i / 512][(i / 32) % 16][i % 32] = *byte;
        }
        reply.valid = true;
        reply.len = len as u16;
        reply
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_stays_consistent_while_lcd_changes() {
        let mut buffers = ScreenBuffers::new();
        buffers.column(0, 0, &[0xf800]);
        buffers.finished(12);
        let info = buffers.capture();
        buffers.column(0, 0, &[0x07e0]);
        let old = buffers.read(ScreenRead {
            snapshot: info.snapshot,
            offset: 0,
        });
        assert!(old.valid);
        assert_eq!(&old.data[0][0][..2], &[0x00, 0xf8]);
        assert_eq!(info.game_frame, 12);
        let next = buffers.capture();
        assert!(
            !buffers
                .read(ScreenRead {
                    snapshot: info.snapshot,
                    offset: 0
                })
                .valid
        );
        assert_eq!(
            &buffers
                .read(ScreenRead {
                    snapshot: next.snapshot,
                    offset: 0
                })
                .data[0][0][..2],
            &[0xe0, 0x07]
        );
    }
    #[test]
    fn mapping_clips_and_preserves_partial_hud_updates() {
        let mut buffers = ScreenBuffers::new();
        buffers.column(318, 239, &[0xffff]); // The final source row is discarded.
        buffers.column(317, 237, &[0x1234]); // Native (238,178), preview (119,89).
        buffers.column(0, 0, &[0xabcd]);
        let info = buffers.capture();
        let end = buffers.read(ScreenRead {
            snapshot: info.snapshot,
            offset: (BYTES - 2) as u16,
        });
        assert_eq!(end.len, 2);
        assert_eq!(&end.data[0][0][..2], &[0x34, 0x12]);
        buffers.column(320, 0, &[1]);
        buffers.column(0, 240, &[1]);
        buffers.column(0, 239, &[1, 2]);
        let next = buffers.capture();
        assert_eq!(
            &buffers
                .read(ScreenRead {
                    snapshot: next.snapshot,
                    offset: 0
                })
                .data[0][0][..2],
            &[0xcd, 0xab]
        );
        assert!(
            !buffers
                .read(ScreenRead {
                    snapshot: next.snapshot,
                    offset: u16::MAX
                })
                .valid
        );
    }
}
