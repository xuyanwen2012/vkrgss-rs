# Design

## Context

- Functions used, from `math.c`, `numeric.c` and `bignum.c` (2026-10-16): Ruby 1.8.7 `Math`
  has `atan2 cos sin tan acos asin atan cosh sinh tanh acosh asinh atanh exp log log10 sqrt
  frexp ldexp hypot erf erfc`; Ruby 1.9.2 adds `log2 cbrt gamma lgamma` (`tgamma`,
  `lgamma_r`). `pow` is used by `Float#**`, `Integer#**` with a float and `bignum.c`.
  `sqrt`, `fmod`, `frexp`, `ldexp`, `modf`, `floor`, `ceil` and `round` are exact in IEEE
  arithmetic and stay with the C library. `log`, `log2` and `log10` are also called in
  `util.c` and `sprintf.c`; whether a result there can reach a value a script sees decides
  whether those calls go through the shim too (check and say so here).
- Error behaviour: Ruby 1.8.7's `math.c` checks `errno` after a call (`domain_check`:
  `EDOM` raises `Errno::EDOM`, `ERANGE` raises `Errno::ERANGE` except for underflow);
  Ruby 1.9.2 checks the domain before the call and raises `Math::DomainError`. CORE-MATH
  guarantees regular inputs only; NaN, infinities, zeros, `errno` and exception flags are not
  necessarily as the C library has them.

## Source of CORE-MATH

- Cloned on 2026-10-16 to `../../vkxp/target/core-math` (read only from the flow's clone):
  `https://gitlab.inria.fr/core-math/core-math.git`, commit
  `dae5dd8c0d5bd2691b45de28c9edbe12f067c3d3` (2026-10-15). The flow has no network; take
  the files from there.
- Vendor into `third_party/core-math/` only `LICENSE`, and for each function the files
  `src/binary64/<name>/<name>.c` with the headers it includes (`dint.h`, `qint.h`, `pow.h`,
  `tint.h`, and those of `src/binary64/support` it needs), keeping their paths. A
  `README.vkrgss.md` names the commit and the selection; a `MANIFEST.sha256` and a test as
  for the other trees ("Vendored source matches upstream"), with the hash of each file as
  it is at that commit.
- A function's own test programs, `.wc` files, MPFR references and Makefiles are not
  vendored.

## Decisions

- **Where it is built.** Each interpreter's build script compiles the vendored files it
  needs into its own static library, from one list of files kept in
  `third_party/core-math/FILES` so the two scripts cannot drift. No new crate. The two
  interpreters are never linked together, so the symbols `cr_*` do not clash.
- **Flags.** `-O2 -ffp-contract=off -fno-fast-math` and the hardening flags of the
  interpreters, with `-fvisibility=hidden`. `-ffp-contract=off` is added to `FLAGS` of both
  interpreter build scripts too.
- **Shim.** One header and one C file (in `third_party/core-math/vkrgss/` or beside the
  interpreter patches; choose and say where) define `vkrgss_<name>` for each function: for a
  regular argument (finite, non-zero, inside the function's domain, result not overflowing)
  call `cr_<name>`; otherwise call the C library's function, so special values, `errno` and
  the error behaviour stay what they are. A recorded patch per interpreter makes `math.c`,
  `numeric.c` and `bignum.c` (and `util.c`, `sprintf.c` if the check above says so) call the
  shim, by including its header, which maps the names; no other line of those files changes.
- **lgamma's sign.** `cr_lgamma` sets the global `signgam`. Ruby 1.9.2 uses `lgamma_r`; the
  shim gives `vkrgss_lgamma_r(x, &sign)` by calling `cr_lgamma` and reading `signgam` right
  after. Calls of `Math` hold the interpreter's lock, so no other thread of the interpreter
  runs between the two.
- **Rounding mode.** The interpreters never change it; CORE-MATH reads it with `fegetround`
  and rounds to nearest. No change.
- **Vectors.** `../../vkxp/target/core-math-vectors/vectors.txt` (5851 lines,
  `name,input[,input2],result` in C99 hex) and its generator `gen.py` (mpmath 1.3.0 at 400
  bits, seed 20261016) are copied into the test data of `vkrgss-ruby-sys` (and used by
  `vkrgss-ruby192-sys` from there or copied), with a note that regenerating needs mpmath.
  A test of each interpreter evaluates each line through `Math` (`**` for `pow`, `Math.gamma`
  for `tgamma`, the first element of `Math.lgamma` for `lgamma`) and compares the bits;
  lines of functions an interpreter does not have are skipped. Ruby 1.8.7 has no `log2`,
  `cbrt`, `gamma` or `lgamma`.
- **Special inputs.** A test per interpreter checks that `Math.log(-1)`, `Math.sqrt(-1)`,
  `Math.acos(2)`, `Math.atanh(1)`, `Math.log(0)`, `Math.exp(1000)` and `0.0 ** -1` behave as
  before (raise the same error or return the same value).

## Out of scope

Matching any function to the original runtime's result (decided only where the oracle
shows a game depends on it, see "Floats in saves"); `Float#to_s` and `String#to_f` (already
the interpreter's own exact code); single-precision functions.

## Follow-ups

None yet.
