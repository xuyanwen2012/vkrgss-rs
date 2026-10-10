//! The raw input a game reads through `Win32API`: Windows virtual-key codes for the physical
//! keys and mouse buttons, and the pointer position in the frame.

use vkxp_proto::{InputSnapshot, Size};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

/// The Windows virtual-key code of a physical key, by the key's place on a US keyboard, or
/// `None` for a key Windows has no code for here.
pub fn virtual_key(key: KeyCode) -> Option<u8> {
    use KeyCode as K;
    let ranges: [(&[KeyCode], u8); 4] = [
        (
            &[
                K::KeyA,
                K::KeyB,
                K::KeyC,
                K::KeyD,
                K::KeyE,
                K::KeyF,
                K::KeyG,
                K::KeyH,
                K::KeyI,
                K::KeyJ,
                K::KeyK,
                K::KeyL,
                K::KeyM,
                K::KeyN,
                K::KeyO,
                K::KeyP,
                K::KeyQ,
                K::KeyR,
                K::KeyS,
                K::KeyT,
                K::KeyU,
                K::KeyV,
                K::KeyW,
                K::KeyX,
                K::KeyY,
                K::KeyZ,
            ],
            0x41,
        ),
        (
            &[
                K::Digit0,
                K::Digit1,
                K::Digit2,
                K::Digit3,
                K::Digit4,
                K::Digit5,
                K::Digit6,
                K::Digit7,
                K::Digit8,
                K::Digit9,
            ],
            0x30,
        ),
        (
            &[
                K::Numpad0,
                K::Numpad1,
                K::Numpad2,
                K::Numpad3,
                K::Numpad4,
                K::Numpad5,
                K::Numpad6,
                K::Numpad7,
                K::Numpad8,
                K::Numpad9,
                K::NumpadMultiply,
                K::NumpadAdd,
                // The separator key, 0x6C.
                K::NumpadComma,
                K::NumpadSubtract,
                K::NumpadDecimal,
                K::NumpadDivide,
            ],
            0x60,
        ),
        (
            &[
                K::F1,
                K::F2,
                K::F3,
                K::F4,
                K::F5,
                K::F6,
                K::F7,
                K::F8,
                K::F9,
                K::F10,
                K::F11,
                K::F12,
                K::F13,
                K::F14,
                K::F15,
                K::F16,
                K::F17,
                K::F18,
                K::F19,
                K::F20,
                K::F21,
                K::F22,
                K::F23,
                K::F24,
            ],
            0x70,
        ),
    ];
    for (keys, first) in ranges {
        if let Some(index) = keys.iter().position(|in_range| *in_range == key) {
            return Some(first + index as u8);
        }
    }
    Some(match key {
        K::Backspace => 0x08,
        K::Tab => 0x09,
        K::Enter | K::NumpadEnter => 0x0D,
        K::Pause => 0x13,
        K::CapsLock => 0x14,
        K::Escape => 0x1B,
        K::Space => 0x20,
        K::PageUp => 0x21,
        K::PageDown => 0x22,
        K::End => 0x23,
        K::Home => 0x24,
        K::ArrowLeft => 0x25,
        K::ArrowUp => 0x26,
        K::ArrowRight => 0x27,
        K::ArrowDown => 0x28,
        K::PrintScreen => 0x2C,
        K::Insert => 0x2D,
        K::Delete => 0x2E,
        K::SuperLeft => 0x5B,
        K::SuperRight => 0x5C,
        K::ContextMenu => 0x5D,
        K::NumLock => 0x90,
        K::ScrollLock => 0x91,
        K::ShiftLeft => 0xA0,
        K::ShiftRight => 0xA1,
        K::ControlLeft => 0xA2,
        K::ControlRight => 0xA3,
        K::AltLeft => 0xA4,
        K::AltRight => 0xA5,
        K::Semicolon => 0xBA,
        K::Equal => 0xBB,
        K::Comma => 0xBC,
        K::Minus => 0xBD,
        K::Period => 0xBE,
        K::Slash => 0xBF,
        K::Backquote => 0xC0,
        K::BracketLeft => 0xDB,
        K::Backslash => 0xDC,
        K::BracketRight => 0xDD,
        K::Quote => 0xDE,
        K::IntlBackslash => 0xE2,
        _ => return None,
    })
}

/// The Windows virtual-key code of a mouse button.
pub fn mouse_key(button: MouseButton) -> Option<u8> {
    match button {
        MouseButton::Left => Some(0x01),
        MouseButton::Right => Some(0x02),
        MouseButton::Middle => Some(0x04),
        MouseButton::Back => Some(0x05),
        MouseButton::Forward => Some(0x06),
        MouseButton::Other(_) => None,
    }
}

/// `snapshot` with the virtual key `key` held and, for the left or right one of Shift,
/// Control and Alt, the code of the modifier whichever side it is on.
#[must_use]
pub(crate) fn with_virtual_key(snapshot: InputSnapshot, key: u8) -> InputSnapshot {
    let snapshot = snapshot.with_key(key);
    match key {
        0xA0 | 0xA1 => snapshot.with_key(0x10),
        0xA2 | 0xA3 => snapshot.with_key(0x11),
        0xA4 | 0xA5 => snapshot.with_key(0x12),
        _ => snapshot,
    }
}

/// `snapshot` with the virtual keys of the physical keys `keys` and the mouse buttons
/// `buttons` held.
#[must_use]
pub(crate) fn with_raw_keys(
    snapshot: InputSnapshot,
    keys: impl IntoIterator<Item = KeyCode>,
    buttons: impl IntoIterator<Item = MouseButton>,
) -> InputSnapshot {
    let keys = keys.into_iter().filter_map(virtual_key);
    let buttons = buttons.into_iter().filter_map(mouse_key);
    keys.chain(buttons).fold(snapshot, with_virtual_key)
}

/// The pixel of a frame of `frame` pixels that the position (`x`, `y`) of a window of `window`
/// pixels shows: the inverse of the letterbox mapping. `None` when the position is on a bar
/// or outside the window.
pub fn frame_pointer(frame: Size, window: Size, x: f64, y: f64) -> Option<(i16, i16)> {
    let rect = vkxp_vk::letterbox(frame, window);
    if rect.is_empty() || !x.is_finite() || !y.is_finite() {
        return None;
    }
    let along = |position: f64, origin: i32, shown: u32, pixels: u32| {
        let offset = position - f64::from(origin);
        if offset < 0.0 || offset >= f64::from(shown) {
            return None;
        }
        // The fraction of a window pixel counts until the position is in frame pixels. The
        // offset is below `shown`, so the pixel is below `pixels` but for rounding.
        let pixel = (offset * f64::from(pixels) / f64::from(shown)).floor() as u64;
        i16::try_from(pixel.min(u64::from(pixels) - 1)).ok()
    };
    Some((
        along(x, rect.x, rect.width, frame.width)?,
        along(y, rect.y, rect.height, frame.height)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(keys: &[u8]) -> InputSnapshot {
        keys.iter().fold(InputSnapshot::EMPTY, |snapshot, key| {
            snapshot.with_key(*key)
        })
    }

    #[test]
    fn keys_have_their_windows_codes() {
        use KeyCode as K;
        for (key, code) in [
            (K::KeyA, 0x41),
            (K::KeyZ, 0x5A),
            (K::Digit0, 0x30),
            (K::Digit9, 0x39),
            (K::Numpad0, 0x60),
            (K::Numpad9, 0x69),
            (K::NumpadMultiply, 0x6A),
            (K::NumpadAdd, 0x6B),
            (K::NumpadSubtract, 0x6D),
            (K::NumpadDecimal, 0x6E),
            (K::NumpadDivide, 0x6F),
            (K::F1, 0x70),
            (K::F12, 0x7B),
            (K::ArrowLeft, 0x25),
            (K::ArrowUp, 0x26),
            (K::ArrowRight, 0x27),
            (K::ArrowDown, 0x28),
            (K::Enter, 0x0D),
            (K::NumpadEnter, 0x0D),
            (K::Escape, 0x1B),
            (K::Space, 0x20),
            (K::ShiftLeft, 0xA0),
            (K::ShiftRight, 0xA1),
            (K::ControlLeft, 0xA2),
            (K::ControlRight, 0xA3),
            (K::AltLeft, 0xA4),
            (K::AltRight, 0xA5),
        ] {
            assert_eq!(virtual_key(key), Some(code), "{key:?}");
        }
        assert_eq!(virtual_key(K::Fn), None);
    }

    #[test]
    fn mouse_buttons_have_their_windows_codes() {
        assert_eq!(mouse_key(MouseButton::Left), Some(0x01));
        assert_eq!(mouse_key(MouseButton::Right), Some(0x02));
        assert_eq!(mouse_key(MouseButton::Middle), Some(0x04));
        assert_eq!(mouse_key(MouseButton::Other(9)), None);
    }

    /// Scenario "Letter and modifier".
    #[test]
    fn a_letter_and_left_shift_hold_their_codes_and_the_shift_of_either_side() {
        let snapshot = with_raw_keys(
            InputSnapshot::EMPTY,
            [KeyCode::KeyA, KeyCode::ShiftLeft],
            [],
        );
        assert_eq!(snapshot, held(&[0x41, 0x10, 0xA0]));
    }

    #[test]
    fn modifiers_of_either_side_hold_the_common_code() {
        let raw = |key| with_raw_keys(InputSnapshot::EMPTY, [key], []);
        assert_eq!(raw(KeyCode::ShiftRight), held(&[0x10, 0xA1]));
        assert_eq!(raw(KeyCode::ControlLeft), held(&[0x11, 0xA2]));
        assert_eq!(raw(KeyCode::ControlRight), held(&[0x11, 0xA3]));
        assert_eq!(raw(KeyCode::AltLeft), held(&[0x12, 0xA4]));
        assert_eq!(raw(KeyCode::AltRight), held(&[0x12, 0xA5]));
    }

    #[test]
    fn mouse_buttons_are_held_with_the_keys_and_leave_the_rest_of_a_snapshot() {
        let before = InputSnapshot::EMPTY
            .with(vkxp_proto::Button::C)
            .with_pointer(Some((3, 4)));
        let snapshot = with_raw_keys(before, [KeyCode::Fn, KeyCode::Space], [MouseButton::Left]);
        assert_eq!(snapshot, before.with_key(0x20).with_key(0x01));
    }

    /// Scenario "Pointer in a letterboxed window".
    #[test]
    fn the_pointer_maps_through_the_letterbox_and_is_none_on_a_bar() {
        let frame = Size::new(640, 480);
        let window = Size::new(1480, 960);
        assert_eq!(frame_pointer(frame, window, 300.0, 200.0), Some((100, 100)));
        assert_eq!(frame_pointer(frame, window, 50.0, 200.0), None);

        assert_eq!(frame_pointer(frame, window, 100.0, 0.0), Some((0, 0)));
        assert_eq!(frame_pointer(frame, window, 99.9, 0.0), None);
        assert_eq!(
            frame_pointer(frame, window, 1379.5, 959.5),
            Some((639, 479))
        );
        assert_eq!(frame_pointer(frame, window, 1380.0, 959.0), None);
        assert_eq!(frame_pointer(frame, window, 300.0, 960.0), None);
        assert_eq!(frame_pointer(frame, window, 300.0, -1.0), None);
    }

    #[test]
    fn a_fraction_of_a_window_pixel_counts_in_the_frame() {
        let frame = Size::new(640, 480);
        // Enlarged by one and a half: 1.75 window pixels are 1.17 frame pixels.
        let enlarged = Size::new(960, 720);
        assert_eq!(frame_pointer(frame, enlarged, 1.75, 1.75), Some((1, 1)));
        assert_eq!(frame_pointer(frame, enlarged, 1.25, 1.25), Some((0, 0)));
        // Reduced to a half: 100.75 window pixels are 201.5 frame pixels.
        let reduced = Size::new(320, 240);
        assert_eq!(
            frame_pointer(frame, reduced, 100.75, 50.75),
            Some((201, 101))
        );
        assert_eq!(
            frame_pointer(frame, reduced, 319.99, 239.99),
            Some((639, 479))
        );
        assert_eq!(frame_pointer(frame, reduced, 320.0, 239.0), None);
    }

    #[test]
    fn without_a_picture_there_is_no_pointer() {
        let frame = Size::new(640, 480);
        assert_eq!(frame_pointer(frame, Size::new(0, 0), 0.0, 0.0), None);
        assert_eq!(frame_pointer(Size::new(0, 0), frame, 10.0, 10.0), None);
        assert_eq!(frame_pointer(frame, frame, f64::NAN, 10.0), None);
        assert_eq!(frame_pointer(frame, frame, 10.0, 10.0), Some((10, 10)));
    }
}
