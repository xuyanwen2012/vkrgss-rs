//! The tables the guest has announced.

use std::collections::BTreeMap;

use vkxp_proto::{TableDelta, TableId};

use crate::TableError;

/// One table: signed 16-bit integers with x varying fastest, then y, then z.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    xsize: u32,
    ysize: u32,
    zsize: u32,
    data: Vec<i16>,
}

impl Table {
    pub fn xsize(&self) -> u32 {
        self.xsize
    }

    pub fn ysize(&self) -> u32 {
        self.ysize
    }

    pub fn zsize(&self) -> u32 {
        self.zsize
    }

    /// All elements, `xsize * ysize * zsize` of them.
    pub fn data(&self) -> &[i16] {
        &self.data
    }
}

/// All live tables, driven by [`TableDelta`]s.
#[derive(Debug, Clone, Default)]
pub struct TableStore {
    tables: BTreeMap<TableId, Table>,
}

impl TableStore {
    pub fn new() -> TableStore {
        TableStore::default()
    }

    /// Number of live tables.
    pub fn len(&self) -> usize {
        self.tables.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    /// The table `id` names, or an unknown-table error.
    pub fn get(&self, id: TableId) -> Result<&Table, TableError> {
        self.tables.get(&id).ok_or(TableError::Unknown { id })
    }

    /// Applies one delta. On error the store is unchanged.
    pub fn apply(&mut self, delta: &TableDelta) -> Result<(), TableError> {
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
                let table = Table {
                    xsize,
                    ysize,
                    zsize,
                    data: data.clone(),
                };
                self.tables.insert(id, table);
                Ok(())
            }
            TableDelta::Destroy { id } => {
                self.get(*id)?;
                self.tables.remove(id);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: TableId = TableId::new(3);

    fn set(id: TableId, sizes: (u32, u32, u32), data: Vec<i16>) -> TableDelta {
        TableDelta::Set {
            id,
            xsize: sizes.0,
            ysize: sizes.1,
            zsize: sizes.2,
            data,
        }
    }

    #[test]
    fn a_table_is_created_replaced_and_destroyed() {
        let mut store = TableStore::new();
        assert_eq!(store.get(ID), Err(TableError::Unknown { id: ID }));
        store.apply(&set(ID, (2, 3, 1), vec![1; 6])).unwrap();
        assert_eq!(store.get(ID).unwrap().data(), [1; 6]);
        store.apply(&set(ID, (4, 1, 1), vec![5, 6, 7, 8])).unwrap();
        let table = store.get(ID).unwrap();
        assert_eq!((table.xsize(), table.ysize(), table.zsize()), (4, 1, 1));
        assert_eq!(table.data(), [5, 6, 7, 8]);
        assert_eq!(store.len(), 1);
        store.apply(&TableDelta::Destroy { id: ID }).unwrap();
        assert!(store.is_empty());
        assert_eq!(
            store.apply(&TableDelta::Destroy { id: ID }),
            Err(TableError::Unknown { id: ID })
        );
    }

    #[test]
    fn data_of_another_length_than_the_sizes_say_is_refused() {
        let mut store = TableStore::new();
        store.apply(&set(ID, (1, 1, 1), vec![9])).unwrap();
        for (sizes, len) in [
            ((2, 2, 2), 7),
            ((0, 5, 1), 1),
            ((u32::MAX, u32::MAX, u32::MAX), 0),
        ] {
            let error = store.apply(&set(ID, sizes, vec![0; len])).unwrap_err();
            assert_eq!(
                error,
                TableError::LengthMismatch {
                    id: ID,
                    xsize: sizes.0,
                    ysize: sizes.1,
                    zsize: sizes.2,
                    found: len,
                }
            );
        }
        assert_eq!(store.get(ID).unwrap().data(), [9], "unchanged");
        // A table without elements is a table.
        store.apply(&set(ID, (0, 5, 1), vec![])).unwrap();
        assert_eq!(store.get(ID).unwrap().data(), [0i16; 0]);
    }
}
