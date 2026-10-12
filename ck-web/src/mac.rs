//! What Chromium needs on macOS. Its library is a framework in the app bundle (ck-web.app),
//! loaded by hand: from `Contents/Frameworks` for ck-web itself, and from three folders up
//! for the helpers it starts its other processes from (`ck-web Helper.app` and the rest, in
//! there), which sandbox themselves first. And the application must tell Chromium when it's
//! handling an event (`CefAppProtocol`).

use std::cell::Cell;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use cef::application_mac::{CefAppProtocol, CrAppControlProtocol, CrAppProtocol};
use objc2::rc::Retained;
use objc2::runtime::{Bool, NSObjectProtocol};
use objc2::{ClassType, DefinedClass, MainThreadMarker, define_class, extern_methods, msg_send};
use objc2_app_kit::{NSApp, NSApplication, NSEvent};

const FRAMEWORK: &str = "Chromium Embedded Framework.framework";

/// Chromium's library, loaded (and a helper's sandbox), until ck-web exits.
pub struct Loaded {
    _library: Library,
    _sandbox: Option<cef::sandbox::Sandbox>,
}

struct Library;

impl Drop for Library {
    fn drop(&mut self) {
        cef::unload_library();
    }
}

/// Load Chromium's library, for ck-web itself or (`helper`) one of its helpers.
pub fn load(args: &cef::args::Args, helper: bool) -> Result<Loaded, String> {
    let exe = std::env::current_exe().map_err(|e| format!("can't tell where it is: {e}"))?;
    let up = if helper { "../../.." } else { "../Frameworks" };
    let framework = exe.parent().unwrap_or(Path::new("/")).join(up).join(FRAMEWORK);
    if !framework.is_dir() {
        return Err(format!("Chromium isn't at {}: ck-web runs from its app, ck-web.app", framework.display()));
    }
    // (`Sandbox` finds its library by the same path, and fails hard without it.)
    let sandbox = if helper && framework.join("Libraries/libcef_sandbox.dylib").is_file() {
        let mut s = cef::sandbox::Sandbox::new();
        s.initialize(args.as_main_args());
        Some(s)
    } else {
        None
    };
    let library = CString::new(framework.join("Chromium Embedded Framework").as_os_str().as_bytes()).map_err(|e| e.to_string())?;
    // SAFETY: a NUL-terminated path, alive for the call.
    if cef::load_library(Some(unsafe { &*library.as_ptr() })) != 1 {
        return Err(format!("Chromium's library in {} didn't load", framework.display()));
    }
    Ok(Loaded { _library: Library, _sandbox: sandbox })
}

/// Whether the application is handling an event.
#[derive(Default)]
pub struct Handling(Cell<Bool>);

define_class!(
    /// The application, as Chromium wants it.
    #[unsafe(super(NSApplication))]
    #[ivars = Handling]
    pub struct WebApplication;

    impl WebApplication {
        #[unsafe(method(sendEvent:))]
        unsafe fn send_event(&self, event: &NSEvent) {
            let was = self.ivars().0.replace(Bool::YES);
            let _: () = unsafe { msg_send![super(self), sendEvent: event] };
            self.ivars().0.set(was);
        }
    }

    unsafe impl CrAppProtocol for WebApplication {
        #[unsafe(method(isHandlingSendEvent))]
        unsafe fn _is_handling_send_event(&self) -> Bool {
            self.ivars().0.get()
        }
    }

    unsafe impl CrAppControlProtocol for WebApplication {
        #[unsafe(method(setHandlingSendEvent:))]
        unsafe fn _set_handling_send_event(&self, handling: Bool) {
            self.ivars().0.set(handling);
        }
    }

    unsafe impl CefAppProtocol for WebApplication {}
);

impl WebApplication {
    extern_methods! {
        #[unsafe(method(sharedApplication))]
        fn shared_application() -> Retained<Self>;
    }
}

/// Make the application a `WebApplication`, before anything else makes it.
pub fn application() -> Result<(), String> {
    let _ = WebApplication::shared_application();
    let mtm = MainThreadMarker::new().ok_or("ck-web isn't on its main thread")?;
    if !NSApp(mtm).isKindOfClass(WebApplication::class()) {
        return Err("the application was made before ck-web's".into());
    }
    Ok(())
}
