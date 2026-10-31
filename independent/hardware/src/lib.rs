#![cfg_attr(not(test), no_std)]
pub mod clock_decode;
pub mod clocks;
pub mod intc;
#[cfg(feature = "time")]
pub mod time_driver_tmra;
