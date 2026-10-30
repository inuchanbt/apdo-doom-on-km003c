// SPDX-License-Identifier: MIT
//! Board register facts: HC32F460 BSD-licensed CMSIS header; HW2.3 wiring.
pub fn r32(a: usize) -> u32 {
    unsafe { (a as *const u32).read_volatile() }
}
pub fn w32(a: usize, v: u32) {
    unsafe { (a as *mut u32).write_volatile(v) }
}
pub fn r16(a: usize) -> u16 {
    unsafe { (a as *const u16).read_volatile() }
}
pub fn w16(a: usize, v: u16) {
    unsafe { (a as *mut u16).write_volatile(v) }
}
pub fn w8(a: usize, v: u8) {
    unsafe { (a as *mut u8).write_volatile(v) }
}
pub fn delay_us(us: u32) {
    cortex_m::asm::delay(us.saturating_mul(192));
}

pub fn pin(port: usize, n: usize, control: u16, function: u16) {
    critical(|| {
        w16(0x40053bfc, 0xa501);
        let a = 0x40053c00 + port * 64 + n * 4;
        w16(a, control);
        w16(a + 2, function);
        w16(0x40053bfc, 0xa500);
    });
}
fn critical(f: impl FnOnce()) {
    cortex_m::interrupt::free(|_| f());
}
pub fn output(port: usize, n: usize, high: bool) {
    w16(0x40053800 + port * 16 + if high { 8 } else { 10 }, 1 << n);
}
pub fn input(port: usize, n: usize) -> bool {
    r16(0x40053800 + port * 16) & (1 << n) != 0
}
pub fn buttons() -> u32 {
    [7, 11, 8, 9]
        .iter()
        .enumerate()
        .fold(0, |m, (i, &n)| m | ((!input(1, n)) as u32) << i)
}
pub fn init() {
    w32(0x40050804, 0x77);
    w32(0x40050800, 0x11001111);
    w32(0x40050804, 0x76);
    use apdo_hardware::clocks::{self, Config, Divider::*, Dividers, Mpll};
    let clocks = unsafe {
        clocks::configure(Config {
            mpll: Mpll::new(2, 48, [2, 4, 8]).unwrap(),
            dividers: Dividers {
                hclk: Div1,
                exclk: Div4,
                pclk: [Div2, Div2, Div4, Div4, Div2],
            },
        })
        .unwrap()
    };
    assert_eq!(clocks.hclk, 192_000_000);
    unsafe {
        apdo_hardware::intc::clear_routes();
        apdo_hardware::time_driver_tmra::init(
            clocks.pclk[1],
            apdo_hardware::intc::Line::new(80).unwrap(),
            apdo_hardware::intc::Line::new(81).unwrap(),
            5,
        )
        .unwrap();
    }
    // DWT measures rendering work only; TimerA provides the monotonic clock.
    w32(0xe000edfc, r32(0xe000edfc) | (1 << 24));
    w32(0xe0001004, 0);
    w32(0xe0001000, r32(0xe0001000) | 1);
    for n in [7, 11, 8, 9] {
        pin(1, n, 0x40, 0);
    }
}

// Synchronous open-drain I2C is used only for panel reset/backlight.
const SCL: (usize, usize) = (5, 2);
const SDA: (usize, usize) = (2, 13);
fn line(p: (usize, usize), high: bool) {
    output(p.0, p.1, high);
    delay_us(5);
}
fn clock(high: bool) {
    line(SCL, high);
    if high {
        for _ in 0..1000 {
            if input(SCL.0, SCL.1) {
                return;
            }
            delay_us(1);
        }
        panic!("I2C clock held low");
    }
}
fn start() {
    line(SDA, true);
    clock(true);
    line(SDA, false);
    clock(false);
}
fn stop() {
    line(SDA, false);
    clock(true);
    line(SDA, true);
}
fn send(byte: u8) {
    for bit in (0..8).rev() {
        line(SDA, byte & (1 << bit) != 0);
        clock(true);
        clock(false);
    }
    line(SDA, true);
    clock(true);
    let ack = !input(SDA.0, SDA.1);
    clock(false);
    if !ack {
        stop();
        panic!("AW9523 did not acknowledge");
    }
}
fn receive() -> u8 {
    line(SDA, true);
    let mut b = 0;
    for _ in 0..8 {
        clock(true);
        b = b << 1 | input(SDA.0, SDA.1) as u8;
        clock(false);
    }
    line(SDA, true);
    clock(true);
    clock(false);
    b
}
fn read(reg: u8) -> u8 {
    start();
    send(0xb6);
    send(reg);
    start();
    send(0xb7);
    let b = receive();
    stop();
    b
}
fn write(reg: u8, value: u8) {
    start();
    send(0xb6);
    send(reg);
    send(value);
    stop();
}
pub fn backlight(percent: u8) {
    assert!((10..=100).contains(&percent));
    let level = (u16::from(percent) * 70 / 100) as u8;
    write(0x22, level);
    write(0x23, level / 2);
}
pub fn panel_reset() {
    for p in [SCL, SDA] {
        output(p.0, p.1, true);
        pin(p.0, p.1, 0x47, 0);
    }
    assert_eq!(read(0x10), 0x23);
    let mode = read(0x13);
    write(0x13, (mode | 2) & !12);
    let out = read(3);
    write(3, out | 2);
    let config = read(5);
    write(5, config & !2);
    write(3, out & !2);
    delay_us(50_000);
    write(3, out | 2);
    delay_us(10_000);
    backlight(100);
}
