/*
 * The mathematical functions of the interpreters, correctly rounded by CORE-MATH.
 *
 * For a regular argument, finite and non-zero, each function returns the result of
 * CORE-MATH, which is correctly rounded, so it is the same on every machine and platform.
 * When that result is not a normal number, the argument was not regular after all, or the
 * result underflows or overflows: the function of the C library is called too. Its value is
 * returned only when the result of CORE-MATH is NaN, so that NaN is what it was before; an
 * infinity, a zero or a subnormal result stays the correctly rounded one, because the C
 * libraries round differently at the edge of overflow (hypot of bionic gives the largest
 * double where the result is infinite). Special arguments (zeros, infinities, NaN) go to
 * the C library alone. CORE-MATH does not set errno as it is
 * compiled here.
 *
 * errno is also set here, on every platform, as glibc 2.43 sets it: EDOM for a NaN from
 * arguments that are not NaN (a domain error); ERANGE for an infinity from finite arguments
 * (a pole or an overflow), and for an underflow of a regular argument, which atan2, exp and
 * pow report when the result is zero, and asinh, atanh, erfc and tgamma when it is zero or
 * subnormal; the other functions report none, and their subnormal results are no error. The
 * libm of Android's bionic never sets errno, and glibc before 2.43 does not report the
 * subnormal results of asinh, atanh, erfc and tgamma, so that Math of Ruby 1.8.7, which
 * reads errno, raises the same errors everywhere. Where the C library sets errno too, it
 * sets the same value.
 *
 * Part of vkrgss, under its MIT licence; not part of CORE-MATH.
 */

#define _GNU_SOURCE 1
#define VKRGSS_MATH_IMPLEMENTATION 1

#include <math.h>

#include "vkrgss_math.h"

#include <errno.h>

/* Which results of a regular argument a function reports as an underflow. */
enum vkrgss_underflow {
    VKRGSS_UNDERFLOW_NONE,
    VKRGSS_UNDERFLOW_ZERO,
    VKRGSS_UNDERFLOW_TINY
};

/* Sets errno for the result of a function whose arguments hold a NaN when nan is non-zero,
   are all finite when finite is non-zero, and are all regular when regular is non-zero, and
   which reports underflow as underflow says; returns the result. */
static double vkrgss_errno(double result, int nan, int finite, int regular,
                           enum vkrgss_underflow underflow)
{
    if (isnan(result) && !nan)
        errno = EDOM;
    else if (isinf(result) && finite)
        errno = ERANGE;
    else if (regular && underflow != VKRGSS_UNDERFLOW_NONE &&
             (result == 0.0 || (underflow == VKRGSS_UNDERFLOW_TINY && !isnormal(result))))
        errno = ERANGE;
    return result;
}

/* The result of a special argument, and that of a regular one. */
#define VKRGSS_SPECIAL1(result, x)                                                           \
    return vkrgss_errno(result, isnan(x), isfinite(x), 0, VKRGSS_UNDERFLOW_NONE)
#define VKRGSS_SPECIAL2(result, x, y)                                                        \
    return vkrgss_errno(result, isnan(x) || isnan(y), isfinite(x) && isfinite(y), 0,         \
                        VKRGSS_UNDERFLOW_NONE)
#define VKRGSS_REGULAR(result, underflow) return vkrgss_errno(result, 0, 1, 1, underflow)

double cr_sin(double);
double cr_cos(double);
double cr_tan(double);
double cr_asin(double);
double cr_acos(double);
double cr_atan(double);
double cr_atan2(double, double);
double cr_sinh(double);
double cr_cosh(double);
double cr_tanh(double);
double cr_asinh(double);
double cr_acosh(double);
double cr_atanh(double);
double cr_exp(double);
double cr_log(double);
double cr_log2(double);
double cr_log10(double);
double cr_pow(double, double);
double cr_cbrt(double);
double cr_hypot(double, double);
double cr_erf(double);
double cr_erfc(double);
double cr_tgamma(double);
double cr_lgamma(double);

#define VKRGSS_UNARY(name, underflow)                                                        \
    double vkrgss_##name(double x)                                                         \
    {                                                                                      \
        double result, library;                                                            \
        if (!isfinite(x) || x == 0.0)                                                      \
            VKRGSS_SPECIAL1(name(x), x);                                                   \
        result = cr_##name(x);                                                             \
        if (isnormal(result))                                                              \
            return result;                                                                 \
        library = name(x);                                                                 \
        VKRGSS_REGULAR(isnan(result) ? library : result, VKRGSS_UNDERFLOW_##underflow);    \
    }

#define VKRGSS_BINARY(name, underflow)                                                       \
    double vkrgss_##name(double x, double y)                                               \
    {                                                                                      \
        double result, library;                                                            \
        if (!isfinite(x) || x == 0.0 || !isfinite(y) || y == 0.0)                          \
            VKRGSS_SPECIAL2(name(x, y), x, y);                                             \
        result = cr_##name(x, y);                                                          \
        if (isnormal(result))                                                              \
            return result;                                                                 \
        library = name(x, y);                                                              \
        VKRGSS_REGULAR(isnan(result) ? library : result, VKRGSS_UNDERFLOW_##underflow);    \
    }

VKRGSS_UNARY(sin, NONE)
VKRGSS_UNARY(cos, NONE)
VKRGSS_UNARY(tan, NONE)
VKRGSS_UNARY(asin, NONE)
VKRGSS_UNARY(acos, NONE)
VKRGSS_UNARY(atan, NONE)
VKRGSS_BINARY(atan2, ZERO)
VKRGSS_UNARY(sinh, NONE)
VKRGSS_UNARY(cosh, NONE)
VKRGSS_UNARY(tanh, NONE)
VKRGSS_UNARY(asinh, TINY)
VKRGSS_UNARY(acosh, NONE)
VKRGSS_UNARY(atanh, TINY)
VKRGSS_UNARY(exp, ZERO)
VKRGSS_UNARY(log, NONE)
VKRGSS_UNARY(log2, NONE)
VKRGSS_UNARY(log10, NONE)
VKRGSS_BINARY(pow, ZERO)
VKRGSS_UNARY(cbrt, NONE)
VKRGSS_BINARY(hypot, NONE)
VKRGSS_UNARY(erf, NONE)
VKRGSS_UNARY(erfc, TINY)
VKRGSS_UNARY(tgamma, TINY)

/* cr_lgamma sets signgam. Every call holds the interpreter's lock, so no other thread of the
   interpreter runs between the call and the read. */
double vkrgss_lgamma_r(double x, int *sign)
{
    double result, library;
    int result_sign;
    if (!isfinite(x) || x == 0.0)
        VKRGSS_SPECIAL1(lgamma_r(x, sign), x);
    result = cr_lgamma(x);
    result_sign = signgam;
    if (isnormal(result)) {
        *sign = result_sign;
        return result;
    }
    library = lgamma_r(x, sign);
    if (!isfinite(result))
        VKRGSS_REGULAR(isnan(result) ? library : result, VKRGSS_UNDERFLOW_NONE);
    *sign = result_sign;
    return result;
}
