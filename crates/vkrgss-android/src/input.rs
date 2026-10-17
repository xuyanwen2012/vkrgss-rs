//! The input of a run with an input script, frame by frame.

use vkrgss_host::{InputScript, InputSource};
use vkrgss_proto::InputSnapshot;

/// Holds the buttons of an input script at each frame, as a headless run does: the snapshot
/// of frame `n` is what the script holds at `n`, with whatever else is live (the keys the
/// game injected, the pointer) on top.
#[derive(Debug)]
pub struct ScriptedInput {
    script: InputScript,
    /// The number of the next frame.
    frame: u64,
}

impl ScriptedInput {
    pub fn new(script: InputScript) -> ScriptedInput {
        ScriptedInput { script, frame: 0 }
    }
}

impl InputSource for ScriptedInput {
    fn next(&mut self, live: InputSnapshot) -> InputSnapshot {
        let held = self.script.held_at(self.frame);
        self.frame += 1;
        InputSnapshot {
            buttons: held.buttons | live.buttons,
            ..live
        }
    }
}

#[cfg(test)]
mod tests {
    use vkrgss_proto::Button;

    use super::*;

    #[test]
    fn each_frame_holds_what_the_script_holds_at_it_and_what_is_live() {
        let mut input = ScriptedInput::new(InputScript::parse("1:RIGHT, 3:").unwrap());
        let live = InputSnapshot::EMPTY
            .with_key(0x41)
            .with_pointer(Some((3, 4)));
        let right = InputSnapshot::EMPTY.with(Button::Right);
        let snapshots: Vec<InputSnapshot> = (0..4).map(|_| input.next(live)).collect();
        assert_eq!(
            snapshots,
            [
                live,
                InputSnapshot {
                    buttons: right.buttons,
                    ..live
                },
                InputSnapshot {
                    buttons: right.buttons,
                    ..live
                },
                live,
            ]
        );
    }
}
