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
