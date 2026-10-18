//! Loading the Vulkan loader.
//!
//! Elsewhere the system's loader is loaded by its name. On macOS an executable that lies in
//! an app bundle, `<x>.app/Contents/MacOS/<name>`, loads the loader of the bundle,
//! `Contents/Frameworks/libvulkan.1.dylib`, and points it at the bundle's MoltenVK, whose
//! manifest is `Contents/Resources/vulkan/icd.d/MoltenVK_icd.json`, through `VK_DRIVER_FILES`
//! unless `VK_DRIVER_FILES` or `VK_ICD_FILENAMES` is set. Outside a bundle the loader is looked
//! up by its name, and Homebrew's, `/opt/homebrew/lib/libvulkan.1.dylib`, is the last
//! fallback, for development.

/// Loads the Vulkan loader.
pub(crate) fn load() -> Result<ash::Entry, ash::LoadingError> {
    #[cfg(target_os = "macos")]
    return macos::load();
    #[cfg(not(target_os = "macos"))]
    // SAFETY: loading the system's Vulkan library runs its initialisers; that is the
    // documented way to obtain the entry points and has no other precondition.
    unsafe {
        ash::Entry::load()
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};
    use std::sync::Once;

    /// The loader of an app bundle, below the bundle's directory.
    const LOADER: &str = "Contents/Frameworks/libvulkan.1.dylib";

    /// The manifest of the bundle's MoltenVK, below the bundle's directory.
    const ICD: &str = "Contents/Resources/vulkan/icd.d/MoltenVK_icd.json";

    /// Homebrew's loader, the last place a program outside a bundle looks.
    const DEVELOPMENT_LOADER: &str = "/opt/homebrew/lib/libvulkan.1.dylib";

    /// The variables that name the loader's drivers. The first is the one set here.
    const DRIVER_VARIABLES: [&str; 2] = ["VK_DRIVER_FILES", "VK_ICD_FILENAMES"];

    pub(super) fn load() -> Result<ash::Entry, ash::LoadingError> {
        let bundle = std::env::current_exe()
            .and_then(std::fs::canonicalize)
            .ok()
            .and_then(|exe| bundle_of(&exe).map(Path::to_path_buf));
        let Some(bundle) = bundle else {
            // SAFETY: loading a Vulkan library runs its initialisers; that is the documented
            // way to obtain the entry points and has no other precondition.
            return unsafe { ash::Entry::load() }.or_else(|error| {
                // SAFETY: as above, for Homebrew's loader.
                unsafe { ash::Entry::load_from(DEVELOPMENT_LOADER) }.map_err(|_| error)
            });
        };
        static DRIVERS: Once = Once::new();
        DRIVERS.call_once(|| {
            let set = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());
            if let Some(icd) = driver_files(&bundle, set) {
                tracing::info!("the Vulkan loader takes the driver {}", icd.display());
                // SAFETY: the variable is set once, before the loader of the bundle is first
                // loaded, so that the loader reads it. The `setenv` and `getenv` of macOS's
                // libc hold the lock of the environment, and std holds its own around its
                // calls, so a reader on another thread sees the environment before or after
                // the change.
                unsafe { std::env::set_var(DRIVER_VARIABLES[0], &icd) };
            }
        });
        // SAFETY: loading the bundle's Vulkan library runs its initialisers; that is the
        // documented way to obtain the entry points and has no other precondition.
        unsafe { ash::Entry::load_from(bundle.join(LOADER)) }
    }

    /// The directory of the app bundle when `exe` is `<x>.app/Contents/MacOS/<name>`.
    pub(super) fn bundle_of(exe: &Path) -> Option<&Path> {
        let macos = exe.parent()?;
        let contents = macos.parent()?;
        let bundle = contents.parent()?;
        let laid_out = macos.file_name() == Some(OsStr::new("MacOS"))
            && contents.file_name() == Some(OsStr::new("Contents"))
            && bundle.extension() == Some(OsStr::new("app"));
        laid_out.then_some(bundle)
    }

    /// The driver the loader of `bundle` is to take: the bundle's MoltenVK, unless one of the
    /// variables that name drivers is `set`.
    pub(super) fn driver_files(bundle: &Path, set: impl Fn(&str) -> bool) -> Option<PathBuf> {
        (!DRIVER_VARIABLES.iter().any(|name| set(name))).then(|| bundle.join(ICD))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn an_executable_in_contents_macos_of_an_app_lies_in_that_bundle() {
            let bundle = |path: &str| bundle_of(Path::new(path)).map(Path::to_path_buf);
            assert_eq!(
                bundle("/Applications/vkrgss.app/Contents/MacOS/vkrgss"),
                Some(PathBuf::from("/Applications/vkrgss.app"))
            );
            assert_eq!(
                bundle("/tmp/a b.app/Contents/MacOS/vkrgss-guest-rgss3"),
                Some(PathBuf::from("/tmp/a b.app"))
            );
            for outside in [
                "/Users/me/vkrgss/target/release/vkrgss",
                "/Applications/vkrgss.app/Contents/Resources/vkrgss",
                "/Applications/vkrgss.app/Other/MacOS/vkrgss",
                "/Applications/vkrgss/Contents/MacOS/vkrgss",
                "Contents/MacOS/vkrgss",
                "vkrgss",
            ] {
                assert_eq!(bundle(outside), None, "{outside}");
            }
        }

        #[test]
        fn the_bundle_s_moltenvk_is_taken_unless_a_driver_variable_is_set() {
            let bundle = Path::new("/Applications/vkrgss.app");
            assert_eq!(
                driver_files(bundle, |_| false),
                Some(PathBuf::from(
                    "/Applications/vkrgss.app/Contents/Resources/vulkan/icd.d/MoltenVK_icd.json"
                ))
            );
            for variable in DRIVER_VARIABLES {
                assert_eq!(
                    driver_files(bundle, |name| name == variable),
                    None,
                    "{variable}"
                );
            }
        }
    }
}
