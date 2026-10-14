## Context

- The tilemap of RGSS2 (change `draw-the-tilemap-of-rgss2`, archived): the scene extracts two
  `Op::TilemapPlane` ops per tilemap (`crates/vkxp-core/src/tilemap/rgss2.rs`), which the CPU
  reference (`crates/vkxp-compose-ref/src/tilemap_rgss2.rs`, `PlaneStep::apply`) and the
  compute shader (`crates/vkxp-vk/shaders/compose.comp`, `apply_plane`) evaluate per pixel
  from the map, the passages table, the flash table and nine sheets. The numbering, the
  shape tables and the neighbour rules have one Rust definition,
  `vkxp_proto::tiles_rgss2`; the shader has a copy whose tables a test compares.
- A tilemap of RGSS2 is told from one of RGSS1 by `Tilemap::rgss2`, a `Some(TilemapRgss2)`
  that `Scene::create_tilemap_rgss2` sets. The `Rules` of a scene have no value for RGSS3
  and `vkxp-binding` maps `Runtime::Rgss3` to `Rules::Rgss2`.
- The measurement of RGSS3, seen from a clone under `wt/`:
  `../../ref/tools/oracle/measurements/ace-v1-tilemap-REPORT.md`, its executable model
  `ace-v1-tilemap-model.py`, the topics `ace-v1-tilemap-topics.py`, the results
  `ace-v1-tilemap-results.txt`, and the generator of the synthetic sheets
  `vx_v2_tilemap_sheets.py`. The report is the authority for every rule below, and the model
  where the prose of the report can be read in two ways. The measurement of RGSS2 it builds
  on is `vx-v2-tilemap-REPORT.md`.
- No script of RGSS3 can run yet: the interpreter and the guest of RGSS3 are other changes.

### Measured rules (RGSS3, versions 3.0.0.1 and 3.0.1.1 identical)

The same as RGSS2, each established on RGSS3 by the report: tile numbers to sheets and
texels, numbers from 8192; the shape tables `FLOOR`, `WALL`, `FALL` (byte-identical in the
DLL images) and the sixteen blocks of A1; the animation (step = updates / 30, floors
0, 1, 2, 1, waterfalls 0, 1, 2); two planes of 672 by 512 pixels at z 0 and z 200, ordered
as elements with y 0; origin, wrapping, viewport and the latch of the viewport's origin; the
flash; the blend with its odd last pixel; how a table tile is drawn (the filling, the pieces
8 pixels lower, the band of rows up to the next multiple of 8 of the screen y).

What differs, with `flag(n)` the element `n` of the flags table, 0 without a table, for a
negative `n` and for `n` not below the table's x size:

1. **Layers.** `map_data` has four layers. Layers 0, 1, 2 hold tile numbers, drawn in that
   order in a cell. Layer 3 is the shadow mask; a fifth layer is ignored.
2. **Table.** A tile is a table when `2816 <= n < 4352` and `flag(n) & 0x80 != 0`, in any
   layer and on either plane. The column of the autotile does not matter, and the bit does
   nothing on numbers of other sheets.
3. **Star.** A tile is drawn by the upper plane when it lies in layer 2 and
   `flag(n) & 0x10 != 0`. Every other tile is drawn by the lower plane, also a tile of layer
   0 or 1 with the bit (RGSS2: not drawn). `flag(0) = 0x10` does nothing: number 0 draws
   nothing.
4. **Shadow.** No shadow follows from walls. For `s = map[x, y, 3] & 15`, bit `q` of `s`
   darkens the 16 by 16 pixels at `(16 * (q & 1), 16 * (q >> 1))` of the cell: each channel
   `v` becomes `(v * 127) >> 8` (255 gives 126, 200 gives 99, 128 gives 63). Lower plane
   only, after the cell's layer-1 tile, before the edge and before layer 2. It does not
   depend on any tile: an empty cell takes it and what lies under the tilemap is darkened.
   Bits above bit 3 (the region id) do nothing.
5. **Edge under a table.** Lower plane, once per cell, after the shadow and before layer 2:
   rows 88 to 95 of the two lower pieces of a table are drawn into rows 0 to 7 of the cell
   when (a) the layer-1 tile of the map cell above (same x, y - 1, not wrapped) is a table
   whose shape has the lower edge (the lower left piece of the shape has y 80: shapes 28 to
   31, 33, 38 to 41, 43 to 46), (b) the layer-1 tile of the cell is no table, and (c) the
   layer-0 tile of the cell is not from 4352 to 8191. A table in layer 0 or 2 above gives no
   edge. A table in layer 0 of the cell, any tile in its layer 2, and the star bit of the
   table above do not matter.
6. **Visible.** `visible = false` hides both planes and with them the flash; the update
   counter goes on. A refresh (the latch) also runs when `visible` changes.
7. **Undefined in the original** (it reads outside its arrays; section 7 of the report):
   numbers 1024 to 1535 (1024 to 1244 drew nothing, a page with 1245 to 1535 ended both DLLs
   with a page fault); negative numbers (of 30 tried, the five multiples of -256 from -256 to
   -1280 drew the block at (0, 0) of A5, A4, A3, A2, A1 and the others nothing); a map with
   fewer than four layers (three layers drew as with an empty fourth; two layers and one
   dimension drew other things, differently on the two DLLs; one layer faulted on both, two
   dimensions on one).

The order inside a cell of the lower plane is therefore: layer 0, layer 1, shadow, edge,
layer 2, flash. The rows of a table tile that reach below its cell are drawn with the tile
and covered by whatever the cell below draws, as in RGSS2.

## Goals / Non-Goals

**Goals:**

- A plane op of the rule set of RGSS3 is drawn pixel for pixel as the model of the
  measurement draws it, by the CPU reference and by the Vulkan compositor, byte-identical to
  each other.
- The scene extracts a tilemap of RGSS3 as such ops, so that the binding over Ruby 1.9.2
  only has to create it and hand over its tables and sheets.
- Frames of XP and VX do not change.

**Non-Goals:**

- The class `Tilemap` of RGSS3 in `vkxp-binding`: `flags` and `flags=`, the absence of
  `passages`, argument checks, errors, `bitmaps[]` on a disposed tilemap (which faults in
  the original), the method list. Nothing of an interpreter.
- A value of `Rules` for RGSS3, and the sprite, window and plane of RGSS3.
- What the report lists as not tried: `ox` and `oy` near 2^30 drawn; tilemaps under
  `Viewport#flash`, `Graphics.brightness`, a freeze or a transition beyond what follows from
  being elements; screen sizes other than 544 by 416 and 640 by 480; how scripts use
  regions, bush and counter flags.
- `docs/compatibility.md` and any statement that VX Ace runs.

## Decisions

1. **One op, a rule set in it.** `TilemapPlane` gets a field `rules` of a new enum
   `PlaneRules` with the values `Rgss2` and `Rgss3`. No new op kind: nine tenths of the
   evaluation are shared, and a second kind would double the record, the binning and the
   checks. The field `passages` keeps its name and, for `Rgss3`, names the flags table; the
   map table carries the fourth layer as it is, so no further table id is needed.
   Alternative considered: a new op `TilemapPlaneRgss3`; rejected for the duplication.
2. **The protocol version goes up by one** (12 to 13 when this was drafted; one above what
   the branch has if another change raised it first). The rule is that of
   `vkxp_proto::codec`: any change to the message types or their encoding increments
   `PROTOCOL_VERSION`, because the binary form encodes fields by position. Consequences: an
   input recording, which is an encoded stream with the version in its header, is refused
   when an earlier build wrote it, as after every step of the version; the repository
   commits no encoded stream. Save states are copies of the guest process kept in memory
   that end with the run, so they never meet another version. Guest and host are of one
   build. The tests of `codec.rs` that name the number are edited to the new one.
3. **One definition of what differs.** A module `vkxp_proto::tiles_rgss3` holds only what
   RGSS3 adds: the bit of a table (0x80), the number of layers and the shadow layer, whether
   a number with a flags entry is a table, a tile resolved with that answer (the `Tile` of
   `tiles_rgss2` with `table` set by rule 2), and which quarter a shadow bit darkens.
   Numbering, shape tables, animation, flash, `shadow`, `is_wall`, `Tile::texels`,
   `Tile::overhang` and `Tile::edge` are used from `tiles_rgss2`; no table is copied. The
   reference compositor and the scene use these; the shader mirrors them.
4. **Undefined cases, as the report proposes.** Numbers 1024 to 1535 and negative numbers
   draw nothing (the shared `tiles_rgss2::tile` already says so); a layer the map lacks is
   empty, the shadow layer among them, so a map of three layers of a game of RGSS2 style
   draws with no shadow; layers beyond the fourth are not read. Known differences that
   follow: the five negative numbers of rule 7 draw a block on the original and nothing in
   vkxp; maps of one or two layers and of fewer dimensions draw their layers in vkxp where
   the original draws other things or faults; where the original faults, vkxp goes on.
5. **The scene.** `TilemapRgss2` gets the rule set (default `Rgss2`) and
   `Scene::create_tilemap_rgss3(viewport)` creates a tilemap with `Rgss3`. Its field
   `passages` holds the flags table; it is not renamed, so `vkxp-binding` compiles without
   an edit. Extraction, origin, counter, latch and the sending of tables are those of the
   tilemap of RGSS2; the one difference is rule 6: while `visible` is false no plane op is
   emitted. The tables stay held and the counter and the latch go on. `Rules` of the scene
   is not touched: a tilemap of RGSS3 can live in a scene of `Rules::Rgss2`, which is what
   the binding gives a game of RGSS3 today, and the order (z, y, creation) with y 0 for the
   planes is the measured one for RGSS3 too.
6. **The Vulkan record.** The record of a plane keeps its 25 words. The rule set and a
   layer count of up to 4 go into free bits of the word `priority` (today bit 0 the plane,
   bits 1 and 2 the layer count, bits 8 to 19 flash level and animation); the comment in
   `shaders/records.glsl` and `encode_plane` say which. The compositor gets no further
   buffer and the record layout test covers the same fields.
7. **Expected values without data of the original.** Two sources, both independent of the
   compositors of this repository:
   - *Rules restated as expectations.* The scenarios of the delta specs give small scenes
     with concrete tile numbers, flags and layers and the texel or value each named pixel
     must show. The tests build these scenes with the generated sheets and compare pixels
     with `sheet_pixel` of the fixture module, or with the stated arithmetic. Every value in
     the scenarios was derived from the measured rules above and checked against the model
     while this change was drafted.
   - *Digests of the model.* The topics `tables2`, `shadow3`, `edge1` and the first forty
     frames of `rand4` of `ace-v1-tilemap-topics.py` are rebuilt by formula in the fixture
     module, as the topics of RGSS2 were (the generator `PyRandom` and the builders are
     there). Their expected frames are committed as FNV-1a digests in
     `crates/vkxp-compose-ref/tests/data/tilemap-rgss3-digests.txt`, in the format of the
     file of RGSS2. The digests are made by the model of the measurement
     (`Scene.render` of `ace-v1-tilemap-model.py` with the sheets of
     `vx_v2_tilemap_sheets.py`), driven by a script of the flow's own that stays under
     `local/` and is not committed. Running that Python model is allowed; running Wine is
     not. `rand4` was never used to fit the model.
   Only rules, numbers and digests of frames of generated sheets enter the repository:
   never a pixel of a tileset of the run-time package, a byte of a DLL, or map data of a
   game. The topic `real` of the report (maps of Crysalis) is not rebuilt.
8. **XP and VX stay.** No expected value of an existing test is edited.
   `crates/vkxp-compose-ref/tests/data/tilemap-rgss2-digests.txt` stays byte for byte, and
   `crates/vkxp-core/tests/data/tilemap-frames.txt` keeps every line it has (a new test that
   draws frames through the stage of the core tests may add lines of its own). Existing
   tests are edited only
   where a literal of `TilemapPlane` needs the new field, which gets `PlaneRules::Rgss2`.
   The fixture `one_op_of_each_kind` is not changed: there is no new op kind. The exception
   is the number of the protocol version in the tests of `codec.rs`.
9. **Where the decisions are recorded.** `docs/decisions.md`, section "VX Ace": decision 4
   with its known differences. `docs/compatibility.md` is not edited.

### Scenes of the scenarios

The scenarios of `frame-composition` use these words.

- *The small map*: a map table of 4 by 3 cells and four layers, all elements 0 except those
  a scenario names; a plane op of the rule set of RGSS3 with corner (0, 0), first cell
  (0, 0), animation step 0, a clip over the whole frame of 544 by 416, the nine generated
  sheets in which every pixel of every sheet has a colour of its own (`Set::Code` of
  `vkxp_compose_ref::fixtures::tilemap_rgss2`), over an opaque black ground unless a
  scenario names another. Without other words the op is the lower plane and there is no
  flash table.
- Cell (c, r) covers the screen pixels x from 32 c and y from 32 r, 32 each way, at corner
  (0, 0). `A2(35, 88)` is the pixel (35, 88) of the sheet A2; the slots are A1 to A5 = 0 to
  4 and B to E = 5 to 8.
- "Flag" is the element of the flags table at the tile's number; the table has 8192
  elements, all 0 except those named.
- The numbers used: 2844 = the first autotile of A2 (block at (0, 0)) in shape 28, whose
  lower pieces are the lower edge; 2816 the same autotile in shape 0; 2864 and 2892 the
  second autotile of A2 (block at (64, 0)) in shapes 0 and 28; 2912 the third (block at
  (128, 0)) in shape 0; 3180 the autotile of column 7 of A2 (block at (448, 0)) in shape 28;
  4352 and 4367 the first autotile of A3 in shapes 0 and 15; 4380 the same in shape 28;
  1545 the tile of A5 at (32, 32); 37 the tile of B at (160, 128); 300 the tile of C at
  (128, 160); 2048 the first autotile of A1 in shape 0.

## Implementation notes

- Order of evaluation of a pixel of a cell under `Rgss3`, as `PlaneStep::apply` is built
  today: (1) in the first 8 rows of a cell that is not in the plane's first row, the
  overhang of each tile this plane draws of the plane cell above, layer by layer, under the
  band rule (unchanged; "this plane draws" follows rule 3); (2) lower plane: layer 0,
  layer 1, shadow, edge, layer 2; upper plane: the star tiles of layer 2; (3) the flash. The
  edge reads the map cell above without wrapping, the overhang the plane cell above with
  wrapping, as in RGSS2.
- The edge is one transfer of 16 by 8 per lower piece, so the odd-last-pixel rule applies to
  it with a span of 16, as to the pieces of an autotile. The shadow has no such rule.
- A constant shared between `records.rs` and the shader goes into `src/shared.rs` and into
  the list in `build.rs`; a test fails when it is missing there.
- Between task groups 1 and 4 the Vulkan backend evaluates every plane op by the rules of
  RGSS2. Nothing outside tests emits an op of `Rgss3` before the binding does, and group 4
  closes the gap.
- The Vulkan tests of RGSS3 stand beside those of RGSS2
  (`crates/vkxp-vk/src/tests/tilemap_rgss2.rs`: `assert_topic` over `Pair`,
  `generated_planes_match_the_reference`, `clipped_planes_match_the_reference`,
  `every_topic_is_compared`). The generated test of RGSS2 keeps its seed and its cases; the
  one for RGSS3 draws maps of one to five layers, flags tables of any size and none with
  the bits 0x10 and 0x80 common, and elements of layer 3 of any value.
- Edits expected outside the crates' tilemap files, each a few lines:
  `crates/vkxp-proto/src/codec.rs` and `frame.rs` (literals in tests),
  `crates/vkxp-compose-ref/src/fixtures.rs` (none if `tilemap_plane` supplies the field),
  `crates/vkxp-core/src/lib.rs` (documentation and exports). If a file of
  `crates/vkxp-binding` does not compile, the only edit allowed there is the new field with
  `PlaneRules::Rgss2` in a literal, or `..` in a pattern; none is expected.

## Follow-ups

None of these leaves a requirement of this change unmet.

- The binding of RGSS3 (with the binding over Ruby 1.9.2): `Tilemap.new` calls
  `Scene::create_tilemap_rgss3`; `flags` and `flags=` take the place of `passages` and
  `passages=` with the same reference and nil rules, and refresh the tilemap;
  `visible=` already refreshes when the value changes; a write to `bitmaps[]` of a disposed
  tilemap is accepted silently (the original faults); the method list of RGSS3.
- A value of `Rules` for RGSS3 comes with the first change that needs one for sprites,
  windows or input; the tilemap does not need it.
- The five negative numbers that draw a block on the original draw nothing here. If a game
  of the corpus is found to use one, the formula of the model (`tile_pieces`, truncating
  division) reproduces them.
- The frame time of the plane ops with a shadow layer on hardware is not measured.
- Both planes are emitted for every visible tilemap with a map, also an upper plane with
  nothing to draw, as for RGSS2.
- The field `passages` of the op and of the scene holds the flags of RGSS3 under its old
  name; a rename would touch the binding and is left to the binding change.
- Found while implementing: `a_reset_request_starts_the_scripts_over` of
  `crates/vkxp-binding/tests/run_game.rs` failed once in eight runs of that test binary and
  passed in the others. It runs no tilemap and nothing it uses was edited; it is not looked
  into here.
