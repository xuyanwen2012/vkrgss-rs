//! Errors of image decoding.

use thiserror::Error;

use crate::ImageFormat;

/// Why image data produced no image.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ImageError {
    /// The data is not PNG, JPEG or BMP, or uses a feature of one of them that is not decoded.
    #[error("unsupported image format: {reason}")]
    UnsupportedFormat { reason: String },
    /// The data is truncated or corrupt.
    #[error("malformed {format} data: {reason}")]
    Malformed { format: ImageFormat, reason: String },
    /// The image declares more pixels than a bitmap may have. Nothing was allocated for them.
    #[error("image of {width}x{height} pixels exceeds the limit of {limit} pixels")]
    TooLarge { width: u32, height: u32, limit: u64 },
}

impl ImageError {
    pub(crate) fn unsupported(reason: impl Into<String>) -> Self {
        ImageError::UnsupportedFormat {
            reason: reason.into(),
        }
    }

    pub(crate) fn malformed(format: ImageFormat, reason: impl Into<String>) -> Self {
        ImageError::Malformed {
            format,
            reason: reason.into(),
        }
    }
}
