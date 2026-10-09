//! Resolution of script-style paths over the game directory and the run-time packages.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use crate::dir::Listing;
use crate::error::VfsError;

/// What a script asks for. The kind selects the extensions that are tried and whether the
/// run-time packages are searched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    /// A picture: searched in the game directory and the run-time packages.
    Image,
    /// Music or a sound: searched in the game directory and the run-time packages.
    Audio,
    /// A data file such as `Data/Scripts.rxdata`: searched in the game directory only, under
    /// the name as given.
    Data,
}

impl FileKind {
    /// Extensions appended, in this order, when the path as given names no file.
    // The original runtime's own order has not been measured yet; it only matters when one name
    // exists with two extensions in the same directory.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            FileKind::Image => &["png", "jpg", "jpeg", "bmp"],
            FileKind::Audio => &["mid", "ogg", "wav", "mp3", "wma"],
            FileKind::Data => &[],
        }
    }

    fn searches_rtps(self) -> bool {
        !matches!(self, FileKind::Data)
    }
}

/// Splits a script-style path at `/` and `\` and removes `.` and `..` components. Returns
/// `None` for a path that names nothing or that would leave the directory it is searched in.
fn components(script_path: &str) -> Option<Vec<&str>> {
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

/// The files of one game: its own directory, then its run-time package directories.
///
/// Directory listings are cached, since games do not add assets while running; see
/// [`GameFiles::refresh`].
#[derive(Debug)]
pub struct GameFiles {
    /// Search roots in order: the game directory first.
    roots: Vec<PathBuf>,
    cache: Mutex<HashMap<PathBuf, Arc<Listing>>>,
}

impl GameFiles {
    /// Creates a resolver over `game_dir` and then `rtp_dirs`, in order.
    pub fn new(game_dir: impl Into<PathBuf>, rtp_dirs: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut roots = vec![game_dir.into()];
        roots.extend(rtp_dirs);
        Self {
            roots,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// The game directory.
    pub fn game_dir(&self) -> &Path {
        &self.roots[0]
    }

    /// The run-time package directories, in search order.
    pub fn rtp_dirs(&self) -> &[PathBuf] {
        &self.roots[1..]
    }

    /// Finds the file a script-style path names.
    ///
    /// Components are separated by `/` or `\` and match directory entries without regard to
    /// letter case, an exact match being preferred. In each root the path as given is tried
    /// first, then with each extension of `kind` appended. The error carries `script_path`
    /// exactly as given.
    pub fn resolve(&self, kind: FileKind, script_path: &str) -> Result<PathBuf, VfsError> {
        let roots = if kind.searches_rtps() {
            &self.roots[..]
        } else {
            &self.roots[..1]
        };
        components(script_path)
            .and_then(|components| {
                roots
                    .iter()
                    .find_map(|root| self.find(root, &components, kind.extensions()))
            })
            .ok_or_else(|| VfsError::NotFound {
                path: script_path.to_owned(),
            })
    }

    /// Resolves a script-style path as [`GameFiles::resolve`] does and returns the complete
    /// contents of the file.
    pub fn read(&self, kind: FileKind, script_path: &str) -> Result<Vec<u8>, VfsError> {
        let path = self.resolve(kind, script_path)?;
        std::fs::read(&path).map_err(|source| VfsError::Io { path, source })
    }

    /// Drops the cached directory listings, so files added or removed since they were listed
    /// are seen.
    pub fn refresh(&self) {
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    /// Finds `components` below `dir`. When several entries match a directory component, each
    /// is searched in turn.
    fn find(&self, dir: &Path, components: &[&str], extensions: &[&str]) -> Option<PathBuf> {
        let (name, rest) = components.split_first()?;
        let listing = self.listing(dir);
        if rest.is_empty() {
            let as_given = std::iter::once((*name).to_owned());
            let extended = extensions.iter().map(|ext| format!("{name}.{ext}"));
            as_given.chain(extended).find_map(|candidate| {
                listing
                    .matches(&candidate)
                    .into_iter()
                    .map(|real| dir.join(real))
                    .find(|path| path.is_file())
            })
        } else {
            listing
                .matches(name)
                .into_iter()
                .find_map(|real| self.find(&dir.join(real), rest, extensions))
        }
    }

    /// The listing of `dir`, read once. Something that cannot be listed has no entries.
    fn listing(&self, dir: &Path) -> Arc<Listing> {
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(listing) = cache.get(dir) {
            return Arc::clone(listing);
        }
        let listing = Arc::new(Listing::read(dir).unwrap_or_default());
        cache.insert(dir.to_owned(), Arc::clone(&listing));
        listing
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
        let lower = put(game.path(), "a.png", b"");
        let upper = put(game.path(), "A.png", b"");
        let files = game_only(game.path());
        assert_eq!(files.resolve(FileKind::Image, "A.png").unwrap(), upper);
        assert_eq!(files.resolve(FileKind::Image, "a.png").unwrap(), lower);
        assert_eq!(files.resolve(FileKind::Image, "A").unwrap(), upper);
    }

    #[test]
    fn directories_that_differ_in_case_are_all_searched() {
        let game = tempfile::tempdir().unwrap();
        let first = put(game.path(), "Graphics/one.png", b"");
        let second = put(game.path(), "graphics/two.png", b"");
        let files = game_only(game.path());
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

    #[test]
    fn listings_are_cached_until_refresh() {
        let game = tempfile::tempdir().unwrap();
        put(game.path(), "Graphics/old.png", b"");
        let files = game_only(game.path());
        assert!(files.resolve(FileKind::Image, "Graphics/new").is_err());
        let new = put(game.path(), "Graphics/new.png", b"");
        assert!(files.resolve(FileKind::Image, "Graphics/new").is_err());
        files.refresh();
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
}
