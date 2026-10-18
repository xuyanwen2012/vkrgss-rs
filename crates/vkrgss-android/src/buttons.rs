//! The on-screen buttons over a game: a direction pad, C, B, A, X, Y, Z, L, R, F3 and F4 (save
//! and load a state) and a menu button, drawn translucent at the sides of the picture, beside
//! the letterbox where the window is wider than the game, and clear of what the system's bars
//! and the display's cutout cover ([`Insets`]).
//!
//! Every finger on the screen holds what is under it; several fingers hold several buttons,
//! and a finger that slides from one button onto another lets go of the first. A finger on
//! the direction pad holds the direction it points to from the pad's centre, two at once on
//! the diagonals. The buttons of `Input` are held as the gamepads' are; F3 and F4 are the keys
//! the host maps to the states; the menu button asks the game to quit, as closing the window
//! does, when the finger lifts on it.
//!
//! The key events of a gamepad, which winit's Android backend gives as keys it cannot name
//! (the codes `KEYCODE_BUTTON_*` of Android), hold the buttons a gamepad holds on the desktop
//! ([`GamepadMap`]); the directional pad of a gamepad arrives as the arrow keys, which the
//! key map already maps. A gamepad's key, its directional pad included, hides the on-screen
//! buttons until the screen is touched again; the arrow keys cannot tell a gamepad from a
//! keyboard, so a keyboard's hide them too.

use std::path::PathBuf;

use vkrgss_host::{GamepadMap, Overlay, OverlayAction, PadButton, PadState};
use vkrgss_proto::{Button, InputSnapshot, Size};
use vkrgss_vk::{OverlayImage, letterbox};
use winit::event::{Touch, TouchPhase};
use winit::keyboard::{KeyCode, NativeKeyCode, PhysicalKey};

use crate::canvas::{Area, Canvas, Color, Insets, Text};

/// What a place on the screen does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Control {
    Button(Button),
    Key(KeyCode),
    Menu,
}

/// A button on the screen.
#[derive(Debug, Clone, PartialEq)]
struct Item {
    control: Control,
    label: &'static str,
    area: Area,
}

/// Where the buttons are in a window of one size.
#[derive(Debug, Clone, PartialEq)]
struct Layout {
    window: Size,
    insets: Insets,
    /// The unit of the layout in pixels: a ninth of the window's height.
    unit: f32,
    items: Vec<Item>,
    /// The centre of the direction pad.
    pad: (f32, f32),
}

/// The fill of a button and of the pad.
const FILL: Color = [255, 255, 255, 64];
/// The labels and the arrows of the pad.
const INK: Color = [255, 255, 255, 176];

impl Layout {
    /// The buttons for a game of `frame` pixels in a window of `window` pixels whose edges
    /// `insets` covers: each half of the controls is centred in its bar beside the picture,
    /// or, where the bar is too narrow, held at the window's edge, clear of the insets, over
    /// the picture.
    fn new(window: Size, frame: Size, insets: Insets) -> Layout {
        let unit = window.height as f32 / 9.0;
        let bar = letterbox(frame, window).x.max(0) as f32;
        let left = (bar / 2.0).max(insets.left + 1.6 * unit);
        let right = window.width as f32 - (bar / 2.0).max(insets.right + 1.6 * unit);
        let middle = window.height as f32 * 0.6;
        let top = insets.top + 0.6 * unit;
        let small = |x: f32, width: f32| Area::centred(x, top, width * unit, 0.7 * unit);
        let round = |dx: f32, dy: f32| {
            Area::centred(
                right + dx * unit,
                middle + dy * unit,
                0.86 * unit,
                0.86 * unit,
            )
        };
        let items = vec![
            Item {
                control: Control::Button(Button::L),
                label: "L",
                area: small(left - 0.7 * unit, 1.2),
            },
            Item {
                control: Control::Menu,
                label: "MENU",
                area: small(left + 0.7 * unit, 1.2),
            },
            Item {
                control: Control::Key(KeyCode::F3),
                label: "F3",
                area: small(right - 1.1 * unit, 1.0),
            },
            Item {
                control: Control::Key(KeyCode::F4),
                label: "F4",
                area: small(right, 1.0),
            },
            Item {
                control: Control::Button(Button::R),
                label: "R",
                area: small(right + 1.1 * unit, 1.0),
            },
            // Y X Z over A and B, with C below, as a pad's buttons lie.
            Item {
                control: Control::Button(Button::Y),
                label: "Y",
                area: round(-1.0, -1.0),
            },
            Item {
                control: Control::Button(Button::X),
                label: "X",
                area: round(0.0, -1.0),
            },
            Item {
                control: Control::Button(Button::Z),
                label: "Z",
                area: round(1.0, -1.0),
            },
            Item {
                control: Control::Button(Button::A),
                label: "A",
                area: round(-1.0, 0.0),
            },
            Item {
                control: Control::Button(Button::B),
                label: "B",
                area: round(1.0, 0.0),
            },
            Item {
                control: Control::Button(Button::C),
                label: "C",
                area: round(0.0, 1.0),
            },
        ];
        Layout {
            window,
            insets,
            unit,
            items,
            pad: (left, middle),
        }
    }

    /// What a finger that touches `(x, y)` holds: the item under it, a little beyond its
    /// drawn edge, or the directions of the pad.
    fn target(&self, x: f32, y: f32) -> Target {
        let margin = 0.1 * self.unit;
        let item = self.items.iter().find(|item| {
            Area::new(
                item.area.x - margin,
                item.area.y - margin,
                item.area.width + 2.0 * margin,
                item.area.height + 2.0 * margin,
            )
            .contains(x, y)
        });
        match item {
            Some(item) => Target::Item(item.control),
            None => match self.pad_reaches(x, y) {
                true => Target::Pad(self.directions(x, y)),
                false => Target::Nothing,
            },
        }
    }

    /// Whether the pad takes a finger at `(x, y)`.
    fn pad_reaches(&self, x: f32, y: f32) -> bool {
        let (dx, dy) = (x - self.pad.0, y - self.pad.1);
        (dx * dx + dy * dy).sqrt() <= 1.9 * self.unit
    }

    /// The directions a finger at `(x, y)` points to from the centre of the pad: one, or two
    /// on a diagonal, of the eight sectors around it; none near the centre.
    fn directions(&self, x: f32, y: f32) -> u32 {
        let (dx, dy) = (x - self.pad.0, y - self.pad.1);
        if (dx * dx + dy * dy).sqrt() < 0.25 * self.unit {
            return 0;
        }
        // The angle from the right, clockwise on the screen, in eighths of a turn.
        let sector = ((dy.atan2(dx).to_degrees() + 360.0 + 22.5) / 45.0) as u32 % 8;
        let buttons: &[Button] = match sector {
            0 => &[Button::Right],
            1 => &[Button::Right, Button::Down],
            2 => &[Button::Down],
            3 => &[Button::Down, Button::Left],
            4 => &[Button::Left],
            5 => &[Button::Left, Button::Up],
            6 => &[Button::Up],
            _ => &[Button::Up, Button::Right],
        };
        buttons
            .iter()
            .fold(InputSnapshot::EMPTY, |held, button| held.with(*button))
            .buttons
    }

    /// Draws the buttons.
    fn draw(&self, text: &mut Text) -> OverlayImage {
        let mut canvas = Canvas::new(self.window);
        let unit = self.unit;
        let (px, py) = self.pad;
        // The pad: a cross with an arrow on each arm.
        canvas.rounded(
            Area::centred(px, py, 2.9 * unit, 0.95 * unit),
            0.2 * unit,
            FILL,
        );
        canvas.rounded(
            Area::centred(px, py - 0.975 * unit, 0.95 * unit, 0.95 * unit),
            0.2 * unit,
            FILL,
        );
        canvas.rounded(
            Area::centred(px, py + 0.975 * unit, 0.95 * unit, 0.95 * unit),
            0.2 * unit,
            FILL,
        );
        let tip = 1.25 * unit;
        let (base, half) = (0.75 * unit, 0.25 * unit);
        for (sx, sy) in [(0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)] {
            // The tip on the axis of the arm, the base across it.
            let (cx, cy) = (px + sx * tip, py + sy * tip);
            let (bx, by) = (px + sx * base, py + sy * base);
            canvas.triangle(
                [
                    (cx, cy),
                    (bx + sy * half, by + sx * half),
                    (bx - sy * half, by - sx * half),
                ],
                INK,
            );
        }
        for item in &self.items {
            let radius = item.area.width.min(item.area.height) / 2.0;
            canvas.rounded(item.area, radius, FILL);
            let size = (item.area.height * 0.55) as u32;
            text.draw(
                &mut canvas,
                item.label,
                size,
                item.area.centre(),
                false,
                item.area.width,
                INK,
            );
        }
        canvas.into_image()
    }
}

/// What one finger holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Nothing,
    Item(Control),
    /// The buttons of the directions of the pad.
    Pad(u32),
}

/// The pad button of an Android key code of a gamepad, `KEYCODE_BUTTON_*`; `Some(None)` for
/// a gamepad's button that holds nothing on the desktop either.
fn pad_button(code: u32) -> Option<Option<PadButton>> {
    let button = match code {
        96 => PadButton::South,          // BUTTON_A
        97 => PadButton::East,           // BUTTON_B
        99 => PadButton::West,           // BUTTON_X
        100 => PadButton::North,         // BUTTON_Y
        102 => PadButton::LeftShoulder,  // BUTTON_L1
        103 => PadButton::RightShoulder, // BUTTON_R1
        104 => PadButton::LeftTrigger,   // BUTTON_L2
        105 => PadButton::RightTrigger,  // BUTTON_R2
        106 => PadButton::LeftStick,     // BUTTON_THUMBL
        107 => PadButton::RightStick,    // BUTTON_THUMBR
        108 => PadButton::Start,         // BUTTON_START
        109 => PadButton::Select,        // BUTTON_SELECT
        // BUTTON_C, BUTTON_Z, BUTTON_MODE and BUTTON_1 to BUTTON_16.
        98 | 101 | 110 | 188..=203 => return Some(None),
        _ => return None,
    };
    Some(Some(button))
}

/// The on-screen buttons of a game and the gamepads' keys; the overlay of the game's window.
pub struct TouchButtons {
    /// The size of the game's picture, which places the buttons beside it.
    frame: Size,
    layout: Option<Layout>,
    /// The fingers on the screen by their identifiers.
    fingers: Vec<(u64, Target)>,
    /// The buttons of the gamepads that are down.
    pad: Vec<PadButton>,
    pads: GamepadMap,
    /// Whether a gamepad's key hid the buttons.
    hidden: bool,
    /// What covers the window's edges now.
    insets: Box<dyn Fn() -> Insets>,
    /// What the overlay last held and pressed.
    held: u32,
    keys: Vec<KeyCode>,
    /// The window size, the visibility and the insets of the picture last given.
    shown: Option<(Size, bool, Insets)>,
    /// The picture drawn last and its layout.
    drawn: Option<(OverlayImage, Layout)>,
    fonts: Vec<PathBuf>,
    text: Option<Text>,
}

impl TouchButtons {
    /// The buttons for a game of `frame` pixels, labelled in the fonts of `fonts`, clear of
    /// what `insets` says covers the window's edges.
    pub fn new(
        frame: Size,
        fonts: Vec<PathBuf>,
        insets: impl Fn() -> Insets + 'static,
    ) -> TouchButtons {
        TouchButtons {
            frame,
            layout: None,
            fingers: Vec::new(),
            pad: Vec::new(),
            pads: GamepadMap::default(),
            hidden: false,
            insets: Box::new(insets),
            held: 0,
            keys: Vec::new(),
            shown: None,
            drawn: None,
            fonts,
            text: None,
        }
    }

    /// The layout for a window of `window` pixels, with the insets of the picture last given.
    fn layout(&mut self, window: Size) -> &Layout {
        let insets = self
            .shown
            .map_or_else(Insets::default, |(_, _, insets)| insets);
        if self
            .layout
            .as_ref()
            .is_none_or(|layout| layout.window != window || layout.insets != insets)
        {
            self.layout = Some(Layout::new(window, self.frame, insets));
        }
        self.layout.as_ref().expect("just made")
    }

    /// A finger `id` touched, moved to or left `(x, y)` of a window of `window` pixels.
    fn finger(
        &mut self,
        window: Size,
        id: u64,
        phase: TouchPhase,
        (x, y): (f32, f32),
    ) -> Vec<OverlayAction> {
        let mut actions = Vec::new();
        if self.hidden && phase == TouchPhase::Started {
            self.hidden = false;
        }
        let layout = self.layout(window);
        let target = layout.target(x, y);
        let on_pad = layout.pad_reaches(x, y) || {
            let (dx, dy) = (x - layout.pad.0, y - layout.pad.1);
            (dx * dx + dy * dy).sqrt() <= 2.6 * layout.unit
        };
        let directions = layout.directions(x, y);
        let known = self.fingers.iter().position(|(finger, _)| *finger == id);
        match phase {
            TouchPhase::Started => {
                if let Some(at) = known {
                    self.fingers.remove(at);
                }
                self.fingers.push((id, target));
            }
            TouchPhase::Moved => {
                if let Some(at) = known {
                    let next = match self.fingers[at].1 {
                        // A finger that started on the pad steers it a little beyond its edge.
                        Target::Pad(_) if on_pad && !matches!(target, Target::Item(_)) => {
                            Target::Pad(directions)
                        }
                        _ => target,
                    };
                    self.fingers[at].1 = next;
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if let Some(at) = known {
                    let (_, last) = self.fingers.remove(at);
                    if phase == TouchPhase::Ended
                        && last == Target::Item(Control::Menu)
                        && target == last
                    {
                        actions.push(OverlayAction::Quit);
                    }
                }
            }
        }
        self.update(&mut actions);
        actions
    }

    /// A key of the window went down or up.
    fn pad_key(&mut self, key: PhysicalKey, pressed: bool) -> Vec<OverlayAction> {
        let button = match key {
            PhysicalKey::Unidentified(NativeKeyCode::Android(code)) => pad_button(code),
            // The directional pad, which the key map holds.
            PhysicalKey::Code(
                KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight,
            ) => Some(None),
            _ => None,
        };
        let Some(button) = button else {
            return Vec::new();
        };
        let mut actions = Vec::new();
        self.hidden = true;
        // The fingers that were on the hidden buttons let go.
        self.fingers.clear();
        if let Some(button) = button {
            self.pad.retain(|held| *held != button);
            if pressed {
                self.pad.push(button);
            }
        }
        self.update(&mut actions);
        actions
    }

    /// Adds to `actions` what has changed of the buttons and the keys held.
    fn update(&mut self, actions: &mut Vec<OverlayAction>) {
        let mut held = self.pads.held(&[PadState {
            held: self.pad.clone(),
            stick: (0.0, 0.0),
        }]);
        let mut keys = Vec::new();
        for (_, target) in &self.fingers {
            match *target {
                Target::Item(Control::Button(button)) => held = held.with(button),
                Target::Item(Control::Key(key)) if !keys.contains(&key) => keys.push(key),
                Target::Pad(directions) => held.buttons |= directions,
                _ => {}
            }
        }
        for key in self.keys.iter().filter(|key| !keys.contains(key)) {
            actions.push(OverlayAction::Key(*key, false));
        }
        for key in keys.iter().filter(|key| !self.keys.contains(key)) {
            actions.push(OverlayAction::Key(*key, true));
        }
        self.keys = keys;
        if held.buttons != self.held {
            self.held = held.buttons;
            actions.push(OverlayAction::Hold(held));
        }
    }
}

impl Overlay for TouchButtons {
    fn touch(&mut self, window: Size, touch: &Touch) -> Vec<OverlayAction> {
        let position = (touch.location.x as f32, touch.location.y as f32);
        self.finger(window, touch.id, touch.phase, position)
    }

    fn key(&mut self, key: PhysicalKey, pressed: bool) -> Vec<OverlayAction> {
        self.pad_key(key, pressed)
    }

    fn changed(&self, window: Size) -> bool {
        self.shown != Some((window, !self.hidden, (self.insets)()))
    }

    fn picture(&mut self, window: Size) -> Option<OverlayImage> {
        let insets = (self.insets)();
        self.shown = Some((window, !self.hidden, insets));
        if self.hidden {
            return None;
        }
        let layout = self.layout(window).clone();
        if let Some((drawn, drawn_layout)) = &self.drawn
            && *drawn_layout == layout
        {
            return Some(drawn.clone());
        }
        let fonts = self.fonts.clone();
        let text = self.text.get_or_insert_with(|| Text::new(fonts));
        let image = layout.draw(text);
        // For the tests, which touch the pad with `adb shell input`.
        tracing::info!(
            "buttons: the pad's centre is at ({:.0}, {:.0}), its arms {:.0} pixels long",
            layout.pad.0,
            layout.pad.1,
            layout.unit
        );
        self.drawn = Some((image.clone(), layout));
        Some(image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Size = Size::new(2400, 1080);
    const FRAME: Size = Size::new(640, 480);

    fn buttons() -> TouchButtons {
        TouchButtons::new(FRAME, Vec::new(), Insets::default)
    }

    /// The centre of the item that does `control`.
    fn at(control: Control) -> (f32, f32) {
        let layout = Layout::new(WINDOW, FRAME, Insets::default());
        let item = layout
            .items
            .iter()
            .find(|item| item.control == control)
            .unwrap();
        item.area.centre()
    }

    fn held(buttons: &[Button]) -> OverlayAction {
        OverlayAction::Hold(
            buttons
                .iter()
                .fold(InputSnapshot::EMPTY, |held, button| held.with(*button)),
        )
    }

    #[test]
    fn the_buttons_lie_in_the_bars_beside_the_picture_and_apart() {
        let layout = Layout::new(WINDOW, FRAME, Insets::default());
        // The picture is 1440 pixels wide in the middle; the bars are 480 wide.
        for item in &layout.items {
            let right_edge = item.area.x + item.area.width;
            assert!(right_edge <= 480.0 || item.area.x >= 1920.0, "{item:?}");
        }
        for (index, item) in layout.items.iter().enumerate() {
            for other in &layout.items[index + 1..] {
                let apart = item.area.x + item.area.width <= other.area.x
                    || other.area.x + other.area.width <= item.area.x
                    || item.area.y + item.area.height <= other.area.y
                    || other.area.y + other.area.height <= item.area.y;
                assert!(apart, "{item:?} and {other:?}");
            }
        }
        assert!(layout.pad.0 - 1.9 * layout.unit >= 0.0);
    }

    /// Scenario "Buttons move the player", for the mapping: the down arm of the pad holds
    /// down while the finger is on it.
    #[test]
    fn a_finger_on_the_pad_holds_its_direction_until_it_lifts() {
        let mut buttons = buttons();
        let layout = Layout::new(WINDOW, FRAME, Insets::default());
        let (x, y) = (layout.pad.0, layout.pad.1 + layout.unit);
        assert_eq!(
            buttons.finger(WINDOW, 3, TouchPhase::Started, (x, y)),
            [held(&[Button::Down])]
        );
        assert_eq!(
            buttons.finger(WINDOW, 3, TouchPhase::Moved, (x + 2.0, y)),
            []
        );
        // To the lower right: a diagonal.
        assert_eq!(
            buttons.finger(WINDOW, 3, TouchPhase::Moved, (x + layout.unit, y)),
            [held(&[Button::Down, Button::Right])]
        );
        assert_eq!(
            buttons.finger(WINDOW, 3, TouchPhase::Ended, (x + layout.unit, y)),
            [held(&[])]
        );
    }

    #[test]
    fn several_fingers_hold_several_buttons_and_a_sliding_finger_changes_button() {
        let mut buttons = buttons();
        let layout = Layout::new(WINDOW, FRAME, Insets::default());
        let up = (layout.pad.0, layout.pad.1 - layout.unit);
        buttons.finger(WINDOW, 1, TouchPhase::Started, up);
        assert_eq!(
            buttons.finger(
                WINDOW,
                2,
                TouchPhase::Started,
                at(Control::Button(Button::C))
            ),
            [held(&[Button::Up, Button::C])]
        );
        assert_eq!(
            buttons.finger(WINDOW, 2, TouchPhase::Moved, at(Control::Button(Button::B))),
            [held(&[Button::Up, Button::B])]
        );
        assert_eq!(
            buttons.finger(WINDOW, 2, TouchPhase::Moved, (1200.0, 500.0)),
            [held(&[Button::Up])]
        );
        assert_eq!(
            buttons.finger(WINDOW, 1, TouchPhase::Cancelled, up),
            [held(&[])]
        );
        // A touch on the picture holds nothing.
        assert_eq!(
            buttons.finger(WINDOW, 4, TouchPhase::Started, (1200.0, 500.0)),
            []
        );
    }

    #[test]
    fn f3_and_f4_press_their_keys_and_the_menu_quits_when_the_finger_lifts_on_it() {
        let mut buttons = buttons();
        let f3 = at(Control::Key(KeyCode::F3));
        assert_eq!(
            buttons.finger(WINDOW, 1, TouchPhase::Started, f3),
            [OverlayAction::Key(KeyCode::F3, true)]
        );
        assert_eq!(
            buttons.finger(WINDOW, 1, TouchPhase::Ended, f3),
            [OverlayAction::Key(KeyCode::F3, false)]
        );
        let menu = at(Control::Menu);
        assert_eq!(buttons.finger(WINDOW, 1, TouchPhase::Started, menu), []);
        assert_eq!(
            buttons.finger(WINDOW, 1, TouchPhase::Ended, menu),
            [OverlayAction::Quit]
        );
        // Slid off before lifting: nothing.
        buttons.finger(WINDOW, 1, TouchPhase::Started, menu);
        assert_eq!(
            buttons.finger(WINDOW, 1, TouchPhase::Ended, (1200.0, 500.0)),
            []
        );
    }

    /// Scenario "Gamepad hides the buttons".
    #[test]
    fn a_gamepad_key_holds_its_button_and_hides_the_buttons_until_a_touch() {
        let mut buttons = buttons();
        assert!(buttons.changed(WINDOW));
        assert!(buttons.picture(WINDOW).is_some());
        assert!(!buttons.changed(WINDOW));

        let a = PhysicalKey::Unidentified(NativeKeyCode::Android(96));
        assert_eq!(buttons.key(a, true), [held(&[Button::C])]);
        assert!(buttons.changed(WINDOW));
        assert_eq!(buttons.picture(WINDOW), None);
        assert_eq!(buttons.key(a, false), [held(&[])]);
        // Keys that are not a gamepad's change nothing.
        let keys = [
            PhysicalKey::Code(KeyCode::F3),
            PhysicalKey::Unidentified(NativeKeyCode::Android(4)),
        ];
        for key in keys {
            assert_eq!(buttons.key(key, true), []);
        }
        // The shoulder buttons hold L and R, as on the desktop.
        let l1 = PhysicalKey::Unidentified(NativeKeyCode::Android(102));
        assert_eq!(buttons.key(l1, true), [held(&[Button::L])]);
        buttons.key(l1, false);
        assert!(!buttons.changed(WINDOW), "still hidden");

        buttons.finger(WINDOW, 9, TouchPhase::Started, (1200.0, 500.0));
        assert!(buttons.changed(WINDOW));
        assert!(buttons.picture(WINDOW).is_some());
    }

    /// A gamepad's directional pad arrives as the arrow keys: they hide the buttons and hold
    /// nothing themselves, since the key map holds the directions.
    #[test]
    fn the_arrow_keys_of_a_directional_pad_hide_the_buttons_too() {
        let mut buttons = buttons();
        buttons.picture(WINDOW);
        let down = PhysicalKey::Code(KeyCode::ArrowDown);
        assert_eq!(buttons.key(down, true), []);
        assert!(buttons.changed(WINDOW));
        assert_eq!(buttons.picture(WINDOW), None);
        assert_eq!(buttons.key(down, false), []);

        // A finger that held the pad lets go when the buttons are hidden.
        buttons.finger(WINDOW, 1, TouchPhase::Started, (1200.0, 500.0));
        let layout = Layout::new(WINDOW, FRAME, Insets::default());
        let up = (layout.pad.0, layout.pad.1 - layout.unit);
        buttons.finger(WINDOW, 2, TouchPhase::Started, up);
        assert_eq!(
            buttons.key(PhysicalKey::Code(KeyCode::ArrowLeft), true),
            [held(&[])]
        );
        assert_eq!(buttons.picture(WINDOW), None);
    }

    /// A navigation bar at the right edge of a window whose bars are narrow.
    #[test]
    fn the_buttons_keep_clear_of_the_insets_and_follow_them() {
        let insets = std::rc::Rc::new(std::cell::Cell::new(Insets::default()));
        let given = insets.clone();
        let mut buttons = TouchButtons::new(FRAME, Vec::new(), move || given.get());
        let window = Size::new(1600, 900);
        buttons.picture(window);
        assert!(!buttons.changed(window));
        let bar = Insets {
            right: 130.0,
            ..Insets::default()
        };
        insets.set(bar);
        assert!(buttons.changed(window), "the bar moved without a resize");
        buttons.picture(window);
        let layout = Layout::new(window, FRAME, bar);
        for item in &layout.items {
            assert!(item.area.x + item.area.width <= 1600.0 - 130.0, "{item:?}");
        }
        // A touch finds the buttons where they are drawn.
        let c = layout
            .items
            .iter()
            .find(|item| item.control == Control::Button(Button::C))
            .unwrap()
            .area
            .centre();
        assert_eq!(
            buttons.finger(window, 1, TouchPhase::Started, c),
            [held(&[Button::C])]
        );
    }

    #[test]
    fn the_picture_is_drawn_for_the_window_and_redrawn_for_another_size() {
        let mut buttons = buttons();
        let picture = buttons.picture(WINDOW).unwrap();
        assert_eq!(picture.size, WINDOW);
        // Something is drawn in the left bar and nothing over the picture.
        let alpha = |image: &OverlayImage, (x, y): (f32, f32)| {
            image.pixels[((y as u32 * image.size.width + x as u32) * 4 + 3) as usize]
        };
        assert!(alpha(&picture, at(Control::Button(Button::C))) > 0);
        assert_eq!(alpha(&picture, (1200.0, 540.0)), 0);
        let smaller = Size::new(1200, 540);
        assert!(buttons.changed(smaller));
        assert_eq!(buttons.picture(smaller).unwrap().size, smaller);
    }
}
