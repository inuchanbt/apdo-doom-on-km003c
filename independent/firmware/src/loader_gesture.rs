// SPDX-License-Identifier: MIT
#[derive(Default)]
pub struct LoaderGesture {
    since: Option<u64>,
}

impl LoaderGesture {
    pub const fn new() -> Self {
        Self { since: None }
    }

    pub fn update(&mut self, buttons: u32, now_ms: u64) -> bool {
        if buttons & 0x0f != 0x0f {
            self.since = None;
            return false;
        }
        let since = *self.since.get_or_insert(now_ms);
        now_ms.saturating_sub(since) >= 2000
    }
}

#[cfg(test)]
mod tests {
    use super::LoaderGesture;

    #[test]
    fn fire_and_use_do_not_request_loader() {
        let mut gesture = LoaderGesture::new();
        for buttons in [1, 3, 5, 9, 7, 11, 13] {
            assert!(!gesture.update(buttons, 0));
            assert!(!gesture.update(buttons, 10_000));
        }
    }

    #[test]
    fn all_buttons_must_remain_held() {
        let mut gesture = LoaderGesture::new();
        assert!(!gesture.update(15, 100));
        assert!(!gesture.update(15, 2099));
        assert!(!gesture.update(7, 2100));
        assert!(!gesture.update(15, 3000));
        assert!(gesture.update(15, 5000));
    }
}
