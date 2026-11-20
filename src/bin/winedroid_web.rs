#[cfg(target_os = "emscripten")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "emscripten")]
static LAUNCH_STARTED: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "emscripten")]
const WEB_APK_PATH: &str = "/winedroid-upload/app.apk";

#[cfg(target_os = "emscripten")]
extern "C" {
    fn winedroid_web_error(data: *const u8, length: usize);
    fn winedroid_web_status(data: *const u8, length: usize);
}

#[cfg(target_os = "emscripten")]
fn main() {
    std::env::set_var("WINEDROID_PREFIX", "/winedroid-prefix");
    set_status("Choose an APK to start");
}

#[cfg(not(target_os = "emscripten"))]
fn main() {
    eprintln!("winedroid-web requires the wasm32-unknown-emscripten target");
}

#[cfg(target_os = "emscripten")]
#[no_mangle]
pub extern "C" fn winedroid_web_launch(width: u32, height: u32) -> i32 {
    if LAUNCH_STARTED.swap(true, Ordering::SeqCst) {
        report_error(
            "A browser activity session is already running. Reload to launch another APK.",
        );
        return 1;
    }

    set_status("Installing and starting the APK…");
    let path = std::path::Path::new(WEB_APK_PATH);
    if !path.is_file() {
        LAUNCH_STARTED.store(false, Ordering::SeqCst);
        report_error("The uploaded APK is missing from the browser filesystem.");
        return 1;
    }

    match winedroid::launch_web_apk(path, width, height) {
        Ok(()) => 0,
        Err(error) => {
            LAUNCH_STARTED.store(false, Ordering::SeqCst);
            report_error(&error.to_string());
            winedroid::report_missing_primitives();
            1
        }
    }
}

#[cfg(target_os = "emscripten")]
fn set_status(message: &str) {
    unsafe {
        winedroid_web_status(message.as_ptr(), message.len());
    }
}

#[cfg(target_os = "emscripten")]
fn report_error(message: &str) {
    eprintln!("{message}");
    unsafe {
        winedroid_web_error(message.as_ptr(), message.len());
    }
}
