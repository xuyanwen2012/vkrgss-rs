/* What the vendored interpreter took from its own `regex.h`, provided from Oniguruma: the
   patched `re.h` and `parse.c` include this file in place of `regex.h`.

   The pattern buffer, the registers and the `re_*` functions come from Oniguruma's
   GNU-compatible interface. The multi-byte helpers that `string.c`, `parse.c` and others use
   follow the default encoding, which `re_mbcinit` sets from `$KCODE` as it did before. */

#ifndef VKXP_REGEX_H
#define VKXP_REGEX_H

/* Keeps Oniguruma's `UChar` out of the interpreter's name space. */
#define ONIG_ESCAPE_UCHAR_COLLISION
#include "oniggnu.h"

#define MBCTYPE_ASCII RE_MBCTYPE_ASCII
#define MBCTYPE_EUC   RE_MBCTYPE_EUC
#define MBCTYPE_SJIS  RE_MBCTYPE_SJIS
#define MBCTYPE_UTF8  RE_MBCTYPE_UTF8

#define re_copy_registers onig_region_copy

/* The length in bytes of the character that starts with the byte `c`. */
static __inline__ int
vkxp_mbclen(c)
    OnigUChar c;
{
    return ONIGENC_MBC_ENC_LEN(onigenc_get_default_encoding(), &c);
}

/* As in `regex.h`: the number of bytes that follow the first one, and the whole length. */
#undef ismbchar
#define ismbchar(c) (vkxp_mbclen((OnigUChar)(c)) - 1)
#define mbclen(c)   vkxp_mbclen((OnigUChar)(c))

#endif /* !VKXP_REGEX_H */
