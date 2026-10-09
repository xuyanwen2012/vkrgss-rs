# Design

## Context

See `proposal.md` for motivation. `vkxp-vfs` and `vkxp-image` are stub crates on the guest side.
Facts that shape the design:

- The test game's `Game.ini` is `[Game]` with `Library=RGSS103J.dll`,
  `Scripts=Data\Scripts.rxdata`, a GBK-encoded title and `RTP1=Standard`. Its `Graphics` folder
  has only `Autotiles`, `Characters`, `Pictures` and `Tilesets`; titles, window skins, icons and
  all music come from the RTP, whose folders are named `Graphics/Titles`, `Audio/BGM` and so on.
- Scripts pass paths such as `Graphics/Titles/001-Title01` (no extension) and
  `Data\Scripts.rxdata` (backslashes), and games written on Windows get letter case wrong
  freely.
- The original runtime finds the RTP through the Windows registry. vkxp has no registry: the
  caller supplies a table from RTP name to directory (from configuration, added later).
- Bitmap pixels in the protocol are red, green, blue, straight alpha. The compositors refuse
  bitmaps above 2^28 pixels.
- Both crates are guest crates: no `unsafe` of their own, and they must not reach host crates.

## Goals / Non-Goals

**Goals:**

- One resolver that the binding can call for every `Bitmap.new(path)`, `load_data` and audio
  request.
- Decoders that turn untrusted file contents into pixels without panicking.

**Non-Goals:**

- The encrypted archive. The resolver is shaped so an archive can become one more search root.
- Decoding audio; this change only finds and reads audio files.
- Reproducing the original runtime's JPEG output bit for bit, and its exact extension order.

## Decisions

### `vkxp-vfs`: a resolver over ordered roots

```text
GameIni::read(game_dir, encoding_override)          -> library, scripts, title, rtp names
resolve_rtps(rtp names, name -> directory table)    -> ordered directories
GameFiles::new(game_dir, rtp directories)
GameFiles::resolve(kind, script_path)               -> real path, or NotFound(script_path)
GameFiles::read(kind, script_path)                  -> bytes
```

`kind` is image, audio or data and selects the extension list and whether RTP roots are
searched. Resolution walks the path one component at a time from a root, so `..` can be
rejected as soon as it would leave the root and symbolic links are not followed out of it by
name. Each directory is listed once and cached as a map from lower-cased name to the real
names; games do not add assets while running, and a `refresh` call drops the cache for the rare
tool that needs it.

Matching "as given" before trying extensions means `Ver1.5` finds `Ver1.5.png` through the
extension step rather than being mistaken for a file with extension `.5`.

Alternative considered: lower-casing the whole tree into an index at start-up. Rejected because
an RTP has thousands of files of which a game touches a few dozen.

### Text encoding

`Game.ini` is decoded as UTF-8 if valid, otherwise with the first of GBK, Shift-JIS and Big5
that decodes the whole file without error, using `encoding_rs`. Short titles are often valid in
more than one of these, so the order is a default, not a detection: GBK first because the
project's own test games are Chinese. The caller can name the encoding, and the later
configuration file will expose that.

### `vkxp-image`: thin wrappers with one output type

`decode(bytes) -> DecodedImage { width, height, rgba }` sniffs the signature and dispatches:

- PNG through the `png` crate with palette and low-bit-depth expansion, `tRNS` to alpha and
  16-bit stripping; gamma and colour profiles are ignored.
- JPEG through `zune-jpeg`, converted to RGBA with alpha 255. If it proves unsuitable,
  `jpeg-decoder` is the fallback; either way the choice stays inside this crate.
- BMP through a small decoder in this crate: the format subset is small and a dependency
  would bring more code than it saves.

Declared dimensions are checked against the 2^28 pixel limit before any pixel buffer is
allocated. All decoder errors map to one error enum (unsupported format, malformed data, too
large).

Alternative considered: the `image` crate. Rejected because it pulls in many formats that the
original runtime does not read and makes the output of each format harder to pin down.

### Tests synthesise their inputs

No image file is committed. Tests encode PNGs with the `png` crate, JPEGs with a
development-only encoder, and build BMPs byte by byte, from generated patterns. File-system
tests build their directory trees in a temporary directory.

## Risks / Trade-offs

- [The default encoding order misreads a Japanese title as GBK] → Cosmetic (the window title);
  the override exists.
- [Extension order differs from the original] → Only matters when one name exists with two
  extensions in the same folder; the order is a single constant.
- [JPEG pixels differ slightly from the original's decoder] → Accepted for now; decisions allow
  switching to libjpeg-turbo if bit-exactness is later required.
- [A directory changes under the cache] → Only assets are cached, saves are not resolved through
  this crate, and `refresh` exists.

## Open Questions

- The original runtime's exact extension order, to be read from the DLL or measured with the
  oracle. It changes a constant, not the design.
