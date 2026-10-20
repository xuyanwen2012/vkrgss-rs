//! The calls between the native code and the Java class of the activity,
//! `io.github.xuyanwen2012.vkrgss.VkrgssActivity` (`java/` of the crate), through JNI.
//!
//! The native code calls the activity's methods `gameDir`, `validation`, `openPicker`,
//! `copyToCache`, `startGame`, `insets` and `finish`, from any thread; the activity calls the native method
//! `nativeImport` with a file to import, which goes to the library's queue
//! ([`crate::library_app::request_import`]).

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use jni::errors::Error;
use jni::objects::{JClass, JIntArray, JObject, JString, JValue};
use jni::refs::Global;
use jni::{EnvUnowned, JavaVM, jni_sig, jni_str};
use winit::platform::android::activity::AndroidApp;

use crate::canvas::Insets;

/// What the picker is opened for: the numbers of the Java class's `PICK_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pick {
    /// The zip of a game.
    Game = 1,
    /// The zip of a run-time package.
    Rtp = 2,
    /// A built-in scripts file.
    BuiltinScripts = 3,
}

impl Pick {
    /// The kind of the number `what` of the Java class, or `None` for another number.
    fn from_java(what: i32) -> Option<Pick> {
        [Pick::Game, Pick::Rtp, Pick::BuiltinScripts]
            .into_iter()
            .find(|pick| *pick as i32 == what)
    }
}

/// The activity's Java object, for calls from any thread.
#[derive(Clone)]
pub(crate) struct Activity {
    vm: JavaVM,
    object: std::sync::Arc<Global<JObject<'static>>>,
}

/// A JNI error as a message, with the message of a Java exception.
fn message(error: Error) -> anyhow::Error {
    match error {
        Error::CaughtJavaException { name, msg, .. } => anyhow!("{name}: {msg}"),
        error => anyhow!("JNI: {error}"),
    }
}

impl Activity {
    pub(crate) fn new(app: &AndroidApp) -> Result<Activity> {
        // SAFETY: the pointer is that of the process's Java VM, which lives as long as the
        // process.
        let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
        let raw = app.activity_as_ptr() as jni::sys::jobject;
        let object = vm
            .attach_current_thread(|env| -> Result<_, Error> {
                // SAFETY: the reference is a global one of the activity, valid while `app`
                // is, which it is here; a global reference of our own is made from it.
                let activity = unsafe { JObject::from_raw(env, raw) };
                env.new_global_ref(activity)
            })
            .map_err(message)?;
        Ok(Activity {
            vm,
            object: std::sync::Arc::new(object),
        })
    }

    /// The directory of the game this activity runs: `None` for the library, an empty path
    /// for the game of the files directory.
    pub(crate) fn game_dir(&self) -> Result<Option<PathBuf>> {
        self.vm
            .attach_current_thread(|env| -> Result<_, Error> {
                let dir = env
                    .call_method(
                        self.object.as_ref(),
                        jni_str!("gameDir"),
                        jni_sig!(() -> JString),
                        &[],
                    )?
                    .l()?;
                if dir.is_null() {
                    return Ok(None);
                }
                let dir = env.cast_local::<JString>(dir)?;
                Ok(Some(PathBuf::from(dir.try_to_string(env)?)))
            })
            .map_err(message)
    }

    /// Whether the game runs under the validation layer, as the intent asks.
    pub(crate) fn validation(&self) -> Result<bool> {
        self.vm
            .attach_current_thread(|env| -> Result<_, Error> {
                env.call_method(
                    self.object.as_ref(),
                    jni_str!("validation"),
                    jni_sig!(() -> jboolean),
                    &[],
                )?
                .z()
            })
            .map_err(message)
    }

    /// Opens the system's picker for a file of the kind `pick`.
    pub(crate) fn open_picker(&self, pick: Pick) -> Result<()> {
        self.vm
            .attach_current_thread(|env| -> Result<_, Error> {
                env.call_method(
                    self.object.as_ref(),
                    jni_str!("openPicker"),
                    jni_sig!((what: jint) -> void),
                    &[JValue::Int(pick as i32)],
                )?;
                Ok(())
            })
            .map_err(message)
    }

    /// Copies what `uri` names into a file of the app's cache directory and gives its path.
    pub(crate) fn copy_to_cache(&self, uri: &str) -> Result<PathBuf> {
        self.vm
            .attach_current_thread(|env| -> Result<_, Error> {
                let uri = env.new_string(uri)?;
                let path = env
                    .call_method(
                        self.object.as_ref(),
                        jni_str!("copyToCache"),
                        jni_sig!((uri: JString) -> JString),
                        &[JValue::Object(&uri)],
                    )?
                    .l()?;
                let path = env.cast_local::<JString>(path)?;
                Ok(PathBuf::from(path.try_to_string(env)?))
            })
            .map_err(message)
    }

    /// Starts the game of `dir` in the activity of games.
    pub(crate) fn start_game(&self, dir: &Path) -> Result<()> {
        self.vm
            .attach_current_thread(|env| -> Result<_, Error> {
                let dir = env.new_string(dir.to_string_lossy())?;
                env.call_method(
                    self.object.as_ref(),
                    jni_str!("startGame"),
                    jni_sig!((dir: JString) -> void),
                    &[JValue::Object(&dir)],
                )?;
                Ok(())
            })
            .map_err(message)
    }

    /// What the system's bars and the display's cutout cover at the window's edges, in pixels.
    pub(crate) fn insets(&self) -> Result<Insets> {
        self.vm
            .attach_current_thread(|env| -> Result<_, Error> {
                let array = env
                    .call_method(
                        self.object.as_ref(),
                        jni_str!("insets"),
                        jni_sig!(() -> jint[]),
                        &[],
                    )?
                    .l()?;
                let array = env.cast_local::<JIntArray>(array)?;
                let mut edges = [0; 4];
                array.get_region(env, 0, &mut edges)?;
                let [left, top, right, bottom] = edges.map(|edge| edge.max(0) as f32);
                Ok(Insets {
                    left,
                    top,
                    right,
                    bottom,
                })
            })
            .map_err(message)
    }

    /// Finishes the activity.
    pub(crate) fn finish(&self) -> Result<()> {
        self.vm
            .attach_current_thread(|env| -> Result<_, Error> {
                env.call_method(
                    self.object.as_ref(),
                    jni_str!("finish"),
                    jni_sig!(() -> void),
                    &[],
                )?;
                Ok(())
            })
            .map_err(message)
    }
}

/// `VkrgssActivity.nativeImport(String uri, int what)`.
#[unsafe(no_mangle)]
extern "system" fn Java_io_github_xuyanwen2012_vkrgss_VkrgssActivity_nativeImport<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    uri: JString<'caller>,
    what: jni::sys::jint,
) {
    let uri = env
        .with_env(|env| -> Result<_, Error> { uri.try_to_string(env) })
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
    match Pick::from_java(what) {
        Some(pick) if !uri.is_empty() => crate::library_app::request_import(uri, pick),
        _ => tracing::error!("library: asked to import {uri:?} as the unknown kind {what}"),
    }
}
