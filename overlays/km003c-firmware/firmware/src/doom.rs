//! One owner serializes LCD and external NOR. USB requests never access QSPI.
use core::ptr::null_mut;
use embassy_hc32::gpio::{InputPin, Port};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, signal::Signal,
};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle,
    text::Text,
};
use km003c_interface::display_control::{DisplayReply, DisplayRequest};
use km003c_interface::{GameCommand, GameInputReply, GameInputRequest, GameReply, RemoteGameInput};
static DISPLAY_REQUEST: Channel<CriticalSectionRawMutex, DisplayRequest, 1> = Channel::new();
static DISPLAY_REPLY: Signal<CriticalSectionRawMutex, DisplayReply> = Signal::new();
static COMMANDS: Channel<CriticalSectionRawMutex, GameCommand, 1> = Channel::new();
static REPLY: Signal<CriticalSectionRawMutex, GameReply> = Signal::new();
static INPUTS: Channel<CriticalSectionRawMutex, GameInputRequest, 1> = Channel::new();
static INPUT_REPLY: Signal<CriticalSectionRawMutex, GameInputReply> = Signal::new();
static mut DISPLAY: *mut crate::display::Display = null_mut();
static mut KEYS: u32 = 0;
unsafe extern "C" {
    fn km_doom_init();
    fn km_doom_step();
    fn km_doom_menu() -> i32;
    fn km_doom_redraw();
}
pub async fn display_control(request: DisplayRequest) -> DisplayReply {
    DISPLAY_REQUEST.send(request).await;
    DISPLAY_REPLY.wait().await
}
pub async fn request(command: GameCommand) -> GameReply {
    COMMANDS.send(command).await;
    REPLY.wait().await
}
pub async fn input(state: GameInputRequest) -> GameInputReply {
    INPUTS.send(state).await;
    INPUT_REPLY.wait().await
}

#[unsafe(no_mangle)]
extern "C" fn km_millis() -> u32 {
    embassy_time::Instant::now().as_millis() as u32
}
#[unsafe(no_mangle)]
extern "C" fn km_sleep(ms: u32) {
    cortex_m::asm::delay(ms.saturating_mul(192000));
}
#[unsafe(no_mangle)]
extern "C" fn km_keys() -> u32 {
    unsafe { KEYS }
}
#[unsafe(no_mangle)]
extern "C" fn km_exit() -> ! {
    crate::return_to_loader(0x600d1000)
}
#[unsafe(no_mangle)]
unsafe extern "C" fn km_fail(message: *const core::ffi::c_char) -> ! {
    let message = unsafe { core::ffi::CStr::from_ptr(message) }
        .to_str()
        .unwrap_or("DOOM error");
    panic!("DOOM: {message}")
}
#[unsafe(no_mangle)]
unsafe extern "C" fn km_flash_read(address: u32, dst: *mut u8, count: u32) {
    unsafe {
        (&mut *DISPLAY).nor_read(
            address,
            core::slice::from_raw_parts_mut(dst, count as usize),
        );
    }
}
#[unsafe(no_mangle)]
unsafe extern "C" fn km_column(x: i32, y: i32, height: i32, pixels: *const u16) {
    // Map the engine's complete 320x240 picture to 240x180, centered vertically.
    if !(0..320).contains(&x) || x % 4 == 3 || y < 0 || height <= 0 || y + height > 240 {
        return;
    }
    let source = unsafe { core::slice::from_raw_parts(pixels, height as usize) };
    crate::capture::column(x as usize, y as usize, source);
    let mut out = [Rgb565::BLACK; 180];
    let mut count = 0;
    for (i, &c) in source.iter().enumerate() {
        let row = y + i as i32;
        if row % 4 != 3 {
            out[count] = Rgb565::new(
                ((c >> 11) & 31) as u8,
                ((c >> 5) & 63) as u8,
                (c & 31) as u8,
            );
            count += 1;
        }
    }
    let first = if y % 4 == 3 { y + 1 } else { y };
    if count > 0 {
        unsafe {
            (&mut *DISPLAY).blit(
                Rectangle::new(
                    Point::new(x - x / 4, 30 + first - first / 4),
                    Size::new(1, count as u32),
                ),
                &out[..count],
            );
        }
    }
}

#[embassy_executor::task]
pub async fn task(aw: crate::aw9523::Aw9523) {
    aw.lcd_reset().await.unwrap();
    aw.backlight(0).await.unwrap();
    static PANEL: static_cell::StaticCell<crate::display::Display> = static_cell::StaticCell::new();
    let display = PANEL.init(unsafe { crate::display::Display::new() });
    let _ = display.factory_settings();
    let jedec = display.nor_jedec();
    display.init().await;
    display.clear(Rgb565::BLACK).unwrap();
    Text::new(
        "KM003C / DOOM",
        Point::new(12, 20),
        MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE),
    )
    .draw(display)
    .unwrap();
    Text::new(
        "Waiting for verified game data",
        Point::new(12, 112),
        MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE),
    )
    .draw(display)
    .unwrap();
    Text::new(
        "Hold OK: service / EXIT 2s: loader",
        Point::new(12, 228),
        MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE),
    )
    .draw(display)
    .unwrap();
    aw.backlight(70).await.unwrap();
    unsafe {
        DISPLAY = display as *mut _;
    }
    let buttons = unsafe {
        [
            InputPin::new_pull_up(Port::B, 7),
            InputPin::new_pull_up(Port::B, 11),
            InputPin::new_pull_up(Port::B, 8),
            InputPin::new_pull_up(Port::B, 9),
        ]
    };
    let mut header = [0; 8];
    display.nor_read(0, &mut header);
    let mut auto_boot = &header == b"DMCDOOM\0";
    let boot_time = embassy_time::Instant::now();
    let mut initialized = false;
    let mut running = false;
    let mut frames = 0u32;
    let mut elapsed_ms = 0u32;
    let mut exit_since = None;
    let mut remote = RemoteGameInput::default();
    let mut brightness = 100u8;
    let mut rotation = 0u8;
    loop {
        if let Ok(request) = DISPLAY_REQUEST.try_receive() {
            let mut accepted = request.valid();
            if accepted {
                match request {
                    DisplayRequest::Brightness(percent) => {
                        // Never exceed the original, verified LED drive level.
                        accepted = aw
                            .backlight((u16::from(percent) * 70 / 100) as u8)
                            .await
                            .is_ok();
                        if accepted {
                            brightness = percent;
                        }
                    }
                    DisplayRequest::Rotation(turns) if turns != rotation => {
                        rotation = turns;
                        display.set_rotation(turns);
                        display.clear(Rgb565::BLACK).unwrap();
                        if initialized {
                            // Repaint the whole HUD, including unchanged digits.
                            unsafe {
                                km_doom_redraw();
                            }
                        }
                    }
                    _ => {}
                }
            }
            DISPLAY_REPLY.signal(DisplayReply {
                accepted,
                brightness,
                rotation,
            });
        }
        let mut mask = 0u32;
        for (i, b) in buttons.iter().enumerate() {
            if b.is_low() {
                mask |= 1 << i;
            }
        }
        if mask & 2 != 0 {
            auto_boot = false;
        }
        if mask & 1 != 0 {
            let since = exit_since.get_or_insert(embassy_time::Instant::now());
            if since.elapsed().as_millis() >= 2000 {
                crate::return_to_loader(0x600d1001);
            }
        } else {
            exit_since = None;
        }
        if let Ok(command) = COMMANDS.try_receive() {
            auto_boot = false;
            let mut reply = GameReply {
                code: 0,
                len: 0,
                data: [[0; 32]; 8],
                frames,
                elapsed_ms,
                jedec,
            };
            let permitted = jedec != 0 && jedec != 0xffffff && jedec & 255 == 0x16 && !running;
            match command {
                GameCommand::Read { address, len }
                    if len <= 256
                        && address
                            .checked_add(u32::from(len))
                            .is_some_and(|end| end <= 0x400000) =>
                {
                    let mut data = [0; 256];
                    display.nor_read(address, &mut data[..usize::from(len)]);
                    for (i, byte) in data.iter().enumerate() {
                        reply.data[i / 32][i % 32] = *byte;
                    }
                    reply.len = len;
                }
                GameCommand::Erase { address, unlock } if permitted && unlock == 0x444f4f4d => {
                    if display.nor_erase_game_sector(address).is_err() {
                        reply.code = 2;
                    }
                }
                GameCommand::Write { address, len, data } if permitted && len > 0 && len <= 256 => {
                    let mut bytes = [0; 256];
                    for i in 0..256 {
                        bytes[i] = data[i / 32][i % 32];
                    }
                    if display
                        .nor_program_game_page(address, &bytes[..usize::from(len)])
                        .is_err()
                    {
                        reply.code = 3;
                    }
                }
                GameCommand::Start if !initialized => {
                    display.clear(Rgb565::BLACK).unwrap();
                    unsafe {
                        km_doom_init();
                    }
                    initialized = true;
                    running = true;
                }
                GameCommand::Start if running => {}
                GameCommand::Pause => {
                    running = false;
                    remote.clear();
                }
                _ => reply.code = 1,
            }
            REPLY.signal(reply);
        }
        if auto_boot && boot_time.elapsed().as_millis() >= 3000 {
            auto_boot = false;
            display.clear(Rgb565::BLACK).unwrap();
            unsafe {
                km_doom_init();
            }
            initialized = true;
            running = true;
        }
        let now_ms = embassy_time::Instant::now().as_millis();
        let response = INPUTS.try_receive().ok().map(|request| {
            let GameInputRequest::Set(state) = request else {
                return true;
            };
            if running || state.buttons == 0 {
                remote.update(state, now_ms)
            } else {
                remote.clear();
                false
            }
        });
        let menu_active = running && unsafe { km_doom_menu() } != 0;
        let mut keys = mask;
        if menu_active {
            keys = ((mask >> 2) & 1) << 1
                | ((mask >> 3) & 1) << 7
                | ((mask >> 1) & 1) << 6
                | (mask & 1) << 5;
        } else if mask & 3 == 3 {
            keys = (keys & !3) | 32;
        } else if mask & 12 == 12 {
            keys = (keys & !12) | 16;
        }
        let remote_buttons = if running { remote.buttons(now_ms) } else { 0 };
        keys |= u32::from(remote_buttons);
        unsafe {
            KEYS = keys;
        }
        if let Some(accepted) = response {
            INPUT_REPLY.signal(GameInputReply {
                accepted,
                running,
                remote_buttons,
                applied_buttons: keys as u8,
                menu_active,
                frames,
                frame_ms: elapsed_ms,
            });
        }
        if running {
            let start = embassy_time::Instant::now();
            unsafe {
                km_doom_step();
            }
            elapsed_ms = start.elapsed().as_millis() as u32;
            frames = frames.wrapping_add(1);
            crate::capture::finished(frames);
        }
        // Leave USB a short burst window while a preview is being transferred.
        embassy_time::Timer::after_millis(if crate::capture::active() { 8 } else { 1 }).await;
    }
}
