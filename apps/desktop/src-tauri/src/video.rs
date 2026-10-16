//! Match video (phase 4 spike): libmpv draws into a native view laid over
//! the video panel; the Svelte UI keeps the view on the panel's bounds and
//! drives playback through these commands. macOS only for now; elsewhere
//! the commands say so.

use serde::Serialize;

/// The panel's place in the window, in CSS pixels from the top left.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// What the transport bar shows.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    /// The file playing, if any.
    pub path: Option<String>,
    /// Seconds into the video.
    pub time: Option<f64>,
    pub duration: Option<f64>,
    pub paused: bool,
    pub speed: f64,
    /// Frames per second of the video stream, for frame stepping and the
    /// timecode.
    pub fps: Option<f64>,
}

#[cfg(target_os = "macos")]
pub use mac::Player;

#[cfg(not(target_os = "macos"))]
pub use other::Player;

// AppKit has no safe binding for embedding a native view in a Tauri
// window: the `unsafe` here is the NSWindow pointer Tauri hands over and
// two AppKit calls objc2 marks unsafe.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod mac {
    use super::{Bounds, PlayerState};
    use libmpv2::Mpv;
    use objc2::rc::Retained;
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{NSView, NSWindow};
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    /// The player and its view. Only touched from Tauri's synchronous
    /// commands, which run on the main thread, as AppKit requires.
    #[derive(Default)]
    pub struct Player {
        mpv: Option<Mpv>,
        view: Option<Retained<NSView>>,
        path: Option<String>,
    }

    // SAFETY: the view is only used on the main thread (every caller checks
    // for a `MainThreadMarker` first); the mutex around the player keeps
    // calls one at a time.
    unsafe impl Send for Player {}

    fn main_thread() -> Result<MainThreadMarker, String> {
        MainThreadMarker::new().ok_or_else(|| "video must be driven from the main thread".into())
    }

    /// `bounds` (top-left origin) in the content view's coordinates
    /// (bottom-left origin unless the view is flipped).
    fn frame(content: &NSView, b: Bounds) -> NSRect {
        let height = content.frame().size.height;
        let y = if content.isFlipped() {
            b.y
        } else {
            height - b.y - b.height
        };
        NSRect::new(NSPoint::new(b.x, y), NSSize::new(b.width, b.height))
    }

    impl Player {
        /// Opens `path` in a view over `bounds` of `window`.
        pub fn open(
            &mut self,
            window: &tauri::WebviewWindow,
            path: &str,
            bounds: Bounds,
        ) -> Result<(), String> {
            let mtm = main_thread()?;
            if self.view.is_none() {
                let ns_window = window.ns_window().map_err(|e| e.to_string())?;
                // SAFETY: Tauri hands back the window's NSWindow pointer,
                // valid while the window lives; we're on the main thread.
                let ns_window: &NSWindow = unsafe { &*ns_window.cast::<NSWindow>() };
                let content = ns_window
                    .contentView()
                    .ok_or("the window has no content view")?;
                let view = NSView::initWithFrame(NSView::alloc(mtm), frame(&content, bounds));
                // Added after the webview, so it's drawn on top of it.
                content.addSubview(&view);
                self.view = Some(view);
            }
            if self.mpv.is_none() {
                let view = self.view.as_ref().ok_or("no view")?;
                let wid = Retained::as_ptr(view) as i64;
                let mpv = Mpv::with_initializer(|init| {
                    init.set_option("wid", wid)?;
                    init.set_option("vo", "gpu-next")?;
                    init.set_option("hwdec", "auto-safe")?;
                    init.set_option("keep-open", "yes")?;
                    init.set_option("idle", "yes")?;
                    init.set_option("force-window", "yes")?;
                    // Exact seeks: the PRD asks for frame-accurate review.
                    init.set_option("hr-seek", "yes")?;
                    init.set_option("osc", "no")?;
                    init.set_option("load-scripts", "no")?;
                    init.set_option("ytdl", "no")?;
                    init.set_option("input-default-bindings", "no")?;
                    // Development: mpv's own log, to see what it chose.
                    if let Ok(log) = std::env::var("COURTSIDE_MPV_LOG") {
                        init.set_option("log-file", log.as_str())?;
                    }
                    Ok(())
                })
                .map_err(|e| format!("mpv: {e}"))?;
                self.mpv = Some(mpv);
            }
            let mpv = self.mpv.as_ref().ok_or("no player")?;
            mpv.command("loadfile", &[path])
                .map_err(|e| format!("mpv: {e}"))?;
            mpv.set_property("pause", true)
                .map_err(|e| format!("mpv: {e}"))?;
            self.path = Some(path.to_owned());
            self.place(bounds, true)
        }

        /// Keeps the view on the panel; `visible` false hides it (a dialog
        /// or another screen over the panel).
        pub fn place(&mut self, bounds: Bounds, visible: bool) -> Result<(), String> {
            main_thread()?;
            let Some(view) = &self.view else {
                return Ok(());
            };
            // SAFETY: main thread; the superview is the window's content view.
            let Some(content) = (unsafe { view.superview() }) else {
                return Ok(());
            };
            view.setFrame(frame(&content, bounds));
            view.setHidden(!visible || self.path.is_none());
            Ok(())
        }

        /// An mpv command, as in input.conf (`seek -5 relative+exact`,
        /// `frame-step`, `cycle pause`).
        pub fn command(&mut self, args: &[String]) -> Result<(), String> {
            let mpv = self.mpv.as_ref().ok_or("no video is open")?;
            let (name, rest) = args.split_first().ok_or("an empty command")?;
            let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
            mpv.command(name, &rest)
                .map_err(|e| format!("mpv {name}: {e}"))
        }

        pub fn set_speed(&mut self, speed: f64) -> Result<(), String> {
            let mpv = self.mpv.as_ref().ok_or("no video is open")?;
            mpv.set_property("speed", speed)
                .map_err(|e| format!("mpv: {e}"))
        }

        pub fn set_paused(&mut self, paused: bool) -> Result<(), String> {
            let mpv = self.mpv.as_ref().ok_or("no video is open")?;
            mpv.set_property("pause", paused)
                .map_err(|e| format!("mpv: {e}"))
        }

        pub fn state(&self) -> PlayerState {
            let Some(mpv) = &self.mpv else {
                return PlayerState::default();
            };
            let num = |name: &str| mpv.get_property::<f64>(name).ok();
            PlayerState {
                path: self.path.clone(),
                time: num("time-pos"),
                duration: num("duration"),
                paused: mpv.get_property::<bool>("pause").unwrap_or(true),
                speed: num("speed").unwrap_or(1.0),
                fps: num("container-fps"),
            }
        }

        /// Stops playback and takes the view off the window.
        pub fn close(&mut self) -> Result<(), String> {
            main_thread()?;
            self.mpv = None;
            if let Some(view) = self.view.take() {
                view.removeFromSuperview();
            }
            self.path = None;
            Ok(())
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod other {
    use super::{Bounds, PlayerState};

    const NOT_YET: &str = "Video playback works on macOS only so far.";

    #[derive(Default)]
    pub struct Player;

    impl Player {
        pub fn open(&mut self, _: &tauri::WebviewWindow, _: &str, _: Bounds) -> Result<(), String> {
            Err(NOT_YET.into())
        }
        pub fn place(&mut self, _: Bounds, _: bool) -> Result<(), String> {
            Ok(())
        }
        pub fn command(&mut self, _: &[String]) -> Result<(), String> {
            Err(NOT_YET.into())
        }
        pub fn set_speed(&mut self, _: f64) -> Result<(), String> {
            Err(NOT_YET.into())
        }
        pub fn set_paused(&mut self, _: bool) -> Result<(), String> {
            Err(NOT_YET.into())
        }
        pub fn state(&self) -> PlayerState {
            PlayerState::default()
        }
        pub fn close(&mut self) -> Result<(), String> {
            Ok(())
        }
    }
}
