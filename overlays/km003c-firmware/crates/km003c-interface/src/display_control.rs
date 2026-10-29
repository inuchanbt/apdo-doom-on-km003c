//! Volatile LCD settings; 100% is the previously verified backlight level 70.
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Schema)]
pub enum DisplayRequest {
    Status,
    Brightness(u8),
    Rotation(u8),
}
impl DisplayRequest {
    pub fn valid(self) -> bool {
        match self {
            Self::Status => true,
            Self::Brightness(percent) => (10..=100).contains(&percent),
            Self::Rotation(turns) => turns < 4,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Schema)]
pub struct DisplayReply {
    pub accepted: bool,
    pub brightness: u8,
    pub rotation: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_dark_overbright_and_invalid_orientations() {
        for value in [0, 9, 101, 255] {
            assert!(!DisplayRequest::Brightness(value).valid());
        }
        for value in [10, 70, 100] {
            assert!(DisplayRequest::Brightness(value).valid());
        }
        for value in 0..4 {
            assert!(DisplayRequest::Rotation(value).valid());
        }
        assert!(!DisplayRequest::Rotation(4).valid());
    }
}
