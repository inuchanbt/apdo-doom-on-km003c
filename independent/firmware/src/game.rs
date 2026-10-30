// SPDX-License-Identifier: MIT
//! The executor's game task alone owns QSPI, game state and display settings.
use crate::{board, bus::Bus};
use apdo_protocol as wire;
use core::{cell::Cell, ptr};
use embassy_sync::{
    blocking_mutex::{Mutex, raw::CriticalSectionRawMutex},
    channel::Channel,
    signal::Signal,
};
use embassy_time::{Duration, Instant, Timer};
static COMMAND: Channel<CriticalSectionRawMutex, [u8; 64], 1> = Channel::new();
static REPLY: Signal<CriticalSectionRawMutex, (u8, u32, [u8; 1024], usize)> = Signal::new();
static REMOTE: Mutex<CriticalSectionRawMutex, Cell<(u8, u64)>> = Mutex::new(Cell::new((0, 0)));
static STATE: Mutex<CriticalSectionRawMutex, Cell<[u32; 8]>> = Mutex::new(Cell::new([0; 8]));
static mut BUS: *mut Bus = ptr::null_mut();
static mut KEYS: u32 = 0;
static mut LIVE: [u8; wire::PREVIEW_BYTES] = [0; wire::PREVIEW_BYTES];
static mut FROZEN: [u8; wire::PREVIEW_BYTES] = [0; wire::PREVIEW_BYTES];
static mut SNAPSHOT: u32 = 0;
static mut SNAPSHOT_FRAME: u32 = 0;
static mut EXIT_SINCE: Option<u64> = None;
unsafe extern "C" {
    fn km_doom_init();
    fn km_doom_step();
    fn km_doom_menu() -> i32;
    fn km_doom_redraw();
    static mut km_engine_ram: u8;
}
pub fn info() -> [u32; 8] {
    STATE.lock(Cell::get)
}
pub fn input(buttons: u8) {
    REMOTE.lock(|c| c.set((buttons, Instant::now().as_millis() + 750)));
}
pub fn snapshot() -> Option<(u32, u32)> {
    let state = info();
    if state[1] == 0 {
        return None;
    }
    // Called from an executor task: no frame drawing can interleave this copy.
    unsafe {
        ptr::copy_nonoverlapping(
            ptr::addr_of!(LIVE).cast::<u8>(),
            ptr::addr_of_mut!(FROZEN).cast::<u8>(),
            wire::PREVIEW_BYTES,
        );
        SNAPSHOT = SNAPSHOT.wrapping_add(1).max(1);
        SNAPSHOT_FRAME = state[0];
        Some((SNAPSHOT, SNAPSHOT_FRAME))
    }
}
pub fn screen(id: u32, offset: u32, out: &mut [u8]) -> bool {
    if offset as usize > wire::PREVIEW_BYTES || out.len() > wire::PREVIEW_BYTES - offset as usize {
        return false;
    }
    unsafe {
        if id == 0 || id != SNAPSHOT {
            return false;
        }
        ptr::copy_nonoverlapping(
            ptr::addr_of!(FROZEN).cast::<u8>().add(offset as usize),
            out.as_mut_ptr(),
            out.len(),
        );
    }
    true
}
pub async fn command(req: [u8; 64]) -> (u8, u32, [u8; 1024], usize) {
    COMMAND.send(req).await;
    REPLY.wait().await
}
#[embassy_executor::task]
pub async fn task() {
    board::w32(0x200f0fc4, 40);
    let mut bus = Bus::new();
    board::w32(0x200f0fc4, 50);
    bus.initialize_panel();
    board::w32(0x200f0fc4, 60);
    let mut signature = [0; 8];
    bus.read(0, &mut signature);
    let boot = Instant::now();
    let mut auto = &signature == b"DMCDOOM\0";
    let mut initialized = false;
    let mut running = false;
    let (mut frames, mut frame_ms) = (0u32, 0u32);
    let (mut brightness, mut rotation, mut smooth) = (100u8, 0u8, true);
    unsafe {
        BUS = &raw mut bus;
        ptr::write_bytes(ptr::addr_of_mut!(km_engine_ram), 0, 32768);
    }
    loop {
        let start = Instant::now();
        if board::buttons() & 2 != 0 {
            auto = false;
        }
        check_exit();
        if let Ok(req) = COMMAND.try_receive() {
            let (a, b) = (wire::word(&req, 8), wire::word(&req, 12));
            let mut data = [0; 1024];
            let mut len = 0;
            let mut status = wire::OK;
            match req[5] {
                wire::BRIGHTNESS if (10..=100).contains(&a) => {
                    brightness = a as u8;
                    board::backlight(brightness);
                }
                wire::ROTATION if a < 4 => {
                    rotation = a as u8;
                    bus.rotation(rotation);
                    bus.clear();
                    if initialized {
                        unsafe { km_doom_redraw() }
                    }
                }
                wire::PACING if a < 2 => smooth = a != 0,
                wire::PAUSE => {
                    auto = false;
                    running = false;
                    input(0);
                }
                wire::PLAY => {
                    auto = false;
                    if !initialized {
                        unsafe { km_doom_init() };
                        initialized = true;
                    }
                    running = true;
                }
                wire::NOR_READ if wire::readable(a, b) => {
                    len = b as usize;
                    bus.read(a, &mut data[..len]);
                }
                wire::NOR_ERASE
                    if !initialized && b == wire::ERASE_CONFIRM && wire::erasable(a) =>
                {
                    auto = false;
                    bus.erase(a);
                }
                wire::NOR_WRITE if !initialized && wire::writable(a, b) => {
                    auto = false;
                    bus.write(a, &req[16..16 + b as usize]);
                }
                _ => status = wire::INVALID,
            }
            STATE.lock(|c| {
                let mut s = c.get();
                s[3] = brightness as u32;
                s[4] = rotation as u32;
                s[5] = smooth as u32;
                c.set(s);
            });
            REPLY.signal((status, 0, data, len));
        }
        if auto && boot.elapsed() >= Duration::from_secs(3) {
            auto = false;
            unsafe {
                km_doom_init();
            }
            initialized = true;
            running = true;
        }
        let physical = board::buttons();
        let menu = running && unsafe { km_doom_menu() != 0 };
        let mut keys = if menu {
            ((physical >> 2) & 1) << 1
                | ((physical >> 3) & 1) << 7
                | ((physical >> 1) & 1) << 6
                | (physical & 1) << 5
        } else if physical & 3 == 3 {
            (physical & !3) | 32
        } else if physical & 12 == 12 {
            (physical & !12) | 16
        } else {
            physical
        };
        let remote = REMOTE.lock(|c| {
            let (buttons, until) = c.get();
            if Instant::now().as_millis() < until {
                buttons
            } else {
                0
            }
        });
        if running {
            keys |= u32::from(remote);
            unsafe {
                KEYS = keys;
                km_doom_step();
            }
            frame_ms = start.elapsed().as_millis() as u32;
            frames = frames.wrapping_add(1);
        }
        STATE.lock(|s| {
            s.set([
                frames,
                running as u32,
                frame_ms,
                brightness as u32,
                rotation as u32,
                smooth as u32,
                remote as u32,
                keys | (menu as u32) << 8,
            ])
        });
        // Service USB while preserving the validated four-tic frame interval.
        Timer::after_millis(1).await;
        if running && smooth {
            Timer::at(start + Duration::from_micros(4_000_000 / 35)).await;
        }
    }
}
fn check_exit() {
    let now = Instant::now().as_millis();
    unsafe {
        if board::buttons() & 1 == 0 {
            EXIT_SINCE = None;
        } else {
            match EXIT_SINCE {
                None => EXIT_SINCE = Some(now),
                Some(t) if now - t >= 2000 => crate::loader(0x600d1001),
                _ => {}
            }
        }
    }
}
#[unsafe(no_mangle)]
extern "C" fn km_millis() -> u32 {
    Instant::now().as_millis() as u32
}
#[unsafe(no_mangle)]
extern "C" fn km_sleep(ms: u32) {
    let end = Instant::now() + Duration::from_millis(ms as u64);
    while Instant::now() < end {
        check_exit();
    }
}
#[unsafe(no_mangle)]
extern "C" fn km_fail(_: *const u8) -> ! {
    crate::loader(0xe100)
}
#[unsafe(no_mangle)]
extern "C" fn km_exit() -> ! {
    crate::loader(0x600d1002)
}
#[unsafe(no_mangle)]
extern "C" fn km_layout(square: i32) {
    assert!(square != 0);
}
#[unsafe(no_mangle)]
extern "C" fn km_cycles() -> u32 {
    board::r32(0xe0001004)
}
#[unsafe(no_mangle)]
extern "C" fn km_keys() -> u32 {
    check_exit();
    unsafe { KEYS }
}
#[unsafe(no_mangle)]
unsafe extern "C" fn km_flash_read(at: u32, out: *mut u8, n: u32) {
    unsafe {
        (*BUS).read(at, core::slice::from_raw_parts_mut(out, n as usize));
    }
}
#[unsafe(no_mangle)]
unsafe extern "C" fn km_column(x: i32, y: i32, h: i32, pixels: *const u16) {
    assert!((0..240).contains(&x) && y >= 0 && h >= 0 && y + h <= 240);
    unsafe {
        let pixels = core::slice::from_raw_parts(pixels, h as usize);
        (*BUS).column(x as u16, y as u16, pixels);
        if x % 2 == 0 {
            for (i, &color) in pixels.iter().enumerate() {
                let row = y as usize + i;
                if row.is_multiple_of(2) {
                    let at = (row / 2 * 120 + x as usize / 2) * 2;
                    let p = ptr::addr_of_mut!(LIVE).cast::<u8>().add(at);
                    p.write(color as u8);
                    p.add(1).write((color >> 8) as u8);
                }
            }
        }
    }
}
#[unsafe(no_mangle)]
unsafe extern "C" fn km_validate_reads(size: u32) {
    assert!(size <= wire::GAME_LIMIT);
    if option_env!("APDO_SAFE_NOR") == Some("1") {
        return;
    }
    let mut slow = [0; 256];
    let mut fast = [0; 256];
    let mut at = 0;
    let mut matched = true;
    unsafe {
        while at < size {
            let n = (size - at).min(256) as usize;
            (*BUS).fast = false;
            (*BUS).read(at, &mut slow[..n]);
            (*BUS).fast = true;
            (*BUS).read(at, &mut fast[..n]);
            matched &= slow[..n] == fast[..n];
            at += n as u32;
            check_exit();
        }
        (*BUS).fast = matched;
    }
}
