//! Pure, deterministic performance analysis.
//!
//! This crate deliberately contains no operating-system I/O. The same normalized
//! inputs always produce the same outputs.

mod comparison;
mod statistics;
mod stutter;

pub use statistics::{
    FrameStatistics, LowFpsMetrics, PercentileSet, StatisticsConfig, StatisticsError,
    compute_statistics,
};
pub use stutter::{
    RefreshAwareStutterConfig, StutterConfig, StutterError, detect_stutters,
    detect_stutters_refresh_aware,
};

pub use comparison::{
    Comparability, ComparisonConfig, ComparisonError, ComparisonInput, ComparisonMetrics,
    ComparisonResult, DeltaDirection, InferenceLimit, MetricDelta, PairedInferenceConfig,
    PairedInferenceOutcome, PairedInferenceResult, PairedRun, compare, compare_low_fps,
    minimum_pairs_for_alpha, paired_infer,
};
