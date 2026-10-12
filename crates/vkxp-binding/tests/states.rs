//! Saving and resuming states: the scenarios of `rgss-runtime` about them, with a copier
//! that copies nothing. Where a real one comes back in a copy of the process, this one comes
//! back in the same guest with the link of a second scripted host, so what a resumed copy
//! sends is what that host records.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use vkxp_binding::testing::host::{self, Game, Played, Record, Script, ScriptedHost};
use vkxp_binding::{Copied, Copier, CopyFailed, RunConfig};
use vkxp_compose_ref::Compositor;
use vkxp_proto::{
    AudioChannel, AudioCommand, BitmapDelta, FramePacket, GuestToHost, HostToGuest, TableDelta,
};

/// What a call of the fake copier does.
#[derive(Debug)]
enum Plan {
    /// The guest goes on; the copy is never heard of.
    GoOn,
    /// No copy can be made.
    Fail,
    /// The guest goes on as the copy that was resumed, against a new host.
    Resume,
}

/// A copier that follows a plan and writes down the slots it was asked for.
#[derive(Debug, Default)]
struct FakeCopier {
    plan: Mutex<VecDeque<Plan>>,
    slots: Mutex<Vec<u8>>,
    resumed: Mutex<Vec<ScriptedHost>>,
}

impl FakeCopier {
    fn new(plan: impl IntoIterator<Item = Plan>) -> Arc<FakeCopier> {
        Arc::new(FakeCopier {
            plan: Mutex::new(plan.into_iter().collect()),
            ..FakeCopier::default()
        })
    }

    /// The slots asked for so far, in order.
    fn slots(&self) -> Vec<u8> {
        self.slots.lock().unwrap().clone()
    }

    /// What the host of the one resumed copy saw.
    fn resumed(&self) -> Record {
        let mut hosts = self.resumed.lock().unwrap();
        assert_eq!(hosts.len(), 1, "one copy was resumed");
        hosts.remove(0).finish()
    }
}

impl Copier for FakeCopier {
    fn park(&self, slot: u8) -> Result<Copied, CopyFailed> {
        self.slots.lock().unwrap().push(slot);
        let plan = self.plan.lock().unwrap().pop_front();
        match plan.unwrap_or(Plan::GoOn) {
            Plan::GoOn => Ok(Copied::WentOn),
            Plan::Fail => Err(CopyFailed),
            Plan::Resume => {
                let (link, host) = host::scripted_host(Script::new());
                self.resumed.lock().unwrap().push(host);
                Ok(Copied::Resumed(link))
            }
        }
    }
}

fn run(game: &Game, copier: Option<Arc<FakeCopier>>, script: Script) -> Played {
    let config = RunConfig {
        seed: Some(7),
        copier: copier.map(|copier| copier as Arc<dyn Copier>),
        ..game.run_config()
    };
    let played = host::run_game(game, config, script, "nil");
    assert!(played.outcome.is_ok(), "{:?}", played.outcome);
    played
}

fn save(slot: u8) -> HostToGuest {
    HostToGuest::SaveState { slot }
}

/// The pictures of the frames of a record, drawn one after another by a compositor that
/// starts with nothing.
///
/// The arena is shared by the games of this binary, so a record can start with the disposal
/// of bitmaps an earlier game left; those the compositor never had are left out.
fn pictures(record: &Record) -> Vec<Vec<u8>> {
    let mut compositor = Compositor::new();
    let mut render = |packet: &FramePacket| {
        let mut packet = packet.clone();
        packet.bitmaps.retain(|delta| match delta {
            BitmapDelta::Destroy { id } => compositor.bitmaps().get(*id).is_ok(),
            _ => true,
        });
        let frame = compositor.render(&packet);
        frame.expect("the frame needs nothing it was not sent").rgba
    };
    record.frames().into_iter().map(&mut render).collect()
}

/// A game that draws from a counter and from `rand` for `frames` frames.
fn counting(name: &str, frames: u32) -> Game {
    let main = format!(
        "sprite = Sprite.new
         sprite.bitmap = Bitmap.new(16, 16)
         {frames}.times do |i|
           sprite.x = i * 3
           sprite.y = rand(200)
           sprite.bitmap.fill_rect(rand(16), rand(16), 4, 4, Color.new(rand(256), i, 255 - i))
           Graphics.update
         end"
    );
    Game::new(name, "Counting", &[("Main", &main)])
}

/// Scenario "Unchanged run".
#[test]
fn a_guest_that_is_copied_sends_the_frames_it_sends_without() {
    let game = counting("states_unchanged", 80);
    let plain = run(&game, None, Script::new());
    let copier = FakeCopier::new([]);
    let saved = run(
        &game,
        Some(copier.clone()),
        Script::new().after_frame(50, save(1)),
    );

    assert_eq!(copier.slots(), [1]);
    assert_eq!(plain.record.frames().len(), 80);
    assert_eq!(pictures(&saved.record), pictures(&plain.record));
    // Nothing but the frames either: the guest that goes on says nothing of the copy.
    assert_eq!(saved.record.others(), plain.record.others());
}

/// A request is served at the next wait for a tick the guest enters.
#[test]
fn the_copy_is_made_in_the_wait_of_the_frame_after_the_request() {
    let game = counting("states_next_wait", 6);
    let copier = FakeCopier::new([Plan::Resume]);
    let played = run(
        &game,
        Some(copier.clone()),
        Script::new().after_frame(2, save(0)),
    );
    // The first host saw the guest up to the wait it was copied in.
    assert_eq!(played.record.frames().len(), 3);
    // The copy sends the frame of that wait again and then the rest.
    assert_eq!(copier.resumed().frames().len(), 4);
}

/// Scenario "Frozen display".
#[test]
fn a_request_between_a_freeze_and_the_end_of_its_transition_waits_for_the_frame_after() {
    let main = "sprite = Sprite.new
                sprite.bitmap = Bitmap.new(8, 8)
                3.times { Graphics.update }
                Graphics.freeze
                2.times { Graphics.update }
                Graphics.transition(4)
                3.times { Graphics.update }";
    let game = Game::new("states_frozen", "Frozen", &[("Main", main)]);
    // Frames 4 and 5 are sent while the display is frozen, 6 to 9 are the transition's.
    for request_at in [3, 4, 5, 7, 9] {
        let copier = FakeCopier::new([Plan::Resume]);
        let script = Script::new().after_frame(request_at, save(0));
        let played = run(&game, Some(copier.clone()), script);
        assert_eq!(
            played.record.frames().len(),
            10,
            "a request after frame {request_at}"
        );
        assert_eq!(copier.resumed().frames().len(), 3);
    }
}

/// A modal message is not a frame to copy at either.
#[test]
fn a_request_during_a_modal_message_waits_for_the_frame_after_it() {
    let main = "Graphics.update
                print 'wait'
                2.times { Graphics.update }";
    let game = Game::new("states_modal", "Modal", &[("Main", main)]);
    let copier = FakeCopier::new([Plan::Resume]);
    // The message is shown for the frames 2 to 4.
    let script = Script::new().dismiss_after(3).after_frame(2, save(0));
    let played = run(&game, Some(copier.clone()), script);
    assert_eq!(played.record.frames().len(), 5);
    assert_eq!(copier.resumed().frames().len(), 2);
}

/// Scenario "No copier".
#[test]
fn a_guest_without_a_copier_answers_with_the_failure_and_goes_on() {
    let game = counting("states_no_copier", 5);
    let script = Script::new()
        .after_frame(2, save(4))
        .after_frame(3, HostToGuest::LoadState { slot: 6 });
    let played = run(&game, None, script);
    assert_eq!(played.record.frames().len(), 5);
    let failed: Vec<&GuestToHost> = played
        .record
        .others()
        .into_iter()
        .filter(|message| matches!(message, GuestToHost::StateFailed { .. }))
        .collect();
    assert_eq!(
        failed,
        [
            &GuestToHost::StateFailed {
                slot: 4,
                load: false
            },
            &GuestToHost::StateFailed {
                slot: 6,
                load: true
            }
        ]
    );
}

#[test]
fn a_copier_that_fails_is_answered_with_the_failure() {
    let game = counting("states_failing_copier", 5);
    let copier = FakeCopier::new([Plan::Fail]);
    let script = Script::new().after_frame(2, save(3));
    let played = run(&game, Some(copier.clone()), script);
    assert_eq!(copier.slots(), [3]);
    assert_eq!(played.record.frames().len(), 5);
    assert!(played.record.others().contains(&&GuestToHost::StateFailed {
        slot: 3,
        load: false
    }));
}

/// Scenarios "Complete frame" and "Music".
#[test]
fn a_resumed_copy_sends_its_title_its_pointer_its_music_and_a_complete_frame() {
    let main = "bitmaps = [8, 12, 16].map do |side|
                  bitmap = Bitmap.new(side, side)
                  bitmap.fill_rect(0, 0, side, side, Color.new(side * 10, 255 - side, side))
                  bitmap.fill_rect(1, 1, 3, 3, Color.new(255, 255, 0))
                  bitmap
                end
                sprites = bitmaps.map do |bitmap|
                  sprite = Sprite.new
                  sprite.bitmap = bitmap
                  sprite.x = bitmap.width * 9
                  sprite.y = 100
                  sprite.z = 500
                  sprite
                end
                tileset = Bitmap.new(256, 64)
                tileset.fill_rect(0, 0, 256, 64, Color.new(10, 200, 30))
                tileset.fill_rect(32, 0, 32, 32, Color.new(200, 0, 40))
                map = Table.new(20, 15, 3)
                map[0, 0, 0] = 384
                map[1, 0, 0] = 385
                map[2, 1, 0] = 392
                tilemap = Tilemap.new
                tilemap.tileset = tileset
                tilemap.map_data = map
                tilemap.priorities = Table.new(400)
                Audio.bgm_play('Audio/BGM/theme', 80, 110)
                Win32API.new('user32', 'ShowCursor', 'i', 'i').call(0)
                6.times do |i|
                  sprites[0].x += 5
                  bitmaps[1].set_pixel(i, i, Color.new(0, 0, 255))
                  map[3, i % 2, 0] = 386
                  tilemap.update
                  Graphics.update
                end";
    let game = Game::new("states_complete", "Complete", &[("Main", main)]);
    let music = b"not really music".to_vec();
    let file = game.dir().join("Audio/BGM/theme.ogg");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, &music).unwrap();

    let copier = FakeCopier::new([Plan::Resume]);
    let script = Script::new().after_frame(3, save(0));
    let played = run(&game, Some(copier.clone()), script);
    let resumed = copier.resumed();

    // The frame of the wait the copy was made in, as the first host was sent it.
    let before = pictures(&played.record);
    assert_eq!(before.len(), 4);

    // Title, the pointer that the game hid, music, then the frame.
    assert_eq!(
        resumed.received[0],
        GuestToHost::SetTitle {
            title: "Complete".to_owned()
        }
    );
    assert_eq!(
        resumed.received[1],
        GuestToHost::PointerVisible { visible: false }
    );
    assert_eq!(
        resumed.received[2],
        GuestToHost::Audio(AudioCommand::Play {
            channel: AudioChannel::Bgm,
            data: music,
            volume: 80,
            pitch: 110,
        })
    );
    let GuestToHost::Frame(first) = &resumed.received[3] else {
        panic!("{:?} is not a frame", resumed.received[3]);
    };

    // The three bitmaps of the sprites and the tileset are created and filled whole.
    let created: Vec<(u32, u32)> = first
        .bitmaps
        .iter()
        .filter_map(|delta| match delta {
            BitmapDelta::Create { width, height, .. } => Some((*width, *height)),
            _ => None,
        })
        .collect();
    for size in [(8, 8), (12, 12), (16, 16), (256, 64)] {
        assert!(created.contains(&size), "{size:?} in {created:?}");
    }
    for pair in first.bitmaps.chunks(2) {
        let [
            BitmapDelta::Create { id, width, height },
            BitmapDelta::Update {
                id: filled,
                rect,
                rgba,
            },
        ] = pair
        else {
            panic!("{pair:?} is not a bitmap created and filled");
        };
        assert_eq!(id, filled);
        assert_eq!(*rect, vkxp_proto::Rect::new(0, 0, *width, *height));
        assert_eq!(rgba.len() as u32, width * height * 4);
    }
    // The map and the priorities.
    let sets = |delta: &&TableDelta| matches!(delta, TableDelta::Set { .. });
    assert_eq!(first.tables.iter().filter(sets).count(), 2);
    assert_eq!(first.tables.len(), 2);

    // A compositor that held nothing draws it like the frame of the original wait, and
    // the copy goes on from there like the guest it was made of.
    let after = pictures(&resumed);
    assert_eq!(after[0], before[3]);
    assert_eq!(after.len(), 3);
    let whole = pictures(&run(&game, None, Script::new()).record);
    assert_eq!(whole.len(), 6);
    assert_eq!(before[..], whole[..4]);
    assert_eq!(after[1..], whole[4..]);
    assert_eq!(resumed.received.last(), Some(&GuestToHost::Shutdown));
}
