//! The Ruby class `Table`.
//!
//! A game of RGSS3 gets other functions for `[]` and `[]=`, which word a wrong count of
//! indices as the original runtime of RGSS3 does, the number of dimensions first. The rest
//! of the class is that of RGSS2.

use vkxp_core::{SharedTable, Table};
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Raise, Raised, Ruby};
use vkxp_vfs::Runtime;

use crate::error::{self, Error};
use crate::guest::Classes;
use crate::rb;
use crate::wrap::{self, Handle, Wrapped};

/// The data of a `Table` object.
///
/// The table is shared: a tilemap the object is assigned to holds the same table, and so
/// reads what a script writes through the object.
#[derive(Debug)]
pub(crate) struct TableData {
    table: SharedTable,
}

impl Wrapped for TableData {
    const CLASS_NAME: &'static str = "Table";

    fn class(classes: &Classes) -> VALUE {
        classes.table
    }

    fn blank() -> Self {
        let empty = Table::new(0).expect("a table without elements can always be made");
        TableData {
            table: SharedTable::new(empty),
        }
    }
}

/// The table of a `Table` argument, shared with the object. Anything else raises
/// `TypeError`.
pub(crate) fn table_arg(ruby: &Ruby, value: VALUE) -> Result<SharedTable, Raise> {
    Ok(this(ruby, value)?.with(|data| data.table.clone()))
}

fn this(ruby: &Ruby, recv: VALUE) -> Result<Handle<TableData>, Raise> {
    wrap::expect(ruby, recv)
}

/// One to three sizes given as arguments.
fn sizes(ruby: &Ruby, args: &[VALUE]) -> Result<Vec<i32>, Raise> {
    rb::check_count(ruby, args, 1, 3)?;
    args.iter().map(|&arg| rb::int(ruby, arg)).collect()
}

/// `initialize(xsize, ysize = 1, zsize = 1)`.
fn initialize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let table = match sizes(ruby, args)?[..] {
        [xsize] => Table::new(xsize),
        [xsize, ysize] => Table::new_2d(xsize, ysize),
        [xsize, ysize, zsize] => Table::new_3d(xsize, ysize, zsize),
        _ => unreachable!("one to three sizes were checked"),
    }
    .map_err(|error| error::to_raise(ruby, Error::Table(error)))?;
    this(ruby, recv)?.with(|data| data.table.write(|own| *own = table));
    Ok(recv)
}

fn initialize_copy(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    // A table of its own with the original's elements, not a second handle to the original.
    let copy = this(ruby, args[0])?.with(|data| data.table.read().clone());
    this(ruby, recv)?.with(|data| data.table.write(|own| *own = copy));
    Ok(recv)
}

/// `resize(xsize, ysize = 1, zsize = 1)`.
fn resize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let sizes = sizes(ruby, args)?;
    this(ruby, recv)?
        .with(|data| {
            data.table.write(|table| match sizes[..] {
                [xsize] => table.resize(xsize),
                [xsize, ysize] => table.resize_2d(xsize, ysize),
                [xsize, ysize, zsize] => table.resize_3d(xsize, ysize, zsize),
                _ => unreachable!("one to three sizes were checked"),
            })
        })
        .map_err(|error| error::to_raise(ruby, Error::Table(error)))?;
    Ok(recv)
}

/// The number of dimensions of the table.
fn dimensions(table: Handle<TableData>) -> usize {
    table.with(|data| data.table.read().dimensions()) as usize
}

/// The x, y and z of the first index arguments, as many as the table has dimensions; the
/// dimensions a table does not use are 0. The caller has checked the count.
fn indices(ruby: &Ruby, table: Handle<TableData>, args: &[VALUE]) -> Result<[i32; 3], Raise> {
    let mut indices = [0; 3];
    for (index, &arg) in indices.iter_mut().zip(&args[..dimensions(table)]) {
        *index = rb::int(ruby, arg)?;
    }
    Ok(indices)
}

/// `[x]`, `[x, y]` or `[x, y, z]`: the element, or `nil` outside the table. Another number
/// of indices raises `ArgumentError`.
fn element(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let table = this(ruby, recv)?;
    let dimensions = dimensions(table);
    rb::check_count(ruby, args, dimensions, dimensions)?;
    element_at(ruby, table, args)
}

/// `[]` of RGSS3: as [`element`], with the count message of the original runtime of RGSS3.
fn element_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let table = this(ruby, recv)?;
    let dimensions = dimensions(table);
    rb::check_count_rgss3(ruby, args, dimensions, dimensions)?;
    element_at(ruby, table, args)
}

fn element_at(ruby: &Ruby, table: Handle<TableData>, args: &[VALUE]) -> HostResult {
    let [x, y, z] = indices(ruby, table, args)?;
    Ok(match table.with(|data| data.table.get(x, y, z)) {
        Some(value) => rb::fix(i32::from(value)),
        None => ffi::Qnil,
    })
}

/// `[x] = value` and so on: stores the value; outside the table nothing changes. Another
/// number of indices raises `ArgumentError`.
fn set_element(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let table = this(ruby, recv)?;
    let count = dimensions(table) + 1;
    rb::check_count(ruby, args, count, count)?;
    set_element_at(ruby, table, args)
}

/// `[]=` of RGSS3: as [`set_element`], with the count message of the original runtime of
/// RGSS3, which expects the number of dimensions when fewer arguments are given.
fn set_element_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let table = this(ruby, recv)?;
    let dimensions = dimensions(table);
    let count = if args.len() < dimensions {
        dimensions
    } else {
        dimensions + 1
    };
    rb::check_count_rgss3(ruby, args, count, count)?;
    set_element_at(ruby, table, args)
}

fn set_element_at(ruby: &Ruby, table: Handle<TableData>, args: &[VALUE]) -> HostResult {
    let [x, y, z] = indices(ruby, table, args)?;
    let assigned = args[args.len() - 1];
    let value = rb::long(ruby, assigned)?;
    table.with(|data| data.table.set(x, y, z, value));
    Ok(assigned)
}

fn xsize(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::fix(
        this(ruby, recv)?.with(|data| data.table.read().xsize()),
    ))
}

fn ysize(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::fix(
        this(ruby, recv)?.with(|data| data.table.read().ysize()),
    ))
}

fn zsize(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::fix(
        this(ruby, recv)?.with(|data| data.table.read().zsize()),
    ))
}

/// `_dump(depth)`: the serialized form `Marshal` stores.
fn dump(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let bytes = this(ruby, recv)?.with(|data| data.table.read().to_bytes());
    Ok(rb::bytes_new(ruby, &bytes)?)
}

/// `_load(string)`: a new object from the serialized form.
fn load(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult {
    let bytes = rb::bytes(ruby, args[0])?;
    let table =
        Table::from_bytes(&bytes).map_err(|error| error::to_raise(ruby, Error::Decode(error)))?;
    let table = SharedTable::new(table);
    wrap::new_object(ruby, TableData { table })
}

/// Defines `Table` as a game of `runtime` has it.
pub(crate) fn define(ruby: &Ruby, runtime: Runtime) -> Result<VALUE, Raised> {
    let class = wrap::define_class::<TableData>(ruby, c"Table")?;
    rb::method(ruby, class, c"initialize", -1, initialize)?;
    rb::method(ruby, class, c"initialize_copy", 1, initialize_copy)?;
    rb::method(ruby, class, c"resize", -1, resize)?;
    rb::method(ruby, class, c"_dump", 1, dump)?;
    rb::class_method(ruby, class, c"_load", 1, load)?;
    if runtime == Runtime::Rgss3 {
        rb::method(ruby, class, c"[]", -1, element_rgss3)?;
        rb::method(ruby, class, c"[]=", -1, set_element_rgss3)?;
    } else {
        rb::method(ruby, class, c"[]", -1, element)?;
        rb::method(ruby, class, c"[]=", -1, set_element)?;
    }
    rb::method(ruby, class, c"xsize", 0, xsize)?;
    rb::method(ruby, class, c"ysize", 0, ysize)?;
    rb::method(ruby, class, c"zsize", 0, zsize)?;
    Ok(class)
}
