//! Match video (docs/adr/0004): libmpv draws into a native view laid over
//! the video panel (an NSView on macOS, a child window on Windows, an X11
//! child window on Linux); the Svelte UI keeps the view on the panel's
//! bounds and drives playback through these commands. Elsewhere the
//! commands say video isn't available.

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

#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
pub use player::Player;

#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
pub use other::Player;

#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
mod player {
    use super::surface::Surface;
    use super::{Bounds, PlayerState};
    use libmpv2::Mpv;

    /// The player and the native view it draws into. Only touched from
    /// Tauri's synchronous commands, which run on the main thread, as the
    /// windowing systems require.
    #[derive(Default)]
    pub struct Player {
        mpv: Option<Mpv>,
        surface: Option<Surface>,
        path: Option<String>,
    }

    // SAFETY: the native view (a raw pointer or handle) is only used on the
    // main thread; the mutex around the player keeps calls one at a time.
    #[allow(unsafe_code)]
    unsafe impl Send for Player {}

    impl Player {
        /// Opens `path` in a view over `bounds` of `window`.
        pub fn open(
            &mut self,
            window: &tauri::WebviewWindow,
            path: &str,
            bounds: Bounds,
        ) -> Result<(), String> {
            if self.surface.is_none() {
                self.surface = Some(Surface::new(window, bounds)?);
            }
            if self.mpv.is_none() {
                let wid = self.surface.as_ref().ok_or("no view")?.wid();
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
            match &self.surface {
                Some(s) => s.place(bounds, visible && self.path.is_some()),
                None => Ok(()),
            }
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
            // mpv lets go of the view before it goes.
            self.mpv = None;
            self.path = None;
            match self.surface.take() {
                Some(s) => s.close(),
                None => Ok(()),
            }
        }
    }
}

// AppKit has no safe binding for embedding a native view in a Tauri
// window: the `unsafe` here is the NSWindow pointer Tauri hands over and
// two AppKit calls objc2 marks unsafe.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod surface {
    use super::Bounds;
    use objc2::rc::Retained;
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{NSView, NSWindow};
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    /// An NSView added over the webview. AppKit works in points, as CSS
    /// pixels do, so bounds need no scaling.
    pub struct Surface {
        view: Retained<NSView>,
    }

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

    impl Surface {
        pub fn new(window: &tauri::WebviewWindow, bounds: Bounds) -> Result<Self, String> {
            let mtm = main_thread()?;
            let ns_window = window.ns_window().map_err(|e| e.to_string())?;
            // SAFETY: Tauri hands back the window's NSWindow pointer, valid
            // while the window lives; we're on the main thread.
            let ns_window: &NSWindow = unsafe { &*ns_window.cast::<NSWindow>() };
            let content = ns_window
                .contentView()
                .ok_or("the window has no content view")?;
            let view = NSView::initWithFrame(NSView::alloc(mtm), frame(&content, bounds));
            // Added after the webview, so it's drawn on top of it.
            content.addSubview(&view);
            Ok(Surface { view })
        }

        /// What mpv's `wid` option takes: the NSView pointer.
        pub fn wid(&self) -> i64 {
            Retained::as_ptr(&self.view) as i64
        }

        pub fn place(&self, bounds: Bounds, visible: bool) -> Result<(), String> {
            main_thread()?;
            // SAFETY: main thread; the superview is the window's content view.
            let Some(content) = (unsafe { self.view.superview() }) else {
                return Ok(());
            };
            self.view.setFrame(frame(&content, bounds));
            self.view.setHidden(!visible);
            Ok(())
        }

        pub fn close(self) -> Result<(), String> {
            main_thread()?;
            self.view.removeFromSuperview();
            Ok(())
        }
    }
}

// Win32 has no safe API: the `unsafe` here is the window calls, on the
// child window this module creates and the parent Tauri hands over.
#[cfg(windows)]
#[allow(unsafe_code)]
mod surface {
    use super::Bounds;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, HWND_TOP, SW_HIDE, SW_SHOWNA, SWP_NOACTIVATE, SetWindowPos,
        ShowWindow, WINDOW_EX_STYLE, WS_CHILD, WS_CLIPSIBLINGS,
    };
    use windows::core::w;

    /// A child window of the Tauri window, kept above the webview's. Win32
    /// works in physical pixels: bounds are scaled by the window's factor.
    pub struct Surface {
        hwnd: HWND,
        scale: f64,
    }

    fn px(v: f64, scale: f64) -> i32 {
        (v * scale).round() as i32
    }

    impl Surface {
        pub fn new(window: &tauri::WebviewWindow, bounds: Bounds) -> Result<Self, String> {
            let parent = window.hwnd().map_err(|e| e.to_string())?;
            let scale = window.scale_factor().map_err(|e| e.to_string())?;
            // SAFETY: `parent` is the live Tauri window; a STATIC control
            // needs no window class of our own. Main thread, which owns it.
            let hwnd = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("STATIC"),
                    w!(""),
                    WS_CHILD | WS_CLIPSIBLINGS,
                    px(bounds.x, scale),
                    px(bounds.y, scale),
                    px(bounds.width, scale),
                    px(bounds.height, scale),
                    Some(parent),
                    None,
                    None,
                    None,
                )
            }
            .map_err(|e| format!("can't make the video view: {e}"))?;
            Ok(Surface { hwnd, scale })
        }

        /// What mpv's `wid` option takes: the HWND.
        pub fn wid(&self) -> i64 {
            self.hwnd.0 as i64
        }

        pub fn place(&self, b: Bounds, visible: bool) -> Result<(), String> {
            let s = self.scale;
            // SAFETY: our own child window, alive until `close`. On top of
            // the webview's window every time, which may have come forward.
            unsafe {
                SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOP),
                    px(b.x, s),
                    px(b.y, s),
                    px(b.width, s),
                    px(b.height, s),
                    SWP_NOACTIVATE,
                )
                .map_err(|e| e.to_string())?;
                let _ = ShowWindow(self.hwnd, if visible { SW_SHOWNA } else { SW_HIDE });
            }
            Ok(())
        }

        pub fn close(self) -> Result<(), String> {
            // SAFETY: our own child window, destroyed once.
            unsafe { DestroyWindow(self.hwnd) }.map_err(|e| e.to_string())
        }
    }
}

// Xlib has no safe API: the `unsafe` here is the Xlib calls on our own
// display connection and the child window it creates.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
mod surface {
    use super::Bounds;
    use gtk::glib::Cast;
    use gtk::prelude::WidgetExt;
    use std::os::raw::c_ulong;
    use x11_dl::xlib::{Display, Xlib};

    /// An X11 child window of the Tauri window's own X window, mapped above
    /// the webview's drawing. Needs X11 (XWayland too); Wayland would need
    /// mpv's render API instead. X works in physical pixels: bounds are
    /// scaled by the window's factor.
    pub struct Surface {
        xlib: Xlib,
        display: *mut Display,
        window: c_ulong,
        scale: f64,
    }

    fn px(v: f64, scale: f64) -> i32 {
        (v * scale).round() as i32
    }

    fn size(v: f64, scale: f64) -> u32 {
        px(v, scale).max(1) as u32
    }

    impl Surface {
        pub fn new(window: &tauri::WebviewWindow, b: Bounds) -> Result<Self, String> {
            let gtk = window.gtk_window().map_err(|e| e.to_string())?;
            let gdk = gtk.window().ok_or("the window isn't shown yet")?;
            let parent = gdk.downcast::<gdkx11::X11Window>().map_err(|_| {
                "Video needs X11: start Courtside with GDK_BACKEND=x11 under Wayland.".to_owned()
            })?;
            let parent = parent.xid();
            let scale = window.scale_factor().map_err(|e| e.to_string())?;
            let xlib = Xlib::open().map_err(|e| format!("can't load libX11: {e}"))?;
            // SAFETY: a display connection of our own, closed in `close`;
            // `parent` is the live Tauri window's X window.
            unsafe {
                let display = (xlib.XOpenDisplay)(std::ptr::null());
                if display.is_null() {
                    return Err("can't open the X display".into());
                }
                let black = (xlib.XBlackPixel)(display, (xlib.XDefaultScreen)(display));
                let window = (xlib.XCreateSimpleWindow)(
                    display,
                    parent,
                    px(b.x, scale),
                    px(b.y, scale),
                    size(b.width, scale),
                    size(b.height, scale),
                    0,
                    black,
                    black,
                );
                (xlib.XFlush)(display);
                Ok(Surface {
                    xlib,
                    display,
                    window,
                    scale,
                })
            }
        }

        /// What mpv's `wid` option takes: the X window id.
        pub fn wid(&self) -> i64 {
            self.window as i64
        }

        pub fn place(&self, b: Bounds, visible: bool) -> Result<(), String> {
            let (x, s) = (&self.xlib, self.scale);
            // SAFETY: our own connection and window, alive until `close`.
            unsafe {
                (x.XMoveResizeWindow)(
                    self.display,
                    self.window,
                    px(b.x, s),
                    px(b.y, s),
                    size(b.width, s),
                    size(b.height, s),
                );
                if visible {
                    (x.XMapRaised)(self.display, self.window);
                } else {
                    (x.XUnmapWindow)(self.display, self.window);
                }
                (x.XFlush)(self.display);
            }
            Ok(())
        }

        pub fn close(self) -> Result<(), String> {
            let x = &self.xlib;
            // SAFETY: our own window and connection, each let go of once.
            unsafe {
                (x.XDestroyWindow)(self.display, self.window);
                (x.XCloseDisplay)(self.display);
            }
            Ok(())
        }
    }
}

#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
mod other {
    use super::{Bounds, PlayerState};

    const NOT_YET: &str = "Video playback isn't available on this system.";

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
