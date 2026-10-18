//! The calls between the native code and the Java class of the activity,
//! `io.github.xuyanwen2012.vkrgss.VkrgssActivity` (`java/` of the crate), through JNI.
//!
//! The native code calls the activity's methods `gameDir`, `openPicker`, `copyToCache`,
//! `startGame` and `finish`, from any thread; the activity calls the native method
//! `nativeImport` with a zip to import, which goes to the library's queue
//! ([`crate::library_app::request_import`]).

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use jni::errors::Error;
use jni::objects::{JClass, JObject, JString, JValue};
use jni::refs::Global;
use jni::{EnvUnowned, JavaVM, jni_sig, jni_str};
use winit::platform::android::activity::AndroidApp;

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

    /// Opens the system's picker for a zip of a game or, with `rtp`, of the RTP.
    pub(crate) fn open_picker(&self, rtp: bool) -> Result<()> {
        self.vm
            .attach_current_thread(|env| -> Result<_, Error> {
                env.call_method(
                    self.object.as_ref(),
                    jni_str!("openPicker"),
                    jni_sig!((rtp: jboolean) -> void),
                    &[JValue::Bool(rtp)],
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

/// `VkrgssActivity.nativeImport(String uri, boolean rtp)`.
#[unsafe(no_mangle)]
extern "system" fn Java_io_github_xuyanwen2012_vkrgss_VkrgssActivity_nativeImport<'caller>(
    mut env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    uri: JString<'caller>,
    rtp: jni::sys::jboolean,
) {
    let uri = env
        .with_env(|env| -> Result<_, Error> { uri.try_to_string(env) })
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
    if !uri.is_empty() {
        crate::library_app::request_import(uri, rtp);
    }
}
