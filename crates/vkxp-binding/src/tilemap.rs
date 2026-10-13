//! The Ruby classes `Tilemap` and `TilemapAutotiles`.
//!
//! See [`element`] for what the scene classes share. A tilemap keeps the `Bitmap` object of
//! its tileset and the `Table` objects of its map data, flash data and priorities. A table
//! is shared with the scene (see `table`), so what a script writes through the `Table`
//! object is what the tilemap draws.
//!
//! `tilemap.autotiles` is a `TilemapAutotiles` object made with the tilemap. It keeps the
//! seven `Bitmap` objects in instance variables of its own and passes each assignment on to
//! the tilemap in the scene.
//!
//! In a game of RGSS2 the class is the one measured on the original runtime of VX: it has
//! `bitmaps`, a `Tilemap::BitmapArray` of nine slots, and `passages`, and neither `tileset`
//! nor `autotiles` nor `priorities`; `TilemapAutotiles` is not defined. The bitmap array is
//! the same kind of object as the autotile list, with nine slots. Every write and every
//! update refreshes the tilemap in the scene (`Scene::refresh_tilemap`), which only a
//! tilemap of RGSS2 notices.

use std::ffi::CStr;

use vkxp_core::{SharedTable, Tilemap, TilemapId};
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Raise, Raised, Ruby};
use vkxp_vfs::Runtime;

use crate::element::{self, kept, kept_after_dispose, reader, writer};
use crate::guest::{Classes, Guest};
use crate::rb;
use crate::table::table_arg;
use crate::wrap::{self, Wrapped};

/// The instance variable that holds the tilemap's `Bitmap` object, or `nil`.
const TILESET: &CStr = c"tileset";
/// The instance variable that holds the tilemap's `TilemapAutotiles` object.
const AUTOTILES: &CStr = c"autotiles";
/// The instance variables that hold the tilemap's `Table` objects, or `nil`.
const MAP_DATA: &CStr = c"map_data";
const FLASH_DATA: &CStr = c"flash_data";
const PRIORITIES: &CStr = c"priorities";

/// The instance variable that holds the `Tilemap::BitmapArray` object of a tilemap of RGSS2.
const BITMAPS: &CStr = c"bitmaps";
/// The instance variable that holds the passages `Table` object of a tilemap of RGSS2, or
/// `nil`.
const PASSAGES: &CStr = c"passages";

/// The instance variable of a `TilemapAutotiles` object that holds its `Tilemap` object.
const TILEMAP: &CStr = c"tilemap";
/// The instance variables of a `TilemapAutotiles` object that hold its `Bitmap` objects,
/// or `nil`.
const AUTOTILE: [&CStr; 7] = [
    c"autotile0",
    c"autotile1",
    c"autotile2",
    c"autotile3",
    c"autotile4",
    c"autotile5",
    c"autotile6",
];

/// The instance variables of a `Tilemap::BitmapArray` object that hold its `Bitmap` objects,
/// or `nil`.
const SHEET: [&CStr; 9] = [
    c"bitmap0", c"bitmap1", c"bitmap2", c"bitmap3", c"bitmap4", c"bitmap5", c"bitmap6", c"bitmap7",
    c"bitmap8",
];

/// The data of a `TilemapAutotiles` object, and of a `Tilemap::BitmapArray` object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Autotiles {
    /// The tilemap whose autotiles these are; `None` for an object a script allocated.
    tilemap: Option<TilemapId>,
}

impl Wrapped for Autotiles {
    const CLASS_NAME: &'static str = "TilemapAutotiles";

    fn class(classes: &Classes) -> VALUE {
        classes.scene.autotiles
    }

    fn blank() -> Self {
        Autotiles { tilemap: None }
    }
}

/// `initialize(viewport = nil)`.
fn initialize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let viewport = element::viewport_arg(ruby, args)?;
    let id = element::create(ruby, recv, args.first(), |scene| {
        scene.create_tilemap(viewport)
    })?;
    let autotiles = wrap::new_object(ruby, Autotiles { tilemap: Some(id) })?;
    rb::ivar_set(ruby, recv, AUTOTILES, autotiles)?;
    // The list keeps its tilemap alive, as the tilemap keeps the list.
    rb::ivar_set(ruby, autotiles, TILEMAP, recv)?;
    Ok(recv)
}

/// `initialize(viewport = nil)` in RGSS2.
fn initialize_rgss2(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let viewport = element::viewport_arg(ruby, args)?;
    let id = element::create(ruby, recv, args.first(), |scene| {
        scene.create_tilemap_rgss2(viewport)
    })?;
    let bitmaps = wrap::new_object(ruby, Autotiles { tilemap: Some(id) })?;
    rb::ivar_set(ruby, recv, BITMAPS, bitmaps)?;
    // The array keeps its tilemap alive, as the tilemap keeps the array.
    rb::ivar_set(ruby, bitmaps, TILEMAP, recv)?;
    Ok(recv)
}

fn update(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let id = element::id::<TilemapId>(ruby, recv)?;
    element::operate(ruby, |guest| {
        guest.scene.tilemap_mut(id)?.update();
        guest.scene.refresh_tilemap(id)
    })?;
    Ok(ffi::Qnil)
}

kept!(tileset, TilemapId, TILESET);

/// `tileset = bitmap_or_nil`: keeps the object.
fn set_tileset(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let id = element::live::<TilemapId>(ruby, recv)?;
    let bitmap = element::bitmap_ref(ruby, args[0])?;
    element::operate(ruby, |guest| {
        guest.scene.tilemap_mut(id)?.tileset = bitmap;
        Ok(())
    })?;
    rb::ivar_set(ruby, recv, TILESET, args[0])?;
    Ok(args[0])
}

kept_after_dispose!(autotiles, TilemapId, AUTOTILES);
kept_after_dispose!(bitmaps, TilemapId, BITMAPS);
kept!(passages, TilemapId, PASSAGES);
kept!(map_data, TilemapId, MAP_DATA);
kept!(flash_data, TilemapId, FLASH_DATA);
kept!(priorities, TilemapId, PRIORITIES);

/// Assigns a `Table` object, or `nil`, to the table property kept in `variable`: the object
/// is kept and `store` gives the scene the table it shares.
fn set_table(
    ruby: &Ruby,
    recv: VALUE,
    assigned: VALUE,
    variable: &CStr,
    store: impl FnOnce(&mut Tilemap, Option<SharedTable>),
) -> HostResult {
    let id = element::live::<TilemapId>(ruby, recv)?;
    let table = if ffi::nil_p(assigned) {
        None
    } else {
        Some(table_arg(ruby, assigned)?)
    };
    element::operate(ruby, |guest| {
        store(guest.scene.tilemap_mut(id)?, table);
        guest.scene.refresh_tilemap(id)
    })?;
    rb::ivar_set(ruby, recv, variable, assigned)?;
    Ok(assigned)
}

/// Raises the `TypeError` of the original runtime of VX when `nil` is assigned to a table
/// property that holds a table: once set, the map data and the passages cannot be taken
/// away. The property stays as it is.
fn keep_table(ruby: &Ruby, recv: VALUE, assigned: VALUE, variable: &CStr) -> Result<(), Raise> {
    element::live::<TilemapId>(ruby, recv)?;
    if ffi::nil_p(assigned) && !ffi::nil_p(rb::ivar_get(ruby, recv, variable)?) {
        let message = "can't convert NilClass into Table";
        return Err(Raise::new(rb::type_error(ruby), message));
    }
    Ok(())
}

fn set_map_data_rgss2(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    keep_table(ruby, recv, args[0], MAP_DATA)?;
    set_map_data(ruby, recv, args)
}

fn set_passages(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    keep_table(ruby, recv, args[0], PASSAGES)?;
    set_table(ruby, recv, args[0], PASSAGES, |tilemap, table| {
        if let Some(rgss2) = &mut tilemap.rgss2 {
            rgss2.passages = table;
        }
    })
}

fn set_map_data(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    set_table(ruby, recv, args[0], MAP_DATA, |tilemap, table| {
        tilemap.map_data = table;
    })
}

fn set_flash_data(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    set_table(ruby, recv, args[0], FLASH_DATA, |tilemap, table| {
        tilemap.flash_data = table;
    })
}

fn set_priorities(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    set_table(ruby, recv, args[0], PRIORITIES, |tilemap, table| {
        tilemap.priorities = table;
    })
}

reader!(visible, TilemapId, |guest, id| guest
    .scene
    .tilemap(id)?
    .visible);
writer!(set_visible, TilemapId, |guest, id, visible: bool| {
    let tilemap = guest.scene.tilemap_mut(id)?;
    let changed = tilemap.visible != visible;
    tilemap.visible = visible;
    // As measured on the original runtime of VX, only a write that changes it refreshes.
    if changed {
        guest.scene.refresh_tilemap(id)?;
    }
});
reader!(ox, TilemapId, |guest, id| guest.scene.tilemap(id)?.ox);
writer!(set_ox, TilemapId, |guest, id, ox: i32| {
    guest.scene.tilemap_mut(id)?.ox = ox;
    guest.scene.refresh_tilemap(id)?;
});
reader!(oy, TilemapId, |guest, id| guest.scene.tilemap(id)?.oy);
writer!(set_oy, TilemapId, |guest, id, oy: i32| {
    guest.scene.tilemap_mut(id)?.oy = oy;
    guest.scene.refresh_tilemap(id)?;
});

/// The position of an autotile index argument in the list, `None` outside 0 to 6.
fn autotile_slot(ruby: &Ruby, index: VALUE) -> Result<Option<usize>, Raise> {
    let index = rb::int(ruby, index)?;
    Ok(usize::try_from(index)
        .ok()
        .filter(|&slot| slot < AUTOTILE.len()))
}

/// `autotiles[index]`: the `Bitmap` object assigned there, or `nil`.
fn autotile(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    wrap::expect::<Autotiles>(ruby, recv)?;
    match autotile_slot(ruby, args[0])? {
        Some(slot) => Ok(rb::ivar_get(ruby, recv, AUTOTILE[slot])?),
        None => Ok(ffi::Qnil),
    }
}

/// `autotiles[index] = bitmap_or_nil`: keeps the object and gives the tilemap its bitmap.
/// An index outside 0 to 6 changes nothing, and so does a tilemap that is disposed.
fn set_autotile(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let tilemap = wrap::expect::<Autotiles>(ruby, recv)?.get().tilemap;
    let slot = autotile_slot(ruby, args[0])?;
    let bitmap = element::bitmap_ref(ruby, args[1])?;
    let Some(slot) = slot else {
        return Ok(args[1]);
    };
    if let Some(id) = tilemap {
        Guest::with(|guest| {
            if let Ok(tilemap) = guest.scene.tilemap_mut(id) {
                tilemap.autotiles[slot] = bitmap;
            }
        });
    }
    rb::ivar_set(ruby, recv, AUTOTILE[slot], args[1])?;
    Ok(args[1])
}

/// The slot of a `Tilemap::BitmapArray` object an index argument names, `None` outside 0 to
/// 8. An object a script made itself has no slots and raises the `TypeError` of the original.
fn sheet_slot(ruby: &Ruby, recv: VALUE, index: VALUE) -> Result<Option<usize>, Raise> {
    if wrap::expect::<Autotiles>(ruby, recv)?
        .get()
        .tilemap
        .is_none()
    {
        let message = "wrong argument type Tilemap::BitmapArray (expected Data)";
        return Err(Raise::new(rb::type_error(ruby), message));
    }
    let index = rb::int(ruby, index)?;
    Ok(usize::try_from(index)
        .ok()
        .filter(|&slot| slot < SHEET.len()))
}

/// `bitmaps[index]`: the `Bitmap` object assigned there, or `nil`.
fn sheet(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    match sheet_slot(ruby, recv, args[0])? {
        Some(slot) => Ok(rb::ivar_get(ruby, recv, SHEET[slot])?),
        None => Ok(ffi::Qnil),
    }
}

/// `bitmaps[index] = bitmap_or_nil`: keeps the object, gives the tilemap its bitmap and
/// refreshes it. An index outside 0 to 8 stores nothing; a disposed tilemap leaves the
/// array working, as measured on the original runtime of VX.
fn set_sheet(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let slot = sheet_slot(ruby, recv, args[0])?;
    let bitmap = element::bitmap_ref(ruby, args[1])?;
    let Some(slot) = slot else {
        return Ok(args[1]);
    };
    if let Some(id) = wrap::expect::<Autotiles>(ruby, recv)?.get().tilemap {
        Guest::with(|guest| {
            let tilemap = guest.scene.tilemap_mut(id).ok();
            if let Some(rgss2) = tilemap.and_then(|tilemap| tilemap.rgss2.as_mut()) {
                rgss2.bitmaps[slot] = bitmap;
                // The tilemap was just found.
                let _ = guest.scene.refresh_tilemap(id);
            }
        });
    }
    rb::ivar_set(ruby, recv, SHEET[slot], args[1])?;
    Ok(args[1])
}

/// Defines `Tilemap` as `runtime` has it: with the tileset, the autotiles and the
/// priorities in RGSS1; with `viewport=`, `bitmaps` and `passages` in RGSS2.
pub(crate) fn define(ruby: &Ruby, runtime: Runtime) -> Result<VALUE, Raised> {
    let class = element::define_class::<TilemapId>(ruby, c"Tilemap")?;
    element::define_in_viewport::<TilemapId>(ruby, class, runtime)?;
    rb::method(ruby, class, c"update", 0, update)?;
    if runtime == Runtime::Rgss1 {
        rb::method(ruby, class, c"initialize", -1, initialize)?;
        rb::method(ruby, class, c"tileset", 0, tileset)?;
        rb::method(ruby, class, c"tileset=", 1, set_tileset)?;
        rb::method(ruby, class, c"autotiles", 0, autotiles)?;
        rb::method(ruby, class, c"map_data=", 1, set_map_data)?;
        rb::method(ruby, class, c"priorities", 0, priorities)?;
        rb::method(ruby, class, c"priorities=", 1, set_priorities)?;
    } else {
        rb::method(ruby, class, c"initialize", -1, initialize_rgss2)?;
        rb::method(ruby, class, c"bitmaps", 0, bitmaps)?;
        rb::method(ruby, class, c"map_data=", 1, set_map_data_rgss2)?;
        rb::method(ruby, class, c"passages", 0, passages)?;
        rb::method(ruby, class, c"passages=", 1, set_passages)?;
    }
    rb::method(ruby, class, c"map_data", 0, map_data)?;
    rb::method(ruby, class, c"flash_data", 0, flash_data)?;
    rb::method(ruby, class, c"flash_data=", 1, set_flash_data)?;
    rb::method(ruby, class, c"visible", 0, visible)?;
    rb::method(ruby, class, c"visible=", 1, set_visible)?;
    rb::method(ruby, class, c"ox", 0, ox)?;
    rb::method(ruby, class, c"ox=", 1, set_ox)?;
    rb::method(ruby, class, c"oy", 0, oy)?;
    rb::method(ruby, class, c"oy=", 1, set_oy)?;
    Ok(class)
}

/// Defines the class of the list of sheets a tilemap of `runtime` has: `TilemapAutotiles`
/// in RGSS1, `Tilemap::BitmapArray` in RGSS2. `Tilemap` must be defined.
pub(crate) fn define_autotiles(ruby: &Ruby, runtime: Runtime) -> Result<VALUE, Raised> {
    if runtime == Runtime::Rgss1 {
        let class = wrap::define_class::<Autotiles>(ruby, c"TilemapAutotiles")?;
        rb::method(ruby, class, c"[]", 1, autotile)?;
        rb::method(ruby, class, c"[]=", 2, set_autotile)?;
        return Ok(class);
    }
    let tilemap = rb::class_at(ruby, c"Tilemap")?;
    let class = wrap::define_class_under::<Autotiles>(ruby, tilemap, c"BitmapArray")?;
    rb::method(ruby, class, c"[]", 1, sheet)?;
    rb::method(ruby, class, c"[]=", 2, set_sheet)?;
    Ok(class)
}
