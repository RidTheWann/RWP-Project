//! PDH-backed system CPU telemetry boundary.

#![allow(unsafe_code)]

use std::mem::MaybeUninit;

use windows_sys::Win32::System::Performance::{
    PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE,
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhGetFormattedCounterValue, PdhOpenQueryW,
};

fn pdh_counter_status_is_valid(status: u32) -> bool {
    matches!(status, PDH_CSTATUS_VALID_DATA | PDH_CSTATUS_NEW_DATA)
}

const MAX_COUNTER_ARRAY_ATTEMPTS: usize = 3;

const PROCESSOR_TOTAL_PATH: &[u16] = &[
    '\\' as u16,
    'P' as u16,
    'r' as u16,
    'o' as u16,
    'c' as u16,
    'e' as u16,
    's' as u16,
    's' as u16,
    'o' as u16,
    'r' as u16,
    '(' as u16,
    '_' as u16,
    'T' as u16,
    'o' as u16,
    't' as u16,
    'a' as u16,
    'l' as u16,
    ')' as u16,
    '\\' as u16,
    '%' as u16,
    ' ' as u16,
    'P' as u16,
    'r' as u16,
    'o' as u16,
    'c' as u16,
    'e' as u16,
    's' as u16,
    's' as u16,
    'o' as u16,
    'r' as u16,
    ' ' as u16,
    'T' as u16,
    'i' as u16,
    'm' as u16,
    'e' as u16,
    0,
];

const GPU_ENGINE_PATH: &[u16] = &[
    '\\' as u16,
    'G' as u16,
    'P' as u16,
    'U' as u16,
    ' ' as u16,
    'E' as u16,
    'n' as u16,
    'g' as u16,
    'i' as u16,
    'n' as u16,
    'e' as u16,
    '(' as u16,
    '*' as u16,
    ')' as u16,
    '\\' as u16,
    'U' as u16,
    't' as u16,
    'i' as u16,
    'l' as u16,
    'i' as u16,
    'z' as u16,
    'a' as u16,
    't' as u16,
    'i' as u16,
    'o' as u16,
    'n' as u16,
    ' ' as u16,
    'P' as u16,
    'e' as u16,
    'r' as u16,
    'c' as u16,
    'e' as u16,
    'n' as u16,
    't' as u16,
    'a' as u16,
    'g' as u16,
    'e' as u16,
    0,
];

/// Vendor-neutral per-process GPU observation selected from Windows GPU Engine instances.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuProcessTelemetry {
    /// Aggregated utilization for the dominant adapter used by the target process.
    pub utilization_percent: f64,
    /// Stable-for-the-run adapter key derived from the GPU Engine instance LUID/physical index.
    pub adapter_key: String,
}

/// Parses a Windows GPU Engine instance name and returns its adapter grouping key.
fn gpu_engine_adapter_key(instance: &str, target_process_id: u32) -> Option<String> {
    let pid_marker = "pid_";
    let pid_start = instance.find(pid_marker)? + pid_marker.len();
    let pid_end = instance[pid_start..].find('_')? + pid_start;
    let pid = instance[pid_start..pid_end].parse::<u32>().ok()?;
    if pid != target_process_id {
        return None;
    }

    let engine_marker = "_eng_";
    let engine_end = instance.find(engine_marker)?;
    let adapter_start = instance.find("luid_").or_else(|| instance.find("phys_"))?;
    if adapter_start >= engine_end {
        return None;
    }
    Some(instance[adapter_start..engine_end].to_owned())
}

fn aggregate_gpu_engine_utilization(
    target_process_id: u32,
    instances: impl IntoIterator<Item = (String, f64)>,
) -> Option<GpuProcessTelemetry> {
    let mut by_adapter = std::collections::BTreeMap::<String, f64>::new();
    for (instance, utilization) in instances {
        if !utilization.is_finite() || utilization < 0.0 {
            continue;
        }
        let Some(adapter_key) = gpu_engine_adapter_key(&instance, target_process_id) else {
            continue;
        };
        *by_adapter.entry(adapter_key).or_default() += utilization;
    }

    by_adapter
        .into_iter()
        .max_by(|(left_key, left_value), (right_key, right_value)| {
            left_value
                .total_cmp(right_value)
                .then_with(|| right_key.cmp(left_key))
        })
        .map(|(adapter_key, utilization_percent)| GpuProcessTelemetry {
            utilization_percent: utilization_percent.clamp(0.0, 100.0),
            adapter_key,
        })
}

/// PDH-backed vendor-neutral GPU Engine telemetry for one target process.
pub struct PdhGpuSampler {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
    ready: bool,
}

impl PdhGpuSampler {
    /// Creates a wildcard GPU Engine query.
    pub fn new() -> Result<Self, PdhError> {
        let mut query = std::ptr::null_mut();
        // SAFETY: query is valid writable storage and the null data-source pointer requests a local query.
        let status = unsafe { PdhOpenQueryW(std::ptr::null(), 0, &mut query) };
        if status != 0 {
            return Err(PdhError::Open(status));
        }

        let mut counter = std::ptr::null_mut();
        // SAFETY: query is valid and GPU_ENGINE_PATH is a NUL-terminated static UTF-16 counter path.
        let status =
            unsafe { PdhAddEnglishCounterW(query, GPU_ENGINE_PATH.as_ptr(), 0, &mut counter) };
        if status != 0 {
            // SAFETY: query was successfully opened above and is closed exactly once on this error path.
            unsafe {
                let _ = PdhCloseQuery(query);
            }
            return Err(PdhError::AddCounter(status));
        }

        Ok(Self {
            query,
            counter,
            ready: false,
        })
    }

    /// Collects the target process GPU utilization.
    ///
    /// The first call establishes the PDH baseline and returns no value.
    pub fn sample(
        &mut self,
        target_process_id: u32,
    ) -> Result<Option<GpuProcessTelemetry>, PdhError> {
        // SAFETY: self.query is owned by this sampler and remains valid until Drop.
        let status = unsafe { PdhCollectQueryData(self.query) };
        if status != 0 {
            return Err(PdhError::Collect(status));
        }

        if !self.ready {
            self.ready = true;
            return Ok(None);
        }

        let item_size = std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        for _attempt in 0..MAX_COUNTER_ARRAY_ATTEMPTS {
            let mut buffer_size = 0_u32;
            let mut sizing_item_count = 0_u32;
            // SAFETY: self.counter is owned by this sampler; a null buffer requests the required size.
            let status = unsafe {
                PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut buffer_size,
                    &mut sizing_item_count,
                    std::ptr::null_mut(),
                )
            };
            if status != PDH_MORE_DATA {
                return Err(PdhError::Format(status));
            }

            let sizing_count =
                usize::try_from(sizing_item_count).map_err(|_| PdhError::InvalidBufferSize)?;
            let buffer_size_usize =
                usize::try_from(buffer_size).map_err(|_| PdhError::InvalidBufferSize)?;
            if buffer_size_usize == 0 {
                return if sizing_count == 0 {
                    Ok(None)
                } else {
                    Err(PdhError::InvalidBufferSize)
                };
            }
            let slot_count = buffer_size_usize.div_ceil(item_size);
            if sizing_count > slot_count {
                return Err(PdhError::InvalidBufferSize);
            }

            let mut items =
                Vec::<MaybeUninit<PDH_FMT_COUNTERVALUE_ITEM_W>>::with_capacity(slot_count);
            items.resize_with(slot_count, MaybeUninit::zeroed);
            let mut filled_buffer_size = buffer_size;
            let mut filled_item_count = 0_u32;
            // SAFETY: items owns aligned storage for slot_count records and is zero-initialized.
            // The allocation covers at least buffer_size bytes; the API writes returned records.
            let status = unsafe {
                PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut filled_buffer_size,
                    &mut filled_item_count,
                    items.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W,
                )
            };
            if status == PDH_MORE_DATA {
                // The wildcard instance set changed; repeat the sizing call with a fresh count.
                continue;
            }
            if status != 0 {
                return Err(PdhError::Format(status));
            }

            let item_count =
                validated_counter_array_item_count(sizing_count, filled_item_count, slot_count)?;
            let mut values = Vec::with_capacity(item_count);
            for item in items.iter().take(item_count) {
                // SAFETY: the successful fill call returned this item and the count is slot-bounded.
                let item = unsafe { item.assume_init_ref() };
                if item.szName.is_null() || !pdh_counter_status_is_valid(item.FmtValue.CStatus) {
                    continue;
                }
                // SAFETY: PDH returns szName as a NUL-terminated string owned by the returned item buffer.
                let name = unsafe {
                    let mut length = 0_usize;
                    while *item.szName.add(length) != 0 {
                        length += 1;
                    }
                    let slice = std::slice::from_raw_parts(item.szName, length);
                    String::from_utf16_lossy(slice)
                };
                // SAFETY: PDH_FMT_DOUBLE selects the anonymous doubleValue union member returned by PDH.
                let value = unsafe { item.FmtValue.Anonymous.doubleValue };
                values.push((name, value));
            }

            return Ok(aggregate_gpu_engine_utilization(target_process_id, values));
        }

        Err(PdhError::Format(PDH_MORE_DATA))
    }
}

impl Drop for PdhGpuSampler {
    fn drop(&mut self) {
        if !self.query.is_null() {
            // SAFETY: self.query is owned by this sampler and has not been closed elsewhere.
            unsafe {
                let _ = PdhCloseQuery(self.query);
            }
        }
    }
}

/// Errors raised by the PDH CPU backend.
#[derive(Debug, thiserror::Error)]
pub enum PdhError {
    /// PdhOpenQueryW failed.
    #[error("PdhOpenQueryW failed with status 0x{0:08x}")]
    Open(u32),
    /// PdhAddEnglishCounterW failed.
    #[error("PdhAddEnglishCounterW failed with status 0x{0:08x}")]
    AddCounter(u32),
    /// PdhCollectQueryData failed.
    #[error("PdhCollectQueryData failed with status 0x{0:08x}")]
    Collect(u32),
    /// The PDH counter did not expose a valid value.
    #[error("PDH counter status is 0x{0:08x}")]
    InvalidStatus(u32),
    /// PdhGetFormattedCounterValue or counter-array formatting failed.
    #[error("PDH formatting failed with status 0x{0:08x}")]
    Format(u32),
    /// PDH reported a counter-array size that cannot be represented safely.
    #[error("PDH returned an invalid counter-array buffer size")]
    InvalidBufferSize,
}

fn validated_counter_array_item_count(
    sizing_item_count: usize,
    filled_item_count: u32,
    slot_count: usize,
) -> Result<usize, PdhError> {
    let filled_item_count =
        usize::try_from(filled_item_count).map_err(|_| PdhError::InvalidBufferSize)?;
    if sizing_item_count > slot_count || filled_item_count > slot_count {
        return Err(PdhError::InvalidBufferSize);
    }

    Ok(filled_item_count)
}

/// PDH-backed total CPU collector.
pub struct PdhCpuSampler {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
    ready: bool,
}

impl PdhCpuSampler {
    /// Creates a PDH query for the localized-independent total processor counter.
    pub fn new() -> Result<Self, PdhError> {
        let mut query = std::ptr::null_mut();
        // SAFETY: query is valid writable storage and the null data-source pointer requests a local query.
        let status = unsafe { PdhOpenQueryW(std::ptr::null(), 0, &mut query) };
        if status != 0 {
            return Err(PdhError::Open(status));
        }

        let mut counter = std::ptr::null_mut();
        let status =
            // SAFETY: query is a valid PDH query handle and PROCESSOR_TOTAL_PATH is a NUL-terminated static UTF-16 path.
            unsafe { PdhAddEnglishCounterW(query, PROCESSOR_TOTAL_PATH.as_ptr(), 0, &mut counter) };
        if status != 0 {
            // SAFETY: query was successfully opened above and is closed exactly once on this error path.
            unsafe {
                let _ = PdhCloseQuery(query);
            }
            return Err(PdhError::AddCounter(status));
        }

        Ok(Self {
            query,
            counter,
            ready: false,
        })
    }

    /// Collects the total CPU percentage.
    ///
    /// The first call establishes the baseline and returns no value.
    pub fn sample(&mut self) -> Result<Option<f64>, PdhError> {
        // SAFETY: self.query is owned by this sampler and remains valid until Drop.
        let status = unsafe { PdhCollectQueryData(self.query) };
        if status != 0 {
            return Err(PdhError::Collect(status));
        }

        if !self.ready {
            self.ready = true;
            return Ok(None);
        }

        let mut counter_type = 0_u32;
        let mut value = PDH_FMT_COUNTERVALUE::default();
        // SAFETY: self.counter is owned by this sampler; output pointers reference valid local storage.
        let status = unsafe {
            PdhGetFormattedCounterValue(self.counter, PDH_FMT_DOUBLE, &mut counter_type, &mut value)
        };
        if status != 0 {
            return Err(PdhError::Format(status));
        }

        if !pdh_counter_status_is_valid(value.CStatus) {
            return Err(PdhError::InvalidStatus(value.CStatus));
        }

        // SAFETY: PDH_FMT_DOUBLE selects the anonymous doubleValue union member returned by PDH.
        let cpu_percent = unsafe { value.Anonymous.doubleValue };
        Ok(Some(cpu_percent.clamp(0.0, 100.0)))
    }
}

impl Drop for PdhCpuSampler {
    fn drop(&mut self) {
        if !self.query.is_null() {
            // SAFETY: self.query is owned by this sampler and has not been closed elsewhere.
            unsafe {
                let _ = PdhCloseQuery(self.query);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PdhCpuSampler, aggregate_gpu_engine_utilization, gpu_engine_adapter_key,
        pdh_counter_status_is_valid,
    };

    #[test]
    fn accepts_valid_pdh_data_statuses() {
        assert!(pdh_counter_status_is_valid(super::PDH_CSTATUS_VALID_DATA));
        assert!(pdh_counter_status_is_valid(super::PDH_CSTATUS_NEW_DATA));
        assert!(!pdh_counter_status_is_valid(u32::MAX));
    }

    #[test]
    fn parses_target_pid_and_adapter_key() {
        assert_eq!(
            gpu_engine_adapter_key(
                "pid_1234_luid_0x00000000_0x000120A6_phys_0_eng_0_engtype_3D",
                1234,
            ),
            Some(String::from("luid_0x00000000_0x000120A6_phys_0"))
        );
        assert_eq!(
            gpu_engine_adapter_key(
                "pid_4321_luid_0x00000000_0x000120A6_phys_0_eng_0_engtype_3D",
                1234,
            ),
            None
        );
    }

    #[test]
    fn selects_dominant_adapter_deterministically() {
        let result = aggregate_gpu_engine_utilization(
            1234,
            [
                (
                    String::from("pid_1234_luid_0x1_phys_0_eng_0_engtype_3D"),
                    40.0,
                ),
                (
                    String::from("pid_1234_luid_0x1_phys_0_eng_1_engtype_Copy"),
                    20.0,
                ),
                (
                    String::from("pid_1234_luid_0x2_phys_1_eng_0_engtype_3D"),
                    80.0,
                ),
            ],
        );
        match result {
            Some(result) => {
                assert_eq!(result.adapter_key, "luid_0x2_phys_1");
                assert_eq!(result.utilization_percent, 80.0);
            }
            None => panic!("missing dominant adapter"),
        }
    }

    #[test]
    fn ignores_non_finite_gpu_engine_values() {
        assert_eq!(
            aggregate_gpu_engine_utilization(
                1234,
                [(
                    String::from("pid_1234_luid_0x1_phys_0_eng_0_engtype_3D"),
                    f64::NAN,
                )],
            ),
            None
        );
    }

    #[test]
    fn filled_array_count_uses_the_successful_call_count() {
        assert!(matches!(
            super::validated_counter_array_item_count(8, 3, 8),
            Ok(3)
        ));
    }

    #[test]
    fn filled_array_count_rejects_counts_beyond_allocated_slots() {
        assert!(matches!(
            super::validated_counter_array_item_count(4, 9, 8),
            Err(super::PdhError::InvalidBufferSize)
        ));
    }

    #[test]
    fn constructor_has_explicit_result_type() {
        let _constructor: fn() -> Result<PdhCpuSampler, super::PdhError> = PdhCpuSampler::new;
    }
}
