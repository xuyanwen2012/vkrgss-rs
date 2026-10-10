//! The tables the guest has announced, and where each lies in the buffer the shader reads.
//!
//! All tables live in one storage buffer of 32-bit integers, one after the other in the order
//! of their ids, each 16-bit element widened. This module has no Vulkan types: it keeps the
//! elements and the layout, and the compositor writes the buffer anew when a table changed.
//! The rules and errors are those of the reference compositor's store.

use std::collections::BTreeMap;

use vkxp_proto::{TableDelta, TableId};

use crate::TableError;

/// One table: its sizes, its elements and its place in the buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub xsize: u32,
    pub ysize: u32,
    pub zsize: u32,
    data: Vec<i16>,
    /// Index of the table's first element in the buffer.
    pub offset: u64,
}

impl Entry {
    /// Number of elements.
    pub(crate) fn len(&self) -> u64 {
        self.data.len() as u64
    }
}

/// All live tables, driven by [`TableDelta`]s.
#[derive(Debug, Clone, Default)]
pub(crate) struct TableStore {
    tables: BTreeMap<TableId, Entry>,
    /// Number of elements of all tables.
    words: u64,
    /// Counts the changes to what the buffer holds.
    revision: u64,
}

impl TableStore {
    pub(crate) fn new() -> TableStore {
        TableStore::default()
    }

    /// The table `id` names, or an unknown-table error.
    pub(crate) fn get(&self, id: TableId) -> Result<&Entry, TableError> {
        self.tables.get(&id).ok_or(TableError::Unknown { id })
    }

    /// Number of elements of all tables: the length of the buffer in words.
    pub(crate) fn words(&self) -> u64 {
        self.words
    }

    /// A number that differs whenever the buffer's contents may.
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    /// The elements of all tables in buffer order, widened to the words the shader reads.
    pub(crate) fn buffer_words(&self) -> impl Iterator<Item = u32> {
        let elements = self.tables.values().flat_map(|entry| &entry.data);
        // Sign-extended: the shader reads signed integers.
        elements.map(|&element| i32::from(element) as u32)
    }

    /// Checks one delta and applies it. On error the store is unchanged.
    pub(crate) fn apply(&mut self, delta: &TableDelta) -> Result<(), TableError> {
        match delta {
            TableDelta::Set {
                id,
                xsize,
                ysize,
                zsize,
                data,
            } => {
                let (id, xsize, ysize, zsize) = (*id, *xsize, *ysize, *zsize);
                // Three `u32` multiply to less than 2^96.
                let expected = u128::from(xsize) * u128::from(ysize) * u128::from(zsize);
                if data.len() as u128 != expected {
                    return Err(TableError::LengthMismatch {
                        id,
                        xsize,
                        ysize,
                        zsize,
                        found: data.len(),
                    });
                }
                let entry = Entry {
                    xsize,
                    ysize,
                    zsize,
                    data: data.clone(),
                    offset: 0,
                };
                self.tables.insert(id, entry);
            }
            TableDelta::Destroy { id } => {
                self.get(*id)?;
                self.tables.remove(id);
            }
        }
        self.words = 0;
        for entry in self.tables.values_mut() {
            entry.offset = self.words;
            self.words += entry.len();
        }
        self.revision += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(id: u32, sizes: (u32, u32, u32), data: Vec<i16>) -> TableDelta {
        TableDelta::Set {
            id: TableId::new(id),
            xsize: sizes.0,
            ysize: sizes.1,
            zsize: sizes.2,
            data,
        }
    }

    #[test]
    fn tables_lie_in_the_buffer_in_the_order_of_their_ids() {
        let mut store = TableStore::new();
        store.apply(&set(7, (2, 1, 1), vec![-1, 2])).unwrap();
        store
            .apply(&set(3, (1, 3, 1), vec![5, 6, i16::MIN]))
            .unwrap();
        let offsets = |store: &TableStore| {
            [3, 7].map(|id| store.get(TableId::new(id)).map(|entry| entry.offset))
        };
        assert_eq!(offsets(&store), [Ok(0), Ok(3)]);
        assert_eq!(store.words(), 5);
        let words: Vec<u32> = store.buffer_words().collect();
        assert_eq!(words, [5, 6, 0xFFFF_8000, 0xFFFF_FFFF, 2]);

        // Replaced by a longer one: what follows moves.
        let revision = store.revision();
        store.apply(&set(3, (4, 1, 1), vec![1; 4])).unwrap();
        assert_eq!(offsets(&store), [Ok(0), Ok(4)]);
        assert_ne!(store.revision(), revision);
        store
            .apply(&TableDelta::Destroy {
                id: TableId::new(3),
            })
            .unwrap();
        assert_eq!(store.get(TableId::new(7)).unwrap().offset, 0);
        assert_eq!(store.words(), 2);
    }

    #[test]
    fn refused_deltas_change_nothing() {
        let mut store = TableStore::new();
        store.apply(&set(1, (2, 2, 1), vec![4; 4])).unwrap();
        let revision = store.revision();
        let id = TableId::new(1);
        assert_eq!(
            store.apply(&set(1, (2, 2, 2), vec![4; 7])),
            Err(TableError::LengthMismatch {
                id,
                xsize: 2,
                ysize: 2,
                zsize: 2,
                found: 7,
            })
        );
        let unknown = TableId::new(9);
        assert_eq!(
            store.apply(&TableDelta::Destroy { id: unknown }),
            Err(TableError::Unknown { id: unknown })
        );
        assert_eq!(
            store.get(unknown).err(),
            Some(TableError::Unknown { id: unknown })
        );
        assert_eq!(store.revision(), revision);
        assert_eq!(store.get(id).unwrap().len(), 4);
    }
}
