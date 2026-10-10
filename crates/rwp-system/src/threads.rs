//! Windows target-process thread CPU telemetry.

#![allow(unsafe_code)]

use std::collections::HashMap;

use rwp_core::ProcessThreadSample;
use windows_sys::Win32::{
    Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
        },
        Threading::{GetThreadTimes, OpenThread, THREAD_QUERY_LIMITED_INFORMATION},
    },
};

const THREAD_QUERY_ACCESS: u32 = THREAD_QUERY_LIMITED_INFORMATION;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ThreadCpuTimes {
    pub(crate) kernel: u64,
    pub(crate) user: u64,
}

impl ThreadCpuTimes {
    fn from_file_times(kernel: FILETIME, user: FILETIME) -> Self {
        Self {
            kernel: filetime_to_u64(kernel),
            user: filetime_to_u64(user),
        }
    }
}

fn filetime_to_u64(value: FILETIME) -> u64 {
    (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime)
}

pub(crate) fn thread_utilization(
    previous: ThreadCpuTimes,
    current: ThreadCpuTimes,
    wall_elapsed_ns: u64,
) -> Option<f64> {
    if wall_elapsed_ns == 0 || current.kernel < previous.kernel || current.user < previous.user {
        return None;
    }

    let cpu_100ns = u128::from(current.kernel - previous.kernel)
        .checked_add(u128::from(current.user - previous.user))?;
    let cpu_ns = cpu_100ns.checked_mul(100)?;
    let utilization = cpu_ns as f64 / wall_elapsed_ns as f64 * 100.0;
    Some(utilization.clamp(0.0, 100.0))
}

pub(crate) fn sample_process_threads(
    process_id: u32,
    previous: &HashMap<u32, ThreadCpuTimes>,
    wall_elapsed_ns: Option<u64>,
) -> Result<
    (Vec<ProcessThreadSample>, HashMap<u32, ThreadCpuTimes>),
    crate::windows::SystemTelemetryError,
> {
    // SAFETY: Windows allocates an owned thread snapshot handle; no pointers are passed from Rust memory.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot.is_null() || snapshot == INVALID_HANDLE_VALUE {
        return Err(crate::windows::SystemTelemetryError::ThreadSnapshot);
    }

    let result = enumerate_threads(snapshot, process_id, previous, wall_elapsed_ns);
    // SAFETY: snapshot was returned by CreateToolhelp32Snapshot and remains owned here.
    let _ = unsafe { CloseHandle(snapshot) };
    result
}

fn enumerate_threads(
    snapshot: windows_sys::Win32::Foundation::HANDLE,
    process_id: u32,
    previous: &HashMap<u32, ThreadCpuTimes>,
    wall_elapsed_ns: Option<u64>,
) -> Result<
    (Vec<ProcessThreadSample>, HashMap<u32, ThreadCpuTimes>),
    crate::windows::SystemTelemetryError,
> {
    let mut entry = THREADENTRY32 {
        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    // SAFETY: snapshot is a valid thread snapshot and entry is initialized with its required size.
    let first = unsafe { Thread32First(snapshot, &mut entry) };
    if first == 0 {
        return Err(crate::windows::SystemTelemetryError::ThreadEnumeration);
    }

    let mut current = HashMap::new();
    let mut samples = Vec::new();

    loop {
        if entry.th32OwnerProcessID == process_id {
            let thread_id = entry.th32ThreadID;
            if let Some(cpu) = read_thread_times(thread_id) {
                current.insert(thread_id, cpu);
                if let (Some(previous_cpu), Some(wall_ns)) =
                    (previous.get(&thread_id).copied(), wall_elapsed_ns)
                    && let Some(utilization) = thread_utilization(previous_cpu, cpu, wall_ns)
                    && let Ok(sample) = ProcessThreadSample::new(thread_id, utilization)
                {
                    samples.push(sample);
                }
            }
        }

        // SAFETY: snapshot is a valid thread snapshot and entry remains valid for enumeration.
        let next = unsafe { Thread32Next(snapshot, &mut entry) };
        if next == 0 {
            break;
        }
    }

    samples.sort_by_key(|sample| sample.thread_id);
    Ok((samples, current))
}

fn read_thread_times(thread_id: u32) -> Option<ThreadCpuTimes> {
    // SAFETY: the thread ID comes from ToolHelp enumeration and no Rust pointer is passed to OpenThread.
    let handle = unsafe { OpenThread(THREAD_QUERY_ACCESS, 0, thread_id) };
    if handle.is_null() {
        return None;
    }

    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: handle is valid while this call executes and all FILETIME outputs are valid mutable storage.
    let ok = unsafe { GetThreadTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) };
    // SAFETY: handle was returned by OpenThread and remains owned by this function.
    let _ = unsafe { CloseHandle(handle) };
    if ok == 0 {
        return None;
    }

    Some(ThreadCpuTimes::from_file_times(kernel, user))
}

#[cfg(test)]
mod tests {
    use super::{
        THREAD_QUERY_ACCESS, THREAD_QUERY_LIMITED_INFORMATION, ThreadCpuTimes, thread_utilization,
    };

    #[test]
    fn computes_one_thread_as_eighty_percent() {
        let previous = ThreadCpuTimes {
            kernel: 10,
            user: 10,
        };
        let current = ThreadCpuTimes {
            kernel: 20,
            user: 20,
        };
        let utilization = match thread_utilization(previous, current, 2_500) {
            Some(value) => value,
            None => return,
        };
        assert!((utilization - 80.0).abs() < 0.001);
    }

    #[test]
    fn uses_limited_thread_query_access() {
        assert_eq!(THREAD_QUERY_ACCESS, THREAD_QUERY_LIMITED_INFORMATION);
    }

    #[test]
    fn rejects_counter_rollback() {
        let previous = ThreadCpuTimes {
            kernel: 20,
            user: 10,
        };
        let current = ThreadCpuTimes {
            kernel: 19,
            user: 10,
        };
        assert_eq!(thread_utilization(previous, current, 1_000), None);
    }
}
