//! Small RGB565 shadow of pixels sent to the LCD, with an immutable snapshot.
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

pub const WIDTH: usize = 120;
pub const HEIGHT: usize = 120; // Maximum; letterbox firmware reports 90.
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
    square: bool,
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
            square: false,
            live: [0; BYTES],
            frozen: [0; BYTES],
            snapshot: 0,
            game_frame: 0,
        }
    }
    pub fn layout(&mut self, square: bool) {
        self.square = square;
        self.live.fill(0);
        self.frozen.fill(0);
        self.snapshot = 0;
    }
    fn height(&self) -> usize {
        if self.square { 120 } else { 90 }
    }
    fn bytes(&self) -> usize {
        WIDTH * self.height() * 2
    }
    /// Match the LCD's 320x240 -> 240x180 mapping, then sample every other pixel.
    pub fn column(&mut self, x: usize, y: usize, pixels: &[u16]) {
        if x >= if self.square { 240 } else { 320 }
            || (!self.square && x % 4 == 3)
            || y >= 240
            || pixels.len() > 240 - y
        {
            return;
        }
        let native_x = if self.square { x } else { x - x / 4 };
        if !native_x.is_multiple_of(2) {
            return;
        }
        for (i, color) in pixels.iter().enumerate() {
            let source_y = y + i;
            let native_y = if self.square {
                source_y
            } else {
                source_y - source_y / 4
            };
            if (!self.square && source_y % 4 == 3) || !native_y.is_multiple_of(2) {
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
            height: self.height() as u16,
            bytes: self.bytes() as u16,
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
        if self.snapshot == 0 || request.snapshot != self.snapshot || offset >= self.bytes() {
            return reply;
        }
        let len = (self.bytes() - offset).min(CHUNK);
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
    fn square_layout_samples_all_edges_and_reports_eight_chunks() {
        let mut buffers = ScreenBuffers::new();
        buffers.layout(true);
        buffers.column(238, 238, &[0x1234]);
        buffers.column(240, 0, &[0xffff]);
        let info = buffers.capture();
        assert_eq!((info.width, info.height, info.bytes), (120, 120, 28800));
        let mut bytes = [0u8; BYTES];
        let mut count = 0;
        for offset in (0..usize::from(info.bytes)).step_by(CHUNK) {
            let chunk = buffers.read(ScreenRead {
                snapshot: info.snapshot,
                offset: offset as u16,
            });
            assert!(chunk.valid);
            for byte in chunk
                .data
                .iter()
                .flatten()
                .flatten()
                .take(usize::from(chunk.len))
            {
                bytes[count] = *byte;
                count += 1;
            }
        }
        assert_eq!(count, 28800);
        assert_eq!(&bytes[28798..], &[0x34, 0x12]);
        buffers.layout(false);
        let info = buffers.capture();
        assert_eq!((info.height, info.bytes), (90, 21600));
        assert_eq!(buffers.live[28798], 0);
    }
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
            offset: (120 * 90 * 2 - 2) as u16,
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
