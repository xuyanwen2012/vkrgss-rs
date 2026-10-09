//! The Ruby class `Table`.

use vkxp_core::Table;
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Raise, Raised, Ruby};

use crate::error::{self, Error};
use crate::guest::Classes;
use crate::rb;
use crate::wrap::{self, Handle, Wrapped};

/// The data of a `Table` object.
///
/// The table is reached only through the object's handle, so that a tilemap can later hold
/// the Ruby object and read the same table a script writes to.
#[derive(Debug, Clone)]
pub(crate) struct TableData {
    table: Table,
}

impl Wrapped for TableData {
    const CLASS_NAME: &'static str = "Table";

    fn class(classes: &Classes) -> VALUE {
        classes.table
    }

    fn blank() -> Self {
        TableData {
            table: Table::new(0).expect("a table without elements can always be made"),
        }
    }
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
    this(ruby, recv)?.with(|data| data.table = table);
    Ok(recv)
}

fn initialize_copy(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let copy = this(ruby, args[0])?.get();
    this(ruby, recv)?.with(|data| *data = copy);
    Ok(recv)
}

/// `resize(xsize, ysize = 1, zsize = 1)`.
fn resize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let sizes = sizes(ruby, args)?;
    this(ruby, recv)?
        .with(|data| match sizes[..] {
            [xsize] => data.table.resize(xsize),
            [xsize, ysize] => data.table.resize_2d(xsize, ysize),
            [xsize, ysize, zsize] => data.table.resize_3d(xsize, ysize, zsize),
            _ => unreachable!("one to three sizes were checked"),
        })
        .map_err(|error| error::to_raise(ruby, Error::Table(error)))?;
    Ok(recv)
}

/// The x, y and z of as many index arguments as the table has dimensions; the dimensions a
/// table does not use are 0. Another number of indices raises `ArgumentError`.
fn indices(
    ruby: &Ruby,
    table: Handle<TableData>,
    args: &[VALUE],
    extra: usize,
) -> Result<[i32; 3], Raise> {
    let dimensions = table.with(|data| data.table.dimensions()) as usize;
    rb::check_count(ruby, args, dimensions + extra, dimensions + extra)?;
    let mut indices = [0; 3];
    for (index, &arg) in indices.iter_mut().zip(&args[..dimensions]) {
        *index = rb::int(ruby, arg)?;
    }
    Ok(indices)
}

/// `[x]`, `[x, y]` or `[x, y, z]`: the element, or `nil` outside the table.
fn element(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let table = this(ruby, recv)?;
    let [x, y, z] = indices(ruby, table, args, 0)?;
    Ok(match table.with(|data| data.table.get(x, y, z)) {
        Some(value) => rb::fix(i32::from(value)),
        None => ffi::Qnil,
    })
}

/// `[x] = value` and so on: stores the value; outside the table nothing changes.
fn set_element(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let table = this(ruby, recv)?;
    let [x, y, z] = indices(ruby, table, args, 1)?;
    let assigned = args[args.len() - 1];
    let value = rb::long(ruby, assigned)?;
    table.with(|data| data.table.set(x, y, z, value));
    Ok(assigned)
}

fn xsize(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::fix(this(ruby, recv)?.with(|data| data.table.xsize())))
}

fn ysize(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::fix(this(ruby, recv)?.with(|data| data.table.ysize())))
}

fn zsize(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::fix(this(ruby, recv)?.with(|data| data.table.zsize())))
}

/// `_dump(depth)`: the serialized form `Marshal` stores.
fn dump(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let bytes = this(ruby, recv)?.with(|data| data.table.to_bytes());
    rb::string_new(ruby, &bytes)
}

/// `_load(string)`: a new object from the serialized form.
fn load(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult {
    let bytes = rb::bytes(ruby, args[0])?;
    let table =
        Table::from_bytes(&bytes).map_err(|error| error::to_raise(ruby, Error::Decode(error)))?;
    wrap::new_object(ruby, TableData { table })
}

/// Defines `Table`.
pub(crate) fn define(ruby: &Ruby) -> Result<VALUE, Raised> {
    let class = wrap::define_class::<TableData>(ruby, c"Table")?;
    rb::method(ruby, class, c"initialize", -1, initialize)?;
    rb::method(ruby, class, c"initialize_copy", 1, initialize_copy)?;
    rb::method(ruby, class, c"resize", -1, resize)?;
    rb::method(ruby, class, c"_dump", 1, dump)?;
    rb::class_method(ruby, class, c"_load", 1, load)?;
    rb::method(ruby, class, c"[]", -1, element)?;
    rb::method(ruby, class, c"[]=", -1, set_element)?;
    rb::method(ruby, class, c"xsize", 0, xsize)?;
    rb::method(ruby, class, c"ysize", 0, ysize)?;
    rb::method(ruby, class, c"zsize", 0, zsize)?;
    Ok(class)
}
