//! Error types of the crate.

/// Why serialized data could not be turned back into a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// The input does not have the length the type's layout requires.
    #[error("serialized {what} must be {expected} bytes long, found {found}")]
    Length {
        what: &'static str,
        expected: usize,
        found: usize,
    },
    /// The input is too short to hold a table header.
    #[error("serialized table needs a header of {expected} bytes, found {found}")]
    TableHeaderTruncated { expected: usize, found: usize },
    /// The table header names a number of dimensions other than 1, 2 or 3.
    #[error("serialized table has {found} dimensions; only 1, 2 and 3 exist")]
    TableDimensions { found: i32 },
    /// The table header holds a negative size.
    #[error("serialized table has the negative size {xsize} by {ysize} by {zsize}")]
    TableNegativeSize { xsize: i32, ysize: i32, zsize: i32 },
    /// The element count of the table header is not the product of its sizes.
    #[error(
        "serialized table of {xsize} by {ysize} by {zsize} claims {count} elements, \
         not the product of its sizes"
    )]
    TableCount {
        xsize: i32,
        ysize: i32,
        zsize: i32,
        count: i32,
    },
}

/// Why a table could not be created or resized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TableError {
    /// The table would hold more elements than its serialized header can count.
    #[error(
        "a table of {xsize} by {ysize} by {zsize} has more than {} elements",
        i32::MAX
    )]
    TooLarge { xsize: i32, ysize: i32, zsize: i32 },
}
