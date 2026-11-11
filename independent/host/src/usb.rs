// SPDX-License-Identifier: MIT
use anyhow::{Context, Result, ensure};
use apdo_protocol::{self as wire, usb as identity};
use km003c_backup::UsbPort;
use nusb::{
    MaybeFuture,
    io::{EndpointRead, EndpointWrite},
    transfer::{Bulk, In, Out},
};
use std::{
    io::{Read, Write},
    time::Duration,
};

pub(crate) struct Client {
    link: Link,
    sequence: u8,
    pub(crate) port: UsbPort,
}

enum Link {
    Hid(hidapi::HidDevice),
    // Host-only migration path for already installed firmware.
    LegacyBulk(Box<BulkLink>),
}

struct BulkLink {
    reader: EndpointRead<Bulk>,
    writer: EndpointWrite<Bulk>,
}

fn select_unique<T>(
    mut devices: Vec<(T, UsbPort)>,
    port: Option<&UsbPort>,
) -> Result<(T, UsbPort)> {
    ensure!(devices.len() <= 1, "Connect only one APDO KM003C device");
    let (device, selected) = devices
        .pop()
        .context("Connect a KM003C running the independent APDO firmware")?;
    ensure!(
        port.is_none_or(|p| *p == selected),
        "APDO is not on the physical port recorded in the qualified backup"
    );
    Ok((device, selected))
}

fn kind(info: &nusb::DeviceInfo) -> Result<Option<identity::Transport>> {
    // Windows does not cache iManufacturer. Read it through the parent USB hub;
    // do not assume manufacturer_string() is populated and do not claim WinUSB.
    if !matches!(
        (info.vendor_id(), info.product_id()),
        (identity::VID, identity::PID) | (0x5fc9, 0x63)
    ) {
        return Ok(None);
    }
    let manufacturer = info.get_manufacturer_string().wait()?;
    let product = info.get_product_string().wait()?;
    Ok(identity::identify(
        info.vendor_id(),
        info.product_id(),
        manufacturer.as_deref(),
        product.as_deref(),
    ))
}

impl Client {
    pub(crate) fn open() -> Result<Self> {
        Self::open_on(None)
    }

    pub(crate) fn open_on(port: Option<&UsbPort>) -> Result<Self> {
        let mut devices = Vec::new();
        for info in nusb::list_devices().wait()? {
            if let Some(kind) = kind(&info)? {
                let selected = UsbPort {
                    bus_id: info.bus_id().to_owned(),
                    port_chain: info.port_chain().to_vec(),
                };
                devices.push(((info, kind), selected));
            }
        }
        // HIDAPI has no portable bus/port API. Requiring exactly one matching
        // USB device AND one matching HID collection avoids guessing a mapping.
        let ((info, kind), selected) = select_unique(devices, port)?;
        let link = match kind {
            identity::Transport::Hid => {
                let api = hidapi::HidApi::new()?;
                let mut found = api.device_list().filter(|d| {
                    identity::identify(
                        d.vendor_id(),
                        d.product_id(),
                        d.manufacturer_string(),
                        d.product_string(),
                    ) == Some(identity::Transport::Hid)
                        && d.usage_page() == identity::USAGE_PAGE
                        && d.usage() == identity::USAGE
                });
                let hid = found
                    .next()
                    .context("Matching KM003C DOOM HID collection was not found")?;
                ensure!(
                    found.next().is_none(),
                    "Connect only one KM003C DOOM HID collection"
                );
                Link::Hid(hid.open_device(&api)?)
            }
            identity::Transport::LegacyBulk => {
                let device = info.open().wait()?;
                let interface = device.claim_interface(0).wait()?;
                Link::LegacyBulk(Box::new(BulkLink {
                    reader: interface
                        .endpoint::<Bulk, In>(0x81)?
                        .reader(1024)
                        .with_read_timeout(Duration::from_secs(10)),
                    writer: interface
                        .endpoint::<Bulk, Out>(0x01)?
                        .writer(64)
                        .with_write_timeout(Duration::from_secs(2)),
                }))
            }
        };
        Ok(Self {
            link,
            sequence: 0,
            port: selected,
        })
    }

    pub(crate) fn call(&mut self, op: u8, a: u32, b: u32) -> Result<(u32, Vec<u8>)> {
        self.exchange(wire::request(op, 0, a, b))
    }

    pub(crate) fn exchange(&mut self, mut req: [u8; 64]) -> Result<(u32, Vec<u8>)> {
        self.sequence = self.sequence.wrapping_add(1);
        req[7] = self.sequence;
        let (header, data) = match &mut self.link {
            Link::Hid(device) => hid_exchange(device, &req)?,
            Link::LegacyBulk(link) => {
                let BulkLink { reader, writer } = link.as_mut();
                writer.write_all(&req)?;
                writer.flush()?;
                let mut header = [0; 16];
                reader.read_exact(&mut header)?;
                let len = check_header(&header, &req)?;
                let mut data = vec![0; len];
                reader.read_exact(&mut data)?;
                (header, data)
            }
        };
        ensure!(
            header[6] == wire::OK,
            "Device rejected command {}: status {}",
            req[5],
            header[6]
        );
        Ok((wire::word(&header, 8), data))
    }

    pub(crate) fn info(&mut self) -> Result<[u32; 8]> {
        let (_, data) = self.call(wire::INFO, 0, 0)?;
        ensure!(data.len() == 32, "Invalid device status");
        Ok(core::array::from_fn(|i| wire::word(&data, i * 4)))
    }
}

fn check_header(header: &[u8; 16], req: &[u8; 64]) -> Result<usize> {
    let len = wire::reply_len(header).context("Invalid APDO reply header")?;
    ensure!(
        header[5] == req[5] && header[7] == req[7],
        "APDO reply sequence mismatch"
    );
    Ok(len)
}

trait Reports {
    fn write_request(&self, request: &[u8; 65]) -> Result<()>;
    fn read_report(&self) -> Result<[u8; 64]>;
}

impl Reports for hidapi::HidDevice {
    fn write_request(&self, request: &[u8; 65]) -> Result<()> {
        // hidapi 2.6.7's windows-native backend returns zero when WriteFile
        // succeeds synchronously (overlapped completion returns the byte count).
        // It still returns Err on failure; the APDO sequence is checked below.
        let n = self.write(request)?;
        ensure!(
            n == request.len() || cfg!(windows) && n == 0,
            "Short HID request write"
        );
        Ok(())
    }
    fn read_report(&self) -> Result<[u8; 64]> {
        let mut report = [0; 64];
        let n = self.read_timeout(&mut report, 10_000)?;
        ensure!(n == report.len(), "Short HID report or timeout: {n} bytes");
        Ok(report)
    }
}

fn hid_exchange(reports: &impl Reports, req: &[u8; 64]) -> Result<([u8; 16], Vec<u8>)> {
    let mut request = [0; 65]; // HIDAPI requires a leading zero for an unnumbered report.
    request[1..].copy_from_slice(req);
    reports.write_request(&request)?;
    let first = reports.read_report()?;
    let header = first[..16].try_into().unwrap();
    let len = check_header(&header, req)?;
    let mut data = Vec::with_capacity(len);
    data.extend_from_slice(&first[16..16 + len.min(48)]);
    while data.len() < len {
        let report = reports.read_report()?;
        let n = (len - data.len()).min(64);
        data.extend_from_slice(&report[..n]);
    }
    Ok((header, data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::VecDeque};
    struct Mock {
        request: RefCell<Vec<u8>>,
        reports: RefCell<VecDeque<[u8; 64]>>,
    }
    impl Reports for Mock {
        fn write_request(&self, request: &[u8; 65]) -> Result<()> {
            *self.request.borrow_mut() = request.to_vec();
            Ok(())
        }
        fn read_report(&self) -> Result<[u8; 64]> {
            self.reports
                .borrow_mut()
                .pop_front()
                .context("Missing HID report")
        }
    }
    fn mock(header: &[u8; 16], data: &[u8]) -> Mock {
        Mock {
            request: RefCell::new(Vec::new()),
            reports: RefCell::new(
                (0..18)
                    .map_while(|i| identity::reply_report(header, data, i))
                    .collect(),
            ),
        }
    }

    #[test]
    fn hid_round_trip_boundaries_and_report_id() {
        for len in [0, 1, 32, 48, 49, 64, 112, 128, 1023, 1024] {
            let req = wire::request(wire::SCREEN, 255, 0, 123);
            let data: Vec<u8> = (0..len).map(|i| i as u8).collect();
            let header = wire::reply_header(req[5], req[7], wire::OK, 123, len);
            let reports = mock(&header, &data);
            assert_eq!(hid_exchange(&reports, &req).unwrap(), (header, data));
            assert_eq!(reports.request.borrow()[0], 0);
            assert_eq!(&reports.request.borrow()[1..], &req);
            assert!(reports.reports.borrow().is_empty());
        }
    }
    #[test]
    fn hid_rejects_stale_malformed_and_missing_replies() {
        let req = wire::request(wire::SCREEN, 4, 0, 1);
        for header in [
            wire::reply_header(req[5], 3, 0, 0, 64),
            wire::reply_header(0, 4, 0, 0, 64),
        ] {
            assert!(hid_exchange(&mock(&header, &[0; 64]), &req).is_err());
        }
        let header = wire::reply_header(req[5], 4, 0, 0, 64);
        let truncated = mock(&header, &[0; 64]);
        truncated.reports.borrow_mut().pop_back();
        assert!(hid_exchange(&truncated, &req).is_err());
        let malformed = Mock {
            request: RefCell::new(Vec::new()),
            reports: RefCell::new([[0; 64]].into()),
        };
        assert!(hid_exchange(&malformed, &req).is_err());
    }

    #[test]
    fn qualified_port_and_unambiguous_selection_are_required() {
        let recorded = UsbPort {
            bus_id: "bus-A".into(),
            port_chain: vec![2, 3],
        };
        let other_port = UsbPort {
            port_chain: vec![2, 4],
            ..recorded.clone()
        };
        let other_bus = UsbPort {
            bus_id: "bus-B".into(),
            ..recorded.clone()
        };
        assert!(select_unique::<()>(vec![], Some(&recorded)).is_err());
        for other in [&other_port, &other_bus] {
            assert!(select_unique(vec![((), other.clone())], Some(&recorded)).is_err());
            // Even with one candidate on the recorded port, HID selection must
            // never guess between a second matching device elsewhere.
            assert!(
                select_unique(
                    vec![((), recorded.clone()), ((), other.clone())],
                    Some(&recorded)
                )
                .is_err()
            );
        }
        assert_eq!(
            select_unique(vec![(42, recorded.clone())], Some(&recorded)).unwrap(),
            (42, recorded.clone())
        );
        assert!(select_unique(vec![((), recorded)], None).is_ok());
    }
}
