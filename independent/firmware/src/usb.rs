// SPDX-License-Identifier: MIT
//! DOOM-only WinUSB bulk interface. No measurement/PD RPC is linked.
use crate::board::*;
use apdo_protocol as wire;
use embassy_usb::{
    Builder, Config,
    driver::{Endpoint, EndpointIn, EndpointOut},
    msos,
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
    let mut config = Config::new(0x5fc9, 0x0063);
    config.manufacturer = Some("APDO project");
    config.product = Some("APDO DOOM");
    config.serial_number = None;
    config.max_power = 100;
    config.device_class = 0xff;
    config.device_sub_class = 0;
    config.device_protocol = 0;
    config.composite_with_iads = false;
    let (mut cfg, mut bos, mut os, mut control) = ([0; 256], [0; 256], [0; 256], [0; 64]);
    let mut builder = Builder::new(driver, config, &mut cfg, &mut bos, &mut os, &mut control);
    builder.msos_descriptor(msos::windows_version::WIN8_1, 0x20);
    builder.msos_feature(msos::CompatibleIdFeatureDescriptor::new("WINUSB", ""));
    builder.msos_feature(msos::RegistryPropertyFeatureDescriptor::new(
        "DeviceInterfaceGUIDs",
        msos::PropertyData::RegMultiSz(&["{C7EE9A08-F519-4D3B-A0D9-308134A4A5EA}"]),
    ));
    let mut function = builder.function(0xff, 0, 0);
    let mut interface = function.interface();
    let mut alt = interface.alt_setting(0xff, 0, 0, None);
    let mut out = alt.endpoint_bulk_out(None, 64);
    let mut input = alt.endpoint_bulk_in(None, 64);
    drop(function);
    let mut usb = builder.build();
    let server = async {
        loop {
            out.wait_enabled().await;
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
            if input.write(&header).await.is_err() {
                continue;
            }
            for chunk in data[..len].chunks(64) {
                if input.write(chunk).await.is_err() {
                    break;
                }
            }
            // End a short transfer even when its last data packet is full.
            // In particular the final 128-byte preview chunk would otherwise
            // leave a host's 1024-byte read waiting for more bytes.
            if len > 0 && len.is_multiple_of(64) {
                let _ = input.write(&[]).await;
            }
            if op == wire::LOADER {
                crate::loader(0x600d1003);
            }
        }
    };
    embassy_futures::join::join(usb.run(), server).await;
}
