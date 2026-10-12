/*
 * What the three fuzz targets share: the options of the sanitizers and the one start of the
 * interpreter. Each target is one C file that includes this header and defines
 * LLVMFuzzerTestOneInput; `tests/fuzz.rs` builds them and `cargo xtask fuzz` runs them.
 */
#ifndef VKXP_FUZZ_HARNESS_H
#define VKXP_FUZZ_HARNESS_H

#include <signal.h>
#include <stddef.h>
#include <stdint.h>

#include "ruby.h"
#include "env.h"
#include "node.h"
#include "re.h"

void ruby_init_stack _((VALUE *));

/*
 * As in the sanitized run of the suites: nothing is freed at exit, and the "fake stack" of
 * use-after-return detection would hide values from the conservative collector. An
 * allocation the machine cannot serve returns a null pointer, as without the sanitizer, and
 * the interpreter raises NoMemoryError.
 */
const char *
__asan_default_options(void)
{
    return "detect_stack_use_after_return=0:detect_leaks=0:allocator_may_return_null=1";
}

const char *
__ubsan_default_options(void)
{
    return "print_stacktrace=1";
}

/*
 * Starts the interpreter as vkxp does: the stack anchored above every frame that will call
 * into it (the argument strings of the process are at its top), `$KCODE` set to UTF-8, and
 * the signal dispositions put back, so that a fault reaches the sanitizer and libFuzzer and
 * not Ruby's own handlers. Warnings are off: they are written to the error stream, where
 * the reports go.
 */
int
LLVMFuzzerInitialize(int *argc, char ***argv)
{
    static struct sigaction saved[NSIG];
    int number;

    for (number = 1; number < NSIG; number++) sigaction(number, 0, &saved[number]);
    ruby_init_stack((VALUE *)*argv);
    ruby_init();
    for (number = 1; number < NSIG; number++) sigaction(number, &saved[number], 0);
    rb_set_kcode("U");
    ruby_verbose = Qnil;
    return 0;
}

/*
 * Calls `body` with the input as a Ruby string. A Ruby exception ends the call and is
 * dropped: it is not a finding.
 */
static void
vkxp_fuzz_protected(VALUE (*body)(VALUE), const uint8_t *data, size_t size)
{
    int state = 0;
    volatile VALUE input = rb_str_new((const char *)data, (long)size);

    rb_protect(body, input, &state);
    ruby_errinfo = Qnil;
}

#endif
