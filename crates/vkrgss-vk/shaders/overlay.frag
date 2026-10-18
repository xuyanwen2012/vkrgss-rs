#version 460
#extension GL_EXT_samplerless_texture_functions : require

// Draws the overlay picture over what the presentation pass drew, blended by its alpha. The
// picture has the size of the target, or another size that is scaled to it by the nearest
// texel, which only happens until the caller has drawn a picture for a new window size. Its
// texels are premultiplied, with sRGB-encoded colour, as the caller draws them.

layout(set = 0, binding = 0) uniform utexture2D picture;

layout(push_constant) uniform Constants {
    // The size of the target in pixels.
    ivec2 target;
    // Only FLAG_SRGB_TARGET.
    uint flags;
} constants;

// The bits of `constants.flags` (`FLAG_*`), generated from `src/shared.rs`.
#include "shared.glsl"

layout(location = 0) out vec4 color;

// The inverse of the encoding an sRGB target applies, so the stored value is the one given.
vec3 srgb_to_linear(vec3 encoded) {
    vec3 low = encoded / 12.92;
    vec3 high = pow((encoded + 0.055) / 1.055, vec3(2.4));
    return mix(high, low, lessThanEqual(encoded, vec3(0.04045)));
}

void main() {
    ivec2 size = textureSize(picture, 0);
    ivec2 at = min(ivec2(gl_FragCoord.xy) * size / max(constants.target, ivec2(1)), size - 1);
    vec4 texel = vec4(texelFetch(picture, at, 0)) / 255.0;
    if ((constants.flags & FLAG_SRGB_TARGET) != 0u && texel.a > 0.0) {
        texel.rgb = srgb_to_linear(texel.rgb / texel.a) * texel.a;
    }
    color = texel;
}
