//! Optional, RAM-only game frame scheduling; independent of LCD orientation.
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

pub type PacingRequest = Option<bool>;

pub const FRAME_PERIOD_US: u64 = 4_000_000 / 35;

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, Schema)]
pub struct PacingReply {
    pub supported: bool,
    pub smooth: bool,
    pub period_us: u32,
}
