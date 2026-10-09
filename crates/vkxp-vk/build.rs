//! Compiles the GLSL under `shaders/` to SPIR-V in `OUT_DIR` and validates the result.
//!
//! Every file whose extension names a shader stage is compiled with `glslc`; `*.glsl` files are
//! includes. The output of `<name>.<stage>` is `<name>.<stage>.spv`, which the library embeds
//! with `include_bytes!`. Compiled binaries never enter the source tree.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

/// Target environment of the compiler and of the validator.
const GLSLC_TARGET: &str = "--target-env=vulkan1.3";
const SPIRV_VAL_TARGET: &str = "vulkan1.3";

/// Extensions that name a shader stage, as `glslc` infers it.
const STAGE_EXTENSIONS: [&str; 3] = ["comp", "vert", "frag"];

fn main() {
    let shader_dir = Path::new("shaders");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    println!("cargo:rerun-if-changed=build.rs");
    // Covers files added to or removed from the directory.
    println!("cargo:rerun-if-changed={}", shader_dir.display());

    let mut sources = Vec::new();
    let entries = fs::read_dir(shader_dir)
        .unwrap_or_else(|error| fail(&format!("cannot read {}: {error}", shader_dir.display())));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|error| fail(&format!("cannot read {}: {error}", shader_dir.display())))
            .path();
        // Sources and includes alike: an include can change any shader.
        println!("cargo:rerun-if-changed={}", path.display());
        let extension = path.extension().and_then(OsStr::to_str).unwrap_or_default();
        if STAGE_EXTENSIONS.contains(&extension) {
            sources.push(path);
        }
    }
    sources.sort();

    for source in &sources {
        let name = source.file_name().expect("a file has a name");
        let mut output_name = name.to_os_string();
        output_name.push(".spv");
        let output = out_dir.join(output_name);

        run(
            Command::new("glslc")
                .arg(GLSLC_TARGET)
                .arg("-Werror")
                .arg("-I")
                .arg(shader_dir)
                .arg("-o")
                .arg(&output)
                .arg(source),
            "glslc",
            "`glslc` or `shaderc`",
            source,
        );
        run(
            Command::new("spirv-val")
                .arg("--target-env")
                .arg(SPIRV_VAL_TARGET)
                .arg(&output),
            "spirv-val",
            "`spirv-tools`",
            source,
        );
    }
}

/// Runs one tool on one shader and stops the build when it is missing or reports an error.
/// The tool's own messages reach the user through the inherited stderr.
fn run(command: &mut Command, tool: &str, package: &str, source: &Path) {
    match command.status() {
        Ok(status) if status.success() => {}
        Ok(status) => fail(&format!(
            "`{tool}` failed on {} ({status}); its messages are above",
            source.display()
        )),
        Err(error) if error.kind() == ErrorKind::NotFound => fail(&format!(
            "`{tool}` was not found on PATH. vkxp-vk compiles its shaders at build time and \
             needs `glslc` and `spirv-val`; install the package that provides `{tool}` \
             ({package})."
        )),
        Err(error) => fail(&format!("cannot run `{tool}`: {error}")),
    }
}

fn fail(message: &str) -> ! {
    eprintln!("error: {message}");
    exit(1);
}
