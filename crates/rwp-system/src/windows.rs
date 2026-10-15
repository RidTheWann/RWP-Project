//! Windows-specific telemetry implementation.

#![allow(unsafe_code)]

use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use nvml_wrapper::{
    Nvml,
    enum_wrappers::device::{Clock, TemperatureSensor},
};
use rwp_core::SystemSample;

use crate::pdh::PdhGpuSampler;
use crate::processes::{ActivityIdAllocator, ProcessCpuTimes, sample_background_processes};
use crate::threads::{ThreadCpuTimes, sample_process_threads};
use windows_sys::Win32::{
    Foundation::FILETIME,
    System::{
        Performance::{QueryPerformanceCounter, QueryPerformanceFrequency},
        SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
        Threading::GetSystemTimes,
    },
};

/// Errors raised when validating privileges required for a PresentMon realtime trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PresentMonPermissionError {
    /// The current token is neither elevated nor a member of Performance Log Users.
    #[error(
        "PresentMon requires Administrator privileges or membership in the Performance Log Users group"
    )]
    InsufficientPrivileges,
    /// A Windows security API failed while checking group membership.
    #[error("{api} failed with Windows error code {code}")]
    Api {
        /// Windows API that failed.
        api: &'static str,
        /// Error code returned by GetLastError.
        code: u32,
    },
}

const SID_ADMINISTRATORS: &str = "S-1-5-32-544";
const SID_PERFORMANCE_LOG_USERS: &str = "S-1-5-32-559";

fn token_has_sid(sid: &str) -> Result<bool, PresentMonPermissionError> {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{GetLastError, HLOCAL, LocalFree};
    use windows_sys::Win32::Security::Authorization::ConvertStringSidToSidW;
    use windows_sys::Win32::Security::{CheckTokenMembership, PSID};

    let wide = sid
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut psid: PSID = null_mut();
    // SAFETY: wide is a null-terminated UTF-16 SID string and psid is a valid out-pointer.
    let converted = unsafe { ConvertStringSidToSidW(wide.as_ptr(), &mut psid) };
    if converted == 0 {
        return Err(PresentMonPermissionError::Api {
            api: "ConvertStringSidToSidW",
            // SAFETY: GetLastError reads the calling thread error state and has no pointer requirements.
            code: unsafe { GetLastError() },
        });
    }

    let mut is_member = 0_i32;
    // SAFETY: psid was allocated by ConvertStringSidToSidW; null selects the current effective token.
    let checked = unsafe { CheckTokenMembership(null_mut(), psid, &mut is_member) };
    let result = if checked == 0 {
        Err(PresentMonPermissionError::Api {
            api: "CheckTokenMembership",
            // SAFETY: GetLastError reads the calling thread error state and has no pointer requirements.
            code: unsafe { GetLastError() },
        })
    } else {
        Ok(is_member != 0)
    };
    // SAFETY: psid is the allocation returned by ConvertStringSidToSidW and LocalFree owns its release.
    let _ = unsafe { LocalFree(psid as HLOCAL) };
    result
}

/// Verifies the Administrator or Performance Log Users prerequisite for PresentMon realtime capture.
pub fn preflight_presentmon_permissions() -> Result<(), PresentMonPermissionError> {
    if token_has_sid(SID_ADMINISTRATORS)? || token_has_sid(SID_PERFORMANCE_LOG_USERS)? {
        Ok(())
    } else {
        Err(PresentMonPermissionError::InsufficientPrivileges)
    }
}
/// Error raised while attaching PresentMon to a Windows Job Object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PresentMonJobError {
    /// Creating or configuring the Job Object failed.
    #[error("{api} failed with Windows error code {code}")]
    Api {
        /// Win32 API function that failed.
        api: &'static str,
        /// Error code returned by GetLastError.
        code: u32,
    },
}

/// Owns a Windows Job Object configured to terminate PresentMon when the owner closes it.
#[cfg(windows)]
pub struct PresentMonJob {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl PresentMonJob {
    /// Creates a kill-on-close Job Object and assigns the given child process to it.
    pub fn assign_process(child: &std::process::Child) -> Result<Self, PresentMonJobError> {
        use std::{os::windows::io::AsRawHandle, ptr::null_mut};
        use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };

        let process_handle = child.as_raw_handle();

        // SAFETY: null security attributes/name request a private Job Object owned by this process.
        let handle = unsafe { CreateJobObjectW(null_mut(), std::ptr::null()) };
        if handle.is_null() {
            return Err(PresentMonJobError::Api {
                api: "CreateJobObjectW",
                // SAFETY: GetLastError reads the calling thread error state.
                code: unsafe { GetLastError() },
            });
        }

        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: handle is a live Job Object; info has the exact documented structure and size.
        let configured = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &mut info as *mut _ as *mut _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if configured == 0 {
            // SAFETY: GetLastError reads the calling thread's last-error state and takes no pointers.
            let code = unsafe { GetLastError() };
            // SAFETY: handle was returned by CreateJobObjectW and is owned here.
            let _ = unsafe { CloseHandle(handle) };
            return Err(PresentMonJobError::Api {
                api: "SetInformationJobObject",
                code,
            });
        }

        // SAFETY: process_handle is the live handle returned by std::process::Child on Windows.
        let assigned = unsafe { AssignProcessToJobObject(handle, process_handle) };
        if assigned == 0 {
            // SAFETY: GetLastError reads the calling thread's last-error state and takes no pointers.
            let code = unsafe { GetLastError() };
            // SAFETY: handle was returned by CreateJobObjectW and is owned here.
            let _ = unsafe { CloseHandle(handle) };
            return Err(PresentMonJobError::Api {
                api: "AssignProcessToJobObject",
                code,
            });
        }
        Ok(Self { handle })
    }
}

#[cfg(windows)]
impl Drop for PresentMonJob {
    fn drop(&mut self) {
        // SAFETY: self.handle is exclusively owned by this RAII wrapper and closed exactly once.
        let _ = unsafe { windows_sys::Win32::Foundation::CloseHandle(self.handle) };
    }
}

/// Authenticode verification outcome for one executable file.
///
/// Infallible by design: every Win32 failure maps to `VerifyFailed` with the
/// raw status code, so advisory callers never need a new error variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticodeStatus {
    /// The file carries a trusted signature.
    Signed,
    /// The file carries no signature.
    NotSigned,
    /// Verification itself failed; holds the raw WinVerifyTrust status.
    VerifyFailed {
        /// Raw status code returned by WinVerifyTrust.
        code: i32,
    },
}

/// WinVerifyTrust statuses meaning "no trusted signature present": no
/// signature at all, or a subject the trust provider does not trust.
/// Tamper-evidence codes (bad digest, revoked, expired) intentionally stay in
/// `VerifyFailed` so unsigned and suspicious inputs never share a bucket.
const TRUST_E_NOSIGNATURE: i32 = 0x800B0100u32 as i32;
const TRUST_E_SUBJECT_NOT_TRUSTED: i32 = 0x800B0003u32 as i32;

/// Verifies the Authenticode signature of one executable file.
///
/// Revocation checks are disabled so the call stays offline-first; a trusted
/// chain against the local machine stores is sufficient for the v1.0 advisory
/// use (doctor preflight and provenance notes, never a capture gate).
pub fn verify_presentmon_authenticode(path: &std::path::Path) -> AuthenticodeStatus {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Security::WinTrust::{
        WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_FILE_INFO, WTD_CHOICE_FILE,
        WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE,
        WinVerifyTrustEx,
    };

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut file = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: wide.as_ptr(),
        hFile: std::ptr::null_mut(),
        pgKnownSubject: std::ptr::null_mut(),
    };
    let mut data = WINTRUST_DATA {
        cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_NONE,
        dwUnionChoice: WTD_CHOICE_FILE,
        dwStateAction: WTD_STATEACTION_VERIFY,
        ..Default::default()
    };
    // SAFETY: data is fully assigned before use and Anonymous.pFile is the
    // documented member for WTD_CHOICE_FILE; file outlives the call.
    let status = unsafe {
        data.Anonymous.pFile = &mut file;
        WinVerifyTrustEx(
            std::ptr::null_mut(),
            &WINTRUST_ACTION_GENERIC_VERIFY_V2 as *const _ as *mut _,
            &mut data,
        )
    };
    data.dwStateAction = WTD_STATEACTION_CLOSE;
    // SAFETY: same initialized structures; the close action frees verification state.
    let _ = unsafe {
        WinVerifyTrustEx(
            std::ptr::null_mut(),
            &WINTRUST_ACTION_GENERIC_VERIFY_V2 as *const _ as *mut _,
            &mut data,
        )
    };
    if status == 0 {
        AuthenticodeStatus::Signed
    } else if status == TRUST_E_NOSIGNATURE || status == TRUST_E_SUBJECT_NOT_TRUSTED {
        AuthenticodeStatus::NotSigned
    } else {
        AuthenticodeStatus::VerifyFailed { code: status }
    }
}

/// Reports whether the calling process is attached to a console.
///
/// The PresentMon spawn path records this once per child: a child spawned by a
/// console-attached caller shares that console, so a direct
/// `GenerateConsoleCtrlEvent` reaches its process group; a child of a
/// console-less caller (release GUI with `windows_subsystem = "windows"`,
/// detached helpers) needs the attach-then-signal path instead.
pub fn caller_has_console() -> bool {
    use windows_sys::Win32::System::Console::GetConsoleProcessList;
    let mut buffer = [0u32; 2];
    // SAFETY: buffer is a live two-element array; the API writes at most the
    // given length and otherwise only reads the calling thread's console
    // attachment state. A nonzero return means a console exists (the exact
    // count does not matter here).
    let count = unsafe { GetConsoleProcessList(buffer.as_mut_ptr(), buffer.len() as u32) };
    count != 0
}

/// Failure of the attach-then-signal BREAK path used by console-less callers.
///
/// Raw Win32 codes are preserved so stop-path diagnostics can name the exact
/// failure (for example `AttachConsole` refusing an already-attached caller).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DetachedBreakError {
    /// `AttachConsole` could not attach to the child's console.
    #[error("AttachConsole failed with Windows error code {code}")]
    AttachFailed {
        /// Error code returned by GetLastError.
        code: u32,
    },
    /// The control event could not be delivered after attaching.
    #[error("GenerateConsoleCtrlEvent failed with Windows error code {code}")]
    SignalFailed {
        /// Error code returned by GetLastError.
        code: u32,
    },
}

/// Serializes console attach/signal sequences.
///
/// Console attachment is process-global: concurrent attach/free pairs from
/// several stop requests would detach each other's console mid-signal.
static CONSOLE_SIGNAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Sends CTRL+BREAK to a process group owned by a console-less caller.
///
/// Attaches to the child's console, delivers `CTRL_BREAK_EVENT` to the given
/// group, and always detaches again — including on failure — so the caller
/// never keeps a borrowed console. The caller must have no console of its own
/// (see [`caller_has_console`]); the child must own one.
pub fn send_break_to_detached_group(process_group_id: u32) -> Result<(), DetachedBreakError> {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::System::Console::{
        AttachConsole, CTRL_BREAK_EVENT, FreeConsole, GenerateConsoleCtrlEvent,
    };

    let _guard = CONSOLE_SIGNAL_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // SAFETY: AttachConsole takes only the target process identifier and
    // touches no caller memory; the serialized guard keeps concurrent stop
    // requests from interleaving attach/free pairs.
    let attached = unsafe { AttachConsole(process_group_id) };
    if attached == 0 {
        // SAFETY: GetLastError reads the calling thread error state.
        let code = unsafe { GetLastError() };
        return Err(DetachedBreakError::AttachFailed { code });
    }
    // SAFETY: the caller is attached to the child's console (just attached
    // above) and passes only the group identifier obtained from Child::id().
    let sent = unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, process_group_id) };
    let signal_code = if sent == 0 {
        // SAFETY: GetLastError reads the calling thread error state and takes
        // no pointers; it runs before FreeConsole on the same thread.
        Some(unsafe { GetLastError() })
    } else {
        None
    };
    // SAFETY: FreeConsole detaches the borrowed console and takes no pointers;
    // it runs on every path after a successful attach so no borrowed console
    // is ever retained, even when delivery failed.
    let _ = unsafe { FreeConsole() };
    match signal_code {
        None => Ok(()),
        Some(code) => Err(DetachedBreakError::SignalFailed { code }),
    }
}

/// Process-wide stop request shared with the installed console handler.
///
/// There is exactly one flag on purpose: the handler is a bare callback with
/// no context, and the CLI runs a single capture per process. [`ConsoleStopGuard`]
/// resets it on install.
static CAPTURE_STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Shared stop flag written by the console handler.
///
/// Pass it as the cancellation input to the session runner so the first
/// CTRL+C or CTRL+BREAK turns into a graceful stop with a persisted partial
/// session instead of process death.
pub fn capture_stop_flag() -> &'static AtomicBool {
    &CAPTURE_STOP_REQUESTED
}

/// RAII console-control handler for live capture.
///
/// Installing routes the first CTRL+C or CTRL+BREAK into
/// [`capture_stop_flag`]; a repeat signal while a stop is already in progress
/// is left to default handling so the process still ends (and the
/// kill-on-close Job Object still terminates PresentMon). Dropping removes
/// the handler again. Close/logoff/shutdown keep default handling.
pub struct ConsoleStopGuard {
    /// Token type: construction installs, drop uninstalls.
    _token: (),
}

impl ConsoleStopGuard {
    /// Installs the handler and arms a fresh stop request.
    pub fn install() -> Result<Self, SystemTelemetryError> {
        use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
        CAPTURE_STOP_REQUESTED.store(false, Ordering::SeqCst);
        // SAFETY: the handler is a plain function item with the documented
        // `unsafe extern "system" fn(u32) -> BOOL` shape that only flips an
        // atomic and reads no caller memory.
        let installed = unsafe { SetConsoleCtrlHandler(Some(console_stop_handler), 1) };
        if installed == 0 {
            return Err(SystemTelemetryError::ConsoleControl);
        }
        Ok(Self { _token: () })
    }
}

impl Drop for ConsoleStopGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
        // SAFETY: removes exactly the routine installed above; the flag is
        // left untouched because it belongs to the finished run.
        let _ = unsafe { SetConsoleCtrlHandler(Some(console_stop_handler), 0) };
    }
}

/// Console callback behind [`ConsoleStopGuard`].
///
/// Atomically claims the stop and then applies the pure
/// [`crate::decide_console_signal`] core to the previous state, so the first
/// CTRL+C or CTRL+BREAK is consumed exactly once even if signals race, while
/// repeats and close/logoff/shutdown keep default handling.
unsafe extern "system" fn console_stop_handler(ctrl_type: u32) -> windows_sys::core::BOOL {
    use windows_sys::Win32::Foundation::{FALSE, TRUE};
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT};
    if ctrl_type != CTRL_C_EVENT && ctrl_type != CTRL_BREAK_EVENT {
        return FALSE;
    }
    // SAFETY: swap on a process-wide atomic needs no pointers; serializing
    // with the install-time reset below keeps one capture's lifetime exact.
    let already_stopping = CAPTURE_STOP_REQUESTED.swap(true, Ordering::SeqCst);
    if crate::decide_console_signal(ctrl_type, already_stopping) {
        TRUE
    } else {
        FALSE
    }
}

/// Requests a CTRL+BREAK signal for a process group created with CREATE_NEW_PROCESS_GROUP.
pub fn request_process_group_break(process_group_id: u32) -> Result<(), SystemTelemetryError> {
    // SAFETY: The API is called with a process-group identifier obtained from Child::id().
    let ok = unsafe {
        windows_sys::Win32::System::Console::GenerateConsoleCtrlEvent(
            windows_sys::Win32::System::Console::CTRL_BREAK_EVENT,
            process_group_id,
        )
    };
    if ok == 0 {
        return Err(SystemTelemetryError::ConsoleControl);
    }
    Ok(())
}

/// PresentMon file version extracted from the Windows version resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentMonVersion {
    /// Major version.
    pub major: u16,
    /// Minor version.
    pub minor: u16,
    /// Patch version.
    pub patch: u16,
    /// Build/revision component.
    pub build: u16,
}

impl PresentMonVersion {
    /// Returns whether the version is in the supported PresentMon 2.x matrix.
    pub const fn is_supported(self) -> bool {
        self.major == 2
    }
}

impl std::fmt::Display for PresentMonVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}.{}.{}.{}",
            self.major, self.minor, self.patch, self.build
        )
    }
}

/// `ERROR_RESOURCE_TYPE_NOT_FOUND` as returned by `GetLastError` when an
/// executable carries no version resource. Observed on the genuine signed
/// PresentMon 2.6.0 binary (980,320 bytes): version detection must treat this
/// as "unknown", never as evidence for or against the 2.x matrix.
const ERROR_RESOURCE_TYPE_NOT_FOUND: u32 = 1813;

/// Maps a version-resource API failure to `UnknownVersion` only for
/// `ERROR_RESOURCE_TYPE_NOT_FOUND` (1813: the file carries no version
/// resource). Any other code stays a hard `Api` error; in particular,
/// non-executable input surfaces 1812 (`ERROR_RESOURCE_DATA_NOT_FOUND`) and
/// must keep failing closed.
fn version_api_error(api: &'static str, code: u32) -> PresentMonVersionError {
    if code == ERROR_RESOURCE_TYPE_NOT_FOUND {
        PresentMonVersionError::UnknownVersion { api, code }
    } else {
        PresentMonVersionError::Api { api, code }
    }
}

/// Errors raised while reading the PresentMon executable version resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PresentMonVersionError {
    /// A Windows API could not read the version resource.
    #[error("{api} failed with Windows error code {code}")]
    Api {
        /// Windows API that failed.
        api: &'static str,
        /// Error code returned by GetLastError.
        code: u32,
    },
    /// The executable carries no version resource, so its version is unknown
    /// (distinct from a detected-but-unsupported version).
    #[error("PresentMon version is unknown ({api} failed with Windows error code {code})")]
    UnknownVersion {
        /// Windows API that reported the missing resource.
        api: &'static str,
        /// Error code returned by GetLastError.
        code: u32,
    },
    /// The version resource did not contain a valid fixed-file-info record.
    #[error("PresentMon version resource is invalid")]
    InvalidResource,
    /// The detected version is outside the supported 2.x matrix.
    #[error("unsupported PresentMon version {detected}; supported major version is 2.x")]
    UnsupportedVersion {
        /// Version detected from the executable.
        detected: PresentMonVersion,
    },
}

/// Detects and validates the file version resource exposed by PresentMon.exe.
pub fn detect_presentmon_version(
    path: &std::path::Path,
) -> Result<PresentMonVersion, PresentMonVersionError> {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt, ptr::null_mut};
    use windows_sys::Win32::{
        Foundation::GetLastError,
        Storage::FileSystem::{
            GetFileVersionInfoSizeW, GetFileVersionInfoW, VS_FIXEDFILEINFO, VerQueryValueW,
        },
    };

    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut handle = 0_u32;
    // SAFETY: `wide` is a null-terminated UTF-16 path owned by this stack frame and `handle` is a valid out-pointer.
    let size = unsafe { GetFileVersionInfoSizeW(wide.as_ptr(), &mut handle) };
    if size == 0 {
        // SAFETY: GetLastError reads the calling thread error state and has no pointer requirements.
        let code = unsafe { GetLastError() };
        return Err(version_api_error("GetFileVersionInfoSizeW", code));
    }

    let mut data = vec![0_u8; size as usize];
    // SAFETY: `data` is writable for `size` bytes as required by GetFileVersionInfoW.
    let ok = unsafe {
        GetFileVersionInfoW(
            wide.as_ptr(),
            handle,
            size,
            data.as_mut_ptr() as *mut c_void,
        )
    };
    if ok == 0 {
        // SAFETY: GetLastError reads the calling thread error state and has no pointer requirements.
        let code = unsafe { GetLastError() };
        return Err(version_api_error("GetFileVersionInfoW", code));
    }

    let root = [92_u16, 0_u16];
    let mut value: *mut c_void = null_mut();
    let mut value_len = 0_u32;
    // SAFETY: `data` is an initialized version-resource buffer and `value`/`value_len` are valid output pointers.
    let ok = unsafe {
        VerQueryValueW(
            data.as_ptr() as *const c_void,
            root.as_ptr(),
            &mut value,
            &mut value_len,
        )
    };
    if ok == 0 || value.is_null() || value_len < std::mem::size_of::<VS_FIXEDFILEINFO>() as u32 {
        return Err(PresentMonVersionError::InvalidResource);
    }

    // SAFETY: VerQueryValueW returned a pointer to a VS_FIXEDFILEINFO record with sufficient length.
    let info = unsafe { &*(value as *const VS_FIXEDFILEINFO) };
    if info.dwSignature != 0xFEEF04BD {
        return Err(PresentMonVersionError::InvalidResource);
    }

    Ok(PresentMonVersion {
        major: (info.dwProductVersionMS >> 16) as u16,
        minor: (info.dwProductVersionMS & 0xFFFF) as u16,
        patch: (info.dwProductVersionLS >> 16) as u16,
        build: (info.dwProductVersionLS & 0xFFFF) as u16,
    })
}

/// Active-display refresh-rate observation for one target process.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayRefreshRate {
    /// Current display refresh rate in Hz.
    pub hz: f64,
    /// Windows display-device name associated with the selected process window.
    pub device_name: String,
}

/// Errors raised while resolving a process window to its current display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DisplayRefreshRateError {
    /// A Windows window/monitor API returned an error.
    #[error("{api} failed with Windows error code {code}")]
    Api {
        /// API that failed.
        api: &'static str,
        /// Win32 error code.
        code: u32,
    },
    /// The resolved display mode did not report a usable refresh rate.
    #[error("active display did not report a usable refresh rate")]
    InvalidMode,
}

fn normalize_display_refresh_rate(frequency_hz: u32) -> Option<f64> {
    match frequency_hz {
        0 | 1 => None,
        frequency_hz => Some(frequency_hz as f64),
    }
}

fn window_process_id(hwnd: windows_sys::Win32::Foundation::HWND) -> u32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

    let mut process_id = 0_u32;
    // SAFETY: hwnd comes from a Win32 window enumeration and process_id is a valid out-pointer.
    unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };
    process_id
}

fn find_process_windows(
    process_id: u32,
) -> Result<Vec<windows_sys::Win32::Foundation::HWND>, DisplayRefreshRateError> {
    use windows_sys::Win32::Foundation::{GetLastError, LPARAM, TRUE};
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, IsWindowVisible};
    use windows_sys::core::BOOL;

    struct WindowSearch {
        process_id: u32,
        windows: Vec<windows_sys::Win32::Foundation::HWND>,
    }

    // SAFETY: EnumWindows calls this callback synchronously with the LPARAM supplied by the caller.
    unsafe extern "system" fn callback(
        hwnd: windows_sys::Win32::Foundation::HWND,
        lparam: LPARAM,
    ) -> BOOL {
        // SAFETY: EnumWindows invokes this callback synchronously with the LPARAM supplied below,
        // which points to the live WindowSearch value for the duration of the call.
        let search = unsafe { &mut *(lparam as *mut WindowSearch) };
        // SAFETY: hwnd is supplied by EnumWindows and is valid for IsWindowVisible.
        if unsafe { IsWindowVisible(hwnd) } == 0 {
            return TRUE;
        }
        if window_process_id(hwnd) == search.process_id {
            search.windows.push(hwnd);
        }
        TRUE
    }

    let mut search = WindowSearch {
        process_id,
        windows: Vec::new(),
    };
    // SAFETY: callback remains valid for the synchronous EnumWindows call and lparam points to search.
    let ok = unsafe { EnumWindows(Some(callback), &mut search as *mut WindowSearch as LPARAM) };
    if ok == 0 {
        return Err(DisplayRefreshRateError::Api {
            api: "EnumWindows",
            // SAFETY: GetLastError reads the calling thread error state and has no pointer requirements.
            code: unsafe { GetLastError() },
        });
    }
    Ok(search.windows)
}

fn select_process_window(
    process_id: u32,
    windows: &[windows_sys::Win32::Foundation::HWND],
) -> Option<windows_sys::Win32::Foundation::HWND> {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    if windows.is_empty() {
        return None;
    }
    // SAFETY: GetForegroundWindow returns the foreground window handle or null.
    let foreground = unsafe { GetForegroundWindow() };
    if !foreground.is_null() && window_process_id(foreground) == process_id {
        return Some(foreground);
    }
    windows.first().copied()
}

fn monitor_device_name(
    window: windows_sys::Win32::Foundation::HWND,
) -> Result<(windows_sys::Win32::Graphics::Gdi::HMONITOR, String), DisplayRefreshRateError> {
    use std::mem::zeroed;
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFOEXW, MonitorFromWindow,
    };

    // SAFETY: window is a valid top-level window selected from EnumWindows or GetForegroundWindow.
    let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return Err(DisplayRefreshRateError::Api {
            api: "MonitorFromWindow",
            // SAFETY: GetLastError reads the calling thread error state and has no pointer requirements.
            code: unsafe { GetLastError() },
        });
    }

    // SAFETY: MONITORINFOEXW is zero-initialized and cbSize is set as required by GetMonitorInfoW.
    let mut info: MONITORINFOEXW = unsafe { zeroed() };
    info.monitorInfo.cbSize = core::mem::size_of::<MONITORINFOEXW>() as u32;
    // SAFETY: monitor is a valid monitor handle and info is a valid writable output.
    let ok = unsafe { GetMonitorInfoW(monitor, &mut info as *mut _ as *mut _) };
    if ok == 0 {
        return Err(DisplayRefreshRateError::Api {
            api: "GetMonitorInfoW",
            // SAFETY: GetLastError reads the calling thread error state and has no pointer requirements.
            code: unsafe { GetLastError() },
        });
    }

    let name_len = match info.szDevice.iter().position(|value| *value == 0) {
        Some(index) => index,
        None => info.szDevice.len(),
    };
    let device_name = std::ffi::OsString::from_wide(&info.szDevice[..name_len])
        .to_string_lossy()
        .into_owned();
    Ok((monitor, device_name))
}

fn current_display_refresh_rate(device_name: &str) -> Result<f64, DisplayRefreshRateError> {
    use std::mem::zeroed;
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Graphics::Gdi::DEVMODEW;
    use windows_sys::Win32::Graphics::Gdi::{ENUM_CURRENT_SETTINGS, EnumDisplaySettingsExW};

    let device_wide = device_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: DEVMODEW is zero-initialized and dmSize is required by EnumDisplaySettingsExW.
    let mut mode: DEVMODEW = unsafe { zeroed() };
    mode.dmSize = core::mem::size_of::<DEVMODEW>() as u16;
    // SAFETY: device_wide is a null-terminated device name and mode is writable.
    let ok = unsafe {
        EnumDisplaySettingsExW(device_wide.as_ptr(), ENUM_CURRENT_SETTINGS, &mut mode, 0)
    };
    if ok == 0 {
        return Err(DisplayRefreshRateError::Api {
            api: "EnumDisplaySettingsExW",
            // SAFETY: GetLastError reads the calling thread error state and has no pointer requirements.
            code: unsafe { GetLastError() },
        });
    }

    normalize_display_refresh_rate(mode.dmDisplayFrequency)
        .ok_or(DisplayRefreshRateError::InvalidMode)
}

/// Detects the current refresh rate of the active display associated with process_id.
///
/// A foreground window owned by the target process is preferred. Otherwise, the process
/// must have visible top-level windows that resolve to one unambiguous monitor.
pub fn detect_process_display_refresh_rate(
    process_id: u32,
) -> Result<Option<DisplayRefreshRate>, DisplayRefreshRateError> {
    let windows = find_process_windows(process_id)?;
    let Some(selected_window) = select_process_window(process_id, &windows) else {
        return Ok(None);
    };
    let (selected_monitor, device_name) = monitor_device_name(selected_window)?;

    if windows.len() > 1 {
        for &window in &windows {
            let (monitor, _) = monitor_device_name(window)?;
            if monitor != selected_monitor {
                return Ok(None);
            }
        }
    }

    let hz = current_display_refresh_rate(&device_name)?;
    Ok(Some(DisplayRefreshRate { hz, device_name }))
}
/// Errors raised by Windows telemetry collection.
#[derive(Debug, thiserror::Error)]
pub enum SystemTelemetryError {
    /// QueryPerformanceFrequency returned failure.
    #[error("QueryPerformanceFrequency failed")]
    QpcFrequency,
    /// QueryPerformanceCounter returned failure.
    #[error("QueryPerformanceCounter failed")]
    QpcCounter,
    /// QPC scaling overflowed.
    #[error("QPC timestamp conversion overflowed")]
    QpcOverflow,
    /// QPC moved backwards relative to the sampler origin.
    #[error("QPC timestamp is non-monotonic")]
    NonMonotonicQpc,
    /// GetSystemTimes returned failure.
    #[error("GetSystemTimes failed")]
    SystemTimes,
    /// System CPU counters moved backwards.
    #[error("system CPU counters are non-monotonic")]
    NonMonotonicCpu,
    /// GlobalMemoryStatusEx returned failure.
    #[error("GlobalMemoryStatusEx failed")]
    MemoryStatus,
    /// The target process identifier was zero.
    #[error("target process ID must be greater than zero")]
    InvalidProcessId,
    /// Windows could not create a thread snapshot.
    #[error("failed to create a Windows thread snapshot")]
    ThreadSnapshot,
    /// Windows could not start thread enumeration.
    #[error("failed to enumerate Windows threads")]
    ThreadEnumeration,
    /// Windows could not create a process snapshot.
    #[error("failed to create a Windows process snapshot")]
    ProcessSnapshot,
    /// Windows could not start process enumeration.
    #[error("failed to enumerate Windows processes")]
    ProcessEnumeration,
    /// Windows could not deliver a console control event to the PresentMon process group.
    #[error("failed to request PresentMon process-group shutdown")]
    ConsoleControl,
}

#[derive(Debug, Clone, Copy)]
struct CpuTimes {
    idle: u64,
    kernel: u64,
    user: u64,
}

impl CpuTimes {
    fn from_file_times(idle: FILETIME, kernel: FILETIME, user: FILETIME) -> Self {
        Self {
            idle: filetime_to_u64(idle),
            kernel: filetime_to_u64(kernel),
            user: filetime_to_u64(user),
        }
    }
}

fn filetime_to_u64(value: FILETIME) -> u64 {
    (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime)
}

fn cpu_utilization(previous: CpuTimes, current: CpuTimes) -> Result<f64, SystemTelemetryError> {
    if current.idle < previous.idle
        || current.kernel < previous.kernel
        || current.user < previous.user
    {
        return Err(SystemTelemetryError::NonMonotonicCpu);
    }

    let idle_delta = current.idle - previous.idle;
    let kernel_delta = current.kernel - previous.kernel;
    let user_delta = current.user - previous.user;
    let total_delta = kernel_delta.saturating_add(user_delta);

    if total_delta == 0 {
        return Ok(0.0);
    }

    let busy_delta = total_delta.saturating_sub(idle_delta);
    Ok((busy_delta as f64 / total_delta as f64 * 100.0).clamp(0.0, 100.0))
}

/// Shared QPC timebase used to align independent Windows telemetry producers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QpcTimebase {
    frequency_hz: u64,
    origin_qpc: u64,
}

impl SystemTelemetryError {
    /// Returns whether the failure can be downgraded to an absent telemetry observation.
    pub const fn is_non_essential(&self) -> bool {
        !matches!(self, Self::QpcFrequency | Self::InvalidProcessId)
    }
}

impl QpcTimebase {
    /// Creates a timebase from the current Windows performance counter.
    pub fn new() -> Result<Self, SystemTelemetryError> {
        let mut frequency = 0_i64;
        // SAFETY: `frequency` is a valid writable output pointer and the Windows API writes one i64 value.
        let ok = unsafe { QueryPerformanceFrequency(&mut frequency) };
        if ok == 0 || frequency <= 0 {
            return Err(SystemTelemetryError::QpcFrequency);
        }

        let mut origin = 0_i64;
        // SAFETY: `origin` is a valid writable output pointer and the Windows API writes one i64 value.
        let ok = unsafe { QueryPerformanceCounter(&mut origin) };
        if ok == 0 || origin < 0 {
            return Err(SystemTelemetryError::QpcCounter);
        }

        Ok(Self {
            frequency_hz: frequency as u64,
            origin_qpc: origin as u64,
        })
    }

    /// Returns the performance-counter frequency in ticks per second.
    pub const fn frequency_hz(self) -> u64 {
        self.frequency_hz
    }

    /// Returns the absolute QPC origin used for capture-relative timestamps.
    pub const fn origin_qpc(self) -> u64 {
        self.origin_qpc
    }

    /// Converts an absolute QPC value into capture-relative nanoseconds.
    pub fn relative_ns(self, qpc: u64) -> Result<u64, SystemTelemetryError> {
        if qpc < self.origin_qpc {
            return Err(SystemTelemetryError::NonMonotonicQpc);
        }

        let delta = qpc - self.origin_qpc;
        u128::from(delta)
            .checked_mul(1_000_000_000)
            .and_then(|value| value.checked_div(u128::from(self.frequency_hz)))
            .and_then(|value| u64::try_from(value).ok())
            .ok_or(SystemTelemetryError::QpcOverflow)
    }
}

struct QpcClock {
    timebase: QpcTimebase,
}

impl QpcClock {
    fn new() -> Result<Self, SystemTelemetryError> {
        Ok(Self {
            timebase: QpcTimebase::new()?,
        })
    }

    fn from_timebase(timebase: QpcTimebase) -> Self {
        Self { timebase }
    }

    fn frequency_hz(&self) -> u64 {
        self.timebase.frequency_hz()
    }

    fn origin_qpc(&self) -> u64 {
        self.timebase.origin_qpc()
    }

    fn now_ns(&self) -> Result<u64, SystemTelemetryError> {
        let mut counter = 0_i64;
        // SAFETY: `counter` is a valid writable output pointer and the Windows API writes one i64 value.
        let ok = unsafe { QueryPerformanceCounter(&mut counter) };
        if ok == 0 || counter < 0 {
            return Err(SystemTelemetryError::QpcCounter);
        }
        self.timebase.relative_ns(counter as u64)
    }
}

struct NvidiaTelemetry {
    nvml: Nvml,
    device_index: u32,
}

impl NvidiaTelemetry {
    fn for_process(process_id: u32) -> Option<Self> {
        let nvml = Nvml::init().ok()?;
        let device_count = nvml.device_count().ok()?;
        let mut matching_device = None;

        for device_index in 0..device_count {
            let device = nvml.device_by_index(device_index).ok()?;
            let uses_device = device
                .running_graphics_processes()
                .ok()?
                .into_iter()
                .any(|process| process.pid == process_id);
            if uses_device {
                if matching_device.is_some() {
                    return None;
                }
                matching_device = Some(device_index);
            }
        }

        matching_device.map(|device_index| Self { nvml, device_index })
    }

    fn provenance(&self) -> (Option<String>, Option<String>) {
        let gpu = self
            .nvml
            .device_by_index(self.device_index)
            .ok()
            .and_then(|device| device.name().ok());
        let driver = self.nvml.sys_driver_version().ok();
        (gpu, driver)
    }

    fn augment(&self, sample: &mut SystemSample, include_gpu_utilization: bool) {
        let Ok(device) = self.nvml.device_by_index(self.device_index) else {
            return;
        };

        if include_gpu_utilization && let Ok(utilization) = device.utilization_rates() {
            sample.gpu_utilization = Some(f64::from(utilization.gpu));
        }
        if let Ok(memory) = device.memory_info() {
            sample.vram_used_mb = Some(memory.used / (1024 * 1024));
            if memory.total > 0 {
                sample.vram_utilization =
                    Some((memory.used as f64 / memory.total as f64 * 100.0).clamp(0.0, 100.0));
            }
        }
        if let Ok(clock) = device.clock_info(Clock::Graphics) {
            sample.gpu_clock_mhz = Some(u64::from(clock));
        }
        if let Ok(power) = device.power_usage() {
            sample.gpu_power_w = Some(f64::from(power) / 1000.0);
        }
        if let Ok(temperature) = device.temperature(TemperatureSensor::Gpu) {
            sample.gpu_temperature_c = Some(f64::from(temperature));
        }
    }
}

/// Number of recent samples retained for the rolling p99 sampler metric.
const SAMPLER_STATS_WINDOW: usize = 4096;

/// Runtime sampler overhead statistics collected without retaining every sample duration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplerStats {
    sample_count: u64,
    total_duration_ns: u128,
    recent_durations_ns: Vec<u64>,
    recent_next: usize,
}

impl Default for SamplerStats {
    fn default() -> Self {
        Self {
            sample_count: 0,
            total_duration_ns: 0,
            recent_durations_ns: Vec::with_capacity(SAMPLER_STATS_WINDOW),
            recent_next: 0,
        }
    }
}

impl SamplerStats {
    fn record(&mut self, duration_ns: u64) {
        self.sample_count = self.sample_count.saturating_add(1);
        self.total_duration_ns = self
            .total_duration_ns
            .saturating_add(u128::from(duration_ns));
        if self.recent_durations_ns.len() < SAMPLER_STATS_WINDOW {
            self.recent_durations_ns.push(duration_ns);
            self.recent_next = self.recent_durations_ns.len() % SAMPLER_STATS_WINDOW;
        } else {
            self.recent_durations_ns[self.recent_next] = duration_ns;
            self.recent_next = (self.recent_next + 1) % SAMPLER_STATS_WINDOW;
        }
    }

    /// Number of successfully completed sampler invocations recorded.
    pub const fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// Mean sampler duration in milliseconds over all recorded successful samples.
    pub fn mean_duration_ms(&self) -> Option<f64> {
        (self.sample_count > 0)
            .then(|| self.total_duration_ns as f64 / self.sample_count as f64 / 1_000_000.0)
    }

    /// Rolling p99 sampler duration in milliseconds over the most recent 4096 samples.
    pub fn p99_duration_ms(&self) -> Option<f64> {
        if self.recent_durations_ns.is_empty() {
            return None;
        }
        let mut durations = self.recent_durations_ns.clone();
        durations.sort_unstable();
        let rank = ((durations.len() as f64 * 0.99).ceil() as usize).max(1) - 1;
        Some(durations[rank] as f64 / 1_000_000.0)
    }

    /// Returns the number of durations retained in the rolling p99 window.
    pub const fn rolling_window_size(&self) -> usize {
        self.recent_durations_ns.len()
    }
}

/// Low-overhead Windows system sampler producing normalized SystemSample values.
pub struct WindowsSystemSampler {
    clock: QpcClock,
    previous_cpu: Option<CpuTimes>,
    nvidia: Option<NvidiaTelemetry>,
    gpu_engine: Option<PdhGpuSampler>,
    last_gpu_engine_timestamp_ns: Option<u64>,
    target_process_id: Option<u32>,
    process_probing_enabled: bool,
    previous_threads: HashMap<u32, ThreadCpuTimes>,
    previous_thread_timestamp_ns: Option<u64>,
    previous_processes: HashMap<u32, ProcessCpuTimes>,
    activity_ids: ActivityIdAllocator,
    stats: SamplerStats,
}

impl WindowsSystemSampler {
    /// Creates a sampler and establishes a QPC origin. NVIDIA telemetry is optional.
    pub fn new() -> Result<Self, SystemTelemetryError> {
        let clock = QpcClock::new()?;
        let nvidia: Option<NvidiaTelemetry> = None;
        Ok(Self {
            clock,
            previous_cpu: None,
            nvidia,
            gpu_engine: None,
            last_gpu_engine_timestamp_ns: None,
            target_process_id: None,
            process_probing_enabled: true,
            previous_threads: HashMap::new(),
            previous_thread_timestamp_ns: None,
            previous_processes: HashMap::new(),
            activity_ids: ActivityIdAllocator::default(),
            stats: SamplerStats::default(),
        })
    }

    /// Creates a sampler using an explicit shared QPC timebase.
    pub fn new_with_timebase(timebase: QpcTimebase) -> Result<Self, SystemTelemetryError> {
        let nvidia = None;
        Ok(Self {
            clock: QpcClock::from_timebase(timebase),
            previous_cpu: None,
            nvidia,
            gpu_engine: None,
            last_gpu_engine_timestamp_ns: None,
            target_process_id: None,
            process_probing_enabled: true,
            previous_threads: HashMap::new(),
            previous_thread_timestamp_ns: None,
            previous_processes: HashMap::new(),
            activity_ids: ActivityIdAllocator::default(),
            stats: SamplerStats::default(),
        })
    }

    /// Creates a sampler that additionally tracks CPU time for one target process.
    pub fn new_for_process(process_id: u32) -> Result<Self, SystemTelemetryError> {
        if process_id == 0 {
            return Err(SystemTelemetryError::InvalidProcessId);
        }
        let mut sampler = Self::new()?;
        sampler.target_process_id = Some(process_id);
        sampler.nvidia = NvidiaTelemetry::for_process(process_id);
        sampler.gpu_engine = PdhGpuSampler::new().ok();
        Ok(sampler)
    }

    /// Creates a target-process sampler using an explicit shared QPC timebase.
    pub fn new_for_process_with_timebase(
        process_id: u32,
        timebase: QpcTimebase,
    ) -> Result<Self, SystemTelemetryError> {
        if process_id == 0 {
            return Err(SystemTelemetryError::InvalidProcessId);
        }
        let mut sampler = Self::new_with_timebase(timebase)?;
        sampler.target_process_id = Some(process_id);
        sampler.nvidia = NvidiaTelemetry::for_process(process_id);
        sampler.gpu_engine = PdhGpuSampler::new().ok();
        Ok(sampler)
    }

    /// Creates a sampler that collects only system-wide CPU and memory telemetry.
    ///
    /// This mode does not enumerate processes or threads and does not query GPU telemetry.
    pub fn new_system_only_with_timebase(
        timebase: QpcTimebase,
    ) -> Result<Self, SystemTelemetryError> {
        let mut sampler = Self::new_with_timebase(timebase)?;
        sampler.process_probing_enabled = false;
        Ok(sampler)
    }

    /// Returns the target process identifier, when thread telemetry is enabled.
    pub fn target_process_id(&self) -> Option<u32> {
        self.target_process_id
    }

    /// Returns a snapshot of measured sampler overhead statistics.
    pub fn stats(&self) -> SamplerStats {
        self.stats.clone()
    }

    /// Returns the authoritative QPC frequency used by this sampler.
    pub fn qpc_frequency_hz(&self) -> u64 {
        self.clock.frequency_hz()
    }

    /// Returns best-effort hardware identity observed by the sampler.
    pub fn hardware_provenance(&self) -> (Option<String>, Option<String>, Option<String>) {
        let cpu = std::env::var("PROCESSOR_IDENTIFIER").ok();
        let (gpu, driver) = self
            .nvidia
            .as_ref()
            .map(NvidiaTelemetry::provenance)
            .unwrap_or((None, None));
        (cpu, gpu, driver)
    }

    /// Returns the shared QPC timebase used by this sampler.
    pub fn qpc_timebase(&self) -> QpcTimebase {
        QpcTimebase {
            frequency_hz: self.clock.frequency_hz(),
            origin_qpc: self.clock.origin_qpc(),
        }
    }

    /// Captures one timestamped normalized Windows system observation.
    pub fn sample(&mut self) -> Result<SystemSample, SystemTelemetryError> {
        let started = Instant::now();
        let timestamp_ns = self.clock.now_ns()?;

        let mut idle = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut kernel = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut user = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };

        // SAFETY: all three FILETIME values are valid writable outputs owned by this stack frame.
        let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
        if ok == 0 {
            return Err(SystemTelemetryError::SystemTimes);
        }

        let current_cpu = CpuTimes::from_file_times(idle, kernel, user);
        let cpu_utilization = self
            .previous_cpu
            .map(|previous| cpu_utilization(previous, current_cpu))
            .transpose()?;
        self.previous_cpu = Some(current_cpu);

        let mut memory = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            dwMemoryLoad: 0,
            ullAvailExtendedVirtual: 0,
            ullTotalPhys: 0,
            ullAvailPhys: 0,
            ullTotalPageFile: 0,
            ullAvailPageFile: 0,
            ullTotalVirtual: 0,
            ullAvailVirtual: 0,
        };
        // SAFETY: `memory.dwLength` is initialized as required and `memory` is a valid writable output structure.
        let memory_ok = unsafe { GlobalMemoryStatusEx(&mut memory) };
        if memory_ok == 0 {
            return Err(SystemTelemetryError::MemoryStatus);
        }

        let thread_sample_start_ns = self.clock.now_ns()?;
        let wall_elapsed_ns = self
            .previous_thread_timestamp_ns
            .map(|previous| thread_sample_start_ns.saturating_sub(previous));
        let (process_threads, current_threads) = match self.target_process_id {
            Some(process_id) => {
                sample_process_threads(process_id, &self.previous_threads, wall_elapsed_ns)?
            }
            None => (Vec::new(), HashMap::new()),
        };
        self.previous_threads = current_threads;
        self.previous_thread_timestamp_ns = Some(self.clock.now_ns()?);

        let (background_activity, current_processes) = if self.process_probing_enabled {
            sample_background_processes(
                self.target_process_id,
                &self.previous_processes,
                &mut self.activity_ids,
                wall_elapsed_ns,
            )?
        } else {
            (Vec::new(), HashMap::new())
        };
        self.previous_processes = current_processes;

        let mut sample = SystemSample::new(timestamp_ns);
        sample.process_threads = process_threads;
        sample.background_activity = background_activity;
        sample.cpu_utilization = cpu_utilization;
        let used_phys = memory.ullTotalPhys.saturating_sub(memory.ullAvailPhys);
        sample.ram_used_mb = Some(used_phys / (1024 * 1024));
        if memory.ullTotalPhys > 0 {
            sample.ram_utilization =
                Some((used_phys as f64 / memory.ullTotalPhys as f64 * 100.0).clamp(0.0, 100.0));
        }

        if let Some(process_id) = self.target_process_id {
            const GPU_ENGINE_MIN_INTERVAL_NS: u64 = 1_000_000_000;
            let due = self.last_gpu_engine_timestamp_ns.is_none_or(|previous| {
                timestamp_ns.saturating_sub(previous) >= GPU_ENGINE_MIN_INTERVAL_NS
            });
            if due {
                self.last_gpu_engine_timestamp_ns = Some(timestamp_ns);
                if let Some(gpu_engine) = &mut self.gpu_engine
                    && let Ok(Some(telemetry)) = gpu_engine.sample(process_id)
                {
                    sample.gpu_utilization = Some(telemetry.utilization_percent);
                }
            }
        }

        if let Some(nvidia) = &self.nvidia {
            nvidia.augment(&mut sample, self.target_process_id.is_none());
        }

        self.stats
            .record(started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64);
        Ok(sample)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CpuTimes, PresentMonJob, PresentMonVersion, SAMPLER_STATS_WINDOW, SamplerStats,
        cpu_utilization, normalize_display_refresh_rate,
    };

    #[test]
    fn sampler_stats_report_mean_and_rolling_p99() {
        let mut stats = SamplerStats::default();
        stats.record(1_000_000);
        stats.record(2_000_000);
        stats.record(100_000_000);

        assert_eq!(stats.sample_count(), 3);
        assert_eq!(stats.rolling_window_size(), 3);
        assert_eq!(stats.mean_duration_ms(), Some(34.333333333333336));
        assert_eq!(stats.p99_duration_ms(), Some(100.0));
    }

    #[test]
    fn sampler_stats_window_is_bounded() {
        let mut stats = SamplerStats::default();
        for value in 1..=SAMPLER_STATS_WINDOW + 17 {
            stats.record(value as u64);
        }
        assert_eq!(stats.sample_count(), (SAMPLER_STATS_WINDOW + 17) as u64);
        assert_eq!(stats.rolling_window_size(), SAMPLER_STATS_WINDOW);
    }

    #[test]
    fn console_detection_and_attach_error_report_real_win32_codes() {
        // Test binaries run under a console, so the detector must say so.
        // Attaching while attached must then refuse with ERROR_ACCESS_DENIED
        // (5) against our own PID; this pins the raw-code error mapping
        // instead of asserting on a stubbed error.
        assert!(super::caller_has_console());
        match super::send_break_to_detached_group(std::process::id()) {
            Err(super::DetachedBreakError::AttachFailed { code }) => assert_eq!(code, 5),
            other => panic!("expected AttachFailed, got {other:?}"),
        }
    }

    #[test]
    fn shared_timebase_constructor_does_not_probe_nvidia_without_target_process() {
        let timebase = super::QpcTimebase {
            frequency_hz: 1_000_000,
            origin_qpc: 0,
        };
        let sampler = match super::WindowsSystemSampler::new_with_timebase(timebase) {
            Ok(sampler) => sampler,
            Err(error) => panic!("unexpected constructor failure: {error}"),
        };
        assert_eq!(sampler.target_process_id(), None);
        assert!(sampler.process_probing_enabled);
    }

    #[test]
    fn system_only_sampler_disables_process_probing() {
        let timebase = super::QpcTimebase {
            frequency_hz: 1_000_000,
            origin_qpc: 0,
        };
        let sampler = match super::WindowsSystemSampler::new_system_only_with_timebase(timebase) {
            Ok(sampler) => sampler,
            Err(error) => panic!("unexpected constructor failure: {error}"),
        };
        assert_eq!(sampler.target_process_id(), None);
        assert!(!sampler.process_probing_enabled);
    }

    #[test]
    fn system_only_sample_keeps_system_telemetry_and_omits_process_gpu_data()
    -> Result<(), super::SystemTelemetryError> {
        let timebase = super::QpcTimebase {
            frequency_hz: 1_000_000,
            origin_qpc: 0,
        };
        let mut sampler = super::WindowsSystemSampler::new_system_only_with_timebase(timebase)?;
        let sample = sampler.sample()?;
        assert!(sample.process_threads.is_empty());
        assert!(sample.background_activity.is_empty());
        assert!(sample.cpu_utilization.is_none());
        assert!(sample.ram_used_mb.is_some());
        assert!(sample.gpu_utilization.is_none());
        assert!(sample.gpu_temperature_c.is_none());
        assert!(sample.vram_used_mb.is_none());
        Ok(())
    }

    #[test]
    fn supports_only_presentmon_v2_major_versions() {
        assert!(
            PresentMonVersion {
                major: 2,
                minor: 6,
                patch: 0,
                build: 0,
            }
            .is_supported()
        );
        assert!(
            !PresentMonVersion {
                major: 1,
                minor: 9,
                patch: 0,
                build: 0,
            }
            .is_supported()
        );
    }

    #[test]
    fn unknown_version_maps_only_resource_type_not_found() {
        assert_eq!(
            super::version_api_error("GetFileVersionInfoSizeW", 1813),
            super::PresentMonVersionError::UnknownVersion {
                api: "GetFileVersionInfoSizeW",
                code: 1813,
            }
        );
        assert_eq!(
            super::version_api_error("GetFileVersionInfoW", 1813),
            super::PresentMonVersionError::UnknownVersion {
                api: "GetFileVersionInfoW",
                code: 1813,
            }
        );
        // 1812 (non-executable input) and unrelated codes stay hard errors.
        assert!(matches!(
            super::version_api_error("GetFileVersionInfoSizeW", 1812),
            super::PresentMonVersionError::Api { .. }
        ));
        assert!(matches!(
            super::version_api_error("GetFileVersionInfoSizeW", 2),
            super::PresentMonVersionError::Api { .. }
        ));
    }

    #[test]
    fn versionless_pe_reports_unknown_version() -> Result<(), Box<dyn std::error::Error>> {
        // Cargo-built executables carry no version resource under the pinned
        // toolchain (verified via FileVersionInfo on target/debug/rwp.exe),
        // so the test binary itself is a genuine versionless PE.
        let path = std::env::current_exe()?;
        match super::detect_presentmon_version(&path) {
            Err(super::PresentMonVersionError::UnknownVersion { api, code }) => {
                assert_eq!(api, "GetFileVersionInfoSizeW");
                assert_eq!(code, super::ERROR_RESOURCE_TYPE_NOT_FOUND);
            }
            other => panic!("versionless PE must report UnknownVersion, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn non_executable_file_stays_hard_error() -> Result<(), Box<dyn std::error::Error>> {
        let path =
            std::env::temp_dir().join(format!("rwp-versionless-probe-{}.exe", std::process::id()));
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, b"synthetic file without a version resource")?;
        let result = super::detect_presentmon_version(&path);
        let _ = std::fs::remove_file(&path);
        match result {
            Err(super::PresentMonVersionError::Api { code, .. }) => {
                assert_ne!(code, super::ERROR_RESOURCE_TYPE_NOT_FOUND);
            }
            other => panic!("non-executable input must stay a hard Api error, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn unknown_and_unsupported_version_errors_differ() {
        let unknown = super::PresentMonVersionError::UnknownVersion {
            api: "GetFileVersionInfoSizeW",
            code: super::ERROR_RESOURCE_TYPE_NOT_FOUND,
        };
        assert!(unknown.to_string().contains("unknown"));
        assert!(!unknown.to_string().contains("unsupported"));
        let unsupported = super::PresentMonVersionError::UnsupportedVersion {
            detected: super::PresentMonVersion {
                major: 1,
                minor: 9,
                patch: 0,
                build: 0,
            },
        };
        assert!(unsupported.to_string().contains("unsupported"));
    }

    #[test]
    fn normalizes_display_refresh_rate() {
        assert_eq!(normalize_display_refresh_rate(0), None);
        assert_eq!(normalize_display_refresh_rate(1), None);
        assert_eq!(normalize_display_refresh_rate(60), Some(60.0));
        assert_eq!(normalize_display_refresh_rate(240), Some(240.0));
    }

    #[test]
    fn computes_busy_cpu_percentage() -> Result<(), super::SystemTelemetryError> {
        let previous = CpuTimes {
            idle: 20,
            kernel: 100,
            user: 80,
        };
        let current = CpuTimes {
            idle: 30,
            kernel: 140,
            user: 90,
        };
        let utilization = cpu_utilization(previous, current)?;
        assert!((utilization - 80.0).abs() < 0.001);
        Ok(())
    }

    #[test]
    fn zero_interval_is_zero_utilization() -> Result<(), super::SystemTelemetryError> {
        let times = CpuTimes {
            idle: 10,
            kernel: 20,
            user: 30,
        };
        assert_eq!(cpu_utilization(times, times)?, 0.0);
        Ok(())
    }

    #[test]
    fn authenticode_accepts_signed_system_binary() {
        assert_eq!(
            super::verify_presentmon_authenticode(std::path::Path::new(
                r"C:\Windows\System32\kernel32.dll"
            )),
            super::AuthenticodeStatus::Signed
        );
    }

    #[test]
    fn authenticode_reports_unsigned_and_missing_files() -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!(
            "rwp-authenticode-unsigned-{}.bin",
            std::process::id()
        ));
        std::fs::write(&path, [0x4Du8, 0x5A, 0x00, 0x01])?;
        assert_eq!(
            super::verify_presentmon_authenticode(&path),
            super::AuthenticodeStatus::NotSigned
        );
        std::fs::remove_file(&path)?;
        assert!(matches!(
            super::verify_presentmon_authenticode(&path),
            super::AuthenticodeStatus::VerifyFailed { .. }
        ));
        Ok(())
    }

    /// Live runtime proof that the kill-on-close Job Object terminates its
    /// child when the last job handle closes. Uses a benign `timeout` sleeper
    /// so no PresentMon binary is required; runs unprivileged.
    #[test]
    fn job_object_kill_on_close_terminates_child() -> Result<(), Box<dyn std::error::Error>> {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut child = Command::new("cmd")
            .args(["/C", "timeout", "/t", "30", "/nobreak"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
        assert!(
            child.try_wait()?.is_none(),
            "sleeper child must be alive before the job closes"
        );

        let job = PresentMonJob::assign_process(&child)?;
        drop(job);

        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if child.try_wait()?.is_some() {
                return Ok(());
            }
            if Instant::now() > deadline {
                return Err("child survived job close past the 10 s deadline".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
