#version 460
#extension GL_EXT_samplerless_texture_functions : require

// Scales the frame into the target: the mix of the held and the newest frame inside the
// letterbox rectangle, black outside it.
//
// The frames are read as integers with texelFetch. At a whole-number factor every target pixel
// takes exactly one texel. At other factors the sampling is "sharp bilinear": the interior of
// each scaled texel keeps its value and only a band one target pixel wide across each texel
// border interpolates.
//
// This pass uses floating point. That is acceptable here and nowhere else: what it produces is
// only looked at, never read back into the compositor.

layout(set = 0, binding = 0) uniform utexture2D newest;
layout(set = 0, binding = 1) uniform utexture2D held;

layout(push_constant) uniform Constants {
    // The letterbox rectangle in target pixels.
    ivec2 origin;
    ivec2 size;
    // Size of both frames in texels.
    ivec2 source;
    // 0 shows the held frame, 1 the newest.
    float weight;
    uint flags;
} constants;

// Each target pixel takes one texel: the factor is a whole number.
const uint FLAG_NEAREST = 1u;
// The target has an sRGB format, which encodes what is written to it.
const uint FLAG_SRGB_TARGET = 2u;

layout(location = 0) out vec4 color;

// The mixed colour of one texel, 0 to 1, clamping the position to the frame.
vec3 texel(ivec2 position) {
    ivec2 at = clamp(position, ivec2(0), constants.source - 1);
    // The end points are taken as they are, so a plain frame passes through untouched.
    if (constants.weight >= 1.0) {
        return vec3(texelFetch(newest, at, 0).rgb) / 255.0;
    }
    vec3 before = vec3(texelFetch(held, at, 0).rgb);
    if (constants.weight <= 0.0) {
        return before / 255.0;
    }
    vec3 after = vec3(texelFetch(newest, at, 0).rgb);
    return mix(before, after, constants.weight) / 255.0;
}

// The inverse of the encoding an sRGB target applies, so the stored value is the one given.
vec3 srgb_to_linear(vec3 encoded) {
    vec3 low = encoded / 12.92;
    vec3 high = pow((encoded + 0.055) / 1.055, vec3(2.4));
    return mix(high, low, lessThanEqual(encoded, vec3(0.04045)));
}

void main() {
    ivec2 pixel = ivec2(gl_FragCoord.xy) - constants.origin;
    if (any(lessThan(pixel, ivec2(0))) || any(greaterThanEqual(pixel, constants.size))) {
        color = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }

    vec3 rgb;
    if ((constants.flags & FLAG_NEAREST) != 0u) {
        rgb = texel((pixel * constants.source) / constants.size);
    } else {
        vec2 scale = vec2(constants.size) / vec2(constants.source);
        // The pixel centre in texel units, measured from the centre of texel (0, 0).
        vec2 position = (vec2(pixel) + 0.5) / scale - 0.5;
        vec2 base = floor(position);
        // Stretching the fraction around one half by the factor leaves a band one target
        // pixel wide to interpolate in; everything else snaps to one of the two texels.
        vec2 blend = clamp((position - base - 0.5) * max(scale, vec2(1.0)) + 0.5, 0.0, 1.0);
        ivec2 corner = ivec2(base);
        vec3 top = mix(texel(corner), texel(corner + ivec2(1, 0)), blend.x);
        vec3 bottom = mix(texel(corner + ivec2(0, 1)), texel(corner + ivec2(1, 1)), blend.x);
        rgb = mix(top, bottom, blend.y);
    }
    if ((constants.flags & FLAG_SRGB_TARGET) != 0u) {
        rgb = srgb_to_linear(rgb);
    }
    color = vec4(rgb, 1.0);
}
