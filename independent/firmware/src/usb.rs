// SPDX-License-Identifier: MIT
//! DOOM-only generic HID interface. No measurement/PD RPC is linked.
use crate::board::*;
use apdo_protocol as wire;
use embassy_usb::{
    Builder, Config,
    class::hid::device::{Config as HidConfig, HidReaderWriter, State},
    class::hid::{HidBootProtocol, HidSubclass},
};
use embassy_usb_synopsys_otg::{Driver, OtgInstance, PhyType, StateStorage, otg_v1::Otg};
static STORAGE: StateStorage<6> =
    StateStorage::new(embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex::new());
pub unsafe extern "C" fn interrupt() {
    unsafe {
        embassy_usb_synopsys_otg::on_interrupt(
            Otg::from_ptr(0x400c0000 as *mut _),
            &STORAGE.as_state(),
        );
    }
}
fn wait(address: usize, mask: u32, set: bool) {
    for _ in 0..2_000_000 {
        if (r32(address) & mask != 0) == set {
            return;
        }
    }
    panic!("USB startup timeout");
}
pub fn initialize(buffer: &'static mut [u8]) -> Driver<'static> {
    w16(0x400543fe, 0xa501);
    w8(0x40054024, 0xa0);
    w16(0x400543fe, 0xa500);
    for n in [9, 11, 12] {
        pin(0, n, 0, 10);
    }
    w32(0x40048004, r32(0x40048004) & !0x100);
    let q = 0x400c0000;
    assert_eq!(r32(q + 0x40), 0x4f54330a);
    w32(q + 8, 0);
    w32(q + 24, 0);
    w32(q + 0x804, r32(q + 0x804) | 2);
    delay_us(12_000);
    w32(q + 12, r32(q + 12) | 64);
    delay_us(6_000);
    wait(q + 16, 1 << 31, true);
    w32(q + 16, 1);
    wait(q + 16, 1, false);
    w32(q + 12, (1 << 30) | 64 | (9 << 10));
    wait(q + 20, 1, false);
    w32(q + 0x38, r32(q + 0x38) | (1 << 19));
    w32(q + 0xe00, 0);
    unsafe {
        apdo_hardware::intc::register(0x113, apdo_hardware::intc::Line::new(0).unwrap(), 8)
            .unwrap();
        Driver::new(
            buffer,
            OtgInstance {
                regs: Otg::from_ptr(q as *mut _),
                state: STORAGE.as_state(),
                fifo_depth_words: 308,
                tx_fifo_count: 6,
                phy_type: PhyType::InternalFullSpeed,
                extra_rx_fifo_words: 32,
                calculate_trdt_fn: |_| 9,
            },
            Default::default(),
        )
    }
}
#[embassy_executor::task]
pub async fn task(driver: Driver<'static>) {
    let mut config = Config::new(wire::usb::VID, wire::usb::PID);
    config.manufacturer = Some(wire::usb::MANUFACTURER);
    config.product = Some(wire::usb::PRODUCT);
    config.serial_number = None;
    config.max_power = 100;
    config.device_class = 0;
    config.device_sub_class = 0;
    config.device_protocol = 0;
    config.composite_with_iads = false;
    let (mut cfg, mut bos, mut os, mut control) = ([0; 256], [0; 256], [0; 256], [0; 64]);
    let mut hid_state = State::new();
    let mut builder = Builder::new(driver, config, &mut cfg, &mut bos, &mut os, &mut control);
    let hid = HidReaderWriter::<_, 64, 64>::new(
        &mut builder,
        &mut hid_state,
        HidConfig {
            report_descriptor: wire::usb::REPORT_DESCRIPTOR,
            request_handler: None,
            poll_ms: 1,
            max_packet_size: 64,
            hid_subclass: HidSubclass::No,
            hid_boot_protocol: HidBootProtocol::None,
        },
    );
    let (mut out, mut input) = hid.split();
    let mut usb = builder.build();
    let server = async {
        loop {
            out.ready().await;
            let mut req = [0; 64];
            let Ok(n) = out.read(&mut req).await else {
                continue;
            };
            if n != 64 || !wire::valid(&req) {
                continue;
            }
            let op = req[5];
            let a = wire::word(&req, 8);
            let b = wire::word(&req, 12);
            let mut data = [0; 1024];
            let (mut status, mut value, mut len) = (wire::OK, 0, 0);
            match op {
                wire::INFO => {
                    for (i, v) in crate::game::info().iter().enumerate() {
                        wire::put(&mut data, i * 4, *v);
                    }
                    len = 32;
                }
                wire::INPUT if a <= 255 => crate::game::input(a as u8),
                wire::LOADER => {}
                wire::SNAPSHOT => match crate::game::snapshot() {
                    Some((id, frame)) => {
                        value = id;
                        wire::put(&mut data, 0, frame);
                        len = 4;
                    }
                    None => status = wire::BUSY,
                },
                wire::SCREEN if b != 0 => {
                    len = 1024.min(wire::PREVIEW_BYTES.saturating_sub(a as usize));
                    value = b;
                    if !crate::game::screen(b, a, &mut data[..len]) {
                        status = wire::STALE;
                        len = 0;
                    }
                }
                _ => {
                    (status, value, data, len) = crate::game::command(req).await;
                }
            }
            let header = wire::reply_header(op, req[7], status, value, len);
            let mut index = 0;
            let mut sent = true;
            while let Some(report) = wire::usb::reply_report(&header, &data[..len], index) {
                if input.write(&report).await.is_err() {
                    sent = false;
                    break;
                }
                index += 1;
            }
            if sent && op == wire::LOADER {
                crate::loader(0x600d1003);
            }
        }
    };
    embassy_futures::join::join(usb.run(), server).await;
}
