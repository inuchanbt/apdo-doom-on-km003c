// SPDX-License-Identifier: MIT
#![no_std]
#![no_main]
use cortex_m_rt::{entry, exception};
mod board;
mod bus;
mod game;
mod usb;

// Native loader metadata ABI. This application never embeds a loader image.
const fn metadata() -> [u8; 64] {
    let mut m = [0; 64];
    m[1] = 0x40;
    let mut n = 8;
    while n < 16 {
        m[n] = 255;
        n += 1;
    }
    let fields: [&[u8]; 3] = [b"KM003C", b"2.0.7-apdo1", b"2026.10.31"];
    let mut f = 0;
    while f < 3 {
        n = 0;
        while n < fields[f].len() {
            m[16 + 12 * f + n] = fields[f][n];
            n += 1;
        }
        f += 1;
    }
    m
}
#[used]
#[unsafe(link_section = ".apdo_metadata")]
static METADATA: [u8; 64] = metadata();

// This hook cannot read application RAM: it runs before .data/.bss startup.
core::arch::global_asm!(
    ".section .text.__pre_init, \"ax\"",
    ".global __pre_init",
    ".thumb_func",
    "__pre_init:",
    "cpsid i",
    "ldr r0, =0xe000e010",
    "movs r1, #0",
    "str r1, [r0]",
    "ldr r0, =0xe000ed04",
    "ldr r1, =0x0a000000",
    "str r1, [r0]",
    "ldr r0, =0xe000e180",
    "ldr r1, =0xe000e280",
    "movs r2, #0",
    "mvns r2, r2",
    "movs r3, #0",
    "2:",
    "str r2, [r0, r3]",
    "str r2, [r1, r3]",
    "adds r3, #4",
    "cmp r3, #20",
    "bne 2b",
    "ldr r0, =0xe000ed08",
    "ldr r1, =0x4000",
    "str r1, [r0]",
    "dsb",
    "isb",
    "bx lr",
);
#[derive(Clone, Copy)]
#[repr(C)]
union Vector {
    handler: unsafe extern "C" fn(),
    reserved: u32,
}
const fn vectors() -> [Vector; 144] {
    let mut v = [Vector {
        handler: unexpected,
    }; 144];
    v[0] = Vector {
        handler: usb::interrupt,
    };
    v[80] = Vector { handler: timer };
    v[81] = Vector { handler: timer };
    v
}
#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".vector_table.interrupts")]
static __INTERRUPTS: [Vector; 144] = vectors();
unsafe extern "C" fn timer() {
    apdo_hardware::time_driver_tmra::on_interrupt();
}
unsafe extern "C" fn unexpected() {
    loader(0xe001);
}
pub fn loader(reason: u32) -> ! {
    cortex_m::interrupt::disable();
    board::w32(0x200f0fcc, reason);
    board::w32(0x200f0ff0, 0x12345678);
    cortex_m::asm::dsb();
    cortex_m::peripheral::SCB::sys_reset()
}
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;
    let mut writer = Retained { used: 0 };
    let _ = write!(writer, "{info}");
    loader(0xe002);
}
struct Retained {
    used: usize,
}
impl core::fmt::Write for Retained {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            if self.used >= 255 {
                break;
            }
            unsafe {
                (0x200f0e00 as *mut u8).add(self.used).write_volatile(b);
            }
            self.used += 1;
        }
        unsafe {
            (0x200f0e00 as *mut u8).add(self.used).write_volatile(0);
        }
        Ok(())
    }
}
#[exception]
unsafe fn HardFault(_: &cortex_m_rt::ExceptionFrame) -> ! {
    loader(0xe003);
}

#[entry]
fn main() -> ! {
    board::w32(0x200f0fc4, 10);
    board::init();
    board::w32(0x200f0fc4, 20);
    static OUT: static_cell::StaticCell<[u8; 256]> = static_cell::StaticCell::new();
    let driver = usb::initialize(OUT.init([0; 256]));
    board::w32(0x200f0fc4, 30);
    static EXECUTOR: static_cell::StaticCell<embassy_executor::Executor> =
        static_cell::StaticCell::new();
    unsafe {
        cortex_m::interrupt::enable();
    }
    EXECUTOR.init(embassy_executor::Executor::new()).run(|s| {
        s.spawn(usb::task(driver).unwrap());
        s.spawn(game::task().unwrap());
    })
}
