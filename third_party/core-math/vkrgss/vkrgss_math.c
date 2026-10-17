/*
 * The mathematical functions of the interpreters, correctly rounded by CORE-MATH.
 *
 * For a regular argument, finite and non-zero, each function returns the result of
 * CORE-MATH, which is correctly rounded, so it is the same on every machine and platform.
 * When that result is not a normal number, the argument was not regular after all, or the
 * result underflows: the function of the C library is called too, so that errno is what it
 * was before, which Math of Ruby 1.8.7 reads. Its value is returned only when the result of
 * CORE-MATH is not finite, so that infinities and NaN are what they were before; a zero or
 * subnormal result stays the correctly rounded one. Special arguments (zeros, infinities,
 * NaN) go to the C library alone. CORE-MATH does not set errno as it is compiled here.
 *
 * The libm of Android's bionic never sets errno. There every function sets it as glibc does
 * for the errors that Math of Ruby 1.8.7 reads: EDOM for a NaN from arguments that are not
 * NaN (a domain error), ERANGE for an infinity from finite arguments (a pole or an
 * overflow). Elsewhere errno stays the C library's.
 *
 * Part of vkrgss, under its MIT licence; not part of CORE-MATH.
 */

#define _GNU_SOURCE 1
#define VKRGSS_MATH_IMPLEMENTATION 1

#include <math.h>

#include "vkrgss_math.h"

#ifdef __ANDROID__
#include <errno.h>

/* Sets errno for the result of a function whose arguments hold a NaN when nan is non-zero
   and are all finite when finite is non-zero, and returns the result. */
static double vkrgss_errno(double result, int nan, int finite)
{
    if (isnan(result) && !nan)
        errno = EDOM;
    else if (isinf(result) && finite)
        errno = ERANGE;
    return result;
}

#define VKRGSS_RETURN1(result, x) return vkrgss_errno(result, isnan(x), isfinite(x))
#define VKRGSS_RETURN2(result, x, y)                                                         \
    return vkrgss_errno(result, isnan(x) || isnan(y), isfinite(x) && isfinite(y))
#else
#define VKRGSS_RETURN1(result, x) return result
#define VKRGSS_RETURN2(result, x, y) return result
#endif

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

#define VKRGSS_UNARY(name)                                                                   \
    double vkrgss_##name(double x)                                                         \
    {                                                                                      \
        double result, library;                                                            \
        if (!isfinite(x) || x == 0.0)                                                      \
            VKRGSS_RETURN1(name(x), x);                                                    \
        result = cr_##name(x);                                                             \
        if (isnormal(result))                                                              \
            return result;                                                                 \
        library = name(x);                                                                 \
        VKRGSS_RETURN1(isfinite(result) ? result : library, x);                            \
    }

#define VKRGSS_BINARY(name)                                                                  \
    double vkrgss_##name(double x, double y)                                               \
    {                                                                                      \
        double result, library;                                                            \
        if (!isfinite(x) || x == 0.0 || !isfinite(y) || y == 0.0)                          \
            VKRGSS_RETURN2(name(x, y), x, y);                                              \
        result = cr_##name(x, y);                                                          \
        if (isnormal(result))                                                              \
            return result;                                                                 \
        library = name(x, y);                                                              \
        VKRGSS_RETURN2(isfinite(result) ? result : library, x, y);                         \
    }

VKRGSS_UNARY(sin)
VKRGSS_UNARY(cos)
VKRGSS_UNARY(tan)
VKRGSS_UNARY(asin)
VKRGSS_UNARY(acos)
VKRGSS_UNARY(atan)
VKRGSS_BINARY(atan2)
VKRGSS_UNARY(sinh)
VKRGSS_UNARY(cosh)
VKRGSS_UNARY(tanh)
VKRGSS_UNARY(asinh)
VKRGSS_UNARY(acosh)
VKRGSS_UNARY(atanh)
VKRGSS_UNARY(exp)
VKRGSS_UNARY(log)
VKRGSS_UNARY(log2)
VKRGSS_UNARY(log10)
VKRGSS_BINARY(pow)
VKRGSS_UNARY(cbrt)
VKRGSS_BINARY(hypot)
VKRGSS_UNARY(erf)
VKRGSS_UNARY(erfc)
VKRGSS_UNARY(tgamma)

/* cr_lgamma sets signgam. Every call holds the interpreter's lock, so no other thread of the
   interpreter runs between the call and the read. */
double vkrgss_lgamma_r(double x, int *sign)
{
    double result, library;
    int result_sign;
    if (!isfinite(x) || x == 0.0)
        VKRGSS_RETURN1(lgamma_r(x, sign), x);
    result = cr_lgamma(x);
    result_sign = signgam;
    if (isnormal(result)) {
        *sign = result_sign;
        return result;
    }
    library = lgamma_r(x, sign);
    if (!isfinite(result))
        VKRGSS_RETURN1(library, x);
    *sign = result_sign;
    return result;
}
