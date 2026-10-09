#version 460
#extension GL_EXT_samplerless_texture_functions : require

// Scales what is shown into the target: the mix of the held and the newest frame, each inside
// its own letterbox rectangle, black outside.
//
// The frames are read as integers with texelFetch, and the flags select one of three filters.
// Nearest: every target pixel takes the texel under its centre. Linear: it interpolates
// between the four texels around its centre. Sharp bilinear, when neither flag is set: the
// interior of each scaled texel keeps its value and only a band one target pixel wide across
// each texel border interpolates.
//
// When the two frames have the same size, which is the usual case, they are mixed texel by
// texel before scaling. A held frame of another size than the newest has its own rectangle;
// the two are then scaled separately and mixed per target pixel.
//
// A transition picture replaces the one weight of the mix by a weight per texel, taken from
// the mask bitmap stretched over the newest frame.
//
// This pass uses floating point. That is acceptable here and nowhere else: what it produces is
// only looked at, never read back into the compositor.

layout(set = 0, binding = 0) uniform utexture2D newest;
layout(set = 0, binding = 1) uniform utexture2D held;
// The transition picture, or one texel that is not read without FLAG_MASK.
layout(set = 0, binding = 2) uniform utexture2D mask;

layout(push_constant) uniform Constants {
    // The letterbox rectangle of the newest frame in target pixels, and its size in texels.
    ivec2 origin;
    ivec2 size;
    ivec2 source;
    // The same for the held frame.
    ivec2 held_origin;
    ivec2 held_size;
    ivec2 held_source;
    // 0 shows the held frame, 1 the newest. With FLAG_MASK it is the progress of the
    // transition.
    float weight;
    uint flags;
    // Width of the soft edge of a masked transition as a fraction of the mask's range.
    float vagueness;
} constants;

// Each target pixel takes one texel of the newest frame: the filter is nearest.
const uint FLAG_NEAREST = 1u;
// The target has an sRGB format, which encodes what is written to it.
const uint FLAG_SRGB_TARGET = 2u;
// The same as FLAG_NEAREST for the held frame.
const uint FLAG_HELD_NEAREST = 4u;
// Both frames have the same size, and so the same rectangle.
const uint FLAG_SAME_GRID = 8u;
// The mix is driven per texel by the mask.
const uint FLAG_MASK = 16u;
// Where a frame is not read with nearest, the filter is linear instead of sharp bilinear.
const uint FLAG_LINEAR = 32u;

// What a texel is read from.
const int NEWEST = 0;
const int HELD = 1;
// Both frames at the same position, mixed. Only with FLAG_SAME_GRID.
const int MIXED = 2;

layout(location = 0) out vec4 color;

// The weight of the newest frame at `position` in a grid of `extent` cells that the newest
// frame fills: the one weight, or with a mask what its texel there gives.
float weight_at(ivec2 position, ivec2 extent) {
    if ((constants.flags & FLAG_MASK) == 0u) {
        return constants.weight;
    }
    // The mask is stretched over the grid; a cell takes the mask texel under its centre.
    ivec2 mask_size = textureSize(mask, 0);
    ivec2 cell = (2 * position + 1) * mask_size / (2 * max(extent, ivec2(1)));
    ivec2 at = clamp(cell, ivec2(0), mask_size - 1);
    // PLACEHOLDER: the formula and the use of the red channel follow mkxp until the original
    // has been measured.
    float level = float(texelFetch(mask, at, 0).r) / 255.0;
    float progress = constants.weight;
    float held_weight =
        (clamp(level, progress, progress + constants.vagueness) - progress) / constants.vagueness;
    return 1.0 - held_weight;
}

// The colour of one texel, 0 to 1, clamping the position to a frame of `source` texels.
vec3 texel(int which, ivec2 position, ivec2 source) {
    ivec2 at = clamp(position, ivec2(0), source - 1);
    // The end points are taken as they are, so a plain frame passes through untouched.
    if (which == NEWEST || (which == MIXED && constants.weight >= 1.0)) {
        return vec3(texelFetch(newest, at, 0).rgb) / 255.0;
    }
    vec3 before = vec3(texelFetch(held, at, 0).rgb);
    if (which == HELD) {
        return before / 255.0;
    }
    float weight = weight_at(at, source);
    if (weight <= 0.0) {
        return before / 255.0;
    }
    vec3 after = vec3(texelFetch(newest, at, 0).rgb);
    if (weight >= 1.0) {
        return after / 255.0;
    }
    return mix(before, after, weight) / 255.0;
}

// The colour at `pixel`, counted from the corner of a rectangle of `size` target pixels that
// shows a frame of `source` texels.
vec3 scaled(int which, ivec2 pixel, ivec2 size, ivec2 source, bool nearest) {
    if (nearest) {
        // The texel under the pixel centre.
        return texel(which, ((2 * pixel + 1) * source) / (2 * size), source);
    }
    vec2 scale = vec2(size) / vec2(source);
    // The pixel centre in texel units, measured from the centre of texel (0, 0).
    vec2 position = (vec2(pixel) + 0.5) / scale - 0.5;
    vec2 base = floor(position);
    vec2 blend = position - base;
    if ((constants.flags & FLAG_LINEAR) == 0u) {
        // Stretching the fraction around one half by the factor leaves a band one target
        // pixel wide to interpolate in; everything else snaps to one of the two texels.
        blend = clamp((blend - 0.5) * max(scale, vec2(1.0)) + 0.5, 0.0, 1.0);
    }
    ivec2 corner = ivec2(base);
    vec3 top = mix(
        texel(which, corner, source),
        texel(which, corner + ivec2(1, 0), source),
        blend.x);
    vec3 bottom = mix(
        texel(which, corner + ivec2(0, 1), source),
        texel(which, corner + ivec2(1, 1), source),
        blend.x);
    return mix(top, bottom, blend.y);
}

bool inside(ivec2 pixel, ivec2 size) {
    return all(greaterThanEqual(pixel, ivec2(0))) && all(lessThan(pixel, size));
}

// The inverse of the encoding an sRGB target applies, so the stored value is the one given.
vec3 srgb_to_linear(vec3 encoded) {
    vec3 low = encoded / 12.92;
    vec3 high = pow((encoded + 0.055) / 1.055, vec3(2.4));
    return mix(high, low, lessThanEqual(encoded, vec3(0.04045)));
}

void main() {
    ivec2 target = ivec2(gl_FragCoord.xy);
    ivec2 pixel = target - constants.origin;
    bool nearest = (constants.flags & FLAG_NEAREST) != 0u;

    // Black wherever neither frame reaches.
    vec3 rgb = vec3(0.0);
    if ((constants.flags & FLAG_SAME_GRID) != 0u) {
        if (inside(pixel, constants.size)) {
            rgb = scaled(MIXED, pixel, constants.size, constants.source, nearest);
        }
    } else {
        // The mask lies over the rectangle of the newest frame.
        float weight = weight_at(pixel, constants.size);
        vec3 after = vec3(0.0);
        if (weight > 0.0 && inside(pixel, constants.size)) {
            after = scaled(NEWEST, pixel, constants.size, constants.source, nearest);
        }
        vec3 before = vec3(0.0);
        ivec2 held_pixel = target - constants.held_origin;
        if (weight < 1.0 && inside(held_pixel, constants.held_size)) {
            before = scaled(
                HELD,
                held_pixel,
                constants.held_size,
                constants.held_source,
                (constants.flags & FLAG_HELD_NEAREST) != 0u);
        }
        if (weight >= 1.0) {
            rgb = after;
        } else if (weight <= 0.0) {
            rgb = before;
        } else {
            rgb = mix(before, after, weight);
        }
    }
    if ((constants.flags & FLAG_SRGB_TARGET) != 0u) {
        rgb = srgb_to_linear(rgb);
    }
    color = vec4(rgb, 1.0);
}
