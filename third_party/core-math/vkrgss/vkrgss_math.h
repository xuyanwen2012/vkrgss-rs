/*
 * The mathematical functions of the interpreters, correctly rounded by CORE-MATH.
 *
 * A file of an interpreter includes this header after <math.h>. Its macros make every call
 * of the functions below call the function of the same name with the prefix vkrgss_, which
 * vkrgss_math.c defines. They are function-like, so a name that is not called, such as a
 * variable named exp, stays as it is. The header is plain C89, as Ruby 1.8.7 is compiled.
 *
 * Part of vkrgss, under its MIT licence; not part of CORE-MATH.
 */

#ifndef VKRGSS_MATH_H
#define VKRGSS_MATH_H 1

double vkrgss_sin(double);
double vkrgss_cos(double);
double vkrgss_tan(double);
double vkrgss_asin(double);
double vkrgss_acos(double);
double vkrgss_atan(double);
double vkrgss_atan2(double, double);
double vkrgss_sinh(double);
double vkrgss_cosh(double);
double vkrgss_tanh(double);
double vkrgss_asinh(double);
double vkrgss_acosh(double);
double vkrgss_atanh(double);
double vkrgss_exp(double);
double vkrgss_log(double);
double vkrgss_log2(double);
double vkrgss_log10(double);
double vkrgss_pow(double, double);
double vkrgss_cbrt(double);
double vkrgss_hypot(double, double);
double vkrgss_erf(double);
double vkrgss_erfc(double);
double vkrgss_tgamma(double);
double vkrgss_lgamma_r(double, int *);

/* vkrgss_math.c defines the functions and calls those of the C library by their names. */
#ifndef VKRGSS_MATH_IMPLEMENTATION
#define sin(x) vkrgss_sin(x)
#define cos(x) vkrgss_cos(x)
#define tan(x) vkrgss_tan(x)
#define asin(x) vkrgss_asin(x)
#define acos(x) vkrgss_acos(x)
#define atan(x) vkrgss_atan(x)
#define atan2(y, x) vkrgss_atan2(y, x)
#define sinh(x) vkrgss_sinh(x)
#define cosh(x) vkrgss_cosh(x)
#define tanh(x) vkrgss_tanh(x)
#define asinh(x) vkrgss_asinh(x)
#define acosh(x) vkrgss_acosh(x)
#define atanh(x) vkrgss_atanh(x)
#define exp(x) vkrgss_exp(x)
#define log(x) vkrgss_log(x)
#define log2(x) vkrgss_log2(x)
#define log10(x) vkrgss_log10(x)
#define pow(x, y) vkrgss_pow(x, y)
#define cbrt(x) vkrgss_cbrt(x)
#define hypot(x, y) vkrgss_hypot(x, y)
#define erf(x) vkrgss_erf(x)
#define erfc(x) vkrgss_erfc(x)
#define tgamma(x) vkrgss_tgamma(x)
#define lgamma_r(x, sign) vkrgss_lgamma_r(x, sign)
#endif

#endif
