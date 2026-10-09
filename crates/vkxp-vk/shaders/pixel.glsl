// Pixel arithmetic of the Vulkan backend.
//
// This file mirrors `crates/vkxp-pixel/src/arith.rs` function for function: the same names,
// the same integer expressions, in the same order. Channels are unsigned integers from 0 to
// 255 held in `uint`, a pixel is a `uvec4` of red, green, blue and straight alpha, `/` is
// integer division rounding down and `>>` on an `int` is an arithmetic shift. Every
// intermediate value fits in 32 bits, as it does in Rust.
//
// When a formula changes in vkxp-pixel, change it here in the same commit. The arithmetic
// sweep tests render every pair of channel values through both and fail on any difference.

#ifndef VKXP_PIXEL_GLSL
#define VKXP_PIXEL_GLSL

// Mirrors `div255` in vkxp-pixel: division by 255, rounded to nearest.
uint div255(uint value) {
    return (value + 127u) / 255u;
}

// Mirrors `scale` in vkxp-pixel: the share of a source channel that is added or subtracted to a
// translucent destination.
uint scale(uint channel, uint alpha) {
    return div255(channel * alpha);
}

// Mirrors `share` in vkxp-pixel: the share of a signed value that an alpha lets through.
int share(int value, uint alpha) {
    return (value * (int(alpha) + 1)) >> 8;
}

// Mirrors `lerp` in vkxp-pixel: `src` at alpha 255, never outside the two.
uint lerp(uint dst, uint src, uint alpha) {
    return uint(int(dst) + share(int(src) - int(dst), alpha));
}

// Mirrors `vkxp_pixel::effective_alpha`.
uint effective_alpha(uint alpha, uint opacity) {
    return div255(alpha * opacity);
}

// Mirrors `BLT_WEIGHT_EXCEPTIONS` in vkxp-pixel: the pairs `(a, dst.a)` for which the weight is
// one below the quotient. They are measured, not derived.
const uvec2 BLT_WEIGHT_EXCEPTIONS[5] = uvec2[5](
    uvec2(30u, 51u), uvec2(30u, 136u), uvec2(105u, 204u), uvec2(180u, 153u), uvec2(210u, 85u));

// Mirrors `blt_weight` in vkxp-pixel: the weight of the source for a destination with alpha.
// `den` is not 0.
uint blt_weight(uint a, uint dst_alpha, uint den) {
    uint weight = a * 65025u / den;
    for (int i = 0; i < 5; i++) {
        if (BLT_WEIGHT_EXCEPTIONS[i].x == a && BLT_WEIGHT_EXCEPTIONS[i].y == dst_alpha) {
            weight -= 1u;
        }
    }
    return weight;
}

// Mirrors `vkxp_pixel::blend_normal_translucent`.
uvec4 blend_normal_translucent(uvec4 dst, uvec4 src, uint opacity) {
    if (opacity == 0u) {
        return dst;
    }
    uint a = effective_alpha(src.a, opacity);
    uint den = 255u * dst.a + 255u * a - dst.a * a;
    if (den == 0u) {
        return uvec4(src.r, src.g, src.b, 0u);
    }
    uint weight = blt_weight(a, dst.a, den);
    if (weight == 0u) {
        return uvec4(dst.r, dst.g, dst.b, den / 255u);
    }
    return uvec4(
        lerp(dst.r, src.r, weight),
        lerp(dst.g, src.g, weight),
        lerp(dst.b, src.b, weight),
        den / 255u);
}

// Mirrors `vkxp_pixel::blend_normal`.
uvec4 blend_normal(uvec4 dst, uvec4 src, uint opacity) {
    if (dst.a != 255u) {
        return blend_normal_translucent(dst, src, opacity);
    }
    uint a = effective_alpha(src.a, opacity);
    if (a == 0u) {
        return dst;
    }
    return uvec4(
        lerp(dst.r, src.r, a),
        lerp(dst.g, src.g, a),
        lerp(dst.b, src.b, a),
        255u);
}

// Mirrors `u8::saturating_add`.
uint saturating_add(uint channel, uint amount) {
    return min(channel + amount, 255u);
}

// Mirrors `u8::saturating_sub`.
uint saturating_sub(uint channel, uint amount) {
    return channel - min(channel, amount);
}

// Mirrors `vkxp_pixel::blend_add_translucent`.
uvec4 blend_add_translucent(uvec4 dst, uvec4 src, uint opacity) {
    uint a = effective_alpha(src.a, opacity);
    return uvec4(
        saturating_add(dst.r, scale(src.r, a)),
        saturating_add(dst.g, scale(src.g, a)),
        saturating_add(dst.b, scale(src.b, a)),
        dst.a);
}

// Mirrors `vkxp_pixel::blend_add`.
uvec4 blend_add(uvec4 dst, uvec4 src, uint opacity) {
    if (dst.a != 255u) {
        return blend_add_translucent(dst, src, opacity);
    }
    uint a = effective_alpha(src.a, opacity);
    if (a == 0u) {
        return dst;
    }
    return uvec4(
        saturating_add(dst.r, uint(share(int(src.r), a))),
        saturating_add(dst.g, uint(share(int(src.g), a))),
        saturating_add(dst.b, uint(share(int(src.b), a))),
        dst.a);
}

// Mirrors `vkxp_pixel::blend_sub_translucent`.
uvec4 blend_sub_translucent(uvec4 dst, uvec4 src, uint opacity) {
    uint a = effective_alpha(src.a, opacity);
    return uvec4(
        saturating_sub(dst.r, scale(src.r, a)),
        saturating_sub(dst.g, scale(src.g, a)),
        saturating_sub(dst.b, scale(src.b, a)),
        dst.a);
}

// Mirrors `vkxp_pixel::blend_sub`.
uvec4 blend_sub(uvec4 dst, uvec4 src, uint opacity) {
    if (dst.a != 255u) {
        return blend_sub_translucent(dst, src, opacity);
    }
    uint a = effective_alpha(src.a, opacity);
    if (a == 0u) {
        return dst;
    }
    return uvec4(
        saturating_sub(dst.r, uint(share(int(src.r), a))),
        saturating_sub(dst.g, uint(share(int(src.g), a))),
        saturating_sub(dst.b, uint(share(int(src.b), a))),
        dst.a);
}

// Mirrors `vkxp_pixel::blend_color`.
uvec4 blend_color(uvec4 dst, uvec4 color) {
    if (color.a == 0u) {
        return dst;
    }
    return uvec4(
        lerp(dst.r, color.r, color.a),
        lerp(dst.g, color.g, color.a),
        lerp(dst.b, color.b, color.a),
        dst.a);
}

// Mirrors `offset` in vkxp-pixel. `amount` is any 16-bit signed value.
uint offset(uint channel, int amount) {
    int sum = int(channel) + amount;
    if (sum < 0) {
        return 0u;
    } else if (sum > 255) {
        return 255u;
    } else {
        return uint(sum);
    }
}

// Mirrors `vkxp_pixel::apply_gray`.
uvec4 apply_gray(uvec4 dst, uint gray) {
    if (gray == 0u) {
        return dst;
    }
    uint luma = (77u * dst.r + 150u * dst.g + 29u * dst.b) >> 8;
    return uvec4(
        lerp(dst.r, luma, gray),
        lerp(dst.g, luma, gray),
        lerp(dst.b, luma, gray),
        dst.a);
}

// Mirrors `vkxp_pixel::apply_tone`.
uvec4 apply_tone(uvec4 dst, int red, int green, int blue, uint gray) {
    uvec4 grayed = apply_gray(dst, gray);
    return uvec4(
        offset(grayed.r, red),
        offset(grayed.g, green),
        offset(grayed.b, blue),
        dst.a);
}

#endif
