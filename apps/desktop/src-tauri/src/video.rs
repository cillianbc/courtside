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
                let surface = self.surface.as_ref().ok_or("no view")?;
                let wid = surface.wid();
                let mpv = Mpv::with_initializer(|init| {
                    if let Some(wid) = wid {
                        init.set_option("wid", wid)?;
                        init.set_option("vo", "gpu-next")?;
                        init.set_option("force-window", "yes")?;
                    } else {
                        // Drawn through the render API (macOS, see `surface`);
                        // the layer draws each frame when it's due.
                        init.set_option("vo", "libmpv")?;
                        init.set_option("video-timing-offset", "0")?;
                    }
                    init.set_option("hwdec", "auto-safe")?;
                    init.set_option("keep-open", "yes")?;
                    init.set_option("idle", "yes")?;
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
                surface.attach(&mpv)?;
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
            if let Some(s) = &self.surface {
                s.detach();
            }
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
// window, and mpv's render API and OpenGL are C: the `unsafe` here is the
// NSWindow pointer Tauri hands over, the CGL and GL calls on the context
// Core Animation hands the layer, and mpv's render context.
//
// mpv ignores `wid` on macOS since 0.37 (its Vulkan output opens a window
// of its own), so mpv renders through its render API (`vo=libmpv`) into
// a CAOpenGLLayer of ours, as IINA does.
#[cfg(target_os = "macos")]
#[allow(unsafe_code, deprecated)]
mod surface {
    use super::Bounds;
    use libmpv2::Mpv;
    use libmpv2_sys as mpv;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, NSObject, NSObjectProtocol};
    use objc2::{
        AllocAnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    };
    use objc2_app_kit::{NSView, NSWindow};
    use objc2_core_foundation::CFTimeInterval;
    use objc2_core_video::CVTimeStamp;
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use objc2_open_gl::{
        CGLChoosePixelFormat, CGLContextObj, CGLCreateContext, CGLLockContext, CGLOpenGLProfile,
        CGLPixelFormatAttribute, CGLPixelFormatObj, CGLReleaseContext, CGLReleasePixelFormat,
        CGLRetainContext, CGLRetainPixelFormat, CGLSetCurrentContext, CGLUnlockContext,
    };
    use objc2_quartz_core::{CALayer, CAOpenGLLayer};
    use std::ffi::{c_char, c_void};
    use std::ptr::{NonNull, null_mut};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    unsafe extern "C" {
        fn glGetIntegerv(pname: u32, data: *mut i32);
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }
    const GL_VIEWPORT: u32 = 0x0BA2;
    const GL_FRAMEBUFFER_BINDING: u32 = 0x8CA6;
    /// `RTLD_DEFAULT` on macOS: search every image loaded.
    const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;

    /// What the layer (drawn by Core Animation) shares with the player:
    /// our GL context, which the layer is handed back to draw in, and
    /// mpv's render context made in it.
    struct Render {
        pf: CGLPixelFormatObj,
        cgl: CGLContextObj,
        /// mpv has a new frame, or the layer changed size.
        dirty: AtomicBool,
        /// mpv's render context, while a player is attached.
        mpv: Mutex<Option<NonNull<mpv::mpv_render_context>>>,
    }

    // SAFETY: the CGL objects are thread-safe behind CGLLockContext; the
    // render context is only used with the mutex held.
    unsafe impl Send for Render {}
    unsafe impl Sync for Render {}

    /// mpv's update callback: no mpv calls allowed, so only a flag the
    /// layer reads on its next display refresh.
    unsafe extern "C" fn on_update(ctx: *mut c_void) {
        // SAFETY: `ctx` is the `Render` the callback was set with; the
        // render context (and so the callback) is freed before it goes.
        let render = unsafe { &*ctx.cast::<Render>() };
        render.dirty.store(true, Ordering::Release);
    }

    unsafe extern "C" fn proc_address(_: *mut c_void, name: *const c_char) -> *mut c_void {
        // SAFETY: the OpenGL framework is linked; `name` is mpv's C string.
        unsafe { dlsym(RTLD_DEFAULT, name) }
    }

    /// Runs `f` with our GL context locked and current.
    fn with_gl<T>(cgl: CGLContextObj, f: impl FnOnce() -> T) -> T {
        // SAFETY: our own live context; lock and current are undone after.
        unsafe {
            CGLLockContext(cgl);
            CGLSetCurrentContext(cgl);
        }
        let out = f();
        // SAFETY: as above.
        unsafe {
            CGLSetCurrentContext(null_mut());
            CGLUnlockContext(cgl);
        }
        out
    }

    impl Render {
        /// OpenGL 3.2 core (mpv needs more than the layer's default 2.1).
        fn new() -> Result<Self, String> {
            let attrs = [
                CGLPixelFormatAttribute(99), // kCGLPFAOpenGLProfile
                CGLPixelFormatAttribute(CGLOpenGLProfile::CGLOGLPVersion_3_2_Core.0),
                CGLPixelFormatAttribute::CGLPFADoubleBuffer,
                CGLPixelFormatAttribute(73), // kCGLPFAAccelerated
                CGLPixelFormatAttribute::CGLPFAColorSize,
                CGLPixelFormatAttribute(24),
                CGLPixelFormatAttribute::CGLPFAAlphaSize,
                CGLPixelFormatAttribute(8),
                CGLPixelFormatAttribute(0),
            ];
            let (mut pf, mut n, mut cgl) = (null_mut(), 0, null_mut());
            // SAFETY: a zero-ended attribute list and out pointers.
            unsafe {
                CGLChoosePixelFormat(
                    NonNull::from(&attrs[0]),
                    NonNull::from(&mut pf),
                    NonNull::from(&mut n),
                );
            }
            if pf.is_null() {
                return Err("no OpenGL 3.2 pixel format for the video".into());
            }
            // SAFETY: a pixel format just chosen; no share context.
            unsafe { CGLCreateContext(pf, null_mut(), NonNull::from(&mut cgl)) };
            if cgl.is_null() {
                // SAFETY: ours, unused.
                unsafe { CGLReleasePixelFormat(pf) };
                return Err("no OpenGL context for the video".into());
            }
            Ok(Render {
                pf,
                cgl,
                dirty: AtomicBool::new(true),
                mpv: Mutex::new(None),
            })
        }

        /// Makes mpv's render context in our GL context.
        fn attach(&self, handle: *mut mpv::mpv_handle) -> Result<(), String> {
            let mut slot = self.mpv.lock().map_err(|_| "video render lock poisoned")?;
            if slot.is_some() {
                return Ok(());
            }
            let mut init = mpv::mpv_opengl_init_params {
                get_proc_address: Some(proc_address),
                get_proc_address_ctx: null_mut(),
            };
            let mut params = [
                mpv::mpv_render_param {
                    type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
                    data: c"opengl".as_ptr().cast_mut().cast(),
                },
                mpv::mpv_render_param {
                    type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
                    data: (&raw mut init).cast(),
                },
                mpv::mpv_render_param {
                    type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
                    data: null_mut(),
                },
            ];
            let mut render = null_mut();
            // SAFETY: a live mpv handle, params ending in INVALID, our GL
            // context current.
            let err = with_gl(self.cgl, || unsafe {
                mpv::mpv_render_context_create(&raw mut render, handle, params.as_mut_ptr())
            });
            let render = NonNull::new(render)
                .filter(|_| err >= 0)
                .ok_or_else(|| format!("mpv can't draw in the video panel ({err})"))?;
            // SAFETY: `self` outlives the render context (see `on_update`).
            unsafe {
                mpv::mpv_render_context_set_update_callback(
                    render.as_ptr(),
                    Some(on_update),
                    std::ptr::from_ref(self).cast_mut().cast(),
                );
            }
            *slot = Some(render);
            self.dirty.store(true, Ordering::Release);
            Ok(())
        }

        /// Draws mpv's current frame into the framebuffer Core Animation
        /// bound in our context (locked and current during its draw).
        fn draw(&self) {
            self.dirty.store(false, Ordering::Release);
            let Ok(slot) = self.mpv.lock() else {
                return;
            };
            let Some(render) = *slot else {
                return;
            };
            let (mut fbo, mut viewport) = (0i32, [0i32; 4]);
            // SAFETY: plain GL queries on the current context.
            unsafe {
                glGetIntegerv(GL_FRAMEBUFFER_BINDING, &raw mut fbo);
                glGetIntegerv(GL_VIEWPORT, viewport.as_mut_ptr());
            }
            let mut target = mpv::mpv_opengl_fbo {
                fbo,
                w: viewport[2],
                h: viewport[3],
                internal_format: 0,
            };
            let mut flip: i32 = 1;
            let mut params = [
                mpv::mpv_render_param {
                    type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_FBO,
                    data: (&raw mut target).cast(),
                },
                mpv::mpv_render_param {
                    type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_FLIP_Y,
                    data: (&raw mut flip).cast(),
                },
                mpv::mpv_render_param {
                    type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
                    data: null_mut(),
                },
            ];
            // SAFETY: the render context lives while the mutex is held; its
            // GL context is current.
            unsafe { mpv::mpv_render_context_render(render.as_ptr(), params.as_mut_ptr()) };
        }

        /// Frees mpv's render context (before mpv itself goes).
        fn detach(&self) {
            let Ok(mut slot) = self.mpv.lock() else {
                return;
            };
            if let Some(render) = slot.take() {
                // SAFETY: made in our context, which is current while it goes.
                with_gl(self.cgl, || unsafe {
                    mpv::mpv_render_context_free(render.as_ptr());
                });
            }
        }
    }

    impl Drop for Render {
        fn drop(&mut self) {
            self.detach();
            // SAFETY: ours; Core Animation holds its own retains.
            unsafe {
                CGLReleaseContext(self.cgl);
                CGLReleasePixelFormat(self.pf);
            }
        }
    }

    struct Ivars {
        render: Arc<Render>,
    }

    define_class!(
        // SAFETY: CAOpenGLLayer may be subclassed; the overrides below keep
        // its signatures, and nothing here implements Drop.
        #[unsafe(super(CAOpenGLLayer, CALayer, NSObject))]
        #[name = "CourtsideVideoLayer"]
        #[ivars = Ivars]
        struct VideoLayer;

        impl VideoLayer {
            /// Our pixel format and context, so mpv's render context (made
            /// in it) is the one Core Animation draws with.
            #[unsafe(method(copyCGLPixelFormatForDisplayMask:))]
            fn copy_pixel_format(&self, _mask: u32) -> CGLPixelFormatObj {
                // SAFETY: ours, live while the layer is; the caller releases.
                unsafe { CGLRetainPixelFormat(self.ivars().render.pf) }
            }

            #[unsafe(method(copyCGLContextForPixelFormat:))]
            fn copy_context(&self, _pf: CGLPixelFormatObj) -> CGLContextObj {
                // SAFETY: as above.
                unsafe { CGLRetainContext(self.ivars().render.cgl) }
            }

            #[unsafe(method(canDrawInCGLContext:pixelFormat:forLayerTime:displayTime:))]
            fn can_draw(
                &self,
                _ctx: CGLContextObj,
                _pf: CGLPixelFormatObj,
                _t: CFTimeInterval,
                _ts: *const CVTimeStamp,
            ) -> bool {
                self.ivars().render.dirty.load(Ordering::Acquire)
            }

            #[unsafe(method(drawInCGLContext:pixelFormat:forLayerTime:displayTime:))]
            fn draw(
                &self,
                ctx: CGLContextObj,
                pf: CGLPixelFormatObj,
                t: CFTimeInterval,
                ts: *const CVTimeStamp,
            ) {
                self.ivars().render.draw();
                // SAFETY: the superclass's draw flushes what was drawn.
                unsafe {
                    let _: () = msg_send![
                        super(self),
                        drawInCGLContext: ctx,
                        pixelFormat: pf,
                        forLayerTime: t,
                        displayTime: ts
                    ];
                }
            }
        }
    );

    impl VideoLayer {
        fn new(render: Arc<Render>) -> Retained<Self> {
            let this = Self::alloc().set_ivars(Ivars { render });
            // SAFETY: CALayer's designated initialiser.
            unsafe { msg_send![super(this), init] }
        }
    }

    /// A layer-hosting NSView over the webview, its layer drawn by mpv.
    /// AppKit works in points, as CSS pixels do, so bounds need no scaling.
    pub struct Surface {
        view: Retained<NSView>,
        /// Where the page is drawn (the content view if none is found).
        webview: Retained<NSView>,
        layer: Retained<VideoLayer>,
        render: Arc<Render>,
    }

    fn main_thread() -> Result<MainThreadMarker, String> {
        MainThreadMarker::new().ok_or_else(|| "video must be driven from the main thread".into())
    }

    /// The window's WKWebView: the page's coordinates are its own, which
    /// start below the title bar, not at the content view's top.
    fn find_webview(view: &NSView) -> Option<Retained<NSView>> {
        let class = AnyClass::get(c"WKWebView")?;
        for sub in view.subviews() {
            if sub.isKindOfClass(class) {
                return Some(sub);
            }
            if let Some(found) = find_webview(&sub) {
                return Some(found);
            }
        }
        None
    }

    /// `bounds` (CSS pixels from the page's top left) in `target`'s
    /// coordinates, by way of the webview's.
    fn frame(webview: &NSView, target: &NSView, b: Bounds) -> NSRect {
        let y = if webview.isFlipped() {
            b.y
        } else {
            webview.bounds().size.height - b.y - b.height
        };
        let rect = NSRect::new(NSPoint::new(b.x, y), NSSize::new(b.width, b.height));
        webview.convertRect_toView(rect, Some(target))
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
            let render = Arc::new(Render::new()?);
            let layer = VideoLayer::new(render.clone());
            layer.setContentsScale(ns_window.backingScaleFactor());
            // Core Animation asks every display refresh whether to draw.
            layer.setAsynchronous(true);
            let webview = find_webview(&content).unwrap_or_else(|| content.clone());
            let view = NSView::initWithFrame(NSView::alloc(mtm), frame(&webview, &content, bounds));
            // Layer-hosting: the layer first, then wantsLayer.
            view.setLayer(Some(&layer));
            view.setWantsLayer(true);
            // Added after the webview, so it's drawn on top of it.
            content.addSubview(&view);
            Ok(Surface {
                view,
                webview,
                layer,
                render,
            })
        }

        /// What mpv's `wid` option takes: nothing here (see the module note).
        pub fn wid(&self) -> Option<i64> {
            None
        }

        /// mpv draws into the layer from now on.
        pub fn attach(&self, player: &Mpv) -> Result<(), String> {
            self.render.attach(player.ctx.as_ptr())
        }

        /// Lets go of mpv (before it goes).
        pub fn detach(&self) {
            self.render.detach();
        }

        pub fn place(&self, bounds: Bounds, visible: bool) -> Result<(), String> {
            main_thread()?;
            // SAFETY: main thread; the superview is the window's content view.
            let Some(content) = (unsafe { self.view.superview() }) else {
                return Ok(());
            };
            self.view.setFrame(frame(&self.webview, &content, bounds));
            self.view.setHidden(!visible);
            if let Some(scale) = self.view.window().map(|w| w.backingScaleFactor()) {
                self.layer.setContentsScale(scale);
            }
            self.render.dirty.store(true, Ordering::Release);
            Ok(())
        }

        pub fn close(self) -> Result<(), String> {
            main_thread()?;
            self.detach();
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
        pub fn wid(&self) -> Option<i64> {
            Some(self.hwnd.0 as i64)
        }

        /// mpv draws into the window by `wid`: nothing to hand over.
        pub fn attach(&self, _: &libmpv2::Mpv) -> Result<(), String> {
            Ok(())
        }

        pub fn detach(&self) {}

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
        pub fn wid(&self) -> Option<i64> {
            Some(self.window as i64)
        }

        /// mpv draws into the window by `wid`: nothing to hand over.
        pub fn attach(&self, _: &libmpv2::Mpv) -> Result<(), String> {
            Ok(())
        }

        pub fn detach(&self) {}

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
