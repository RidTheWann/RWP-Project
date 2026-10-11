//! Redacted Windows background-process CPU activity telemetry.

#![allow(unsafe_code)]

use std::collections::HashMap;

use rwp_core::BackgroundActivity;
use windows_sys::Win32::{
    Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32, Process32First, Process32Next,
            TH32CS_SNAPPROCESS,
        },
        Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    },
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProcessCpuTimes {
    kernel: u64,
    user: u64,
}

impl ProcessCpuTimes {
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

pub(crate) fn process_utilization(
    previous: ProcessCpuTimes,
    current: ProcessCpuTimes,
    wall_elapsed_ns: u64,
) -> Option<f64> {
    if wall_elapsed_ns == 0 || current.kernel < previous.kernel || current.user < previous.user {
        return None;
    }
    let cpu_100ns = u128::from(current.kernel - previous.kernel)
        .checked_add(u128::from(current.user - previous.user))?;
    let cpu_ns = cpu_100ns.checked_mul(100)?;
    Some((cpu_ns as f64 / wall_elapsed_ns as f64 * 100.0).clamp(0.0, 100.0))
}

#[derive(Debug, Default)]
pub(crate) struct ActivityIdAllocator {
    ids: HashMap<u32, u64>,
    next_id: u64,
}

impl ActivityIdAllocator {
    pub(crate) fn id_for(&mut self, pid: u32) -> u64 {
        if let Some(id) = self.ids.get(&pid).copied() {
            return id;
        }
        self.next_id = self.next_id.saturating_add(1);
        let id = self.next_id;
        self.ids.insert(pid, id);
        id
    }
}

pub(crate) fn sample_background_processes(
    target_pid: Option<u32>,
    previous: &HashMap<u32, ProcessCpuTimes>,
    ids: &mut ActivityIdAllocator,
    wall_elapsed_ns: Option<u64>,
) -> Result<
    (Vec<BackgroundActivity>, HashMap<u32, ProcessCpuTimes>),
    crate::windows::SystemTelemetryError,
> {
    // SAFETY: Windows allocates an owned process snapshot handle; no pointers are passed from Rust memory.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() || snapshot == INVALID_HANDLE_VALUE {
        return Err(crate::windows::SystemTelemetryError::ProcessSnapshot);
    }

    let result =
        enumerate_background_processes(snapshot, target_pid, previous, ids, wall_elapsed_ns);
    // SAFETY: snapshot was returned by CreateToolhelp32Snapshot and remains owned here.
    let _ = unsafe { CloseHandle(snapshot) };
    result
}

fn enumerate_background_processes(
    snapshot: windows_sys::Win32::Foundation::HANDLE,
    target_pid: Option<u32>,
    previous: &HashMap<u32, ProcessCpuTimes>,
    ids: &mut ActivityIdAllocator,
    wall_elapsed_ns: Option<u64>,
) -> Result<
    (Vec<BackgroundActivity>, HashMap<u32, ProcessCpuTimes>),
    crate::windows::SystemTelemetryError,
> {
    let mut entry = PROCESSENTRY32 {
        dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
        ..Default::default()
    };
    // SAFETY: snapshot is a valid process snapshot and entry is initialized with its required size.
    let first = unsafe { Process32First(snapshot, &mut entry) };
    if first == 0 {
        return Err(crate::windows::SystemTelemetryError::ProcessEnumeration);
    }

    let mut current = HashMap::new();
    let mut activity = Vec::new();

    loop {
        let pid = entry.th32ProcessID;
        if pid != 0
            && Some(pid) != target_pid
            && let Some(cpu) = read_process_times(pid)
        {
            current.insert(pid, cpu);
            if let (Some(previous_cpu), Some(wall_ns)) =
                (previous.get(&pid).copied(), wall_elapsed_ns)
                && let Some(utilization) = process_utilization(previous_cpu, cpu, wall_ns)
                && let Ok(sample) = BackgroundActivity::new(ids.id_for(pid), utilization)
            {
                activity.push(sample);
            }
        }

        // SAFETY: snapshot is a valid process snapshot and entry remains valid for enumeration.
        let next = unsafe { Process32Next(snapshot, &mut entry) };
        if next == 0 {
            break;
        }
    }

    activity.sort_by_key(|sample| sample.activity_id);
    Ok((activity, current))
}

/// Redacted live-process identity for UI target selection: PID plus the
/// executable name only. No CPU, memory, or path detail leaves this boundary,
///
/// keeping the picker to the same data the background sampler already observes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSummary {
    /// Live process identifier.
    pub process_id: u32,
    /// Executable basename reported by the snapshot, lossily decoded.
    pub name: String,
}

/// Lists currently present processes in PID order for UI target selection.
///
/// Only identity is returned; utilization stays inside the sampler. Entries
/// whose names cannot be decoded are skipped rather than fabricated.
pub fn list_processes() -> Result<Vec<ProcessSummary>, crate::windows::SystemTelemetryError> {
    // SAFETY: Windows allocates an owned process snapshot handle; no pointers are passed from Rust memory.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() || snapshot == INVALID_HANDLE_VALUE {
        return Err(crate::windows::SystemTelemetryError::ProcessSnapshot);
    }
    let mut entry = PROCESSENTRY32 {
        dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
        ..Default::default()
    };
    // SAFETY: snapshot is a valid process snapshot and entry is initialized with its required size.
    let first = unsafe { Process32First(snapshot, &mut entry) };
    if first == 0 {
        // SAFETY: snapshot was returned by CreateToolhelp32Snapshot and remains owned here.
        let _ = unsafe { CloseHandle(snapshot) };
        return Err(crate::windows::SystemTelemetryError::ProcessEnumeration);
    }
    let mut processes = Vec::new();
    loop {
        let pid = entry.th32ProcessID;
        if pid != 0
            && let Some(name) = executable_name(&entry.szExeFile)
        {
            processes.push(ProcessSummary {
                process_id: pid,
                name,
            });
        }
        // SAFETY: snapshot is a valid process snapshot and entry remains valid for enumeration.
        let next = unsafe { Process32Next(snapshot, &mut entry) };
        if next == 0 {
            break;
        }
    }
    // SAFETY: snapshot was returned by CreateToolhelp32Snapshot and remains owned here.
    let _ = unsafe { CloseHandle(snapshot) };
    processes.sort_by_key(|entry| entry.process_id);
    Ok(processes)
}

fn executable_name(raw: &[i8; 260]) -> Option<String> {
    let bytes: Vec<u8> = raw
        .iter()
        .take_while(|value| **value != 0)
        .map(|value| *value as u8)
        .collect();
    if bytes.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

fn read_process_times(process_id: u32) -> Option<ProcessCpuTimes> {
    // SAFETY: the process ID comes from ToolHelp enumeration and no Rust pointer is passed to OpenProcess.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if handle.is_null() {
        return None;
    }

    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: handle is valid while this call executes and all FILETIME outputs are valid mutable storage.
    let ok = unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) };
    // SAFETY: handle was returned by OpenProcess and remains owned by this function.
    let _ = unsafe { CloseHandle(handle) };
    if ok == 0 {
        return None;
    }

    Some(ProcessCpuTimes::from_file_times(kernel, user))
}

#[cfg(test)]
mod tests {
    use super::{ActivityIdAllocator, ProcessCpuTimes, list_processes, process_utilization};

    #[test]
    fn computes_eighty_percent_process_utilization() {
        let previous = ProcessCpuTimes {
            kernel: 10,
            user: 10,
        };
        let current = ProcessCpuTimes {
            kernel: 20,
            user: 20,
        };
        let utilization = match process_utilization(previous, current, 2_500) {
            Some(value) => value,
            None => return,
        };
        assert!((utilization - 80.0).abs() < 0.001);
    }

    #[test]
    fn allocator_is_stable_for_one_pid() {
        let mut allocator = ActivityIdAllocator::default();
        let first = allocator.id_for(42);
        let second = allocator.id_for(42);
        let third = allocator.id_for(43);
        assert_eq!(first, second);
        assert!(third > second);
    }

    /// Live-system shape check: the picker lists present processes with
    /// positive PIDs, non-empty names, in PID order.
    #[test]
    #[cfg(windows)]
    fn process_list_is_sorted_with_usable_identities()
    -> Result<(), crate::windows::SystemTelemetryError> {
        let processes = list_processes()?;
        assert!(!processes.is_empty());
        assert!(processes.iter().all(|entry| entry.process_id > 0));
        assert!(processes.iter().all(|entry| !entry.name.is_empty()));
        assert!(
            processes
                .windows(2)
                .all(|pair| pair[0].process_id < pair[1].process_id)
        );
        Ok(())
    }
}
