//! Resolution of script-style paths over the encrypted archive, the game directory and the
//! run-time packages.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use crate::archive::{Archive, Version};
use crate::dir::Listing;
use crate::error::VfsError;
use crate::runtime::Runtime;

/// What a script asks for. The kind selects the extensions that are tried and whether the
/// run-time packages are searched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    /// A picture: searched in the game directory and the run-time packages.
    Image,
    /// Music or a sound: searched in the game directory and the run-time packages.
    Audio,
    /// Music or a sound of a game of RGSS3: searched as [`FileKind::Audio`], with the
    /// extensions in the order of RGSS3.
    AudioRgss3,
    /// A data file such as `Data/Scripts.rxdata`: searched in the game directory only, under
    /// the name as given.
    Data,
}

impl FileKind {
    /// Extensions appended, in this order, when the path as given names no file.
    // The order of RGSS1 and RGSS2 has not been measured yet; it only matters when one name
    // exists with two extensions in the same directory. That of RGSS3 was measured for 17 sets
    // of names (oracle measurement 2026-10-13, RGSS301, `ace-v1-api-REPORT.md` section 6.4);
    // its tail `mid`, `mp3`, `wav`, `wma` is taken as a fixed list (section 8, point 5).
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            FileKind::Image => &["png", "jpg", "jpeg", "bmp"],
            FileKind::Audio => &["mid", "ogg", "wav", "mp3", "wma"],
            FileKind::AudioRgss3 => &["ogg", "mid", "mp3", "wav", "wma"],
            FileKind::Data => &[],
        }
    }

    fn searches_rtps(self) -> bool {
        !matches!(self, FileKind::Data)
    }
}

/// Splits a script-style path at `/` and `\` and removes `.` and `..` components. Returns
/// `None` for a path that names nothing or that would leave the directory it is searched in.
pub(crate) fn components(script_path: &str) -> Option<Vec<&str>> {
    let mut components = Vec::new();
    for component in script_path.split(['/', '\\']) {
        match component {
            "" | "." => {}
            ".." => {
                components.pop()?;
            }
            name => components.push(name),
        }
    }
    (!components.is_empty()).then_some(components)
}

/// The names tried for the last component of a path: as given, then with each extension.
pub(crate) fn candidates(name: &str, extensions: &[&str]) -> impl Iterator<Item = String> {
    let as_given = std::iter::once(name.to_owned());
    let extended = extensions.iter().map(move |ext| format!("{name}.{ext}"));
    as_given.chain(extended)
}

/// The components of the name of a data file of RGSS3 under which the archive is asked, or
/// `None` for a name the archive is not asked for: one that starts with a separator or a
/// drive, ends with a dot or a space, or has a component that is empty, `.` or `..`.
fn archive_components_rgss3(name: &str) -> Option<Vec<&str>> {
    let bytes = name.as_bytes();
    let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if drive || name.starts_with(['/', '\\']) || name.ends_with(['.', ' ']) {
        return None;
    }
    let components: Vec<&str> = name.split(['/', '\\']).collect();
    let named = components
        .iter()
        .all(|component| !matches!(*component, "" | "." | ".."));
    named.then_some(components)
}

/// The name of a loose data file of RGSS3 with the dots and spaces at the end of its last
/// component dropped, as Windows drops them; a last component of `.` or `..` stays.
fn trim_last_component(name: &str) -> String {
    let start = name.rfind(['/', '\\']).map_or(0, |separator| separator + 1);
    let (directories, last) = name.split_at(start);
    if matches!(last, "." | "..") {
        return name.to_owned();
    }
    format!("{directories}{}", last.trim_end_matches(['.', ' ']))
}

/// The names of the normal components of `path`, or `None` when one is not valid Unicode.
fn component_names(path: &Path) -> Option<Vec<String>> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_str().map(str::to_owned)),
            _ => None,
        })
        .collect()
}

/// What a data file of a game of RGSS3 is found as ([`GameFiles::data_rgss3`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataFile {
    /// The decrypted bytes of an entry of the encrypted archive.
    Entry(Vec<u8>),
    /// A loose file inside the game directory.
    Loose(PathBuf),
}

/// Whether `path`, an entry of a directory inside `root`, is still inside `root` once a symbolic
/// link is followed. Only a link can lead out, so anything else is inside without a look at
/// the file system beyond the entry itself.
fn stays_inside(root: &Path, path: &Path) -> bool {
    match path.symlink_metadata() {
        Ok(metadata) if !metadata.file_type().is_symlink() => true,
        Ok(_) => match (root.canonicalize(), path.canonicalize()) {
            (Ok(root), Ok(target)) => target.starts_with(root),
            _ => false,
        },
        Err(_) => false,
    }
}

/// The files of one game: its encrypted archive if it was opened with one, then its own
/// directory, then its run-time package directories.
///
/// Directory listings are cached, and a cached listing is used only while its directory still
/// has the modification time it was read at, so a file or directory that a script makes,
/// removes or renames is seen by the next lookup. A listing read within two seconds of that
/// time is read again at each lookup, so a change within the same tick of the file system's
/// clock is seen too. The writers the binding knows of also call [`GameFiles::refresh`].
#[derive(Debug)]
pub struct GameFiles {
    /// Search roots in order: the game directory first.
    roots: Vec<PathBuf>,
    /// The encrypted archive, searched before the roots.
    archive: Option<Archive>,
    cache: Mutex<HashMap<PathBuf, Arc<Listing>>>,
}

impl GameFiles {
    /// Creates a resolver over `game_dir` and then `rtp_dirs`, in order. The encrypted archive
    /// is not looked for; see [`GameFiles::open`].
    pub fn new(game_dir: impl Into<PathBuf>, rtp_dirs: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut roots = vec![game_dir.into()];
        roots.extend(rtp_dirs);
        Self {
            roots,
            archive: None,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Creates a resolver as [`GameFiles::new`] does, over the encrypted archive of the game
    /// first if `game_dir` holds the archive of `runtime` under any letter case:
    /// `Game.rgssad` for RGSS1 and `Game.rgss2a` for RGSS2, read as RGSSAD version 1, and
    /// `Game.rgss3a` for RGSS3, read as version 3. The archive of another runtime is not
    /// looked at. An archive that is damaged or cannot be read is an error that names it.
    pub fn open(
        game_dir: impl Into<PathBuf>,
        rtp_dirs: impl IntoIterator<Item = PathBuf>,
        runtime: Runtime,
    ) -> Result<Self, VfsError> {
        let mut files = Self::new(game_dir, rtp_dirs);
        let archive = files
            .listing(files.game_dir())
            .matches(runtime.archive_name())
            .into_iter()
            .map(|real| files.game_dir().join(real))
            .find(|path| path.is_file());
        if let Some(path) = archive {
            let version = match runtime {
                Runtime::Rgss1 | Runtime::Rgss2 => Version::One,
                Runtime::Rgss3 => Version::Three,
            };
            files.archive = Some(Archive::open(path, version)?);
        }
        Ok(files)
    }

    /// The game directory.
    pub fn game_dir(&self) -> &Path {
        &self.roots[0]
    }

    /// The run-time package directories, in search order.
    pub fn rtp_dirs(&self) -> &[PathBuf] {
        &self.roots[1..]
    }

    /// Finds the file a script-style path names. Entries of the encrypted archive are not
    /// files and are never found; [`GameFiles::read`] returns their contents.
    ///
    /// Components are separated by `/` or `\` and match directory entries without regard to
    /// letter case, an exact match being preferred. In each root the path as given is tried
    /// first, then with each extension of `kind` appended. A symbolic link is followed only if
    /// its target lies inside the root it was found in. The error carries `script_path` exactly
    /// as given.
    pub fn resolve(&self, kind: FileKind, script_path: &str) -> Result<PathBuf, VfsError> {
        let roots = if kind.searches_rtps() {
            &self.roots[..]
        } else {
            &self.roots[..1]
        };
        components(script_path)
            .and_then(|components| {
                roots.iter().find_map(|root| {
                    self.find(root, root, &components, kind.extensions(), Path::is_file)
                })
            })
            .ok_or_else(|| VfsError::NotFound {
                path: script_path.to_owned(),
            })
    }

    /// Returns the complete contents of the entry of the encrypted archive that a script-style
    /// path names, matched as [`GameFiles::resolve`] matches files, or else of the file that
    /// [`GameFiles::resolve`] finds.
    pub fn read(&self, kind: FileKind, script_path: &str) -> Result<Vec<u8>, VfsError> {
        if let Some(archive) = &self.archive {
            let entry = components(script_path)
                .and_then(|components| archive.find(&components, kind.extensions()));
            if let Some(entry) = entry {
                return archive.read(entry);
            }
        }
        let path = self.resolve(kind, script_path)?;
        std::fs::read(&path).map_err(|source| VfsError::Io { path, source })
    }

    /// Finds the data file a game of RGSS3 names, by the rules measured on the original
    /// runtime (`ace-v1-api-REPORT.md` sections 2.1 and 2.2):
    ///
    /// - The encrypted archive first, unless the name starts with a separator or a drive,
    ///   ends with a dot or a space, or has a component that is empty, `.` or `..`. Its
    ///   components, split at `/` and `\`, match an entry without regard to letter case when
    ///   they are the entry's name, or else its name without the extension of its last
    ///   component; of several such entries the first of the table is taken. No extension is
    ///   added, and the current directory does not count.
    /// - Then a loose file, after the dots and spaces at the end of the last component are
    ///   dropped: an absolute name as it is, a relative one from `current_dir`, the current
    ///   directory of the process. `.` and `..` are resolved by the name, and a path that
    ///   then lies outside the game directory is not found; below it, components match as
    ///   [`GameFiles::resolve`] matches them for [`FileKind::Data`].
    ///
    /// A name that finds a directory of the game is [`VfsError::IsDirectory`]; a name that
    /// finds nothing is [`VfsError::NotFound`]. Both carry `name` exactly as given.
    pub fn data_rgss3(&self, name: &str, current_dir: &Path) -> Result<DataFile, VfsError> {
        if let Some(archive) = &self.archive
            && let Some(entry) =
                archive_components_rgss3(name).and_then(|parts| archive.find_data_rgss3(&parts))
        {
            return archive.read(entry).map(DataFile::Entry);
        }
        let root = self.game_dir();
        let components = self.loose_components_rgss3(name, current_dir);
        let components: Vec<&str> = components.iter().flatten().map(String::as_str).collect();
        if !components.is_empty() {
            if let Some(path) = self.find(root, root, &components, &[], Path::is_file) {
                return Ok(DataFile::Loose(path));
            }
            if self
                .find(root, root, &components, &[], Path::is_dir)
                .is_some()
            {
                return Err(VfsError::IsDirectory {
                    path: name.to_owned(),
                });
            }
        }
        Err(VfsError::NotFound {
            path: name.to_owned(),
        })
    }

    /// The components below the game directory of the loose file a data file of RGSS3 names
    /// (see [`GameFiles::data_rgss3`]), or `None` for a name outside the game directory.
    fn loose_components_rgss3(&self, name: &str, current_dir: &Path) -> Option<Vec<String>> {
        let game = self.game_dir();
        let game = component_names(&game.canonicalize().unwrap_or_else(|_| game.to_owned()))?;
        let name = trim_last_component(name);
        let mut path = if name.starts_with(['/', '\\']) {
            Vec::new()
        } else {
            component_names(current_dir)?
        };
        for component in name.split(['/', '\\']) {
            match component {
                "" | "." => {}
                ".." => {
                    path.pop();
                }
                component => path.push(component.to_owned()),
            }
        }
        path.starts_with(&game).then(|| path.split_off(game.len()))
    }

    /// Drops the cached directory listings, so files added or removed since they were listed
    /// are seen.
    pub fn refresh(&self) {
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    /// Finds `components` below `dir`, a directory inside `root`, as something `wanted`
    /// accepts: a file, or a directory. When several entries match a directory component,
    /// each is searched in turn. Entries that are symbolic links to somewhere outside `root`
    /// are passed over.
    fn find(
        &self,
        root: &Path,
        dir: &Path,
        components: &[&str],
        extensions: &[&str],
        wanted: fn(&Path) -> bool,
    ) -> Option<PathBuf> {
        let (name, rest) = components.split_first()?;
        let listing = self.listing(dir);
        if rest.is_empty() {
            candidates(name, extensions).find_map(|candidate| {
                listing
                    .matches(&candidate)
                    .into_iter()
                    .map(|real| dir.join(real))
                    .find(|path| stays_inside(root, path) && wanted(path))
            })
        } else {
            listing
                .matches(name)
                .into_iter()
                .map(|real| dir.join(real))
                .filter(|path| stays_inside(root, path))
                .find_map(|path| self.find(root, &path, rest, extensions, wanted))
        }
    }

    /// The listing of `dir`, read anew unless the cached one is still current. Something that
    /// cannot be listed has no entries and is not cached, so a directory made later is seen.
    fn listing(&self, dir: &Path) -> Arc<Listing> {
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(listing) = cache.get(dir)
            && listing.is_current(dir)
        {
            return Arc::clone(listing);
        }
        match Listing::read(dir) {
            Ok(listing) => {
                let listing = Arc::new(listing);
                cache.insert(dir.to_owned(), Arc::clone(&listing));
                listing
            }
            Err(_) => {
                cache.remove(dir);
                Arc::new(Listing::empty())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates `relative` below `root`, with its parent directories, holding `contents`.
    fn put(root: &Path, relative: &str, contents: &[u8]) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, contents).unwrap();
        path
    }

    fn game_only(game: &Path) -> GameFiles {
        GameFiles::new(game, [])
    }

    /// Whether the file system of `dir` folds case, as that of macOS does by default: a name
    /// that differs from that of a file only in case names that file.
    fn folds_case(dir: &Path) -> bool {
        let probe = put(dir, "case-probe", b"");
        let folds = dir.join("CASE-PROBE").exists();
        std::fs::remove_file(probe).unwrap();
        folds
    }

    #[test]
    fn backslashes_and_case() {
        let game = tempfile::tempdir().unwrap();
        let file = put(game.path(), "Graphics/Titles/001-Title01.png", b"");
        let files = game_only(game.path());
        let found = files.resolve(FileKind::Image, "graphics\\TITLES\\001-title01.PNG");
        assert_eq!(found.unwrap(), file);
        let found = files.resolve(FileKind::Image, "./Graphics//Titles/001-Title01.png");
        assert_eq!(found.unwrap(), file);
    }

    #[test]
    fn exact_case_wins() {
        let game = tempfile::tempdir().unwrap();
        let folds = folds_case(game.path());
        let lower = put(game.path(), "a.png", b"");
        let upper = put(game.path(), "A.png", b"");
        let files = game_only(game.path());
        if folds {
            // The second name wrote the first file, which keeps its name and is found by
            // either.
            for name in ["A.png", "a.png", "A"] {
                assert_eq!(files.resolve(FileKind::Image, name).unwrap(), lower);
            }
            return;
        }
        assert_eq!(files.resolve(FileKind::Image, "A.png").unwrap(), upper);
        assert_eq!(files.resolve(FileKind::Image, "a.png").unwrap(), lower);
        assert_eq!(files.resolve(FileKind::Image, "A").unwrap(), upper);
    }

    #[test]
    fn directories_that_differ_in_case_are_all_searched() {
        let game = tempfile::tempdir().unwrap();
        let folds = folds_case(game.path());
        let first = put(game.path(), "Graphics/one.png", b"");
        let second = put(game.path(), "graphics/two.png", b"");
        let files = game_only(game.path());
        if folds {
            // There is one directory, under the name it was made with.
            assert_eq!(
                files.resolve(FileKind::Image, "GRAPHICS/one").unwrap(),
                first
            );
            assert_eq!(
                files.resolve(FileKind::Image, "GRAPHICS/two").unwrap(),
                game.path().join("Graphics/two.png")
            );
            return;
        }
        assert_eq!(
            files.resolve(FileKind::Image, "GRAPHICS/one").unwrap(),
            first
        );
        assert_eq!(
            files.resolve(FileKind::Image, "GRAPHICS/two").unwrap(),
            second
        );
    }

    #[test]
    fn escaping_the_root() {
        let outer = tempfile::tempdir().unwrap();
        put(outer.path(), "secret.png", b"");
        let game = outer.path().join("deep/game");
        let inside = put(&game, "Graphics/inside.png", b"");
        let files = game_only(&game);
        for path in [
            "Graphics/../../secret.png",
            "Graphics/../../../secret.png",
            "../../secret.png",
            "..\\..\\secret",
            "..",
        ] {
            assert!(
                matches!(
                    files.resolve(FileKind::Image, path),
                    Err(VfsError::NotFound { .. })
                ),
                "{path}"
            );
        }
        // Staying inside the root is fine.
        let found = files.resolve(FileKind::Image, "Graphics/../Graphics/inside");
        assert_eq!(found.unwrap(), inside);
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_links_do_not_lead_out_of_the_root() {
        use std::os::unix::fs::symlink;

        let outer = tempfile::tempdir().unwrap();
        put(outer.path(), "outside/secret.png", b"secret");
        put(outer.path(), "outside/Sub/deep.png", b"secret");
        for root in ["game", "rtp"] {
            let root = outer.path().join(root);
            std::fs::create_dir(&root).unwrap();
            symlink("../outside", root.join("link")).unwrap();
            symlink(outer.path().join("outside"), root.join("absolute")).unwrap();
            symlink("../outside/secret.png", root.join("direct.png")).unwrap();
            symlink("../outside/secret.png", root.join("bare")).unwrap();
            symlink("missing", root.join("dangling.png")).unwrap();
        }
        let files = GameFiles::new(outer.path().join("game"), [outer.path().join("rtp")]);
        for path in [
            "link/secret",
            "link/secret.png",
            "LINK\\Sub\\deep",
            "absolute/secret",
            "direct",
            "direct.png",
            "bare",
            "dangling",
        ] {
            for kind in [FileKind::Image, FileKind::Data] {
                assert!(
                    matches!(files.read(kind, path), Err(VfsError::NotFound { .. })),
                    "{path}"
                );
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_links_inside_the_root_are_followed() {
        use std::os::unix::fs::symlink;

        let outer = tempfile::tempdir().unwrap();
        let game = outer.path().join("game");
        let rtp = outer.path().join("rtp");
        put(&game, "Graphics/Real/picture.png", b"own");
        symlink("Real", game.join("Graphics/Alias")).unwrap();
        symlink("Real/picture.png", game.join("Graphics/copy.png")).unwrap();
        put(&rtp, "Audio/Real/theme.ogg", b"package");
        symlink("Real", rtp.join("Audio/BGM")).unwrap();
        // The roots themselves may be reached through links.
        symlink(&game, outer.path().join("game-link")).unwrap();
        symlink(&rtp, outer.path().join("rtp-link")).unwrap();
        let files = GameFiles::new(
            outer.path().join("game-link"),
            [outer.path().join("rtp-link")],
        );
        let read = |kind, path| files.read(kind, path).unwrap();
        assert_eq!(read(FileKind::Image, "graphics/alias/Picture"), b"own");
        assert_eq!(read(FileKind::Image, "Graphics/copy"), b"own");
        assert_eq!(read(FileKind::Audio, "Audio/BGM/theme"), b"package");
        // A link from a package into the game directory leaves the package.
        symlink(game.join("Graphics/Real"), rtp.join("Audio/Game")).unwrap();
        files.refresh();
        assert!(files.read(FileKind::Image, "Audio/Game/picture").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_out_of_the_root_does_not_hide_a_later_match() {
        use std::os::unix::fs::symlink;

        let outer = tempfile::tempdir().unwrap();
        put(outer.path(), "outside/Fog.png", b"secret");
        let game = outer.path().join("game");
        let rtp = outer.path().join("rtp");
        let jpg = put(&game, "Fog.jpg", b"own");
        symlink("../outside/Fog.png", game.join("Fog.png")).unwrap();
        symlink("../outside", game.join("Graphics")).unwrap();
        let packaged = put(&rtp, "Graphics/Fog.png", b"package");
        let files = GameFiles::new(&game, [rtp]);
        // The next extension in the same directory, then the next root.
        assert_eq!(files.resolve(FileKind::Image, "Fog").unwrap(), jpg);
        let found = files.resolve(FileKind::Image, "Graphics/Fog").unwrap();
        assert_eq!(found, packaged);
    }

    /// Lets the clock pass the resolution of the file system's time stamps, so that the next
    /// change of a directory gives it a modification time of its own.
    fn let_time_stamps_advance() {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    /// Scenario "New file in a directory already looked into".
    #[test]
    fn a_new_file_in_a_directory_already_looked_into_is_found() {
        let game = tempfile::tempdir().unwrap();
        let a = put(game.path(), "Graphics/Pictures/a.png", b"");
        let files = game_only(game.path());
        let found = files.resolve(FileKind::Image, "Graphics/Pictures/a");
        assert_eq!(found.unwrap(), a);
        let_time_stamps_advance();
        let b = put(game.path(), "Graphics/Pictures/b.png", b"");
        let found = files.resolve(FileKind::Image, "Graphics/Pictures/b");
        assert_eq!(found.unwrap(), b);
    }

    /// Scenario "Change within one tick". The directory's time is set by hand, so the test
    /// does not depend on how coarse the clock of the file system is.
    #[test]
    fn a_change_within_one_tick_is_found() {
        let game = tempfile::tempdir().unwrap();
        let a = put(game.path(), "Graphics/Pictures/a.png", b"");
        let pictures = std::fs::File::open(game.path().join("Graphics/Pictures")).unwrap();
        let listed_at = std::time::SystemTime::now();
        pictures.set_modified(listed_at).unwrap();
        let files = game_only(game.path());
        let found = files.resolve(FileKind::Image, "Graphics/Pictures/a");
        assert_eq!(found.unwrap(), a);
        let b = put(game.path(), "Graphics/Pictures/b.png", b"");
        pictures.set_modified(listed_at).unwrap();
        let found = files.resolve(FileKind::Image, "Graphics/Pictures/b");
        assert_eq!(found.unwrap(), b);
    }

    /// Scenario "New directory".
    #[test]
    fn a_new_directory_is_found() {
        let game = tempfile::tempdir().unwrap();
        let files = game_only(game.path());
        assert!(matches!(
            files.resolve(FileKind::Image, "Save/1"),
            Err(VfsError::NotFound { .. })
        ));
        let_time_stamps_advance();
        let file = put(game.path(), "Save/1.cdat", b"");
        let found = files.resolve(FileKind::Image, "Save/1.cdat");
        assert_eq!(found.unwrap(), file);
    }

    /// Scenario "Removed file".
    #[test]
    fn a_removed_file_is_missing() {
        let game = tempfile::tempdir().unwrap();
        let file = put(game.path(), "Graphics/Pictures/a.png", b"");
        let files = game_only(game.path());
        let found = files.resolve(FileKind::Image, "Graphics/Pictures/a");
        assert_eq!(found.unwrap(), file);
        let_time_stamps_advance();
        std::fs::remove_file(file).unwrap();
        let error = files
            .resolve(FileKind::Image, "Graphics/Pictures/a")
            .unwrap_err();
        assert!(
            matches!(&error, VfsError::NotFound { path } if path == "Graphics/Pictures/a"),
            "{error}"
        );
    }

    #[test]
    fn a_file_renamed_away_is_missing_and_found_under_its_new_name() {
        let game = tempfile::tempdir().unwrap();
        let old = put(game.path(), "Graphics/old.png", b"");
        let files = game_only(game.path());
        assert_eq!(files.resolve(FileKind::Image, "Graphics/old").unwrap(), old);
        let_time_stamps_advance();
        let new = game.path().join("Graphics/new.png");
        std::fs::rename(old, &new).unwrap();
        assert!(files.resolve(FileKind::Image, "Graphics/old").is_err());
        assert_eq!(files.resolve(FileKind::Image, "Graphics/new").unwrap(), new);
    }

    #[test]
    fn extension_omitted() {
        let game = tempfile::tempdir().unwrap();
        let file = put(game.path(), "Graphics/Titles/001-Title01.jpg", b"");
        let files = game_only(game.path());
        let found = files.resolve(FileKind::Image, "Graphics/Titles/001-Title01");
        assert_eq!(found.unwrap(), file);
        // The extension is matched without regard to case too.
        let shout = put(game.path(), "Graphics/Titles/Shout.BMP", b"");
        files.refresh();
        let found = files.resolve(FileKind::Image, "Graphics/Titles/shout");
        assert_eq!(found.unwrap(), shout);
    }

    #[test]
    fn order_of_extensions() {
        let game = tempfile::tempdir().unwrap();
        for ext in ["bmp", "jpeg", "jpg"] {
            put(game.path(), &format!("Fog.{ext}"), b"");
        }
        let files = game_only(game.path());
        let name = |path: PathBuf| path.file_name().unwrap().to_str().unwrap().to_owned();
        assert_eq!(
            name(files.resolve(FileKind::Image, "Fog").unwrap()),
            "Fog.jpg"
        );
        put(game.path(), "Fog.png", b"");
        files.refresh();
        assert_eq!(
            name(files.resolve(FileKind::Image, "Fog").unwrap()),
            "Fog.png"
        );

        for ext in ["wma", "mp3", "wav", "ogg", "mid"] {
            put(game.path(), &format!("Theme.{ext}"), b"");
            files.refresh();
            let found = name(files.resolve(FileKind::Audio, "Theme").unwrap());
            assert_eq!(found, format!("Theme.{ext}"));
        }
        // Extensions of another kind are not tried.
        assert!(files.resolve(FileKind::Audio, "Fog").is_err());
        assert!(files.resolve(FileKind::Data, "Fog").is_err());
    }

    #[test]
    fn name_that_contains_a_dot() {
        let game = tempfile::tempdir().unwrap();
        let file = put(game.path(), "Ver1.5.png", b"");
        let files = game_only(game.path());
        assert_eq!(files.resolve(FileKind::Image, "Ver1.5").unwrap(), file);
    }

    #[test]
    fn name_as_given_wins_over_an_appended_extension() {
        let game = tempfile::tempdir().unwrap();
        put(game.path(), "Fog.png", b"");
        let bare = put(game.path(), "Fog", b"");
        std::fs::create_dir(game.path().join("Mist")).unwrap();
        let mist = put(game.path(), "Mist.png", b"");
        let files = game_only(game.path());
        assert_eq!(files.resolve(FileKind::Image, "Fog").unwrap(), bare);
        // A directory is not a file, so the extension step is reached.
        assert_eq!(files.resolve(FileKind::Image, "Mist").unwrap(), mist);
    }

    #[test]
    fn game_overrides_the_package() {
        let game = tempfile::tempdir().unwrap();
        let rtp = tempfile::tempdir().unwrap();
        // An extension step in the game directory still comes before the package.
        let own = put(game.path(), "Graphics/Pictures/Logo.bmp", b"");
        put(rtp.path(), "Graphics/Pictures/Logo", b"");
        put(rtp.path(), "Graphics/Pictures/Logo.png", b"");
        let files = GameFiles::new(game.path(), [rtp.path().to_owned()]);
        let found = files.resolve(FileKind::Image, "Graphics/Pictures/Logo");
        assert_eq!(found.unwrap(), own);
    }

    #[test]
    fn falls_back_to_the_package() {
        let game = tempfile::tempdir().unwrap();
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        put(game.path(), "Graphics/Pictures/Other.png", b"");
        put(first.path(), "Graphics/Titles/Other.png", b"");
        let file = put(second.path(), "Graphics/Titles/001-Title01.png", b"");
        let both = put(second.path(), "Audio/BGM/Theme.ogg", b"");
        put(first.path(), "Audio/BGM/Theme.wma", b"");
        let files = GameFiles::new(
            game.path(),
            [first.path().to_owned(), second.path().to_owned()],
        );
        assert_eq!(files.game_dir(), game.path());
        assert_eq!(files.rtp_dirs().len(), 2);
        let found = files.resolve(FileKind::Image, "Graphics/Titles/001-Title01");
        assert_eq!(found.unwrap(), file);
        // The first package that has the name wins, whatever the extension.
        let found = files.resolve(FileKind::Audio, "Audio/BGM/Theme").unwrap();
        assert_ne!(found, both);
        assert!(found.starts_with(first.path()));
    }

    #[test]
    fn data_is_not_taken_from_a_package() {
        let game = tempfile::tempdir().unwrap();
        let rtp = tempfile::tempdir().unwrap();
        put(rtp.path(), "Data/Scripts.rxdata", b"");
        let files = GameFiles::new(game.path(), [rtp.path().to_owned()]);
        assert!(
            files
                .resolve(FileKind::Data, "Data/Scripts.rxdata")
                .is_err()
        );
        let own = put(game.path(), "Data/Scripts.rxdata", b"");
        files.refresh();
        let found = files.resolve(FileKind::Data, "Data\\Scripts.rxdata");
        assert_eq!(found.unwrap(), own);
    }

    #[test]
    fn missing_file() {
        let game = tempfile::tempdir().unwrap();
        let files = game_only(game.path());
        for path in ["Graphics/Pictures/NoSuchPicture", "", "a\\..\\..\\b"] {
            let error = files.resolve(FileKind::Image, path).unwrap_err();
            assert!(matches!(&error, VfsError::NotFound { path: carried } if carried == path));
        }
        let error = files
            .read(FileKind::Image, "Graphics/Pictures/NoSuchPicture")
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Graphics/Pictures/NoSuchPicture")
        );
    }

    /// Writes an archive holding `entries` as `name` into `game` and opens the game's files
    /// as those of a game of `runtime`.
    fn with_archive(
        game: &Path,
        name: &str,
        runtime: Runtime,
        entries: &[(&str, &[u8])],
    ) -> GameFiles {
        std::fs::write(game.join(name), crate::archive::tests::build(entries)).unwrap();
        GameFiles::open(game, [], runtime).unwrap()
    }

    #[test]
    fn data_from_the_archive() {
        let game = tempfile::tempdir().unwrap();
        let entries: [(&str, &[u8]); 1] = [("Data\\Scripts.rxdata", b"scripts")];
        let files = with_archive(game.path(), "Game.rgssad", Runtime::Rgss1, &entries);
        let bytes = files.read(FileKind::Data, "Data/Scripts.rxdata");
        assert_eq!(bytes.unwrap(), b"scripts");
        // The archive's own name is matched without regard to case.
        let other = tempfile::tempdir().unwrap();
        let files = with_archive(other.path(), "game.RGSSAD", Runtime::Rgss1, &entries);
        let bytes = files.read(FileKind::Data, "data\\scripts.RXDATA");
        assert_eq!(bytes.unwrap(), b"scripts");
    }

    #[test]
    fn image_from_the_archive_with_extension_search() {
        let game = tempfile::tempdir().unwrap();
        let files = with_archive(
            game.path(),
            "Game.rgssad",
            Runtime::Rgss1,
            &[("Graphics\\Titles\\Title.png", b"picture")],
        );
        let bytes = files.read(FileKind::Image, "graphics/titles/title");
        assert_eq!(bytes.unwrap(), b"picture");
        // The extensions of another kind are not tried.
        assert!(
            files
                .read(FileKind::Audio, "graphics/titles/title")
                .is_err()
        );
    }

    #[test]
    fn archive_overrides_loose_files_and_those_the_package() {
        let game = tempfile::tempdir().unwrap();
        let rtp = tempfile::tempdir().unwrap();
        put(game.path(), "Data/Map001.rxdata", b"loose");
        put(game.path(), "Data/Map002.rxdata", b"loose only");
        put(game.path(), "Graphics/Fogs/Fog.png", b"loose fog");
        put(rtp.path(), "Graphics/Fogs/Fog.png", b"package fog");
        put(rtp.path(), "Graphics/Fogs/Mist.png", b"package mist");
        let archive = crate::archive::tests::build(&[
            ("Data\\Map001.rxdata", b"archive"),
            // An extension step in the archive still comes before a loose file.
            ("Graphics\\Fogs\\Fog.bmp", b"archive fog"),
        ]);
        std::fs::write(game.path().join("Game.rgssad"), archive).unwrap();
        let files = GameFiles::open(game.path(), [rtp.path().to_owned()], Runtime::Rgss1).unwrap();
        let read = |kind, path| files.read(kind, path).unwrap();
        assert_eq!(read(FileKind::Data, "Data/Map001.rxdata"), b"archive");
        assert_eq!(read(FileKind::Data, "Data/Map002.rxdata"), b"loose only");
        assert_eq!(read(FileKind::Image, "Graphics/Fogs/Fog"), b"archive fog");
        assert_eq!(read(FileKind::Image, "Graphics/Fogs/Mist"), b"package mist");
        // Without the archive, loose files still come before the package.
        std::fs::remove_file(game.path().join("Game.rgssad")).unwrap();
        let files = GameFiles::open(game.path(), [rtp.path().to_owned()], Runtime::Rgss1).unwrap();
        assert_eq!(
            files.read(FileKind::Image, "Graphics/Fogs/Fog").unwrap(),
            b"loose fog"
        );
    }

    #[test]
    fn no_archive() {
        let game = tempfile::tempdir().unwrap();
        let rtp = tempfile::tempdir().unwrap();
        let own = put(game.path(), "Data/Scripts.rxdata", b"own");
        let packaged = put(rtp.path(), "Graphics/Titles/Title.png", b"package");
        let files = GameFiles::open(game.path(), [rtp.path().to_owned()], Runtime::Rgss1).unwrap();
        let found = files.resolve(FileKind::Data, "data\\scripts.rxdata");
        assert_eq!(found.unwrap(), own);
        let found = files.resolve(FileKind::Image, "Graphics/Titles/Title");
        assert_eq!(found.unwrap(), packaged);
        assert_eq!(
            files.read(FileKind::Data, "Data/Scripts.rxdata").unwrap(),
            b"own"
        );
        let error = files
            .read(FileKind::Data, "Data/Absent.rxdata")
            .unwrap_err();
        assert!(matches!(error, VfsError::NotFound { .. }));
        // A game directory that does not exist has no archive either.
        assert!(GameFiles::open(game.path().join("absent"), [], Runtime::Rgss1).is_ok());
    }

    #[test]
    fn archive_of_vx() {
        let game = tempfile::tempdir().unwrap();
        let entries: [(&str, &[u8]); 1] = [("Data\\Scripts.rvdata", b"scripts")];
        let files = with_archive(game.path(), "Game.rgss2a", Runtime::Rgss2, &entries);
        let bytes = files.read(FileKind::Data, "Data/Scripts.rvdata");
        assert_eq!(bytes.unwrap(), b"scripts");
    }

    #[test]
    fn archive_of_another_runtime_is_ignored() {
        let game = tempfile::tempdir().unwrap();
        let own = put(game.path(), "Data/Scripts.rxdata", b"own");
        // Not an archive at all: it is not even opened.
        std::fs::write(game.path().join("Game.rgss2a"), b"other").unwrap();
        let files = GameFiles::open(game.path(), [], Runtime::Rgss1).unwrap();
        let found = files.resolve(FileKind::Data, "Data/Scripts.rxdata");
        assert_eq!(found.unwrap(), own);
        assert_eq!(
            files.read(FileKind::Data, "Data/Scripts.rxdata").unwrap(),
            b"own"
        );
        std::fs::rename(
            game.path().join("Game.rgss2a"),
            game.path().join("Game.rgssad"),
        )
        .unwrap();
        for runtime in [Runtime::Rgss2, Runtime::Rgss3] {
            assert!(GameFiles::open(game.path(), [], runtime).is_ok());
        }
    }

    /// Entries of a version 3 archive with the file keys of the editor's first entries.
    fn three(entries: &[(&'static str, &'static [u8])]) -> Vec<u8> {
        let entries: Vec<(&str, u32, &[u8])> = entries
            .iter()
            .zip([41, 18467, 6334])
            .map(|(&(name, contents), key)| (name, key, contents))
            .collect();
        crate::archive::tests::Three::seed(0x2A38).build(&entries)
    }

    #[test]
    fn archive_of_vx_ace() {
        let game = tempfile::tempdir().unwrap();
        let archive = three(&[
            ("Data\\Scripts.rvdata2", b"scripts"),
            ("Graphics\\System\\Window.png", b"window"),
        ]);
        std::fs::write(game.path().join("Game.RGSS3A"), archive).unwrap();
        let files = GameFiles::open(game.path(), [], Runtime::Rgss3).unwrap();
        let bytes = files.read(FileKind::Data, "Data/Scripts.rvdata2");
        assert_eq!(bytes.unwrap(), b"scripts");
        let bytes = files.read(FileKind::Image, "graphics/system/window");
        assert_eq!(bytes.unwrap(), b"window");
    }

    #[test]
    fn archive_of_vx_ace_overrides_loose_files() {
        let game = tempfile::tempdir().unwrap();
        put(game.path(), "Data/Scripts.rvdata2", b"loose");
        put(game.path(), "Data/Map001.rvdata2", b"loose only");
        let archive = three(&[("Data\\Scripts.rvdata2", b"archive")]);
        std::fs::write(game.path().join("Game.rgss3a"), archive).unwrap();
        let files = GameFiles::open(game.path(), [], Runtime::Rgss3).unwrap();
        let read = |path| files.read(FileKind::Data, path).unwrap();
        assert_eq!(read("Data/Scripts.rvdata2"), b"archive");
        assert_eq!(read("Data/Map001.rvdata2"), b"loose only");
    }

    #[test]
    fn other_archives_beside_vx_ace() {
        // Neither file is an archive at all: it is not even opened.
        for (name, runtime, scripts) in [
            ("Game.rgss2a", Runtime::Rgss3, "Data/Scripts.rvdata2"),
            ("Game.rgss3a", Runtime::Rgss2, "Data/Scripts.rvdata"),
        ] {
            let game = tempfile::tempdir().unwrap();
            put(game.path(), scripts, b"own");
            std::fs::write(game.path().join(name), b"other").unwrap();
            let files = GameFiles::open(game.path(), [], runtime).unwrap();
            assert_eq!(files.read(FileKind::Data, scripts).unwrap(), b"own");
        }
    }

    #[test]
    fn damaged_archive_of_vx_ace_fails_the_opening() {
        let game = tempfile::tempdir().unwrap();
        let path = game.path().join("Game.rgss3a");
        let whole = three(&[("Data\\Scripts.rvdata2", b"scripts")]);
        let version_one = crate::archive::tests::build(&[("Data\\Scripts.rvdata2", b"scripts")]);
        for bytes in [&version_one[..], &whole[..20], &whole[..whole.len() - 3]] {
            std::fs::write(&path, bytes).unwrap();
            let error = GameFiles::open(game.path(), [], Runtime::Rgss3).unwrap_err();
            assert!(
                matches!(&error, VfsError::DamagedArchive { path: named, .. } if *named == path)
            );
            assert!(error.to_string().contains("Game.rgss3a"));
        }
    }

    #[test]
    fn damaged_archive_fails_the_opening() {
        let game = tempfile::tempdir().unwrap();
        let path = game.path().join("Game.rgssad");
        let whole = crate::archive::tests::build(&[("Data\\Scripts.rxdata", b"scripts")]);
        for bytes in [b"not an archive at all", &whole[..whole.len() - 3]] {
            std::fs::write(&path, bytes).unwrap();
            let error = GameFiles::open(game.path(), [], Runtime::Rgss1).unwrap_err();
            assert!(
                matches!(&error, VfsError::DamagedArchive { path: named, .. } if *named == path)
            );
            assert!(error.to_string().contains("Game.rgssad"));
        }
    }

    #[test]
    fn read() {
        let game = tempfile::tempdir().unwrap();
        let rtp = tempfile::tempdir().unwrap();
        let contents: Vec<u8> = (0..=255u8).cycle().take(70_000).collect();
        put(rtp.path(), "Audio/SE/001-System01.ogg", &contents);
        let files = GameFiles::new(game.path(), [rtp.path().to_owned()]);
        let bytes = files.read(FileKind::Audio, "Audio/SE/001-System01");
        assert_eq!(bytes.unwrap(), contents);
    }

    #[test]
    fn order_of_the_audio_of_rgss3() {
        let game = tempfile::tempdir().unwrap();
        let name = |path: PathBuf| path.file_name().unwrap().to_str().unwrap().to_owned();
        let all = ["ogg", "mid", "mp3", "wav", "wma"];
        for ext in all {
            put(game.path(), &format!("Theme.{ext}"), b"");
        }
        let files = game_only(game.path());
        let found = name(files.resolve(FileKind::AudioRgss3, "Theme").unwrap());
        assert_eq!(found, "Theme.ogg");
        // The other runtimes keep their order.
        let found = name(files.resolve(FileKind::Audio, "Theme").unwrap());
        assert_eq!(found, "Theme.mid");
        // Each pair of the tail.
        let tail = ["mid", "mp3", "wav", "wma"];
        for (index, first) in tail.iter().enumerate() {
            for second in &tail[index + 1..] {
                let game = tempfile::tempdir().unwrap();
                put(game.path(), &format!("Pair.{second}"), b"");
                put(game.path(), &format!("Pair.{first}"), b"");
                let files = game_only(game.path());
                let found = name(files.resolve(FileKind::AudioRgss3, "Pair").unwrap());
                assert_eq!(found, format!("Pair.{first}"));
            }
        }
    }

    #[test]
    fn audio_of_rgss3_in_a_package_and_in_the_archive() {
        let game = tempfile::tempdir().unwrap();
        let rtp = tempfile::tempdir().unwrap();
        put(rtp.path(), "Audio/BGM/Theme.ogg", b"package");
        let files = GameFiles::open(game.path(), [rtp.path().to_owned()], Runtime::Rgss3).unwrap();
        let bytes = files.read(FileKind::AudioRgss3, "Audio/BGM/Theme");
        assert_eq!(bytes.unwrap(), b"package");

        let other = tempfile::tempdir().unwrap();
        let archive = three(&[("Audio\\SE\\Hit.ogg", b"archive")]);
        std::fs::write(other.path().join("Game.rgss3a"), archive).unwrap();
        let files = GameFiles::open(other.path(), [], Runtime::Rgss3).unwrap();
        let bytes = files.read(FileKind::AudioRgss3, "Audio/SE/Hit");
        assert_eq!(bytes.unwrap(), b"archive");
    }

    /// The game directory of a game of RGSS3 whose archive holds `entries`, opened, and its
    /// path with links resolved, which is what the current directory of the process names.
    fn ace_with_archive(entries: &[(&str, &[u8])]) -> (tempfile::TempDir, GameFiles, PathBuf) {
        let game = tempfile::tempdir().unwrap();
        let entries: Vec<(&str, u32, &[u8])> = entries
            .iter()
            .zip(1u32..)
            .map(|(&(name, contents), key)| (name, key * 7919, contents))
            .collect();
        let archive = crate::archive::tests::Three::seed(0x2A38).build(&entries);
        std::fs::write(game.path().join("Game.rgss3a"), archive).unwrap();
        let files = GameFiles::open(game.path(), [], Runtime::Rgss3).unwrap();
        let dir = game.path().canonicalize().unwrap();
        (game, files, dir)
    }

    /// The contents of what `name` finds as a data file of RGSS3 from `current`: `Entry` or
    /// `Loose` and the bytes, or the error.
    fn data(
        files: &GameFiles,
        name: &str,
        current: &Path,
    ) -> Result<(&'static str, Vec<u8>), VfsError> {
        match files.data_rgss3(name, current)? {
            DataFile::Entry(bytes) => Ok(("Entry", bytes)),
            DataFile::Loose(path) => Ok(("Loose", std::fs::read(path).unwrap())),
        }
    }

    fn not_found(result: Result<(&'static str, Vec<u8>), VfsError>, name: &str) {
        match result {
            Err(VfsError::NotFound { path }) => assert_eq!(path, name),
            other => panic!("{name}: {other:?}"),
        }
    }

    /// Requirement "Data files of RGSS3", scenario "Archive first".
    #[test]
    fn a_data_file_of_rgss3_comes_from_the_archive_first() {
        let (_game, files, dir) = ace_with_archive(&[("Data\\both.rvdata2", b"archive")]);
        // Written while the game runs, after the archive was opened.
        put(&dir, "Data/both.rvdata2", b"loose");
        for name in [
            "Data/both.rvdata2",
            "Data\\both.rvdata2",
            "DATA/BOTH.rvdata2",
        ] {
            let found = data(&files, name, &dir).unwrap();
            assert_eq!(found, ("Entry", b"archive".to_vec()), "{name}");
        }
    }

    /// Requirement "Data files of RGSS3", scenario "Without the extension".
    #[test]
    fn an_entry_of_rgss3_is_found_by_its_name_without_the_extension() {
        let (_game, files, dir) = ace_with_archive(&[
            ("Data\\arconly.rvdata2", b"arconly"),
            ("Other\\x.bin", b"x"),
            ("Data\\noext", b"noext"),
        ]);
        for (name, bytes) in [
            ("Data/arconly", &b"arconly"[..]),
            ("Data\\arconly", b"arconly"),
            ("data/ARCONLY.RVDATA2", b"arconly"),
            ("Other/x", b"x"),
            ("Data/noext", b"noext"),
        ] {
            let found = data(&files, name, &dir).unwrap();
            assert_eq!(found, ("Entry", bytes.to_vec()), "{name}");
        }
        for name in [
            "Data/arconly.rvdata",
            "Data/arconly.rvdata2.rvdata2",
            "Data/arconly.zzz",
            "Data/arconl",
            "Data/arconly.",
            "Data/noext.rvdata2",
        ] {
            not_found(data(&files, name, &dir), name);
        }
    }

    /// Requirement "Data files of RGSS3", scenario "Names the archive is not asked for".
    #[test]
    fn names_with_dots_and_separators_skip_the_archive_of_rgss3() {
        let (_game, files, dir) = ace_with_archive(&[
            ("Data\\arconly.rvdata2", b"arconly"),
            ("Data\\both.rvdata2", b"archive"),
        ]);
        put(&dir, "Data/both.rvdata2", b"loose");
        let absolute = |name: &str| format!("{}/{name}", dir.display());
        for stem in ["arconly", "both"] {
            let names = [
                format!("./Data/{stem}.rvdata2"),
                format!("Data//{stem}.rvdata2"),
                format!("Data/./{stem}.rvdata2"),
                format!("Data/Sub/../{stem}.rvdata2"),
                absolute(&format!("Data/{stem}.rvdata2")),
            ];
            for name in &names {
                let found = data(&files, name, &dir);
                if stem == "both" {
                    assert_eq!(found.unwrap(), ("Loose", b"loose".to_vec()), "{name}");
                } else {
                    not_found(found, name);
                }
            }
        }
    }

    /// Requirement "Data files of RGSS3", scenario "No extension for a loose file".
    #[test]
    fn a_loose_data_file_of_rgss3_is_found_by_its_whole_name() {
        let game = tempfile::tempdir().unwrap();
        put(game.path(), "d/p.bin", b"p");
        let files = GameFiles::open(game.path(), [], Runtime::Rgss3).unwrap();
        let dir = game.path().canonicalize().unwrap();
        not_found(data(&files, "d/p", &dir), "d/p");
        for name in [
            "d/p.bin",
            "d\\p.bin",
            "./d/p.bin",
            "D/P.BIN",
            "d//p.bin",
            "d/../d/p.bin",
            "d/p.bin.",
            "d/p.bin ",
        ] {
            let found = data(&files, name, &dir).unwrap();
            assert_eq!(found, ("Loose", b"p".to_vec()), "{name}");
        }
        not_found(data(&files, " d/p.bin", &dir), " d/p.bin");
    }

    /// Requirement "Data files of RGSS3", scenarios "The current directory" and "The current
    /// directory does not move the archive".
    #[test]
    fn a_loose_data_file_of_rgss3_is_found_from_the_current_directory() {
        let (_game, files, dir) = ace_with_archive(&[("Data\\arconly.rvdata2", b"arconly")]);
        put(&dir, "d/p.bin", b"p");
        put(&dir, "Game.ini", b"ini");
        let inside = dir.join("d");
        assert_eq!(
            data(&files, "p.bin", &inside).unwrap(),
            ("Loose", b"p".to_vec())
        );
        not_found(data(&files, "d/p.bin", &inside), "d/p.bin");
        assert_eq!(
            data(&files, "../Game.ini", &inside).unwrap(),
            ("Loose", b"ini".to_vec())
        );
        std::fs::create_dir(dir.join("Data")).unwrap();
        let data_dir = dir.join("Data");
        assert_eq!(
            data(&files, "Data/arconly.rvdata2", &data_dir).unwrap(),
            ("Entry", b"arconly".to_vec())
        );
        not_found(
            data(&files, "arconly.rvdata2", &data_dir),
            "arconly.rvdata2",
        );
        // From a current directory outside the game directory a relative name finds nothing
        // there, and finds a file of the game only by a path that leads back into it.
        let outer = dir.parent().unwrap();
        let outside = tempfile::tempdir_in(outer).unwrap();
        put(outside.path(), "p.bin", b"outside");
        not_found(data(&files, "p.bin", outside.path()), "p.bin");
        not_found(data(&files, "d/p.bin", outside.path()), "d/p.bin");
        let back = format!("../{}/d/p.bin", dir.file_name().unwrap().to_str().unwrap());
        assert_eq!(
            data(&files, &back, outside.path()).unwrap(),
            ("Loose", b"p".to_vec())
        );
    }

    /// Requirement "Data files of RGSS3", scenarios "A directory" and "Outside the game
    /// directory".
    #[test]
    fn a_directory_and_a_file_outside_are_no_data_files_of_rgss3() {
        let outer = tempfile::tempdir().unwrap();
        let outer = outer.path().canonicalize().unwrap();
        let game = outer.join("game");
        put(&game, "Data/Actors.rvdata2", b"actors");
        put(&outer, "other.rvdata2", b"other");
        let files = GameFiles::open(&game, [], Runtime::Rgss3).unwrap();
        for name in ["Data", "data", "Data/", "Data."] {
            match files.data_rgss3(name, &game) {
                Err(VfsError::IsDirectory { path }) => assert_eq!(path, name),
                other => panic!("{name}: {other:?}"),
            }
        }
        let absolute = outer.join("other.rvdata2").display().to_string();
        for name in ["../other.rvdata2", "Data/../../other.rvdata2", &absolute] {
            not_found(data(&files, name, &game), name);
        }
    }

    /// Requirement "Data files of RGSS3": the other lookups of data files are unchanged.
    #[test]
    fn the_other_lookups_of_data_files_take_no_extension_from_the_archive() {
        let (_game, files, _dir) = ace_with_archive(&[("Data\\arconly.rvdata2", b"arconly")]);
        assert!(files.read(FileKind::Data, "Data/arconly").is_err());
        assert!(files.read(FileKind::Data, "./Data/arconly.rvdata2").is_ok());
    }

    #[test]
    fn other_kinds_with_the_audio_of_rgss3() {
        let game = tempfile::tempdir().unwrap();
        put(game.path(), "Fog.png", b"");
        let files = game_only(game.path());
        assert!(files.resolve(FileKind::AudioRgss3, "Fog").is_err());
        assert!(files.read(FileKind::AudioRgss3, "Fog").is_err());
    }
}
