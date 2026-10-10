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
// The mix of a transition is integer arithmetic on the frames' 8-bit values, as measured on
// the original runtime. A transition picture replaces the one weight of the mix by a weight
// per texel, taken from the mask bitmap tiled over the newest frame.
//
// The scaling uses floating point. That is acceptable here and nowhere else: what it produces
// is only looked at, never read back into the compositor.

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
    // The weight of the newest frame in a cross-fade, 0 to 255. Only with FLAG_MIX.
    int weight;
    uint flags;
    // With FLAG_MASK: the mask level up to which the newest frame is shown alone, 0 to 255,
    // and the width of the soft edge above it in mask levels.
    int threshold;
    int vagueness;
} constants;

// The bits of `constants.flags` (`FLAG_*`). The build generates this file from
// `src/shared.rs`, which says what each means.
#include "shared.glsl"

// What a texel is read from.
const int NEWEST = 0;
const int HELD = 1;
// Both frames at the same position, mixed. Only with FLAG_SAME_GRID.
const int MIXED = 2;

layout(location = 0) out vec4 color;

// The weight of the held frame, 0 to 255, at the pixel `position` of the newest frame under
// the mask, which is tiled from the frame's corner.
int held_weight(ivec2 position) {
    ivec3 texel = ivec3(texelFetch(mask, position % textureSize(mask, 0), 0).rgb);
    // PLACEHOLDER: floor(0.2989 r + 0.5866 g + 0.1145 b). Exact for a grey mask; the original
    // is one higher for a few colours whose sum lies just below a whole number.
    int level = (2989 * texel.r + 5866 * texel.g + 1145 * texel.b) / 10000;
    if (constants.vagueness == 0) {
        return level >= constants.threshold ? 255 : 0;
    }
    if (level <= constants.threshold) {
        return 0;
    }
    return min(255, (level - constants.threshold) * 255 / constants.vagueness);
}

// The colour of one texel, 0 to 1, clamping the position to a frame of `source` texels.
vec3 texel(int which, ivec2 position, ivec2 source) {
    ivec2 at = clamp(position, ivec2(0), source - 1);
    if (which == NEWEST || (which == MIXED && (constants.flags & FLAG_MIX) == 0u)) {
        return vec3(texelFetch(newest, at, 0).rgb) / 255.0;
    }
    ivec3 before = ivec3(texelFetch(held, at, 0).rgb);
    if (which == HELD) {
        return vec3(before) / 255.0;
    }
    ivec3 after = ivec3(texelFetch(newest, at, 0).rgb);
    if ((constants.flags & FLAG_MASK) == 0u) {
        int weight = constants.weight;
        return vec3((before * (256 - weight) + after * weight) >> 8) / 255.0;
    }
    int weight = held_weight(at);
    if (weight == 0) {
        return vec3(after) / 255.0;
    }
    return vec3(after + (((before - after) * (weight + 1)) >> 8)) / 255.0;
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
        // The frames are scaled separately, so the same weights mix what the scaling gave.
        // What the held frame weighs, and whether the newest frame is shown alone.
        float held_part = 0.0;
        bool alone = (constants.flags & FLAG_MIX) == 0u;
        if (!alone && (constants.flags & FLAG_MASK) == 0u) {
            held_part = float(256 - constants.weight) / 256.0;
        } else if (!alone) {
            // The pixel of the newest frame under this one, also outside its rectangle.
            ivec2 cell = (2 * pixel + 1) * constants.source / (2 * max(constants.size, ivec2(1)));
            int weight = held_weight(clamp(cell, ivec2(0), constants.source - 1));
            alone = weight == 0;
            held_part = float(weight + 1) / 256.0;
        }
        vec3 after = vec3(0.0);
        if (held_part < 1.0 && inside(pixel, constants.size)) {
            after = scaled(NEWEST, pixel, constants.size, constants.source, nearest);
        }
        vec3 before = vec3(0.0);
        ivec2 held_pixel = target - constants.held_origin;
        if (!alone && inside(held_pixel, constants.held_size)) {
            before = scaled(
                HELD,
                held_pixel,
                constants.held_size,
                constants.held_source,
                (constants.flags & FLAG_HELD_NEAREST) != 0u);
        }
        rgb = alone ? after : mix(after, before, held_part);
    }
    if ((constants.flags & FLAG_SRGB_TARGET) != 0u) {
        rgb = srgb_to_linear(rgb);
    }
    color = vec4(rgb, 1.0);
}
