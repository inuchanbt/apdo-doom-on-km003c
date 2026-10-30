//! LCD output shadow. Both callers run on the thread executor; no QSPI reads.
use core::cell::RefCell;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use km003c_interface::preview::{ScreenBuffers, ScreenChunk, ScreenInfo, ScreenRead};

static BUFFERS: Mutex<CriticalSectionRawMutex, RefCell<ScreenBuffers>> =
    Mutex::new(RefCell::new(ScreenBuffers::new()));
static ACTIVE: Mutex<CriticalSectionRawMutex, RefCell<u64>> = Mutex::new(RefCell::new(0));

pub fn layout(square: bool) {
    BUFFERS.lock(|buffers| buffers.borrow_mut().layout(square));
}
pub fn column(x: usize, y: usize, pixels: &[u16]) {
    BUFFERS.lock(|buffers| buffers.borrow_mut().column(x, y, pixels));
}
pub fn finished(frame: u32) {
    BUFFERS.lock(|buffers| buffers.borrow_mut().finished(frame));
}
fn touch() {
    ACTIVE.lock(|until| *until.borrow_mut() = embassy_time::Instant::now().as_millis() + 2000);
}
pub fn active() -> bool {
    ACTIVE.lock(|until| embassy_time::Instant::now().as_millis() < *until.borrow())
}
pub fn snapshot() -> ScreenInfo {
    touch();
    BUFFERS.lock(|buffers| buffers.borrow_mut().capture())
}
pub fn read(request: ScreenRead) -> ScreenChunk {
    touch();
    BUFFERS.lock(|buffers| buffers.borrow().read(request))
}
