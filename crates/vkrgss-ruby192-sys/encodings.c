/* The encodings that are linked into vkrgss-ruby192-sys and registered at the first look-up
 * of their name, not at start-up: the original runtime lists four encodings when it starts
 * and yet knows Shift_JIS, Windows-31J and ISO-8859-1.
 *
 * The patched load_encoding() of encoding.c (patch `static-encodings`) calls
 * ruby_vkxp_static_encoding() with the normalised name of an encoding that is not
 * registered, where the release would require the file `enc/<name>.so`. */
#include "ruby/ruby.h"
#include "ruby/encoding.h"
#include <string.h>

void Init_shift_jis(void);
void Init_iso_8859_1(void);
/* Defined by encoding.c and declared in no header. */
int rb_enc_registered(const char *name);

/* Shift_JIS and its replica Windows-31J, reached from either name. enc/shift_jis.c defines
 * Shift_JIS alone: in the release the replica is declared by encdb.h, which is not linked.
 * After the first call both names are registered and this function is not reached again. */
static void
init_shift_jis(void)
{
    if (rb_enc_registered("Shift_JIS") < 0) {
	Init_shift_jis();
    }
    if (rb_enc_registered("Windows-31J") < 0) {
	rb_enc_replicate("Windows-31J", rb_enc_from_index(rb_enc_registered("Shift_JIS")));
    }
}

static const struct {
    const char *name;
    void (*init)(void);
} static_encodings[] = {
    {"shift_jis", init_shift_jis},
    {"windows_31j", init_shift_jis},
    {"iso_8859_1", Init_iso_8859_1},
};

void (*ruby_vkxp_static_encoding(const char *name))(void)
{
    size_t i;

    for (i = 0; i < sizeof(static_encodings) / sizeof(static_encodings[0]); i++) {
	if (strcmp(name, static_encodings[i].name) == 0) {
	    return static_encodings[i].init;
	}
    }
    return 0;
}
