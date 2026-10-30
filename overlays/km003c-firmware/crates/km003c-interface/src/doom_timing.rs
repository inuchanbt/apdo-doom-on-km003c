//! Per-frame cycle counters. LCD cycles include the synchronous panel driver.
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, Schema)]
pub struct DoomTiming {
    pub frame: u32,
    pub hclk_hz: u32,
    pub total_cycles: u32,
    pub nor_cycles: u32,
    pub lcd_cycles: u32,
    pub capture_cycles: u32,
    pub nor_bytes: u32,
    pub nor_calls: u32,
    pub lcd_pixels: u32,
    pub arena_address: u32,
    pub view_width: u32,
    pub nor_read_hz: u32,
    pub verified_read_bytes: u32,
    pub interval_cycles: u32,
    pub interval_us: u32,
    pub pacing_enabled: u32,
    pub simulation_cycles: u32,
    pub gametic: u32,
    pub player_x: i32,
    pub player_y: i32,
    pub player_angle: u32,
    pub applied_buttons: u32,
    pub command_forward: i32,
    pub command_turn: i32,
    pub game_flags: u32,
}
