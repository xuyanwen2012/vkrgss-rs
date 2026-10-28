//! The proof of being on a thread that holds the interpreter.

use std::marker::PhantomData;

use crate::interpreter::Interpreter;

/// Proof of being on a thread that holds the interpreter `I` while it is alive: the
/// interpreter thread, or over Ruby 1.9.2 a script thread that Ruby calls a host function on.
///
/// The closures given to the `Interpreter::run` of an interpreter crate and the host functions
/// that Ruby calls receive a `&Ruby`; the operations that touch the interpreter are its
/// methods. It is neither `Send` nor `Sync`, so it cannot leave the thread.
pub struct Ruby<I> {
    not_send: PhantomData<*mut ()>,
    interpreter: PhantomData<I>,
}

impl<I: Interpreter> Ruby<I> {
    /// The proof, for the interpreter crate that hands it out.
    ///
    /// # Safety
    ///
    /// Only on a thread that holds the interpreter, between the end of start-up and
    /// finalisation.
    pub unsafe fn new() -> Self {
        Self {
            not_send: PhantomData,
            interpreter: PhantomData,
        }
    }

    /// Tells the interpreter that the game's scripts begin here.
    ///
    /// Over Ruby 1.8.7 symbol numbers follow the original runtime: a name it has before its
    /// first game script has the number measured there, whenever it is interned. Any other
    /// name interned before this call gets a number from a separate high range; from this
    /// call on, a new name gets the next number of the original's sequence, and the first use
    /// of a name of the high range consumes one number of that sequence. Calling it again
    /// changes nothing. Over Ruby 1.9.2 it does nothing: the interpreter numbers its symbols
    /// as the release does, and a dump does not depend on the numbers.
    pub fn mark_game_scripts(&self) {
        // SAFETY: `&self` proves the thread; the function cannot raise.
        unsafe { I::mark_game_scripts() };
    }
}
