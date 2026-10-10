// The two records the compositor reads, in an include of their own so that
// `record_layout_test.comp` reads the very declarations `compose.comp` does. The word each
// field is in is named once on the Rust side, in `src/records.rs`; a test compares the two.

#ifndef VKXP_RECORDS_GLSL
#define VKXP_RECORDS_GLSL

#include "transform.glsl"

// One validated op, as `OpRecord` in `src/records.rs` lays it out: 24 words of 32 bits
// (`RECORD_WORDS`), which is also the stride of the array of them in `compose.comp`.
struct OpRecord {
    uint kind;
    // Clip rectangle, already intersected with the screen: x0 <= x < x1, y0 <= y < y1.
    int clip_x0;
    int clip_y0;
    int clip_x1;
    int clip_y1;
    // Quad: bitmap slot and source rectangle, which lies inside the bitmap and is not empty.
    // Tilemap layer: `bitmap` is the index of its record in `tilemaps`.
    uint bitmap;
    int src_x;
    int src_y;
    int src_width;
    int src_height;
    // Quad, affine sampling: the transform. Stretch sampling: the same six words hold the
    // left and top of the destination rectangle (`xx`, `xy`) and the steps along x (`yx` the
    // high word, `yy` the low) and along y (`tx`, `ty`); the clip rectangle is then inside
    // the destination rectangle.
    Transform transform;
    uint opacity;
    uint blend;
    // Red in the lowest byte, then green, blue, alpha. Clear: the colour written. Quad: the
    // colour blended over the sampled texel. Rectangle effect: the colour blended over the pixel.
    uint color;
    // Quad: the tone of the sampled texel. Rectangle effect: the tone of the pixel.
    int tone_red;
    int tone_green;
    int tone_blue;
    uint tone_gray;
    // Quad: how the six words of `transform` are read.
    uint sampling;
};

// What a tilemap layer op reads, as `TilemapRecord` in `src/records.rs` lays it out: 25 words
// of 32 bits (`TILEMAP_WORDS`).
struct TilemapRecord {
    // The first element of the map's layer in `table_words`, and the map's size in cells,
    // neither of which is 0.
    uint map_offset;
    uint map_xsize;
    uint map_ysize;
    // The first priority in `table_words` and how many there are.
    uint priorities_offset;
    uint priorities_count;
    // The priority drawn.
    int priority;
    // The cell under the top-left pixel of the screen, inside the map, and the place of that
    // pixel in the cell, from 0 to 31.
    uint cell_x;
    uint cell_y;
    uint sub_x;
    uint sub_y;
    // Bitmap slots, or `NO_BITMAP` for a sheet the layer goes without. A slot named here
    // holds an image.
    uint tileset;
    uint autotiles[7];
    // The animation frame each autotile is drawn at, with `WHOLE_TILES`.
    uint frames[7];
};

#endif
