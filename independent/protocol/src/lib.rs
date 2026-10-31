// SPDX-License-Identifier: MIT
#![no_std]
pub const VERSION: u8 = 1;
pub const REQUEST_BYTES: usize = 64;
pub const MAX_DATA: usize = 1024;
pub const INFO: u8 = 0;
pub const INPUT: u8 = 1;
pub const BRIGHTNESS: u8 = 2;
pub const ROTATION: u8 = 3;
pub const PACING: u8 = 4;
pub const LOADER: u8 = 5;
pub const PAUSE: u8 = 6;
pub const PLAY: u8 = 7;
pub const SNAPSHOT: u8 = 8;
pub const SCREEN: u8 = 9;
pub const NOR_READ: u8 = 10;
pub const NOR_ERASE: u8 = 11;
pub const NOR_WRITE: u8 = 12;
pub const OK: u8 = 0;
pub const INVALID: u8 = 1;
pub const BUSY: u8 = 2;
pub const STALE: u8 = 3;
pub const PREVIEW_BYTES: usize = 120 * 120 * 2;
pub const BOOT_SPLASH_FLAG: u32 = 1 << 9;
pub const GAME_LIMIT: u32 = 0x300000;
pub const ERASE_CONFIRM: u32 = 0x4150444f;

pub fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
pub fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
pub fn valid(bytes: &[u8]) -> bool {
    bytes.len() >= 16 && &bytes[..4] == b"APDO" && bytes[4] == VERSION
}
pub fn request(op: u8, sequence: u8, a: u32, b: u32) -> [u8; REQUEST_BYTES] {
    let mut bytes = [0; REQUEST_BYTES];
    bytes[..4].copy_from_slice(b"APDO");
    bytes[4] = VERSION;
    bytes[5] = op;
    bytes[7] = sequence;
    put(&mut bytes, 8, a);
    put(&mut bytes, 12, b);
    bytes
}
pub fn reply_header(op: u8, sequence: u8, status: u8, a: u32, len: usize) -> [u8; 16] {
    assert!(len <= MAX_DATA);
    let mut header = [0; 16];
    header.copy_from_slice(&request(op, sequence, a, 0)[..16]);
    header[6] = status;
    header[12..14].copy_from_slice(&(len as u16).to_le_bytes());
    header
}
pub fn reply_len(bytes: &[u8; 16]) -> Option<usize> {
    let n = u16::from_le_bytes([bytes[12], bytes[13]]) as usize;
    (valid(bytes) && n <= MAX_DATA).then_some(n)
}
pub fn readable(address: u32, len: u32) -> bool {
    len <= MAX_DATA as u32 && address.checked_add(len).is_some_and(|end| end <= 0x400000)
}
pub fn writable(address: u32, len: u32) -> bool {
    len > 0
        && len <= 48
        && address
            .checked_add(len)
            .is_some_and(|end| end <= GAME_LIMIT)
        && address / 256 == (address + len - 1) / 256
}
pub fn erasable(address: u32) -> bool {
    address < GAME_LIMIT && address.is_multiple_of(4096)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges_reject_overflow_and_protected_tail() {
        assert!(!writable(u32::MAX, 2));
        assert!(!writable(GAME_LIMIT, 1));
        assert!(!writable(255, 2));
        assert!(!writable(0, 49));
        assert!(writable(GAME_LIMIT - 48, 48));
        assert!(erasable(GAME_LIMIT - 4096));
        assert!(!erasable(GAME_LIMIT));
        assert!(!erasable(1));
        assert!(!readable(u32::MAX, 2));
        assert!(readable(0x400000 - 1024, 1024));
    }
    #[test]
    fn header_is_bounded_and_versioned() {
        let mut h = reply_header(SCREEN, 255, OK, 123, 1024);
        assert_eq!(reply_len(&h), Some(1024));
        assert_eq!(word(&h, 8), 123);
        h[13] = 255;
        assert_eq!(reply_len(&h), None);
        h[4] = 0;
        assert!(!valid(&h));
    }
}
