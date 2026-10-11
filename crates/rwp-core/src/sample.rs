use crate::CoreError;
use serde::{Deserialize, Serialize};

/// A normalized frame observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrameSample {
    /// Capture-relative timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Frame interval used by the analysis engine, in milliseconds.
    pub frame_time_ms: f64,
    /// CPU execution time attributed to the frame, when available.
    pub cpu_busy_ms: Option<f64>,
    /// CPU-side wait time attributed to the frame, when available.
    pub cpu_wait_ms: Option<f64>,
    /// GPU execution time attributed to the frame, when available.
    pub gpu_time_ms: Option<f64>,
    /// GPU busy interval, when exposed separately by the source.
    pub gpu_busy_ms: Option<f64>,
    /// GPU wait interval, when source semantics are established.
    pub gpu_wait_ms: Option<f64>,
    /// Display latency, when a display-time source is available.
    pub display_latency_ms: Option<f64>,
}

impl FrameSample {
    /// Creates a frame sample after validating the frame interval.
    pub fn new(timestamp_ns: u64, frame_time_ms: f64) -> Result<Self, CoreError> {
        validate_positive("frame_time_ms", frame_time_ms)?;
        Ok(Self {
            timestamp_ns,
            frame_time_ms,
            cpu_busy_ms: None,
            cpu_wait_ms: None,
            gpu_time_ms: None,
            gpu_busy_ms: None,
            gpu_wait_ms: None,
            display_latency_ms: None,
        })
    }

    /// Replaces optional timing telemetry after validating all supplied values.
    pub fn with_telemetry(
        mut self,
        cpu_busy_ms: Option<f64>,
        cpu_wait_ms: Option<f64>,
        gpu_time_ms: Option<f64>,
        gpu_busy_ms: Option<f64>,
        gpu_wait_ms: Option<f64>,
        display_latency_ms: Option<f64>,
    ) -> Result<Self, CoreError> {
        for (name, value) in [
            ("cpu_busy_ms", cpu_busy_ms),
            ("cpu_wait_ms", cpu_wait_ms),
            ("gpu_time_ms", gpu_time_ms),
            ("gpu_busy_ms", gpu_busy_ms),
            ("gpu_wait_ms", gpu_wait_ms),
            ("display_latency_ms", display_latency_ms),
        ] {
            validate_optional_non_negative(name, value)?;
        }
        self.cpu_busy_ms = cpu_busy_ms;
        self.cpu_wait_ms = cpu_wait_ms;
        self.gpu_time_ms = gpu_time_ms;
        self.gpu_busy_ms = gpu_busy_ms;
        self.gpu_wait_ms = gpu_wait_ms;
        self.display_latency_ms = display_latency_ms;
        Ok(self)
    }
}

/// A normalized process-thread observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessThreadSample {
    /// Stable per-run thread identity.
    pub thread_id: u32,
    /// Captured thread CPU utilization in percent.
    pub cpu_utilization: f64,
}

impl ProcessThreadSample {
    /// Creates a validated thread utilization sample.
    pub fn new(thread_id: u32, cpu_utilization: f64) -> Result<Self, CoreError> {
        validate_percentage("cpu_utilization", cpu_utilization)?;
        Ok(Self {
            thread_id,
            cpu_utilization,
        })
    }
}

/// Timestamped system telemetry observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemSample {
    /// Capture-relative timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Total CPU utilization in percent, when available.
    pub cpu_utilization: Option<f64>,
    /// GPU utilization in percent, when available.
    pub gpu_utilization: Option<f64>,
    /// Used VRAM in MiB, when available.
    pub vram_used_mb: Option<u64>,
    /// VRAM utilization as a percentage of total adapter memory, when available.
    pub vram_utilization: Option<f64>,
    /// Used system RAM in MiB, when available.
    pub ram_used_mb: Option<u64>,
    /// System RAM utilization as a percentage of total physical memory, when available.
    pub ram_utilization: Option<f64>,
    /// Observed GPU clock in MHz, when available.
    pub gpu_clock_mhz: Option<u64>,
    /// Observed CPU clock in MHz, when available.
    pub cpu_clock_mhz: Option<u64>,
    /// Observed GPU power in watts, when available.
    pub gpu_power_w: Option<f64>,
    /// Observed GPU temperature in degrees Celsius, when available.
    pub gpu_temperature_c: Option<f64>,
    /// Per-thread CPU observations for the target process.
    pub process_threads: Vec<ProcessThreadSample>,
    /// Background activity observations subject to privacy policy.
    pub background_activity: Vec<BackgroundActivity>,
}

impl SystemSample {
    /// Creates a system sample with optional metrics absent.
    pub fn new(timestamp_ns: u64) -> Self {
        Self {
            timestamp_ns,
            cpu_utilization: None,
            gpu_utilization: None,
            vram_used_mb: None,
            vram_utilization: None,
            ram_used_mb: None,
            ram_utilization: None,
            gpu_clock_mhz: None,
            cpu_clock_mhz: None,
            gpu_power_w: None,
            gpu_temperature_c: None,
            process_threads: Vec::new(),
            background_activity: Vec::new(),
        }
    }
}

/// Redacted description of a background process or workload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundActivity {
    /// Stable per-run identifier assigned by the collector.
    pub activity_id: u64,
    /// CPU utilization attributed to the activity in percent.
    pub cpu_utilization: f64,
    /// Optional I/O activity rate in bytes per second.
    pub io_bytes_per_sec: Option<u64>,
    /// Optional network activity rate in bytes per second.
    pub network_bytes_per_sec: Option<u64>,
}

impl BackgroundActivity {
    /// Creates a validated background activity sample.
    pub fn new(activity_id: u64, cpu_utilization: f64) -> Result<Self, CoreError> {
        validate_percentage("cpu_utilization", cpu_utilization)?;
        Ok(Self {
            activity_id,
            cpu_utilization,
            io_bytes_per_sec: None,
            network_bytes_per_sec: None,
        })
    }
}

/// A deterministic stutter event derived from frame timing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StutterEvent {
    /// Timestamp of the first candidate frame in the event.
    pub start_timestamp_ns: u64,
    /// Timestamp of the last candidate frame in the event.
    pub end_timestamp_ns: u64,
    /// Maximum frame time among candidate frames, in milliseconds.
    pub peak_frame_time_ms: f64,
    /// Robust local baseline associated with the event, in milliseconds.
    pub baseline_frame_time_ms: f64,
    /// Number of candidate frames merged into the event.
    pub candidate_frame_count: u32,
    /// Detector parameters serialized in stable insertion order.
    pub detector_parameters: Vec<(String, String)>,
}

fn validate_finite(field: &'static str, value: f64) -> Result<(), CoreError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(CoreError::NonFinite { field })
    }
}

fn validate_positive(field: &'static str, value: f64) -> Result<(), CoreError> {
    validate_finite(field, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(CoreError::NonPositive { field })
    }
}

fn validate_optional_non_negative(
    field: &'static str,
    value: Option<f64>,
) -> Result<(), CoreError> {
    if let Some(value) = value {
        validate_finite(field, value)?;
        if value < 0.0 {
            return Err(CoreError::NegativeValue { field });
        }
    }
    Ok(())
}

fn validate_percentage(field: &'static str, value: f64) -> Result<(), CoreError> {
    validate_finite(field, value)?;
    if !(0.0..=100.0).contains(&value) {
        return Err(CoreError::InvalidRange {
            field,
            min: 0,
            max: 100,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{BackgroundActivity, FrameSample, ProcessThreadSample};
    use crate::CoreError;

    #[test]
    fn validates_frame_time() {
        assert!(FrameSample::new(0, 0.0).is_err());
        assert_eq!(
            FrameSample::new(0, f64::NAN).err(),
            Some(CoreError::NonFinite {
                field: "frame_time_ms"
            })
        );
    }

    #[test]
    fn validates_percentage_bounds() {
        assert!(ProcessThreadSample::new(1, 0.0).is_ok());
        assert!(ProcessThreadSample::new(1, 100.0).is_ok());
        assert_eq!(
            ProcessThreadSample::new(1, 100.1).err(),
            Some(CoreError::InvalidRange {
                field: "cpu_utilization",
                min: 0,
                max: 100,
            })
        );
        assert_eq!(
            BackgroundActivity::new(1, -0.1).err(),
            Some(CoreError::InvalidRange {
                field: "cpu_utilization",
                min: 0,
                max: 100,
            })
        );
    }

    #[test]
    fn optional_frame_telemetry_rejects_negative_values() -> Result<(), CoreError> {
        let frame = FrameSample::new(1, 8.0)?;
        assert_eq!(
            frame
                .with_telemetry(Some(-1.0), None, None, None, None, None)
                .err(),
            Some(CoreError::NegativeValue {
                field: "cpu_busy_ms"
            })
        );
        Ok(())
    }

    #[test]
    fn optional_frame_telemetry_rejects_nonfinite_values() -> Result<(), CoreError> {
        let frame = FrameSample::new(1, 8.0)?;
        assert_eq!(
            frame
                .with_telemetry(None, Some(f64::INFINITY), None, None, None, None)
                .err(),
            Some(CoreError::NonFinite {
                field: "cpu_wait_ms"
            })
        );
        Ok(())
    }

    #[test]
    fn constructors_reject_negative_and_nonfinite_input() {
        assert_eq!(
            FrameSample::new(0, -2.0).err(),
            Some(CoreError::NonPositive {
                field: "frame_time_ms"
            })
        );
        assert_eq!(
            ProcessThreadSample::new(1, f64::NAN).err(),
            Some(CoreError::NonFinite {
                field: "cpu_utilization"
            })
        );
        assert_eq!(
            BackgroundActivity::new(1, f64::INFINITY).err(),
            Some(CoreError::NonFinite {
                field: "cpu_utilization"
            })
        );
    }
}
