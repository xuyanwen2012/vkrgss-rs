//! Failures of the guest's operations and the Ruby exceptions they become.

use std::ffi::CStr;

use vkrgss_core::{BitmapError, DecodeError, SceneError, TableError};
use vkrgss_image::ImageError;
use vkrgss_ruby_sys::ffi::VALUE;
use vkrgss_ruby_sys::{Raise, Raised, Ruby};
use vkrgss_vfs::VfsError;

use crate::guest;
use crate::rb;

/// A failure of an operation behind a Ruby method.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A bitmap was disposed or could not be created.
    #[error(transparent)]
    Bitmap(#[from] BitmapError),
    /// A file was not found or could not be read.
    #[error(transparent)]
    Files(#[from] VfsError),
    /// A file is not an image that can be decoded.
    #[error(transparent)]
    Image(#[from] ImageError),
    /// Serialized data is malformed.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// A table cannot have the requested size.
    #[error(transparent)]
    Table(#[from] TableError),
    /// A scene element was disposed or could not be created.
    #[error(transparent)]
    Scene(#[from] SceneError),
}

/// The exception classes the mapping raises that Ruby has no global for.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ErrorClasses {
    /// `RGSSError`.
    pub(crate) rgss_error: VALUE,
    /// `Errno::ENOENT`.
    pub(crate) no_entry: VALUE,
    /// `Errno::EACCES`.
    pub(crate) access: VALUE,
}

/// Defines `RGSSError`, a subclass of `StandardError`, and looks up `Errno::ENOENT` and
/// `Errno::EACCES`.
pub(crate) fn define(ruby: &Ruby) -> Result<ErrorClasses, Raised> {
    const RGSS_ERROR: &CStr = c"RGSSError";
    // SAFETY: `StandardError` is a live class.
    let rgss_error = unsafe { ruby.define_class(RGSS_ERROR, rb::standard_error(ruby)) }?;
    let no_entry = rb::class_at(ruby, c"Errno::ENOENT")?;
    let access = rb::class_at(ruby, c"Errno::EACCES")?;
    Ok(ErrorClasses {
        rgss_error,
        no_entry,
        access,
    })
}

/// The Ruby exception of a failure. This is the one place where Rust errors become the
/// exceptions of the original runtime:
///
/// | Failure | Exception |
/// |---|---|
/// | a disposed bitmap | `RGSSError`, `disposed bitmap` |
/// | a bitmap that cannot be created, an image that cannot be decoded | `RGSSError`, `failed to create bitmap` |
/// | a file that is not found | `Errno::ENOENT`, `No such file or directory - <path as given>` |
/// | a data file of RGSS3 that is a directory | `Errno::EACCES`, `Permission denied - <path as given>` |
/// | a file that cannot be read | `IOError` |
/// | malformed serialized data, an impossible table size | `ArgumentError` |
/// | a disposed scene element | `RGSSError`, `disposed sprite` and so on for each class |
/// | a scene element that cannot be created | `RGSSError` |
pub(crate) fn to_raise(ruby: &Ruby, error: Error) -> Raise {
    let classes = guest::classes().errors;
    match error {
        Error::Bitmap(BitmapError::Disposed) => Raise::new(classes.rgss_error, "disposed bitmap"),
        Error::Bitmap(BitmapError::CreateFailed(_)) | Error::Image(_) => {
            Raise::new(classes.rgss_error, "failed to create bitmap")
        }
        // `Errno::ENOENT.new` puts "No such file or directory - " before its argument.
        Error::Files(VfsError::NotFound { path }) => Raise::new(classes.no_entry, path),
        // `Errno::EACCES.new` puts "Permission denied - " before its argument.
        Error::Files(VfsError::IsDirectory { path }) => Raise::new(classes.access, path),
        Error::Files(other) => Raise::new(rb::io_error(ruby), other.to_string()),
        Error::Decode(malformed) => Raise::new(rb::arg_error(ruby), malformed.to_string()),
        Error::Table(impossible) => Raise::new(rb::arg_error(ruby), impossible.to_string()),
        Error::Scene(failure) => Raise::new(classes.rgss_error, failure.to_string()),
    }
}
