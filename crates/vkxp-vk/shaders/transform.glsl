// Exact evaluation of a quad's transform with 32-bit words only.
//
// `Transform::source_position` in vkxp-proto forms each sum of two products exactly, rounds it
// towards negative infinity to 16 fractional bits and adds the translation. The products need
// 64 bits. 64-bit shader integers are an optional device feature, so a signed 64-bit value is
// carried here as two 32-bit words and only the handful of operations the transform needs are
// written out.
//
// Ranges, for a logical size of at most 16384: the pixel centre `(p << 16) + 0x8000` is below
// 2^30, a coefficient is a 32-bit signed value, so a product is within +-2^61 and the sum of
// two within +-2^62. Nothing below can overflow 64 bits.

#ifndef VKXP_TRANSFORM_GLSL
#define VKXP_TRANSFORM_GLSL

// A signed 64-bit integer in two's complement: `hi` holds bits 32 to 63, `lo` bits 0 to 31.
struct I64 {
    int hi;
    uint lo;
};

// The exact product of two signed 32-bit integers.
I64 i64_mul(int a, int b) {
    int hi;
    uint lo;
    imulExtended(a, b, hi, lo);
    return I64(hi, lo);
}

// The sum of two 64-bit integers; the caller guarantees it does not overflow.
I64 i64_add(I64 a, I64 b) {
    uint carry;
    uint lo = uaddCarry(a.lo, b.lo, carry);
    return I64(a.hi + b.hi + int(carry), lo);
}

// A signed 32-bit integer widened to 64 bits.
I64 i64_from_int(int a) {
    return I64(a >> 31, uint(a));
}

// `a >> 16` with sign extension: division by 65536 rounded towards negative infinity.
I64 i64_shr16(I64 a) {
    return I64(a.hi >> 16, (uint(a.hi) << 16) | (a.lo >> 16));
}

// Bits 16 to 47 of `a` as a signed integer: `a >> 16` for any value it fits in 32 bits.
int i64_shr16_to_int(I64 a) {
    return int((uint(a.hi) << 16) | (a.lo >> 16));
}

// The coefficients of `vkxp_proto::Transform` as raw fixed-point bits (value * 65536).
struct Transform {
    int xx;
    int xy;
    int yx;
    int yy;
    int tx;
    int ty;
};

// `(a * x + b * y) >> 16`, plus `t`: one coordinate of the source position.
I64 transform_axis(int a, int b, int t, int x, int y) {
    I64 sum = i64_add(i64_mul(a, x), i64_mul(b, y));
    return i64_add(i64_shr16(sum), i64_from_int(t));
}

// Mirrors `vkxp_proto::Transform::source_position`: the position (`u`, `v`) that the centre of
// screen pixel `pixel` maps to, as fixed-point numbers with 16 fractional bits. Both pixel
// coordinates must be in 0..16384.
void source_position(Transform transform, ivec2 pixel, out I64 u, out I64 v) {
    int x = (pixel.x << 16) + 0x8000;
    int y = (pixel.y << 16) + 0x8000;
    u = transform_axis(transform.xx, transform.xy, transform.tx, x, y);
    v = transform_axis(transform.yx, transform.yy, transform.ty, x, y);
}

// The texel under the source position of `pixel`, relative to the source rectangle: the floor
// of each coordinate. A position is within +-2^47, so its floor fits in a signed 32-bit
// integer and can be compared with the rectangle's size as it is.
ivec2 source_texel(Transform transform, ivec2 pixel) {
    I64 u;
    I64 v;
    source_position(transform, pixel, u, v);
    return ivec2(i64_shr16_to_int(u), i64_shr16_to_int(v));
}

#endif
