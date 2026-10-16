/*
 * What the three fuzz targets of Ruby 1.9.2 share: the options of the sanitizers and the one
 * start of the interpreter. Each target is one C file that includes this header and defines
 * LLVMFuzzerTestOneInput; `tests/fuzz.rs` builds them and `cargo xtask fuzz` runs them as
 * `marshal192`, `parse192` and `regex192`.
 */
#ifndef VKRGSS_FUZZ_HARNESS_H
#define VKRGSS_FUZZ_HARNESS_H

#include <pthread.h>
#include <signal.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <sys/resource.h>

#include "ruby/ruby.h"
#include "ruby/encoding.h"

/* The settings that patches of the tree add, as `vkrgss-ruby192-sys` sets them. */
extern int ruby_vkxp_lazy_timer_thread;
extern int ruby_vkxp_hashseed_fixed;
extern unsigned long ruby_vkxp_hashseed;
extern size_t ruby_vkxp_stack_size;

void Init_utf_16le(void);
void Init_japanese_sjis(void);
void Init_single_byte(void);
void Init_utf_16_32(void);
void Init_zlib(void);

/*
 * As in the sanitized run of the suites: nothing is freed at exit, and the "fake stack" of
 * use-after-return detection would hide values from the conservative collector. Unlike
 * there, the sanitizer keeps its alternate signal stack, on which it reports a stack
 * overflow: no input makes a Ruby thread, at whose end it would unmap the stack. A single
 * allocation of more than 256 MiB, which a length field of a few bytes can ask for, gets a
 * null pointer, and the interpreter raises NoMemoryError as it does when memory is short.
 */
const char *
__asan_default_options(void)
{
    return "detect_stack_use_after_return=0:detect_leaks=0:"
           "allocator_may_return_null=1:max_allocation_size_mb=256";
}

const char *
__ubsan_default_options(void)
{
    return "print_stacktrace=1";
}

/* How much of the machine stack is kept free below Ruby's own limit, as
   `STACK_HEADROOM` of the crate. */
#define VKRGSS_FUZZ_STACK_HEADROOM (1 << 20)

static VALUE
vkrgss_fuzz_setup(VALUE unused)
{
    rb_encoding *utf8 = rb_utf8_encoding();

    Init_utf_16le();
    Init_japanese_sjis();
    Init_single_byte();
    Init_utf_16_32();
    Init_zlib();
    rb_enc_set_default_external(rb_enc_from_encoding(utf8));
    return Qnil;
}

/*
 * Starts the interpreter as vkrgss does: no timer thread before a script makes a thread,
 * the hashes seeded with the crate's seed, the limit of the machine stack taken from the
 * stack of this thread less the headroom, the stack anchored above every frame that will
 * call into it (the argument strings of the process are at its top), the encodings and
 * `Zlib` that vkrgss registers, UTF-8 as the default external encoding, and the signal
 * dispositions, the mask and the alternate signal stack put back, so that a fault reaches
 * the sanitizer and libFuzzer and not Ruby's own handlers, and the sanitizer reports it on
 * its own stack and not on the smaller one that Ruby sets. Warnings are off: they are
 * written to the error stream, where the reports go.
 */
int
LLVMFuzzerInitialize(int *argc, char ***argv)
{
    static struct sigaction saved[NSIG];
    sigset_t mask;
    stack_t altstack;
    struct rlimit limit;
    int number, state = 0;

    (void)argc;
    for (number = 1; number < NSIG; number++) sigaction(number, 0, &saved[number]);
    pthread_sigmask(SIG_SETMASK, 0, &mask);
    sigaltstack(0, &altstack);
    ruby_vkxp_lazy_timer_thread = 1;
    ruby_vkxp_hashseed_fixed = 1;
    ruby_vkxp_hashseed = 1;
    if (getrlimit(RLIMIT_STACK, &limit) == 0 && limit.rlim_cur != RLIM_INFINITY &&
        limit.rlim_cur > 2 * VKRGSS_FUZZ_STACK_HEADROOM) {
        ruby_vkxp_stack_size = limit.rlim_cur - VKRGSS_FUZZ_STACK_HEADROOM;
    }
    else {
        ruby_vkxp_stack_size = (8 << 20) - VKRGSS_FUZZ_STACK_HEADROOM;
    }
    ruby_init_stack((VALUE *)*argv);
    ruby_init();
    rb_protect(vkrgss_fuzz_setup, Qnil, &state);
    for (number = 1; number < NSIG; number++) sigaction(number, &saved[number], 0);
    pthread_sigmask(SIG_SETMASK, &mask, 0);
    sigaltstack(&altstack, 0);
    if (state) abort();
    ruby_verbose = Qnil;
    return 0;
}

/*
 * Calls `body` with the input as a Ruby string of the encoding `encoding`. A Ruby exception
 * ends the call and is dropped: it is not a finding.
 */
static void
vkrgss_fuzz_protected(VALUE (*body)(VALUE), const uint8_t *data, size_t size,
                      rb_encoding *encoding)
{
    int state = 0;
    volatile VALUE input = rb_enc_str_new((const char *)data, (long)size, encoding);

    rb_protect(body, input, &state);
    rb_set_errinfo(Qnil);
}

#endif
