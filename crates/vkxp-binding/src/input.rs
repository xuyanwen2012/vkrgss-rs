//! The Ruby module `Input`, on top of core's input state and the host's snapshots.

use vkxp_proto::Button;
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Raised, Ruby};

use crate::guest::Guest;
use crate::rb;
use crate::runtime::Await;

/// Takes the snapshots that have arrived, without waiting, and advances the state.
fn update(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Guest::with(|guest| {
        guest.pump(Await::Nothing);
        guest.input.update();
        guest.win32api.input_updated(&guest.input);
    });
    Ok(ffi::Qnil)
}

/// The button of an `Input` constant's number, or `None` for any other number.
fn button_arg(ruby: &Ruby, value: VALUE) -> Result<Option<Button>, vkxp_ruby_sys::Raise> {
    let number = rb::long(ruby, value)?;
    Ok(Button::ALL
        .into_iter()
        .find(|&button| i64::from(button as u8) == number))
}

fn press(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let button = button_arg(ruby, args[0])?;
    Ok(rb::boolean(button.is_some_and(|button| {
        Guest::with(|guest| guest.input.is_pressed(button))
    })))
}

fn trigger(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let button = button_arg(ruby, args[0])?;
    Ok(rb::boolean(button.is_some_and(|button| {
        Guest::with(|guest| guest.input.is_triggered(button))
    })))
}

fn repeat(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let button = button_arg(ruby, args[0])?;
    Ok(rb::boolean(button.is_some_and(|button| {
        Guest::with(|guest| guest.input.is_repeated(button))
    })))
}

fn dir4(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::fix(i32::from(Guest::with(|guest| guest.input.dir4()))))
}

fn dir8(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::fix(i32::from(Guest::with(|guest| guest.input.dir8()))))
}

/// Defines `Input` and its twenty button constants, whose names are those of [`Button`] in
/// capitals and whose values are its discriminants.
pub(crate) fn define(ruby: &Ruby) -> Result<(), Raised> {
    let module = ruby.define_module(c"Input")?;
    rb::class_method(ruby, module, c"update", 0, update)?;
    rb::class_method(ruby, module, c"press?", 1, press)?;
    rb::class_method(ruby, module, c"trigger?", 1, trigger)?;
    rb::class_method(ruby, module, c"repeat?", 1, repeat)?;
    rb::class_method(ruby, module, c"dir4", 0, dir4)?;
    rb::class_method(ruby, module, c"dir8", 0, dir8)?;
    let mut constants = String::from("module Input\n");
    for button in Button::ALL {
        let name = format!("{button:?}").to_uppercase();
        constants.push_str(&format!("  {name} = {}\n", button as u8));
    }
    constants.push_str("end\n");
    ruby.eval(&constants, "Input", 1)?;
    Ok(())
}
