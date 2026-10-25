//! Runtime glue: routes Rust `log` output and panics into the engine's
//! `hpl::Log` / `Warning` / `Error`.

use std::ffi::{CString, c_char};
use std::sync::atomic::{AtomicPtr, Ordering};

/// Log levels passed to [`HplLogCallback`].
pub const HPL_LOG_LEVEL_ERROR: i32 = 0;
pub const HPL_LOG_LEVEL_WARNING: i32 = 1;
pub const HPL_LOG_LEVEL_INFO: i32 = 2;
pub const HPL_LOG_LEVEL_DEBUG: i32 = 3;

/// Receives one log line (UTF-8, no trailing newline). Must be thread safe.
pub type HplLogCallback = Option<unsafe extern "C" fn(level: i32, message: *const c_char)>;

static CALLBACK: AtomicPtr<()> = AtomicPtr::new(std::ptr::null_mut());

fn emit(level: i32, message: &str) {
    let cb = CALLBACK.load(Ordering::Acquire);
    if cb.is_null() {
        return;
    }
    // SAFETY: only ever stored from a valid HplLogCallback.
    let cb: unsafe extern "C" fn(i32, *const c_char) = unsafe { std::mem::transmute(cb) };
    let text = CString::new(message.replace('\0', "\\0")).unwrap_or_default();
    // SAFETY: the callback contract.
    unsafe { cb(level, text.as_ptr()) };
}

struct BridgeLogger;

impl log::Log for BridgeLogger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        !CALLBACK.load(Ordering::Relaxed).is_null()
    }

    fn log(&self, record: &log::Record<'_>) {
        let level = match record.level() {
            log::Level::Error => HPL_LOG_LEVEL_ERROR,
            log::Level::Warn => HPL_LOG_LEVEL_WARNING,
            log::Level::Info => HPL_LOG_LEVEL_INFO,
            log::Level::Debug | log::Level::Trace => HPL_LOG_LEVEL_DEBUG,
        };
        emit(level, &record.args().to_string());
    }

    fn flush(&self) {}
}

static LOGGER: BridgeLogger = BridgeLogger;

/// Initialise the Rust side of the engine. Call once at startup, before any
/// other `hpl_*` function if logging is wanted. Safe to call again to
/// replace the callback; pass null to silence Rust logging.
#[unsafe(no_mangle)]
pub extern "C" fn hpl_rs_init(log_callback: HplLogCallback) {
    let ptr = log_callback.map_or(std::ptr::null_mut(), |f| f as *mut ());
    CALLBACK.store(ptr, Ordering::Release);

    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = log::set_logger(&LOGGER);
        log::set_max_level(if cfg!(debug_assertions) { log::LevelFilter::Debug } else { log::LevelFilter::Info });

        // panic = "abort": make sure the reason reaches the engine log first.
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            emit(HPL_LOG_LEVEL_ERROR, &format!("Rust panic: {info}"));
            default_hook(info);
        }));
    });
}

/// Version string of the Rust library (static, NUL-terminated).
#[unsafe(no_mangle)]
pub extern "C" fn hpl_rs_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}
