//! Last-resort crash reporting: a daemon that dies leaves a line in its log saying why.
//!
//! The release profile sets `panic = "abort"`, and a native fault (access violation, failed
//! delay-load, stack overflow) never reaches Rust at all. Either way the process ends without
//! unwinding, so nothing the tracing subscriber would have printed afterwards reaches the log. Under
//! the Windows SCM, stderr goes nowhere either, so a crash left only a service that stopped: a
//! v0.17.17 Windows daemon without Npcap died on its first poll with nothing logged after
//! "Daemon ready".
//!
//! Both handlers write straight to the log file rather than through `tracing`: a crash can happen
//! while the subscriber holds its own locks, and the handler must not wait on them.

use std::fs::OpenOptions;
use std::io::Write;
use std::panic::PanicHookInfo;
use std::path::PathBuf;
use std::sync::OnceLock;

use chrono::{SecondsFormat, Utc};

static LOG_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Install the panic hook (all platforms) and the unhandled-exception filter (Windows). Call once,
/// right after logging is initialised; later calls are ignored.
pub fn install(log_path: Option<PathBuf>) {
    if LOG_PATH.set(log_path).is_err() {
        return;
    }

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        append_crash_line(&panic_detail(info));
        // The default hook still prints the panic (and backtrace, if enabled) to stderr.
        previous(info);
    }));

    #[cfg(windows)]
    seh::install();
}

fn panic_detail(info: &PanicHookInfo<'_>) -> String {
    let payload = info
        .payload()
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
        .unwrap_or("<non-string panic payload>");
    let location = info
        .location()
        .map(|l| format!("{}:{}", l.file(), l.line()))
        .unwrap_or_else(|| "<unknown location>".to_string());
    let thread = std::thread::current();
    format!(
        "panicked at {location} (thread '{}'): {payload}",
        thread.name().unwrap_or("<unnamed>")
    )
}

/// Append `Daemon crashed: <detail>` to the log file, in the same shape as a tracing line.
fn append_crash_line(detail: &str) {
    let Some(Some(path)) = LOG_PATH.get() else {
        return;
    };
    let line = format!(
        "{}  ERROR daemon: Daemon crashed: {detail}\n",
        Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true)
    );
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
        let _ = file.flush();
    }
}

#[cfg(windows)]
mod seh {
    use std::ffi::{CStr, c_void};

    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::Diagnostics::Debug::{
        EXCEPTION_POINTERS, SetUnhandledExceptionFilter,
    };
    use windows::Win32::System::LibraryLoader::{
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        GetModuleFileNameW, GetModuleHandleExW,
    };
    use windows::core::PCWSTR;

    const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
    /// `VcppException(ERROR_SEVERITY_ERROR, ERROR_MOD_NOT_FOUND)`, raised by the MSVC delay-load
    /// helper when a delay-loaded DLL can't be found.
    const DELAYLOAD_MODULE_NOT_FOUND: u32 = 0xC06D_007E;
    /// `VcppException(ERROR_SEVERITY_ERROR, ERROR_PROC_NOT_FOUND)`: the DLL loaded but lacks the
    /// export.
    const DELAYLOAD_PROC_NOT_FOUND: u32 = 0xC06D_007F;

    /// `DelayLoadProc` from `delayimp.h`.
    #[repr(C)]
    struct DelayLoadProc {
        import_by_name: i32,
        /// `szProcName` when `import_by_name`, else the ordinal in the low 32 bits.
        name_or_ordinal: usize,
    }

    /// `DelayLoadInfo` from `delayimp.h`: what the MSVC delay-load helper passes in
    /// `ExceptionInformation[0]` of the two exceptions above. (windows-rs' `DELAYLOAD_INFO` is the
    /// different `ResolveDelayLoadedAPI` structure.)
    #[repr(C)]
    struct DelayLoadInfo {
        cb: u32,
        pidd: *const c_void,
        ppfn: *const c_void,
        sz_dll: *const i8,
        dlp: DelayLoadProc,
        hmod_cur: *const c_void,
        pfn_cur: *const c_void,
        dw_last_error: u32,
    }

    pub(super) fn install() {
        unsafe {
            SetUnhandledExceptionFilter(Some(filter));
        }
    }

    /// Log the exception, then let it continue to Windows Error Reporting, so it still lands in
    /// Event Viewer and the process still ends the way it would have.
    unsafe extern "system" fn filter(info: *const EXCEPTION_POINTERS) -> i32 {
        let Some(record) =
            (unsafe { info.as_ref() }).and_then(|i| unsafe { i.ExceptionRecord.as_ref() })
        else {
            return EXCEPTION_CONTINUE_SEARCH;
        };

        let code = record.ExceptionCode.0 as u32;
        let address = record.ExceptionAddress;
        let mut detail = format!(
            "exception 0x{code:08X} ({}) at {address:p} in {}",
            code_name(code),
            module_at(address)
        );

        if (code == DELAYLOAD_MODULE_NOT_FOUND || code == DELAYLOAD_PROC_NOT_FOUND)
            && record.NumberParameters >= 1
            && let Some(dli) =
                unsafe { (record.ExceptionInformation[0] as *const DelayLoadInfo).as_ref() }
        {
            let dll = unsafe { c_str(dli.sz_dll) };
            if code == DELAYLOAD_MODULE_NOT_FOUND {
                detail.push_str(&format!(
                    ": failed to load {dll} (Win32 error {}). Npcap is missing or not on the DLL \
                     search path",
                    dli.dw_last_error
                ));
            } else {
                let export = if dli.dlp.import_by_name != 0 {
                    unsafe { c_str(dli.dlp.name_or_ordinal as *const i8) }
                } else {
                    format!("ordinal {}", dli.dlp.name_or_ordinal as u32)
                };
                detail.push_str(&format!(
                    ": {dll} has no export {export}. The installed Npcap is incompatible"
                ));
            }
        }

        super::append_crash_line(&detail);
        eprintln!("Daemon crashed: {detail}");
        EXCEPTION_CONTINUE_SEARCH
    }

    fn code_name(code: u32) -> &'static str {
        match code {
            0xC000_0005 => "access violation",
            0xC000_00FD => "stack overflow",
            0xC000_001D => "illegal instruction",
            0xC000_0094 => "integer divide by zero",
            0xC000_0409 => "fast-fail / stack buffer overrun",
            DELAYLOAD_MODULE_NOT_FOUND => "delay-load: module not found",
            DELAYLOAD_PROC_NOT_FOUND => "delay-load: procedure not found",
            _ => "unhandled exception",
        }
    }

    /// Path of the module containing `address`, or `<unknown module>`.
    fn module_at(address: *mut c_void) -> String {
        let mut module = HMODULE::default();
        let found = unsafe {
            GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                    | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                PCWSTR(address as *const u16),
                &mut module,
            )
        };
        if found.is_err() {
            return "<unknown module>".to_string();
        }
        let mut buf = [0u16; 260];
        let len = unsafe { GetModuleFileNameW(module, &mut buf) } as usize;
        if len == 0 {
            return "<unknown module>".to_string();
        }
        String::from_utf16_lossy(&buf[..len])
    }

    unsafe fn c_str(ptr: *const i8) -> String {
        if ptr.is_null() {
            return "<unknown>".to_string();
        }
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A panic leaves `Daemon crashed: panicked at <file:line> ...: <message>` in the log file on
    /// its own, with no tracing subscriber and no flush from anyone else. (The test profile
    /// unwinds, so `catch_unwind` observes the hook; release aborts right after it.)
    #[test]
    fn panic_leaves_a_crash_line_in_the_log_file() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("daemon.log");
        install(Some(log.clone()));

        let result = std::panic::catch_unwind(|| panic!("boom from the crash_log test"));
        assert!(result.is_err());
        // Other tests' hooks are chained, not replaced; restore the default for the rest of the run.
        let _ = std::panic::take_hook();

        let contents = std::fs::read_to_string(&log).unwrap();
        let line = contents
            .lines()
            .find(|l| l.contains("boom from the crash_log test"))
            .expect("the panic should be logged");
        assert!(line.contains("ERROR daemon: Daemon crashed: panicked at"));
        assert!(line.contains(file!()), "names the panic's location: {line}");
    }

    /// Windows only: a native fault, which no panic hook ever sees, still leaves a line. Runs the
    /// fault in a child copy of this test binary, since the fault ends whatever process it's in.
    #[cfg(windows)]
    mod native {
        use std::process::Command;

        const CHILD_ENV: &str = "SCANOPY_CRASH_LOG_CHILD";
        const LOG_ENV: &str = "SCANOPY_CRASH_LOG_PATH";

        /// Child entry point; does nothing unless launched by one of the tests below.
        #[test]
        fn crash_child() {
            let (Ok(crash_type), Ok(log)) = (std::env::var(CHILD_ENV), std::env::var(LOG_ENV))
            else {
                return;
            };
            super::install(Some(log.into()));
            match crash_type.as_str() {
                "access_violation" => unsafe {
                    std::ptr::write_volatile(std::ptr::null_mut::<u8>().wrapping_add(0x10), 1)
                },
                "delay_load" => {
                    // Any Packet export: packet.dll is delay-loaded (build.rs), so with no Npcap
                    // this first call is where the loader raises.
                    #[link(name = "Packet")]
                    unsafe extern "C" {
                        fn PacketGetAdapterNames(buffer: *mut i8, size: *mut u32) -> u8;
                    }
                    let mut size = 0u32;
                    unsafe { PacketGetAdapterNames(std::ptr::null_mut(), &mut size) };
                }
                other => panic!("unknown crash type {other}"),
            }
        }

        fn run_child(crash_type: &str) -> String {
            let dir = tempfile::tempdir().unwrap();
            let log = dir.path().join("daemon.log");
            let status = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "daemon::runtime::crash_log::tests::native::crash_child",
                    "--nocapture",
                ])
                .env(CHILD_ENV, crash_type)
                .env(LOG_ENV, &log)
                .status()
                .unwrap();
            assert!(!status.success(), "the child should have crashed");
            std::fs::read_to_string(&log).unwrap_or_default()
        }

        #[test]
        fn access_violation_leaves_a_crash_line() {
            let log = run_child("access_violation");
            assert!(
                log.contains("Daemon crashed: exception 0xC0000005 (access violation)"),
                "{log}"
            );
        }

        #[test]
        fn failed_delay_load_names_the_dll() {
            if pnet::datalink::winpcap::packet_dll_available() {
                eprintln!("skipped: packet.dll is loadable on this host, so no delay-load fails");
                return;
            }
            let log = run_child("delay_load");
            assert!(log.contains("exception 0xC06D007E"), "{log}");
            assert!(
                log.to_lowercase().contains("failed to load packet.dll"),
                "{log}"
            );
        }
    }
}
