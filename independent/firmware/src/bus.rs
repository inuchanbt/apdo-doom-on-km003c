// SPDX-License-Identifier: MIT
//! Exclusive synchronous owner of the shared LCD/NOR QSPI controller.
use crate::board::*;
const Q: usize = 0x9c000000;
pub struct Bus {
    pub fast: bool,
}
fn idle() {
    for _ in 0..2_000_000 {
        if r32(Q + 12) & 1 == 0 {
            return;
        }
    }
    panic!("QSPI timeout");
}
fn byte(b: u8) {
    w32(Q + 16, u32::from(b));
}
impl Bus {
    pub fn new() -> Self {
        w32(0x40048004, r32(0x40048004) & !8);
        output(1, 15, true);
        pin(1, 15, 3, 0);
        for n in [1, 2, 10, 12, 13, 14] {
            pin(1, n, 0, 7);
        }
        // Exit inherited QPI before selecting standard SPI, mode 3, HCLK/24.
        w32(Q, 0x00172aa7);
        byte(0xff);
        idle();
        w32(Q, 0x00172a87);
        w32(Q, 0x00170080);
        w32(Q + 4, 0x2f);
        w32(Q + 8, 0x8572);
        w32(Q + 20, 0x0b);
        let mut bus = Self { fast: false };
        let mut id = [0; 3];
        bus.transaction(&[0x9f], &mut id);
        assert!(
            id[0] != 0 && id[0] != 255 && id[2] == 0x16,
            "Unexpected NOR"
        );
        bus
    }
    fn transaction(&mut self, command: &[u8], out: &mut [u8]) {
        idle();
        let cr = r32(Q);
        w32(Q, cr | 32);
        for &b in command {
            byte(b);
        }
        idle();
        for b in out {
            *b = r32(Q + 16) as u8;
            idle();
        }
        w32(Q, cr & !32);
    }
    pub fn read(&mut self, address: u32, out: &mut [u8]) {
        assert!(
            address
                .checked_add(out.len() as u32)
                .is_some_and(|e| e <= 0x400000)
        );
        let cr = r32(Q);
        if self.fast {
            w32(Q, (cr & !0x003f0000) | (11 << 16));
        }
        self.transaction(
            &[
                3,
                (address >> 16) as u8,
                (address >> 8) as u8,
                address as u8,
            ],
            out,
        );
        w32(Q, cr);
    }
    fn ready(&mut self) {
        let deadline = embassy_time::Instant::now() + embassy_time::Duration::from_secs(3);
        loop {
            let mut status = [0];
            self.transaction(&[5], &mut status);
            if status[0] & 1 == 0 {
                break;
            }
            assert!(embassy_time::Instant::now() < deadline, "NOR write timeout");
        }
    }
    pub fn erase(&mut self, address: u32) {
        assert!(apdo_protocol::erasable(address));
        self.ready();
        self.transaction(&[6], &mut []);
        self.transaction(
            &[
                0x20,
                (address >> 16) as u8,
                (address >> 8) as u8,
                address as u8,
            ],
            &mut [],
        );
        self.ready();
    }
    pub fn write(&mut self, address: u32, data: &[u8]) {
        assert!(apdo_protocol::writable(address, data.len() as u32));
        let mut packet = [0; 52];
        packet[0] = 2;
        packet[1] = (address >> 16) as u8;
        packet[2] = (address >> 8) as u8;
        packet[3] = address as u8;
        packet[4..4 + data.len()].copy_from_slice(data);
        self.ready();
        self.transaction(&[6], &mut []);
        self.transaction(&packet[..4 + data.len()], &mut []);
        self.ready();
    }
    fn select_lcd(&mut self) -> u32 {
        idle();
        let cr = r32(Q);
        pin(1, 1, 0x40, 0);
        assert!(input(1, 1), "NOR CS must be inactive");
        output(1, 15, false);
        w32(Q, (cr & 0xffc0fff8) | 0x20020);
        cr
    }
    fn release_lcd(&mut self, cr: u32) {
        idle();
        w32(Q, r32(Q) & !32);
        output(1, 15, true);
        pin(1, 1, 0, 7);
        w32(Q, cr);
    }
    fn dc(data: bool) {
        idle();
        let f = r32(Q + 8);
        w32(Q + 8, if data { f | 64 } else { f & !64 });
    }
    fn command(command: u8, params: &[u8]) {
        Self::dc(false);
        byte(command);
        Self::dc(true);
        for &b in params {
            byte(b);
        }
    }
    pub fn initialize_panel(&mut self) {
        crate::board::panel_reset();
        self.command_only(0x11, &[]);
        delay_us(120_000);
        self.command_only(0x3a, &[5]);
        self.rotation(0);
        self.command_only(0x21, &[]);
        self.command_only(0x29, &[]);
        self.clear();
    }
    fn command_only(&mut self, c: u8, p: &[u8]) {
        let cr = self.select_lcd();
        Self::command(c, p);
        self.release_lcd(cr);
    }
    pub fn rotation(&mut self, quarter_turns: u8) {
        self.command_only(0x36, &[[0x60, 0xc0, 0xa0, 0x00][quarter_turns as usize]]);
    }
    pub fn column(&mut self, x: u16, y: u16, pixels: &[u16]) {
        if pixels.is_empty() {
            return;
        }
        assert!(x < 240 && y as usize + pixels.len() <= 240);
        let cr = self.select_lcd();
        Self::command(0x2a, &[0, x as u8, 0, x as u8]);
        Self::command(
            0x2b,
            &[0, y as u8, 0, (y as usize + pixels.len() - 1) as u8],
        );
        Self::command(0x2c, &[]);
        for &p in pixels {
            byte((p >> 8) as u8);
            byte(p as u8);
        }
        self.release_lcd(cr);
    }
    pub fn clear(&mut self) {
        for x in 0..240 {
            self.column(x, 0, &[0; 240]);
        }
    }
}
