//! `Table`: a one-, two- or three-dimensional array of signed 16-bit integers, and
//! `SharedTable`, the handle through which several owners see one table.

use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::{DecodeError, TableError};

/// Length of the serialized header: five little-endian `i32`.
const HEADER_LEN: usize = 20;

/// An array of `i16` with one to three dimensions, all zero when created. Dimensions a table
/// does not use have size 1.
///
/// Elements are stored with x varying fastest, then y, then z, which is also their serialized
/// order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Table {
    dimensions: i32,
    xsize: i32,
    ysize: i32,
    zsize: i32,
    data: Vec<i16>,
}

/// The number of elements of a table with the given sizes, when the serialized header can
/// express it. Sizes must not be negative.
fn element_count(xsize: i32, ysize: i32, zsize: i32) -> Option<i32> {
    // Three `i32` multiply to less than 2^93, which only `i128` holds.
    let count = i128::from(xsize) * i128::from(ysize) * i128::from(zsize);
    i32::try_from(count).ok()
}

impl Table {
    /// A one-dimensional table of `xsize` zeros. A negative size counts as 0.
    pub fn new(xsize: i32) -> Result<Table, TableError> {
        Table::with_sizes(1, xsize, 1, 1)
    }

    /// A two-dimensional table of zeros. A negative size counts as 0.
    pub fn new_2d(xsize: i32, ysize: i32) -> Result<Table, TableError> {
        Table::with_sizes(2, xsize, ysize, 1)
    }

    /// A three-dimensional table of zeros. A negative size counts as 0.
    pub fn new_3d(xsize: i32, ysize: i32, zsize: i32) -> Result<Table, TableError> {
        Table::with_sizes(3, xsize, ysize, zsize)
    }

    fn with_sizes(
        dimensions: i32,
        xsize: i32,
        ysize: i32,
        zsize: i32,
    ) -> Result<Table, TableError> {
        let (xsize, ysize, zsize) = (xsize.max(0), ysize.max(0), zsize.max(0));
        let count = element_count(xsize, ysize, zsize).ok_or(TableError::TooLarge {
            xsize,
            ysize,
            zsize,
        })?;
        Ok(Table {
            dimensions,
            xsize,
            ysize,
            zsize,
            data: vec![0; count as usize],
        })
    }

    /// How many dimensions the table was created or last resized with: 1, 2 or 3.
    pub fn dimensions(&self) -> i32 {
        self.dimensions
    }

    pub fn xsize(&self) -> i32 {
        self.xsize
    }

    pub fn ysize(&self) -> i32 {
        self.ysize
    }

    pub fn zsize(&self) -> i32 {
        self.zsize
    }

    /// All elements, with x varying fastest, then y, then z.
    pub(crate) fn data(&self) -> &[i16] {
        &self.data
    }

    /// Position of an element in `data`, or `None` outside the table.
    fn offset(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        let inside = |index: i32, size: i32| index >= 0 && index < size;
        if !(inside(x, self.xsize) && inside(y, self.ysize) && inside(z, self.zsize)) {
            return None;
        }
        let (x, y, z) = (x as usize, y as usize, z as usize);
        Some((z * self.ysize as usize + y) * self.xsize as usize + x)
    }

    /// The element at (`x`, `y`, `z`), or `None` outside the table. Pass 0 for the dimensions
    /// the table does not use.
    pub fn get(&self, x: i32, y: i32, z: i32) -> Option<i16> {
        self.offset(x, y, z).map(|offset| self.data[offset])
    }

    /// Writes the low 16 bits of `value` to the element at (`x`, `y`, `z`). Outside the table
    /// nothing changes.
    ///
    /// PLACEHOLDER: keeping the low 16 bits (rather than clamping) is not yet checked against
    /// the original runtime.
    pub fn set(&mut self, x: i32, y: i32, z: i32, value: i64) {
        if let Some(offset) = self.offset(x, y, z) {
            self.data[offset] = value as i16;
        }
    }

    /// Makes the table one-dimensional with `xsize` elements. See [`Table::resize_3d`].
    pub fn resize(&mut self, xsize: i32) -> Result<(), TableError> {
        self.resize_to(1, xsize, 1, 1)
    }

    /// Makes the table two-dimensional with the given sizes. See [`Table::resize_3d`].
    pub fn resize_2d(&mut self, xsize: i32, ysize: i32) -> Result<(), TableError> {
        self.resize_to(2, xsize, ysize, 1)
    }

    /// Makes the table three-dimensional with the given sizes. Every element whose coordinates
    /// exist before and after keeps its value; every new element is 0. On error the table is
    /// unchanged.
    pub fn resize_3d(&mut self, xsize: i32, ysize: i32, zsize: i32) -> Result<(), TableError> {
        self.resize_to(3, xsize, ysize, zsize)
    }

    fn resize_to(
        &mut self,
        dimensions: i32,
        xsize: i32,
        ysize: i32,
        zsize: i32,
    ) -> Result<(), TableError> {
        let mut resized = Table::with_sizes(dimensions, xsize, ysize, zsize)?;
        let kept = self.xsize.min(resized.xsize) as usize;
        if kept > 0 {
            for z in 0..self.zsize.min(resized.zsize) {
                for y in 0..self.ysize.min(resized.ysize) {
                    let from = self.offset(0, y, z).expect("inside the old table");
                    let to = resized.offset(0, y, z).expect("inside the new table");
                    resized.data[to..to + kept].copy_from_slice(&self.data[from..from + kept]);
                }
            }
        }
        *self = resized;
        Ok(())
    }

    /// The serialized form: the number of dimensions, the x, y and z sizes and the element
    /// count as little-endian `i32`, then the elements as little-endian `i16`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_LEN + self.data.len() * 2);
        // The constructors keep the count within `i32`.
        let count = self.data.len() as i32;
        for field in [self.dimensions, self.xsize, self.ysize, self.zsize, count] {
            bytes.extend_from_slice(&field.to_le_bytes());
        }
        for element in &self.data {
            bytes.extend_from_slice(&element.to_le_bytes());
        }
        bytes
    }

    /// Restores a table from its serialized form.
    pub fn from_bytes(bytes: &[u8]) -> Result<Table, DecodeError> {
        let Some((header, elements)) = bytes.split_at_checked(HEADER_LEN) else {
            return Err(DecodeError::TableHeaderTruncated {
                expected: HEADER_LEN,
                found: bytes.len(),
            });
        };
        let mut fields = [0; 5];
        for (field, chunk) in fields.iter_mut().zip(header.as_chunks::<4>().0) {
            *field = i32::from_le_bytes(*chunk);
        }
        let [dimensions, xsize, ysize, zsize, count] = fields;
        if !(1..=3).contains(&dimensions) {
            return Err(DecodeError::TableDimensions { found: dimensions });
        }
        if xsize < 0 || ysize < 0 || zsize < 0 {
            return Err(DecodeError::TableNegativeSize {
                xsize,
                ysize,
                zsize,
            });
        }
        if element_count(xsize, ysize, zsize) != Some(count) {
            return Err(DecodeError::TableCount {
                xsize,
                ysize,
                zsize,
                count,
            });
        }
        let expected = HEADER_LEN + count as usize * 2;
        if bytes.len() != expected {
            return Err(DecodeError::Length {
                what: "table",
                expected,
                found: bytes.len(),
            });
        }
        let data = elements
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| i16::from_le_bytes(*chunk))
            .collect();
        Ok(Table {
            dimensions,
            xsize,
            ysize,
            zsize,
            data,
        })
    }
}

/// A table and the number of writes it has seen.
#[derive(Debug)]
struct Shared {
    table: Table,
    revision: u64,
}

/// A handle to a [`Table`] that several owners share: a clone of the handle names the same
/// table, so a write through one handle is seen through every other. This is what a script's
/// `Table` object is, and what a tilemap keeps of the tables assigned to it.
///
/// Every write through a handle increments the table's revision, which tells whoever drew
/// from the table that it may have changed. Two handles are equal when they name the same
/// table. The handle is for one thread: the scene and its tables live on the guest thread.
#[derive(Debug, Clone)]
pub struct SharedTable(Rc<RefCell<Shared>>);

impl SharedTable {
    /// Wraps `table` in a handle of its own, at revision 0.
    pub fn new(table: Table) -> SharedTable {
        SharedTable(Rc::new(RefCell::new(Shared { table, revision: 0 })))
    }

    /// The table, for reading. Do not hold it across a write through any handle to the same
    /// table.
    pub fn read(&self) -> Ref<'_, Table> {
        Ref::map(self.0.borrow(), |shared| &shared.table)
    }

    /// Changes the table with `change` and increments the revision, whether or not `change`
    /// altered anything.
    pub fn write<R>(&self, change: impl FnOnce(&mut Table) -> R) -> R {
        let mut shared = self.0.borrow_mut();
        shared.revision += 1;
        change(&mut shared.table)
    }

    /// [`Table::get`] of the shared table.
    pub fn get(&self, x: i32, y: i32, z: i32) -> Option<i16> {
        self.read().get(x, y, z)
    }

    /// [`Table::set`] on the shared table. Increments the revision.
    pub fn set(&self, x: i32, y: i32, z: i32, value: i64) {
        self.write(|table| table.set(x, y, z, value));
    }

    /// Number of writes made to the table through any of its handles.
    pub fn revision(&self) -> u64 {
        self.0.borrow().revision
    }
}

impl PartialEq for SharedTable {
    fn eq(&self, other: &SharedTable) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for SharedTable {}

impl From<Table> for SharedTable {
    fn from(table: Table) -> SharedTable {
        SharedTable::new(table)
    }
}

#[cfg(test)]
mod tests;
