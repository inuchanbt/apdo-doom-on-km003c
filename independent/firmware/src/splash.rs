// SPDX-License-Identifier: MIT
//! Original Freedoom logo pixels are BSD-licensed; see assets/*.json and notices.
const IMAGE: &[u8; 4 + 159 * 37 * 2] = include_bytes!("../assets/freedoom-banner.rgb565");
const WIDTH: usize = 159;
const HEIGHT: usize = 37;
const DRAW_WIDTH: usize = 216;
const DRAW_HEIGHT: usize = DRAW_WIDTH * HEIGHT / WIDTH;

/// Fill the whole display so neither the LCD nor its preview retains old pixels.
/// One small column is sufficient; the banner consumes no framebuffer RAM.
pub fn draw(mut column: impl FnMut(u16, &[u16])) {
    let left = (240 - DRAW_WIDTH) / 2;
    let top = (240 - DRAW_HEIGHT) / 2;
    for x in 0..240 {
        let mut pixels = [0; 240];
        if (left..left + DRAW_WIDTH).contains(&x) {
            let source_x = (x - left) * WIDTH / DRAW_WIDTH;
            for y in 0..DRAW_HEIGHT {
                let source_y = y * HEIGHT / DRAW_HEIGHT;
                let at = 4 + (source_y * WIDTH + source_x) * 2;
                pixels[top + y] = u16::from_le_bytes([IMAGE[at], IMAGE[at + 1]]);
            }
        }
        column(x as u16, &pixels);
    }
}
