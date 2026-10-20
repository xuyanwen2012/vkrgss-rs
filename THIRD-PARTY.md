# Third-party source

The MIT licence of this repository does not cover the directories below. Each stays under
its own terms.

## Ruby 1.8.7-p374

[`third_party/ruby-1.8.7`](third_party/ruby-1.8.7) holds the part of the Ruby 1.8.7-p374
release that `vkrgss-ruby-sys` builds into the embedded interpreter. It stays under Ruby's own
terms: the Ruby licence or the GPL version 2, with the exceptions its `LEGAL` file lists,
such as the LGPL for `regex.c`, which is vendored but no longer built. The texts are
[`COPYING`](third_party/ruby-1.8.7/COPYING), [`GPL`](third_party/ruby-1.8.7/GPL),
[`LGPL`](third_party/ruby-1.8.7/LGPL) and [`LEGAL`](third_party/ruby-1.8.7/LEGAL).

Its [`README.vkrgss.md`](third_party/ruby-1.8.7/README.vkrgss.md) states the origin and how the
subset was chosen, `MANIFEST.sha256` the upstream hash of every file and
[`PATCHES.md`](third_party/ruby-1.8.7/PATCHES.md) every local change.

## Ruby 1.9.2-p0

[`third_party/ruby-1.9.2`](third_party/ruby-1.9.2) holds the part of the Ruby 1.9.2-p0
release (`ruby-1.9.2-p0.tar.gz`, SHA-256
`8c0c4e261a921b5c406bf9e76ac23bf3c915651534e9d1b9e8c5d0bee4a7285c`) that `vkrgss-ruby192-sys`
builds into the interpreter for RGSS3, and the files its own tests need. It stays under
Ruby's own terms: the Ruby licence or the GPL version 2, with the exceptions its `LEGAL`
file lists. Oniguruma, the regular expression engine with the encodings, is part of this
release and is built in under its BSD terms; `parse.c`, `util.c`, `random.c`, `st.c`,
`vsnprintf.c` and `strftime.c` have the terms that `LEGAL` or the file gives for them. No
file under the LGPL is involved. The texts are [`COPYING`](third_party/ruby-1.9.2/COPYING),
[`GPL`](third_party/ruby-1.9.2/GPL) and [`LEGAL`](third_party/ruby-1.9.2/LEGAL). The guest
executable `vkrgss-guest-rgss3` links this interpreter, and the release package carries the
three texts under `licenses/ruby-1.9.2/`.

Its [`README.vkrgss.md`](third_party/ruby-1.9.2/README.vkrgss.md) states the origin, the parts
with terms of their own and how the subset was chosen,
[`MANIFEST.sha256`](third_party/ruby-1.9.2/MANIFEST.sha256) the upstream hash of every file
and [`PATCHES.md`](third_party/ruby-1.9.2/PATCHES.md) every local change.

## Oniguruma 5.9.6

[`third_party/oniguruma-5.9.6`](third_party/oniguruma-5.9.6) holds the part of the Oniguruma
5.9.6 release that regular expressions of the embedded interpreter use. It is under its BSD
licence; the text is [`COPYING`](third_party/oniguruma-5.9.6/COPYING). Its
[`README.vkrgss.md`](third_party/oniguruma-5.9.6/README.vkrgss.md) states the origin, and
`MANIFEST.sha256` the upstream hash of every file.

## CORE-MATH

[`third_party/core-math`](third_party/core-math) holds the part of CORE-MATH, the library of
correctly rounded mathematical functions of INRIA, at commit
`dae5dd8c0d5bd2691b45de28c9edbe12f067c3d3` of `https://gitlab.inria.fr/core-math/core-math.git`,
that both embedded interpreters build: the binary64 functions of `Math` and of the power of
floats. It is under the MIT licence; the text is
[`LICENSE`](third_party/core-math/LICENSE), and the release package carries it as
`licenses/core-math/LICENSE`. Its [`README.vkrgss.md`](third_party/core-math/README.vkrgss.md)
states the origin and how the subset was chosen, and
[`MANIFEST.sha256`](third_party/core-math/MANIFEST.sha256) the hash of every file at that
commit. The directory `vkrgss/` in it, through which the interpreters call the functions,
is vkrgss's own, under the licence of this repository.

`LICENSE` holds the terms but no copyright notice; each vendored file holds its own, and its
permission notice is the text of `LICENSE` (in `log/dint.h` and `log10/dint.h` with
"EXPRESS" misspelt "exp_dRESS" upstream). These are the copyright notices of the vendored
files, each distinct notice once, as the files have them; the release package carries them
in this file:

```text
Copyright (c) 2024-2025 Alexei Sibidanov.

Copyright (c) 2023 Alexei Sibidanov.

Copyright (c) 2022-2026 Alexei Sibidanov <sibid@uvic.ca>.

Copyright (c) 2023-2025 Alexei Sibidanov <sibid@uvic.ca>.

Copyright (c) 2023-2026 Alexei Sibidanov <sibid@uvic.ca>.

Copyright (c) 2024-2025 Paul Zimmermann and Alexei Sibidanov

Copyright (c) 2023-2024 Paul Zimmermann and Alexei Sibidanov

Copyright (c) 2021-2022 Alexei Sibidanov.

Copyright (c) 2022-2025 Paul Zimmermann and Tom Hubrecht

Copyright (c) 2023-2025 Alexei Sibidanov <sibid@uvic.ca>

Copyright (c) 2023-2025 Paul Zimmermann

Copyright (c) 2022-2026 Alexei Sibidanov, Paul Zimmermann, Tom Hubrecht and
Claude-Pierre Jeannerod.

Copyright (c) 2025 Alexei Sibidanov <sibid@uvic.ca>

Copyright (c) 2022 CERN.
Author: Tom Hubrecht

Copyright (c) 2022 INRIA and CERN.
Authors: Paul Zimmermann and Tom Hubrecht.

Copyright (c) 2022-2025 INRIA and CERN.
Authors: Paul Zimmermann and Tom Hubrecht.

Copyright (c) 2022, 2023 CERN and Inria
Authors: Tom Hubrecht and Paul Zimmermann

Copyright (c) 2022-2025 CERN and Inria
Authors: Tom Hubrecht and Paul Zimmermann

Copyright (c) 2022-2026 Paul Zimmermann and Tom Hubrecht and Alexei Sibidanov

Copyright (c) 2023-2026 Alexei Sibidanov and Paul Zimmermann.

Copyright (c) 2023-2026 Alexei Sibidanov, Cyprien Peignier, Paul Zimmermann

Copyright (c) 2024 Alexei Sibidanov <sibid@uvic.ca>.
```

## libxcrypt

The three executables of the release package, which `cargo xtask dist` builds, link libcrypt
statically: the libxcrypt of the distribution the package is built on, Ubuntu 24.04. No
other build does, and its source is not in this repository. libxcrypt is under the GNU
Lesser General Public License version 2.1 or later, whose text the package carries as
`licenses/ruby-1.8.7/LGPL`. Its source is at <https://github.com/besser82/libxcrypt>, and
that of the version in the package is the source package `libxcrypt` of Ubuntu 24.04. The
source of everything else that is linked is public, so the executables can be linked again
with another libxcrypt.

## MoltenVK and the Vulkan loader

The macOS app that `cargo xtask app` assembles carries two libraries whose source is not in
this repository, both under the Apache License 2.0: MoltenVK (`libMoltenVK.dylib`,
<https://github.com/KhronosGroup/MoltenVK>), copied from Homebrew, and the Vulkan loader of
Khronos (`libvulkan.1.dylib`, <https://github.com/KhronosGroup/Vulkan-Loader>), which
`cargo xtask macos-loader` builds unmodified from its source at the tag
`vulkan-sdk-1.4.363.0`, against the Vulkan headers of the same tag
(<https://github.com/KhronosGroup/Vulkan-Headers>, Apache License 2.0 or MIT), which are
compiled into it. The app carries their licence texts as
`Contents/Resources/licenses/MoltenVK/LICENSE`, from Homebrew's `molten-vk`, and
`Contents/Resources/licenses/Vulkan-Loader/LICENSE.txt`, from the loader's source. The
version of MoltenVK is the one that Homebrew's `molten-vk` had installed when the app was
built.

## Fonts used by tests

[`crates/vkrgss-text/tests/fonts`](crates/vkrgss-text/tests/fonts) holds two fonts that the tests
of `vkrgss-text` read. They are not part of the executable.

- `LiberationSans-Regular.ttf`, Liberation Sans 2.1.5, under the SIL Open Font License 1.1;
  [`LICENSE.LiberationSans.txt`](crates/vkrgss-text/tests/fonts/LICENSE.LiberationSans.txt)
  holds its copyright notices and the text of the licence.
- `NotoSansSymbols2-Regular.ttf`, Noto Sans Symbols 2 version 2.008, which names the SIL Open
  Font License 1.1 in its `name` table;
  [`LICENSE.NotoSansSymbols2.txt`](crates/vkrgss-text/tests/fonts/LICENSE.NotoSansSymbols2.txt)
  is the licence file received with it and holds the text of the Apache License 2.0, under
  which earlier Noto releases were published.

[`crates/vkrgss-text/tests/fonts-cff`](crates/vkrgss-text/tests/fonts-cff) holds a third, with CFF
outlines:

- `SourceCodePro-Regular.otf`, Source Code Pro 2.042, under the SIL Open Font License 1.1;
  [`LICENSE.SourceCodePro.md`](crates/vkrgss-text/tests/fonts-cff/LICENSE.SourceCodePro.md)
  holds its copyright notice and the text of the licence.

The [`README.md`](crates/vkrgss-text/tests/fonts/README.md) of the first directory states the
origins of all three.

## Rust crates

The Rust dependencies are not bundled. `Cargo.lock` names every crate and its version,
[`deny.toml`](deny.toml) the licences and sources that are allowed, and `cargo xtask audit`
checks the one against the other; see
[Checks outside the gate](docs/development.md#checks-outside-the-gate).
