//! The Ruby module `Input`, on top of core's input state and the host's snapshots.
//!
//! A game of RGSS3 has the constants and the argument rule of that runtime: its constants are
//! the Symbols of their names, and `press?`, `trigger?` and `repeat?` take such a Symbol or the
//! number of RGSS2 of a button and answer `false` for anything else. Its input state follows
//! the rules of RGSS3, under which `Input.update` sees what had arrived when `Graphics` last
//! showed a frame (see `graphics`).

use std::cell::Cell;

use vkrgss_core::InputState;
use vkrgss_proto::Button;
use vkrgss_ruby_sys::ffi::{self, ID, VALUE};
use vkrgss_ruby_sys::{HostResult, Raised, Ruby};
use vkrgss_vfs::Runtime;

use crate::guest::Guest;
use crate::rb;
use crate::runtime::Await;

thread_local! {
    /// The IDs of the names of the buttons in the order of [`Button::ALL`], taken when a
    /// module of RGSS3 is defined.
    static NAMES: Cell<Option<[ID; Button::ALL.len()]>> = const { Cell::new(None) };
}

/// Takes the snapshots that have arrived, without waiting, and advances the state.
fn update(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Guest::with(|guest| {
        guest.pump(Await::Nothing);
        guest.input.update();
        guest.win32api.input_updated(&guest.input);
    });
    Ok(ffi::Qnil)
}

/// The button of `number`, the value of an `Input` constant of RGSS1 and RGSS2.
fn button_of(number: i64) -> Option<Button> {
    Button::ALL
        .into_iter()
        .find(|&button| i64::from(button as u8) == number)
}

/// The button of an `Input` constant's number, or `None` for any other number.
fn button_arg(ruby: &Ruby, value: VALUE) -> Result<Option<Button>, vkrgss_ruby_sys::Raise> {
    Ok(button_of(rb::long(ruby, value)?))
}

/// The button of an argument of RGSS3: the Symbol of a button's name, exactly as written, or
/// a Fixnum equal to its number of RGSS2. Anything else is `None`, and no method of the value
/// is called, so nothing raises and an unknown Symbol is never interned.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 versions 3.0.0.1 and 3.0.1.1,
/// `ace-v1-sprite-REPORT.md` section 6. An object that defines `to_int` or `to_sym` was not
/// measured; it gives `None` here.
fn button_arg_rgss3(value: VALUE) -> Option<Button> {
    if ffi::symbol_p(value) {
        let id = ffi::sym2id(value);
        let names = NAMES.get()?;
        Button::ALL
            .into_iter()
            .zip(names)
            .find_map(|(button, name)| (name == id).then_some(button))
    } else if ffi::fixnum_p(value) {
        button_of(ffi::fix2long(value))
    } else {
        None
    }
}

/// `true` when `button` is one and `query` holds for it.
fn answer(button: Option<Button>, query: fn(&InputState, Button) -> bool) -> VALUE {
    rb::boolean(button.is_some_and(|button| Guest::with(|guest| query(&guest.input, button))))
}

fn press(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    Ok(answer(button_arg(ruby, args[0])?, InputState::is_pressed))
}

fn trigger(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    Ok(answer(button_arg(ruby, args[0])?, InputState::is_triggered))
}

fn repeat(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    Ok(answer(button_arg(ruby, args[0])?, InputState::is_repeated))
}

/// `press?` of RGSS3.
fn press_rgss3(_ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    Ok(answer(button_arg_rgss3(args[0]), InputState::is_pressed))
}

/// `trigger?` of RGSS3.
fn trigger_rgss3(_ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    Ok(answer(button_arg_rgss3(args[0]), InputState::is_triggered))
}

/// `repeat?` of RGSS3.
fn repeat_rgss3(_ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    Ok(answer(button_arg_rgss3(args[0]), InputState::is_repeated))
}

fn dir4(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::fix(i32::from(Guest::with(|guest| guest.input.dir4()))))
}

fn dir8(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::fix(i32::from(Guest::with(|guest| guest.input.dir8()))))
}

/// Defines `Input` of `runtime` and its twenty button constants, whose names are those of
/// [`Button`] in capitals. Their values are its discriminants in RGSS1 and RGSS2, and the
/// Symbols of their names in RGSS3, whose IDs are kept for the arguments of the queries.
pub(crate) fn define(ruby: &Ruby, runtime: Runtime) -> Result<(), Raised> {
    let module = ruby.define_module(c"Input")?;
    rb::class_method(ruby, module, c"update", 0, update)?;
    if runtime == Runtime::Rgss3 {
        rb::class_method(ruby, module, c"press?", 1, press_rgss3)?;
        rb::class_method(ruby, module, c"trigger?", 1, trigger_rgss3)?;
        rb::class_method(ruby, module, c"repeat?", 1, repeat_rgss3)?;
    } else {
        rb::class_method(ruby, module, c"press?", 1, press)?;
        rb::class_method(ruby, module, c"trigger?", 1, trigger)?;
        rb::class_method(ruby, module, c"repeat?", 1, repeat)?;
    }
    rb::class_method(ruby, module, c"dir4", 0, dir4)?;
    rb::class_method(ruby, module, c"dir8", 0, dir8)?;
    let names = Button::ALL.map(|button| format!("{button:?}").to_uppercase());
    let mut constants = String::from("module Input\n");
    for (button, name) in Button::ALL.into_iter().zip(&names) {
        match runtime {
            Runtime::Rgss1 | Runtime::Rgss2 => {
                constants.push_str(&format!("  {name} = {}\n", button as u8));
            }
            Runtime::Rgss3 => constants.push_str(&format!("  {name} = :{name}\n")),
        }
    }
    constants.push_str("end\n");
    ruby.eval(&constants, "Input", 1)?;
    if runtime == Runtime::Rgss3 {
        // The constants have interned the names; their Symbols are immediate values.
        let symbols = ruby.eval(&format!("[:{}]", names.join(", :")), "Input", 1)?;
        let symbols = rb::array_elements(ruby, symbols).expect("an Array of the names");
        let ids: Vec<ID> = symbols.into_iter().map(ffi::sym2id).collect();
        NAMES.set(Some(ids.try_into().expect("one ID for each button")));
    }
    Ok(())
}
