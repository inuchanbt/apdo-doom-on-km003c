// SPDX-License-Identifier: MIT
//! Shared generic HID identity; stock application/DFU identities are separate.
pub const VID: u16 = 0x16c0;
pub const PID: u16 = 0x05df;
pub const MANUFACTURER: &str = "usb.pd.epr.240w@gmail.com";
pub const PRODUCT: &str = "KM003C DOOM";
pub const USAGE_PAGE: u16 = 0xffad;
pub const USAGE: u16 = 1;
pub const REPORT_BYTES: usize = 64;
// Unnumbered, vendor-defined 64-byte input/output reports; no keyboard or mouse.
pub const REPORT_DESCRIPTOR: &[u8] = &[
    0x06, 0xad, 0xff, 0x09, 0x01, 0xa1, 0x01, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x40,
    0x09, 0x02, 0x81, 0x02, 0x95, 0x40, 0x09, 0x03, 0x91, 0x02, 0xc0,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transport {
    Hid,
    LegacyBulk,
}

/// Compare entire strings, as required by the shared-ID terms. Legacy support
/// is host-only, so existing installations can still enter their native loader.
pub fn identify(
    vid: u16,
    pid: u16,
    manufacturer: Option<&str>,
    product: Option<&str>,
) -> Option<Transport> {
    match (vid, pid, manufacturer, product) {
        (VID, PID, Some(MANUFACTURER), Some(PRODUCT)) => Some(Transport::Hid),
        (0x5fc9, 0x0063, Some("APDO project"), Some("APDO DOOM")) => Some(Transport::LegacyBulk),
        _ => None,
    }
}

/// The first report contains the 16-byte APDO header and up to 48 data bytes.
/// Continuations contain up to 64 data bytes. Padding is never application data.
pub fn reply_report(header: &[u8; 16], data: &[u8], index: usize) -> Option<[u8; REPORT_BYTES]> {
    if super::reply_len(header)? != data.len() {
        return None;
    }
    let mut report = [0; REPORT_BYTES];
    if index == 0 {
        report[..16].copy_from_slice(header);
        let n = data.len().min(48);
        report[16..16 + n].copy_from_slice(&data[..n]);
    } else {
        let offset = index
            .checked_sub(1)?
            .checked_mul(REPORT_BYTES)?
            .checked_add(48)?;
        let remaining = data.get(offset..)?;
        if remaining.is_empty() {
            return None;
        }
        let n = remaining.len().min(REPORT_BYTES);
        report[..n].copy_from_slice(&remaining[..n]);
    }
    Some(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_id_requires_both_complete_strings() {
        assert_eq!(
            identify(VID, PID, Some(MANUFACTURER), Some(PRODUCT)),
            Some(Transport::Hid)
        );
        for (manufacturer, product) in [
            (None, Some(PRODUCT)),
            (Some(MANUFACTURER), None),
            (Some("usb.pd.epr.240w@gmail.com extra"), Some(PRODUCT)),
            (Some(MANUFACTURER), Some("KM003C DOOM extra")),
            (Some(MANUFACTURER), Some("T3P DOOM V66")),
            (Some("other@example.com"), Some(PRODUCT)),
        ] {
            assert_eq!(identify(VID, PID, manufacturer, product), None);
        }
        assert_eq!(
            identify(0x5fc9, 0x63, Some("APDO project"), Some("APDO DOOM")),
            Some(Transport::LegacyBulk)
        );
        assert_eq!(
            identify(0x5fc9, 0x63, Some("ChargerLAB"), Some("POWER-Z KM003C")),
            None
        );
        assert_eq!(
            identify(0x5fc9, 0x63, Some(MANUFACTURER), Some(PRODUCT)),
            None
        );
    }

    #[test]
    fn hid_reports_preserve_every_payload_boundary() {
        for len in 0..=super::super::MAX_DATA {
            let data: std::vec::Vec<u8> = (0..len).map(|i| i as u8).collect();
            let header = super::super::reply_header(9, 255, 0, 12, len);
            let mut decoded = std::vec::Vec::new();
            for index in 0..18 {
                let Some(report) = reply_report(&header, &data, index) else {
                    break;
                };
                let start = if index == 0 {
                    assert_eq!(&report[..16], &header);
                    16
                } else {
                    0
                };
                let n = (len - decoded.len()).min(REPORT_BYTES - start);
                decoded.extend_from_slice(&report[start..start + n]);
                assert!(report[start + n..].iter().all(|&x| x == 0));
            }
            assert_eq!(decoded, data);
        }
        let header = super::super::reply_header(0, 0, 0, 0, 1);
        assert!(reply_report(&header, &[], 0).is_none());
        assert!(reply_report(&header, &[1], usize::MAX).is_none());
    }
}
