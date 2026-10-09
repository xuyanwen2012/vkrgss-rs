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
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// The five header fields of a serialized table.
    fn header(bytes: &[u8]) -> [i32; 5] {
        let mut fields = [0; 5];
        for (field, chunk) in fields
            .iter_mut()
            .zip(bytes[..HEADER_LEN].as_chunks::<4>().0)
        {
            *field = i32::from_le_bytes(*chunk);
        }
        fields
    }

    /// A serialized table with the given header and that many zero bytes of elements.
    fn raw(fields: [i32; 5], element_bytes: usize) -> Vec<u8> {
        let mut bytes: Vec<u8> = fields
            .iter()
            .flat_map(|field| field.to_le_bytes())
            .collect();
        bytes.resize(HEADER_LEN + element_bytes, 0);
        bytes
    }

    /// Every element of a table, in (x, y, z) loops, with its coordinates.
    fn elements(table: &Table) -> Vec<((i32, i32, i32), i16)> {
        let mut all = Vec::new();
        for z in 0..table.zsize() {
            for y in 0..table.ysize() {
                for x in 0..table.xsize() {
                    all.push(((x, y, z), table.get(x, y, z).unwrap()));
                }
            }
        }
        all
    }

    #[test]
    fn new_table_is_zero() {
        let table = Table::new_3d(4, 3, 2).unwrap();
        let all = elements(&table);
        assert_eq!(all.len(), 24);
        assert!(all.iter().all(|(_, value)| *value == 0));
    }

    #[test]
    fn reading_outside_the_bounds_yields_no_value() {
        let table = Table::new_3d(4, 3, 2).unwrap();
        assert_eq!(table.get(3, 2, 1), Some(0));
        assert_eq!(table.get(4, 0, 0), None);
        assert_eq!(table.get(0, 3, 0), None);
        assert_eq!(table.get(0, 0, 2), None);
        assert_eq!(table.get(-1, 0, 0), None);
        assert_eq!(table.get(0, -1, 0), None);
        assert_eq!(table.get(0, 0, -1), None);
        assert_eq!(table.get(i32::MAX, i32::MIN, 0), None);

        let line = Table::new(17).unwrap();
        assert_eq!(line.get(16, 0, 0), Some(0));
        assert_eq!(line.get(17, 0, 0), None);
        assert_eq!(line.get(0, 1, 0), None);
    }

    #[test]
    fn writing_outside_the_bounds_changes_nothing() {
        let mut table = Table::new_2d(2, 2).unwrap();
        for (index, (x, y)) in [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().enumerate() {
            table.set(x, y, 0, index as i64 + 1);
        }
        let before = table.clone();
        for (x, y, z) in [
            (2, 0, 0),
            (0, 2, 0),
            (0, 0, 1),
            (-1, 0, 0),
            (0, -1, 0),
            (0, 0, -1),
        ] {
            table.set(x, y, z, 99);
        }
        assert_eq!(table, before);
    }

    #[test]
    fn values_wrap_to_16_bits() {
        let mut table = Table::new(4).unwrap();
        table.set(0, 0, 0, 40000);
        table.set(1, 0, 0, -32769);
        table.set(2, 0, 0, 65536 + 7);
        table.set(3, 0, 0, -1);
        assert_eq!(table.get(0, 0, 0), Some(-25536));
        assert_eq!(table.get(1, 0, 0), Some(32767));
        assert_eq!(table.get(2, 0, 0), Some(7));
        assert_eq!(table.get(3, 0, 0), Some(-1));
    }

    #[test]
    fn unused_dimensions_have_size_one() {
        let table = Table::new(17).unwrap();
        assert_eq!(
            (
                table.dimensions(),
                table.xsize(),
                table.ysize(),
                table.zsize()
            ),
            (1, 17, 1, 1)
        );
        let table = Table::new_2d(5, 6).unwrap();
        assert_eq!(
            (
                table.dimensions(),
                table.xsize(),
                table.ysize(),
                table.zsize()
            ),
            (2, 5, 6, 1)
        );
    }

    #[test]
    fn negative_sizes_count_as_zero() {
        let table = Table::new_2d(-3, 4).unwrap();
        assert_eq!((table.xsize(), table.ysize()), (0, 4));
        assert_eq!(table.get(0, 0, 0), None);
    }

    #[test]
    fn a_table_too_large_to_serialize_is_refused() {
        assert_eq!(
            Table::new_3d(65536, 65536, 2),
            Err(TableError::TooLarge {
                xsize: 65536,
                ysize: 65536,
                zsize: 2,
            })
        );
        assert!(Table::new_3d(i32::MAX, i32::MAX, i32::MAX).is_err());

        let mut table = Table::new(3).unwrap();
        table.set(1, 0, 0, 5);
        let before = table.clone();
        assert!(table.resize_2d(i32::MAX, 2).is_err());
        assert_eq!(table, before);
    }

    #[test]
    fn growing_keeps_the_contents() {
        let mut table = Table::new_2d(2, 2).unwrap();
        table.set(0, 0, 0, 11);
        table.set(1, 0, 0, 12);
        table.set(0, 1, 0, 21);
        table.set(1, 1, 0, 22);
        table.resize_2d(3, 3).unwrap();
        assert_eq!(
            elements(&table),
            vec![
                ((0, 0, 0), 11),
                ((1, 0, 0), 12),
                ((2, 0, 0), 0),
                ((0, 1, 0), 21),
                ((1, 1, 0), 22),
                ((2, 1, 0), 0),
                ((0, 2, 0), 0),
                ((1, 2, 0), 0),
                ((2, 2, 0), 0),
            ]
        );
    }

    #[test]
    fn shrinking_drops_the_rest() {
        let mut table = Table::new_2d(3, 3).unwrap();
        for y in 0..3 {
            for x in 0..3 {
                table.set(x, y, 0, i64::from(10 * y + x + 1));
            }
        }
        table.resize_2d(2, 1).unwrap();
        assert_eq!(elements(&table), vec![((0, 0, 0), 1), ((1, 0, 0), 2)]);
        assert_eq!(table.get(2, 0, 0), None);
        assert_eq!(table.get(0, 1, 0), None);
    }

    #[test]
    fn resizing_across_dimension_counts_keeps_shared_coordinates() {
        let mut table = Table::new_2d(2, 2).unwrap();
        table.set(0, 0, 0, 1);
        table.set(1, 1, 0, 4);
        table.resize_3d(2, 2, 2).unwrap();
        assert_eq!(table.dimensions(), 3);
        assert_eq!(table.get(0, 0, 0), Some(1));
        assert_eq!(table.get(1, 1, 0), Some(4));
        assert_eq!(table.get(1, 1, 1), Some(0));
        table.resize(1).unwrap();
        assert_eq!(table.dimensions(), 1);
        assert_eq!(elements(&table), vec![((0, 0, 0), 1)]);
        // Elements dropped by a resize do not come back.
        table.resize_3d(2, 2, 2).unwrap();
        assert_eq!(table.get(1, 1, 0), Some(0));
    }

    #[test]
    fn map_sized_table_serializes_to_1820_bytes() {
        let bytes = Table::new_3d(20, 15, 3).unwrap().to_bytes();
        assert_eq!(bytes.len(), 1820);
        assert_eq!(header(&bytes), [3, 20, 15, 3, 900]);
    }

    #[test]
    fn one_dimensional_header_has_unit_sizes() {
        let bytes = Table::new(17).unwrap().to_bytes();
        assert_eq!(bytes.len(), 20 + 34);
        assert_eq!(header(&bytes), [1, 17, 1, 1, 17]);
    }

    #[test]
    fn elements_are_serialized_with_x_fastest() {
        let mut table = Table::new_2d(2, 2).unwrap();
        table.set(0, 0, 0, 0x0101);
        table.set(1, 0, 0, 0x0202);
        table.set(0, 1, 0, 0x0303);
        table.set(1, 1, 0, -2);
        let bytes = table.to_bytes();
        assert_eq!(bytes[HEADER_LEN..], [1, 1, 2, 2, 3, 3, 0xFE, 0xFF]);

        let mut cube = Table::new_3d(2, 1, 2).unwrap();
        cube.set(1, 0, 0, 1);
        cube.set(0, 0, 1, 2);
        assert_eq!(cube.to_bytes()[HEADER_LEN..], [0, 0, 1, 0, 2, 0, 0, 0]);
    }

    #[test]
    fn inconsistent_header_fails() {
        assert_eq!(
            Table::from_bytes(&raw([2, 2, 2, 1, 5], 10)),
            Err(DecodeError::TableCount {
                xsize: 2,
                ysize: 2,
                zsize: 1,
                count: 5,
            })
        );
        // The product overflows every integer width short of 128 bits.
        assert!(matches!(
            Table::from_bytes(&raw([3, i32::MAX, i32::MAX, i32::MAX, 1], 2)),
            Err(DecodeError::TableCount { .. })
        ));
        assert_eq!(
            Table::from_bytes(&raw([2, -2, -2, 1, 4], 8)),
            Err(DecodeError::TableNegativeSize {
                xsize: -2,
                ysize: -2,
                zsize: 1,
            })
        );
        for dimensions in [0, 4, -1] {
            assert_eq!(
                Table::from_bytes(&raw([dimensions, 1, 1, 1, 1], 2)),
                Err(DecodeError::TableDimensions { found: dimensions })
            );
        }
    }

    #[test]
    fn wrong_lengths_fail() {
        for len in [0, 19] {
            assert_eq!(
                Table::from_bytes(&vec![0; len]),
                Err(DecodeError::TableHeaderTruncated {
                    expected: 20,
                    found: len,
                })
            );
        }
        for element_bytes in [0, 7, 9, 16] {
            assert_eq!(
                Table::from_bytes(&raw([2, 2, 2, 1, 4], element_bytes)),
                Err(DecodeError::Length {
                    what: "table",
                    expected: 28,
                    found: 20 + element_bytes,
                })
            );
        }
        assert!(Table::from_bytes(&raw([2, 2, 2, 1, 4], 8)).is_ok());
    }

    #[test]
    fn writes_through_one_handle_are_seen_through_another() {
        let first = SharedTable::new(Table::new_2d(4, 3).unwrap());
        let second = first.clone();
        assert_eq!(first, second);
        assert_eq!(first.revision(), 0);

        first.set(1, 2, 0, 7);
        assert_eq!(second.get(1, 2, 0), Some(7));
        assert_eq!(second.read().get(1, 2, 0), Some(7));
        assert_eq!((first.revision(), second.revision()), (1, 1));

        second.write(|table| table.resize_2d(8, 8)).unwrap();
        assert_eq!((first.read().xsize(), first.read().ysize()), (8, 8));
        assert_eq!(first.get(1, 2, 0), Some(7));
        assert_eq!(first.revision(), 2);

        // A table of its own is another table, also with the same contents.
        let other = SharedTable::from(first.read().clone());
        assert_ne!(other, first);
        other.set(0, 0, 0, 1);
        assert_eq!(first.get(0, 0, 0), Some(0));
        assert_eq!((other.revision(), first.revision()), (1, 2));
    }

    #[test]
    fn reading_a_shared_table_keeps_its_revision() {
        let table = SharedTable::new(Table::new(4).unwrap());
        table.set(0, 0, 0, 5);
        let revision = table.revision();
        assert_eq!(table.get(0, 0, 0), Some(5));
        assert_eq!(table.get(9, 0, 0), None);
        assert_eq!(table.read().xsize(), 4);
        let _ = table.read().to_bytes();
        let _ = table.clone();
        assert_eq!(table.revision(), revision);
    }

    fn any_table() -> impl Strategy<Value = Table> {
        (1..=3i32, 0..6i32, 0..6i32, 0..6i32)
            .prop_flat_map(|(dimensions, xsize, ysize, zsize)| {
                let (ysize, zsize) = match dimensions {
                    1 => (1, 1),
                    2 => (ysize, 1),
                    _ => (ysize, zsize),
                };
                let count = (xsize * ysize * zsize) as usize;
                (
                    Just((dimensions, xsize, ysize, zsize)),
                    proptest::collection::vec(any::<i16>(), count),
                )
            })
            .prop_map(|((dimensions, xsize, ysize, zsize), data)| Table {
                dimensions,
                xsize,
                ysize,
                zsize,
                data,
            })
    }

    proptest! {
        #[test]
        fn every_table_round_trips(table in any_table()) {
            let bytes = table.to_bytes();
            prop_assert_eq!(bytes.len(), 20 + 2 * elements(&table).len());
            prop_assert_eq!(Table::from_bytes(&bytes), Ok(table));
        }

        #[test]
        fn resize_keeps_exactly_the_shared_elements(
            table in any_table(),
            xsize in 0..6i32,
            ysize in 0..6i32,
            zsize in 0..6i32,
        ) {
            let mut resized = table.clone();
            resized.resize_3d(xsize, ysize, zsize).unwrap();
            prop_assert_eq!(
                (resized.xsize(), resized.ysize(), resized.zsize()),
                (xsize, ysize, zsize)
            );
            for ((x, y, z), value) in elements(&resized) {
                prop_assert_eq!(value, table.get(x, y, z).unwrap_or(0));
            }
        }
    }
}
