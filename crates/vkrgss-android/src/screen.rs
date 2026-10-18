//! The library screen: the imported games by title, and the buttons "Import", "Import RTP" and
//! "Delete", drawn by the app itself as the on-screen buttons are ([`crate::canvas`]).
//!
//! A tap on a game selects it, a tap on the selected game starts it; the last one played is
//! listed first and selected when the screen opens. "Delete" asks for a second tap before it
//! deletes the selected game. A finger dragged up or down the list scrolls it. A line at the
//! bottom says what the app is doing or what went wrong. The screen keeps clear of what the
//! system's bars and the display's cutout cover ([`Insets`]).

use std::path::PathBuf;

use vkrgss_proto::Size;
use vkrgss_vk::OverlayImage;
use winit::event::TouchPhase;

use crate::canvas::{Area, Canvas, Color, Insets, Text};
use crate::library::Game;
use crate::unpack::Kind;

/// What a tap asks of the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Open the picker for a zip of this kind.
    Import(Kind),
    Play(Game),
    Delete(Game),
}

/// A button of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Import,
    ImportRtp,
    Delete,
}

const BACKGROUND: Color = [24, 26, 32, 255];
const BUTTON: Color = [70, 76, 92, 255];
const DISABLED: Color = [44, 47, 56, 255];
const SELECTED: Color = [52, 92, 160, 255];
const ROW: Color = [36, 39, 48, 255];
const INK: Color = [236, 238, 242, 255];
const FAINT: Color = [150, 156, 168, 255];

/// Where things are in a window of one size.
struct Layout {
    /// The part of the window that nothing covers, which holds everything but the background.
    inside: Area,
    unit: f32,
    buttons: [(Action, &'static str, Area); 3],
    /// The list, between the buttons and the status line.
    list: Area,
    row_height: f32,
}

impl Layout {
    fn new(window: Size, insets: Insets) -> Layout {
        let inside = insets.inside(window);
        let unit = inside.height / 10.0;
        let (left, top, width) = (inside.x, inside.y, inside.width);
        let button = |index: f32| {
            let w = 2.6 * unit;
            Area::new(
                left + width - (3.0 - index) * (w + 0.2 * unit),
                top + 0.2 * unit,
                w,
                0.9 * unit,
            )
        };
        Layout {
            inside,
            unit,
            buttons: [
                (Action::Import, "Import", button(0.0)),
                (Action::ImportRtp, "Import RTP", button(1.0)),
                (Action::Delete, "Delete", button(2.0)),
            ],
            list: Area::new(
                left + 0.3 * unit,
                top + 1.3 * unit,
                width - 0.6 * unit,
                7.6 * unit,
            ),
            row_height: 0.95 * unit,
        }
    }

    fn rows(&self) -> usize {
        (self.list.height / self.row_height) as usize
    }

    /// The area of row `index` of those shown.
    fn row(&self, index: usize) -> Area {
        Area::new(
            self.list.x,
            self.list.y + index as f32 * self.row_height,
            self.list.width,
            self.row_height - 0.1 * self.unit,
        )
    }
}

/// The finger that is on the screen: where it started, the first row then, and whether it
/// has moved far enough to scroll.
#[derive(Debug, Clone, Copy)]
struct Finger {
    id: u64,
    start: (f32, f32),
    first: usize,
    dragged: bool,
}

/// The state of the screen.
pub struct Screen {
    games: Vec<Game>,
    selected: Option<usize>,
    /// The first row shown.
    first: usize,
    /// Whether "Delete" waits for its second tap.
    confirming: bool,
    /// Whether an import runs; the buttons wait meanwhile.
    busy: bool,
    status: String,
    finger: Option<Finger>,
    insets: Insets,
    /// The window size of the picture last given, or `None` when something has changed since.
    shown: Option<Size>,
    fonts: Vec<PathBuf>,
    text: Option<Text>,
}

impl Screen {
    /// The screen for `games`, labelled in the fonts of `fonts`.
    pub fn new(games: Vec<Game>, fonts: Vec<PathBuf>) -> Screen {
        let mut screen = Screen {
            games: Vec::new(),
            selected: None,
            first: 0,
            confirming: false,
            busy: false,
            status: String::new(),
            finger: None,
            insets: Insets::default(),
            shown: None,
            fonts,
            text: None,
        };
        screen.set_games(games);
        screen
    }

    /// Shows `games`, the last one played first, and selects the first.
    pub fn set_games(&mut self, games: Vec<Game>) {
        self.selected = (!games.is_empty()).then_some(0);
        self.first = 0;
        self.games = games;
        self.confirming = false;
        self.shown = None;
        if self.status.is_empty() && self.games.is_empty() {
            self.status = "No game yet: import the zip of a game's folder.".to_owned();
        }
    }

    /// Sets the line at the bottom.
    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
        self.shown = None;
    }

    /// Sets what covers the edges of the window.
    pub fn set_insets(&mut self, insets: Insets) {
        if insets != self.insets {
            self.insets = insets;
            self.shown = None;
        }
    }

    /// Sets whether an import runs.
    pub fn set_busy(&mut self, busy: bool) {
        self.busy = busy;
        self.shown = None;
    }

    /// The games shown.
    pub fn games(&self) -> &[Game] {
        &self.games
    }

    /// A finger `id` touched, moved to or left `(x, y)` of a window of `window` pixels.
    pub fn touch(
        &mut self,
        window: Size,
        id: u64,
        phase: TouchPhase,
        (x, y): (f32, f32),
    ) -> Option<Request> {
        let layout = Layout::new(window, self.insets);
        match phase {
            TouchPhase::Started => {
                self.finger = Some(Finger {
                    id,
                    start: (x, y),
                    first: self.first,
                    dragged: false,
                });
                None
            }
            TouchPhase::Moved => {
                let finger = self.finger.as_mut().filter(|finger| finger.id == id)?;
                let moved = y - finger.start.1;
                if moved.abs() > layout.row_height / 3.0 {
                    finger.dragged = true;
                }
                if finger.dragged {
                    let rows = (moved / layout.row_height).round() as isize;
                    let last = self.games.len().saturating_sub(layout.rows());
                    let first = (finger.first as isize - rows).clamp(0, last as isize) as usize;
                    if first != self.first {
                        self.first = first;
                        self.shown = None;
                    }
                }
                None
            }
            TouchPhase::Ended => {
                let finger = self.finger.take().filter(|finger| finger.id == id)?;
                if finger.dragged {
                    return None;
                }
                self.tap(&layout, x, y)
            }
            TouchPhase::Cancelled => {
                self.finger = None;
                None
            }
        }
    }

    /// A tap at `(x, y)`.
    fn tap(&mut self, layout: &Layout, x: f32, y: f32) -> Option<Request> {
        let confirming = std::mem::take(&mut self.confirming);
        self.shown = None;
        let button = layout
            .buttons
            .iter()
            .find(|(_, _, area)| area.contains(x, y))
            .map(|(action, _, _)| *action);
        if let Some(action) = button {
            if self.busy {
                return None;
            }
            return match action {
                Action::Import => Some(Request::Import(Kind::Game)),
                Action::ImportRtp => Some(Request::Import(Kind::Rtp)),
                Action::Delete => {
                    let game = self.games.get(self.selected?)?.clone();
                    if confirming {
                        return Some(Request::Delete(game));
                    }
                    self.confirming = true;
                    self.status =
                        format!("Tap Delete again to delete {} and its saves.", game.title);
                    None
                }
            };
        }
        let row = (0..layout.rows())
            .find(|row| layout.row(*row).contains(x, y))
            .map(|row| row + self.first)
            .filter(|index| *index < self.games.len())?;
        if self.selected == Some(row) && !self.busy {
            return Some(Request::Play(self.games[row].clone()));
        }
        self.selected = Some(row);
        None
    }

    /// Whether the picture for a window of `window` pixels differs from the one last given.
    pub fn changed(&self, window: Size) -> bool {
        self.shown != Some(window)
    }

    /// Draws the screen for a window of `window` pixels.
    pub fn picture(&mut self, window: Size) -> OverlayImage {
        self.shown = Some(window);
        let layout = Layout::new(window, self.insets);
        let Area {
            x: left,
            y: top,
            width,
            ..
        } = layout.inside;
        let fonts = self.fonts.clone();
        let text = self.text.get_or_insert_with(|| Text::new(fonts));
        let mut canvas = Canvas::new(window);
        let unit = layout.unit;
        canvas.rounded(
            Area::new(0.0, 0.0, window.width as f32, window.height as f32),
            0.0,
            BACKGROUND,
        );
        let size = (0.5 * unit) as u32;
        text.draw(
            &mut canvas,
            "vkrgss",
            (0.6 * unit) as u32,
            (left + 0.5 * unit, top + 0.65 * unit),
            true,
            4.0 * unit,
            INK,
        );
        for (action, label, area) in layout.buttons {
            let enabled = !self.busy && (action != Action::Delete || self.selected.is_some());
            let label = match (action, self.confirming) {
                (Action::Delete, true) => "Delete?",
                _ => label,
            };
            canvas.rounded(area, 0.2 * unit, if enabled { BUTTON } else { DISABLED });
            let ink = if enabled { INK } else { FAINT };
            text.draw(
                &mut canvas,
                label,
                size,
                area.centre(),
                false,
                area.width,
                ink,
            );
        }
        for row in 0..layout.rows() {
            let index = row + self.first;
            let Some(game) = self.games.get(index) else {
                break;
            };
            let area = layout.row(row);
            let fill = if self.selected == Some(index) {
                SELECTED
            } else {
                ROW
            };
            canvas.rounded(area, 0.15 * unit, fill);
            let (_, y) = area.centre();
            text.draw(
                &mut canvas,
                &game.title,
                size,
                (area.x + 0.3 * unit, y),
                true,
                area.width - 0.6 * unit,
                INK,
            );
        }
        if self.games.len() > layout.rows() {
            let shown = format!(
                "{}–{} of {}",
                self.first + 1,
                (self.first + layout.rows()).min(self.games.len()),
                self.games.len()
            );
            text.draw(
                &mut canvas,
                &shown,
                (0.4 * unit) as u32,
                (left + width - 0.3 * unit - 3.0 * unit, top + 9.4 * unit),
                true,
                3.0 * unit,
                FAINT,
            );
        }
        text.draw(
            &mut canvas,
            &self.status,
            (0.42 * unit) as u32,
            (left + 0.4 * unit, top + 9.4 * unit),
            true,
            width - 4.0 * unit,
            FAINT,
        );
        canvas.into_image()
    }

    /// Where row `index` of the list lies in a window of `window` pixels while the list is
    /// scrolled to its top: its centre, for the log, so that a test can tap it.
    pub fn row_centre(&self, window: Size, index: usize) -> Option<(f32, f32)> {
        let layout = Layout::new(window, self.insets);
        (index < layout.rows()).then(|| layout.row(index).centre())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Size = Size::new(2400, 1080);

    fn game(id: u64, title: &str) -> Game {
        Game {
            id,
            dir: PathBuf::from(format!("/games/{id}")),
            title: title.to_owned(),
        }
    }

    fn tap(screen: &mut Screen, (x, y): (f32, f32)) -> Option<Request> {
        assert_eq!(screen.touch(WINDOW, 0, TouchPhase::Started, (x, y)), None);
        screen.touch(WINDOW, 0, TouchPhase::Ended, (x, y))
    }

    fn button(action: Action) -> (f32, f32) {
        let layout = Layout::new(WINDOW, Insets::default());
        let (_, _, area) = layout
            .buttons
            .iter()
            .find(|(known, _, _)| *known == action)
            .unwrap();
        area.centre()
    }

    /// Scenario "Import a zip", for the screen: the game is listed and starts when chosen.
    #[test]
    fn the_first_game_is_selected_and_a_tap_on_the_selected_game_starts_it() {
        let games = vec![game(2, "群战示范工程"), game(1, "Other")];
        let mut screen = Screen::new(games.clone(), Vec::new());
        assert_eq!(screen.games(), games);
        let first = screen.row_centre(WINDOW, 0).unwrap();
        let second = screen.row_centre(WINDOW, 1).unwrap();
        assert_eq!(
            tap(&mut screen, first),
            Some(Request::Play(games[0].clone()))
        );
        assert_eq!(tap(&mut screen, second), None, "selects");
        assert_eq!(
            tap(&mut screen, second),
            Some(Request::Play(games[1].clone()))
        );
        // Below the last game nothing happens.
        let empty = screen.row_centre(WINDOW, 4).unwrap();
        assert_eq!(tap(&mut screen, empty), None);
    }

    #[test]
    fn the_buttons_ask_for_imports_and_delete_asks_twice() {
        let games = vec![game(1, "A")];
        let mut screen = Screen::new(games.clone(), Vec::new());
        assert_eq!(
            tap(&mut screen, button(Action::Import)),
            Some(Request::Import(Kind::Game))
        );
        assert_eq!(
            tap(&mut screen, button(Action::ImportRtp)),
            Some(Request::Import(Kind::Rtp))
        );
        assert_eq!(tap(&mut screen, button(Action::Delete)), None);
        assert_eq!(
            tap(&mut screen, button(Action::Delete)),
            Some(Request::Delete(games[0].clone()))
        );
        // Another tap in between ends the question.
        assert_eq!(tap(&mut screen, button(Action::Delete)), None);
        tap(&mut screen, (10.0, 1070.0));
        assert_eq!(tap(&mut screen, button(Action::Delete)), None);

        screen.set_busy(true);
        assert_eq!(tap(&mut screen, button(Action::Import)), None);
        let first = screen.row_centre(WINDOW, 0).unwrap();
        assert_eq!(
            tap(&mut screen, first),
            None,
            "no start while an import runs"
        );
    }

    #[test]
    fn a_drag_scrolls_the_list_and_is_no_tap() {
        let games: Vec<Game> = (0..20).map(|id| game(id, &format!("Game {id}"))).collect();
        let mut screen = Screen::new(games.clone(), Vec::new());
        let layout = Layout::new(WINDOW, Insets::default());
        let (x, y) = screen.row_centre(WINDOW, 3).unwrap();
        screen.touch(WINDOW, 5, TouchPhase::Started, (x, y));
        screen.touch(
            WINDOW,
            5,
            TouchPhase::Moved,
            (x, y - 2.0 * layout.row_height),
        );
        assert_eq!(screen.first, 2);
        screen.touch(WINDOW, 5, TouchPhase::Moved, (x, y));
        assert_eq!(screen.touch(WINDOW, 5, TouchPhase::Ended, (x, y)), None);
        assert_eq!(screen.first, 0, "back where it started");
        screen.touch(WINDOW, 5, TouchPhase::Started, (x, y));
        screen.touch(
            WINDOW,
            5,
            TouchPhase::Moved,
            (x, y - 2.0 * layout.row_height),
        );
        screen.touch(
            WINDOW,
            5,
            TouchPhase::Ended,
            (x, y - 2.0 * layout.row_height),
        );
        assert_eq!(screen.first, 2);
        // A tap on the first row shown now selects the third game.
        let first = screen.row_centre(WINDOW, 0).unwrap();
        assert_eq!(tap(&mut screen, first), None);
        assert_eq!(
            tap(&mut screen, first),
            Some(Request::Play(games[2].clone()))
        );
    }

    /// A navigation bar at the right edge, as on a phone with three buttons in landscape.
    #[test]
    fn the_screen_keeps_clear_of_the_insets() {
        let mut screen = Screen::new(vec![game(1, "A")], Vec::new());
        screen.picture(WINDOW);
        let insets = Insets {
            right: 130.0,
            ..Insets::default()
        };
        screen.set_insets(insets);
        assert!(screen.changed(WINDOW));
        let layout = Layout::new(WINDOW, insets);
        let clear = WINDOW.width as f32 - insets.right;
        for (_, _, area) in layout.buttons {
            assert!(area.x + area.width <= clear, "{area:?}");
        }
        assert!(layout.list.x + layout.list.width <= clear);
        let delete = layout.buttons[2].2.centre();
        assert_eq!(tap(&mut screen, delete), None);
        assert_eq!(
            tap(&mut screen, delete),
            Some(Request::Delete(game(1, "A")))
        );
        let picture = screen.picture(WINDOW);
        let at = |x: u32, y: u32| {
            let start = ((y * WINDOW.width + x) * 4) as usize;
            picture.pixels[start..start + 4].to_vec()
        };
        // Under the bar only the background.
        assert!((0..WINDOW.height).all(|y| at(WINDOW.width - 1, y) == BACKGROUND));
    }

    #[test]
    fn the_picture_is_redrawn_only_after_a_change() {
        let mut screen = Screen::new(vec![game(1, "A")], Vec::new());
        assert!(screen.changed(WINDOW));
        let picture = screen.picture(WINDOW);
        assert_eq!(picture.size, WINDOW);
        assert_eq!(&picture.pixels[..4], &BACKGROUND, "opaque");
        assert!(!screen.changed(WINDOW));
        screen.set_status("Importing");
        assert!(screen.changed(WINDOW));
    }
}
