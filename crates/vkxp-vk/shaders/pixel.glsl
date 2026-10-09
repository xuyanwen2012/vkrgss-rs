// Pixel arithmetic of the Vulkan backend.
//
// This file mirrors `crates/vkxp-pixel/src/arith.rs` function for function: the same names,
// the same integer expressions, in the same order. Channels are unsigned integers from 0 to
// 255 held in `uint`, a pixel is a `uvec4` of red, green, blue and straight alpha, and `/` is
// integer division rounding down. Every intermediate value fits in 32 bits, as it does in Rust.
//
// When a formula changes in vkxp-pixel, change it here in the same commit. The arithmetic
// sweep tests render every pair of channel values through both and fail on any difference.

#ifndef VKXP_PIXEL_GLSL
#define VKXP_PIXEL_GLSL

// Mirrors `div255` in vkxp-pixel: division by 255, rounded to nearest.
uint div255(uint value) {
    return (value + 127u) / 255u;
}

// Mirrors `mix` in vkxp-pixel: `dst` at alpha 0, `src` at alpha 255. It overloads the builtin
// of the same name, which has no all-`uint` form.
uint mix(uint dst, uint src, uint alpha) {
    return div255(src * alpha + dst * (255u - alpha));
}

// Mirrors `scale` in vkxp-pixel: the share of a source channel that is added or subtracted.
uint scale(uint channel, uint alpha) {
    return div255(channel * alpha);
}

// Mirrors `vkxp_pixel::effective_alpha`.
uint effective_alpha(uint alpha, uint opacity) {
    return scale(alpha, opacity);
}

// Mirrors `over` in vkxp-pixel: one colour channel of source-over.
uint over(uint dst, uint src, uint a, uint weight, uint total) {
    return (255u * a * src + weight * dst + total / 2u) / total;
}

// Mirrors `vkxp_pixel::blend_normal`.
uvec4 blend_normal(uvec4 dst, uvec4 src, uint opacity) {
    uint a = effective_alpha(src.a, opacity);
    uint weight = dst.a * (255u - a);
    uint total = 255u * a + weight;
    if (total == 0u) {
        return dst;
    }
    return uvec4(
        over(dst.r, src.r, a, weight, total),
        over(dst.g, src.g, a, weight, total),
        over(dst.b, src.b, a, weight, total),
        div255(total));
}

// Mirrors `u8::saturating_add`.
uint saturating_add(uint channel, uint amount) {
    return min(channel + amount, 255u);
}

// Mirrors `u8::saturating_sub`.
uint saturating_sub(uint channel, uint amount) {
    return channel - min(channel, amount);
}

// Mirrors `vkxp_pixel::blend_add`.
uvec4 blend_add(uvec4 dst, uvec4 src, uint opacity) {
    uint a = effective_alpha(src.a, opacity);
    return uvec4(
        saturating_add(dst.r, scale(src.r, a)),
        saturating_add(dst.g, scale(src.g, a)),
        saturating_add(dst.b, scale(src.b, a)),
        dst.a);
}

// Mirrors `vkxp_pixel::blend_sub`.
uvec4 blend_sub(uvec4 dst, uvec4 src, uint opacity) {
    uint a = effective_alpha(src.a, opacity);
    return uvec4(
        saturating_sub(dst.r, scale(src.r, a)),
        saturating_sub(dst.g, scale(src.g, a)),
        saturating_sub(dst.b, scale(src.b, a)),
        dst.a);
}

// Mirrors `vkxp_pixel::blend_color`.
uvec4 blend_color(uvec4 dst, uvec4 color) {
    return uvec4(
        mix(dst.r, color.r, color.a),
        mix(dst.g, color.g, color.a),
        mix(dst.b, color.b, color.a),
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

// Mirrors `vkxp_pixel::apply_tone`.
uvec4 apply_tone(uvec4 dst, int red, int green, int blue, uint gray) {
    uint luma = (77u * dst.r + 150u * dst.g + 29u * dst.b) / 256u;
    return uvec4(
        offset(mix(dst.r, luma, gray), red),
        offset(mix(dst.g, luma, gray), green),
        offset(mix(dst.b, luma, gray), blue),
        dst.a);
}

#endif
