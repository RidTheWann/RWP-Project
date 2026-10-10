//! Deterministic evidence-based diagnosis rules.

use rwp_analysis::FrameStatistics;
use rwp_core::{
    Diagnostic, DiagnosticCategory, EvidenceItem, EvidencePolarity, FrameSample, StutterEvent,
    SystemSample,
};

/// Explicit thresholds used by the M5 rule engine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuleThresholds {
    /// Minimum frame observations required for frame-based rules.
    pub minimum_frame_samples: usize,
    /// Minimum system observations required for telemetry-based rules.
    pub minimum_system_samples: usize,
    /// CPU time share within a frame considered high.
    pub cpu_frame_share: f64,
    /// GPU time share within a frame considered high.
    pub gpu_frame_share: f64,
    /// Target-thread utilization considered high.
    pub thread_utilization: f64,
    /// System GPU utilization considered high.
    pub gpu_utilization: f64,
    /// GPU temperature considered thermal-risk evidence when paired with clock reduction.
    pub gpu_temperature_c: f64,
    /// GPU clock reduction required while the GPU is thermally hot.
    pub gpu_clock_drop_percent: f64,
    /// Background CPU utilization considered an interference burst.
    pub background_cpu_utilization: f64,
    /// Minimum fraction of stutter events that must overlap background bursts before a low alignment is treated as contradicting evidence.
    pub background_min_aligned_fraction: f64,
    /// System RAM utilization considered memory-pressure evidence.
    pub ram_utilization: f64,
    /// VRAM utilization considered video-memory-pressure evidence.
    pub vram_utilization: f64,
    /// P99/P50 frame-time ratio considered strong pacing instability.
    pub frame_pacing_ratio: f64,
    /// Minimum number of stutter events required for frame-pacing-rate evidence.
    pub frame_pacing_min_stutter_events: usize,
    /// Minimum stutter event rate considered persistent pacing instability.
    pub frame_pacing_min_events_per_minute: f64,
    /// Minimum heuristic score required to emit a non-Unknown diagnosis.
    pub activation_score: u8,
}

impl Default for RuleThresholds {
    fn default() -> Self {
        Self {
            minimum_frame_samples: 60,
            minimum_system_samples: 10,
            cpu_frame_share: 0.80,
            gpu_frame_share: 0.80,
            thread_utilization: 85.0,
            gpu_utilization: 90.0,
            gpu_temperature_c: 85.0,
            gpu_clock_drop_percent: 15.0,
            background_cpu_utilization: 10.0,
            background_min_aligned_fraction: 0.5,
            ram_utilization: 90.0,
            vram_utilization: 90.0,
            frame_pacing_ratio: 1.75,
            frame_pacing_min_stutter_events: 3,
            frame_pacing_min_events_per_minute: 6.0,
            activation_score: 60,
        }
    }
}

/// Inputs required by the deterministic M5 diagnosis engine.
#[derive(Debug)]
pub struct DiagnosisInput<'a> {
    /// Deterministic frame-time statistics.
    pub statistics: &'a FrameStatistics,
    /// Original normalized frame observations.
    pub frames: &'a [FrameSample],
    /// Derived frame-pacing events.
    pub stutters: &'a [StutterEvent],
    /// System telemetry snapshots.
    pub system_samples: &'a [SystemSample],
}

/// Deterministic diagnosis engine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiagnosticEngine {
    thresholds: RuleThresholds,
}

impl DiagnosticEngine {
    /// Creates an engine with explicit thresholds.
    pub fn new(thresholds: RuleThresholds) -> Self {
        Self { thresholds }
    }

    /// Returns the thresholds used by this engine.
    pub fn thresholds(&self) -> RuleThresholds {
        self.thresholds
    }

    /// Evaluates all supported rules in deterministic category order.
    pub fn diagnose(&self, input: DiagnosisInput<'_>) -> Result<Vec<Diagnostic>, DiagnosticError> {
        validate_thresholds(self.thresholds)?;

        let mut diagnostics = Vec::new();
        if input.frames.len() >= self.thresholds.minimum_frame_samples {
            if let Some(diagnostic) = self.evaluate_cpu_bound(&input)? {
                diagnostics.push(diagnostic);
            }
            if let Some(diagnostic) = self.evaluate_gpu_bound(&input)? {
                diagnostics.push(diagnostic);
            }
            if let Some(diagnostic) = self.evaluate_frame_pacing(&input)? {
                diagnostics.push(diagnostic);
            }
            if input.system_samples.len() >= self.thresholds.minimum_system_samples {
                if let Some(diagnostic) = self.evaluate_thermal(&input)? {
                    diagnostics.push(diagnostic);
                }
                if let Some(diagnostic) = self.evaluate_memory_pressure(&input)? {
                    diagnostics.push(diagnostic);
                }
                if let Some(diagnostic) = self.evaluate_vram_pressure(&input)? {
                    diagnostics.push(diagnostic);
                }
            }
        }
        if input.system_samples.len() >= self.thresholds.minimum_system_samples
            && let Some(diagnostic) = self.evaluate_background_interference(&input)?
        {
            diagnostics.push(diagnostic);
        }

        if diagnostics.is_empty() {
            return Ok(vec![self.unknown_diagnostic(input)]);
        }

        diagnostics.sort_by(|left, right| {
            right
                .evidence_score
                .cmp(&left.evidence_score)
                .then_with(|| category_rank(left.category).cmp(&category_rank(right.category)))
        });
        Ok(diagnostics)
    }

    fn evaluate_cpu_bound(
        &self,
        input: &DiagnosisInput<'_>,
    ) -> Result<Option<Diagnostic>, DiagnosticError> {
        let cpu_share = mean_frame_share(input.frames, |frame| frame.cpu_busy_ms);
        let gpu_share = mean_frame_share(input.frames, |frame| frame.gpu_time_ms);
        let thread_utilization = mean_peak_thread_utilization(input.system_samples);

        let mut support = Vec::new();
        let mut contradiction_evidence = Vec::new();

        if let Some(value) = cpu_share
            && value >= self.thresholds.cpu_frame_share
        {
            support.push(supporting(
                "cpu.frame_share",
                35,
                format!("Mean CPU frame-time share is {value:.3}"),
            ));
        }
        if let Some(value) = thread_utilization
            && value >= self.thresholds.thread_utilization
        {
            support.push(supporting(
                "cpu.target_thread",
                35,
                format!("Mean peak target-thread utilization is {value:.1}%"),
            ));
        }
        if let Some(value) = gpu_share
            && value >= self.thresholds.gpu_frame_share
        {
            contradiction_evidence.push(contradicting(
                "gpu.strong_share",
                40,
                format!("Mean GPU frame-time share is {value:.3}"),
            ));
        }

        make_diagnostic(
            DiagnosticCategory::CpuBound,
            "cpu_bound.v1",
            support,
            contradiction_evidence,
            self.thresholds,
            input.frames.len(),
            self.thresholds.minimum_frame_samples,
        )
    }

    fn evaluate_gpu_bound(
        &self,
        input: &DiagnosisInput<'_>,
    ) -> Result<Option<Diagnostic>, DiagnosticError> {
        let gpu_share = mean_frame_share(input.frames, |frame| frame.gpu_busy_ms);
        let cpu_share = mean_frame_share(input.frames, |frame| frame.cpu_busy_ms);
        let gpu_utilization = mean_gpu_utilization(input.system_samples);

        let mut support = Vec::new();
        let mut contradiction_evidence = Vec::new();

        if let Some(value) = gpu_share
            && value >= self.thresholds.gpu_frame_share
        {
            support.push(supporting(
                "gpu.busy_share",
                35,
                format!("Mean GPU-busy frame share is {value:.3}"),
            ));
        }
        if let Some(value) = gpu_utilization
            && value >= self.thresholds.gpu_utilization
        {
            support.push(supporting(
                "gpu.utilization",
                35,
                format!("Mean system GPU utilization is {value:.1}%"),
            ));
        }
        if let Some(value) = cpu_share
            && value >= self.thresholds.cpu_frame_share
        {
            contradiction_evidence.push(contradicting(
                "cpu.strong_share",
                40,
                format!("Mean CPU frame-time share is {value:.3}"),
            ));
        }

        make_diagnostic(
            DiagnosticCategory::GpuBound,
            "gpu_bound.v2",
            support,
            contradiction_evidence,
            self.thresholds,
            input.frames.len(),
            self.thresholds.minimum_frame_samples,
        )
    }

    fn evaluate_frame_pacing(
        &self,
        input: &DiagnosisInput<'_>,
    ) -> Result<Option<Diagnostic>, DiagnosticError> {
        let ratio = input.statistics.percentiles.p99_ms / input.statistics.percentiles.p50_ms;
        let event_rate = stutter_events_per_minute(input.frames, input.stutters);
        let mut support = Vec::new();

        if input.stutters.len() >= self.thresholds.frame_pacing_min_stutter_events
            && event_rate
                .is_some_and(|value| value >= self.thresholds.frame_pacing_min_events_per_minute)
        {
            let rate = event_rate.unwrap_or(0.0);
            support.push(supporting(
                "frame_pacing.event_rate",
                35,
                format!("Stutter event rate is {rate:.1} events/minute"),
            ));
        }
        if ratio >= self.thresholds.frame_pacing_ratio {
            support.push(supporting(
                "frame_pacing.tail_ratio",
                35,
                format!("P99/P50 frame-time ratio is {ratio:.2}"),
            ));
        }
        make_diagnostic(
            DiagnosticCategory::FramePacing,
            "frame_pacing.v2",
            support,
            Vec::new(),
            self.thresholds,
            input.frames.len(),
            self.thresholds.minimum_frame_samples,
        )
    }

    fn evaluate_thermal(
        &self,
        input: &DiagnosisInput<'_>,
    ) -> Result<Option<Diagnostic>, DiagnosticError> {
        let max_temp = input
            .system_samples
            .iter()
            .filter_map(|sample| sample.gpu_temperature_c)
            .max_by(|a, b| a.total_cmp(b));
        let Some(max_temp) = max_temp else {
            return Ok(None);
        };

        if max_temp < self.thresholds.gpu_temperature_c {
            return Ok(None);
        }

        let max_clock = input
            .system_samples
            .iter()
            .filter_map(|sample| sample.gpu_clock_mhz)
            .max();
        let Some(max_clock) = max_clock else {
            return Ok(None);
        };
        if max_clock == 0 {
            return Ok(None);
        }

        let hot_min_clock = input
            .system_samples
            .iter()
            .filter(|sample| {
                sample
                    .gpu_temperature_c
                    .is_some_and(|temperature| temperature >= self.thresholds.gpu_temperature_c)
            })
            .filter_map(|sample| sample.gpu_clock_mhz)
            .min();
        let Some(hot_min_clock) = hot_min_clock else {
            return Ok(None);
        };

        let clock_drop_percent =
            (max_clock.saturating_sub(hot_min_clock) as f64 / max_clock as f64) * 100.0;
        if clock_drop_percent < self.thresholds.gpu_clock_drop_percent {
            return Ok(None);
        }

        let support = vec![
            supporting(
                "thermal.gpu_temperature",
                35,
                format!("Peak observed GPU temperature is {max_temp:.1} C"),
            ),
            supporting(
                "thermal.gpu_clock_drop",
                35,
                format!(
                    "GPU clock dropped {clock_drop_percent:.1}% while GPU temperature met the thermal threshold"
                ),
            ),
        ];

        make_diagnostic(
            DiagnosticCategory::Thermal,
            "thermal.v2",
            support,
            Vec::new(),
            self.thresholds,
            input.system_samples.len(),
            self.thresholds.minimum_system_samples,
        )
    }

    fn evaluate_memory_pressure(
        &self,
        input: &DiagnosisInput<'_>,
    ) -> Result<Option<Diagnostic>, DiagnosticError> {
        let mean_utilization =
            mean_optional_percentage(input.system_samples, |sample| sample.ram_utilization);
        let Some(mean_utilization) = mean_utilization else {
            return Ok(None);
        };

        let mut support = Vec::new();
        if mean_utilization >= self.thresholds.ram_utilization {
            support.push(supporting(
                "memory.ram_utilization",
                70,
                format!("Mean system RAM utilization is {mean_utilization:.1}%"),
            ));
        }
        if has_high_telemetry_overlap(
            input.stutters,
            input.system_samples,
            |sample| sample.ram_utilization,
            self.thresholds.ram_utilization,
        ) {
            support.push(supporting(
                "memory.ram_stutter_overlap",
                30,
                String::from("High RAM utilization overlapped a detected stutter window"),
            ));
        }

        make_diagnostic(
            DiagnosticCategory::MemoryPressure,
            "memory_pressure.v1",
            support,
            Vec::new(),
            self.thresholds,
            input.system_samples.len(),
            self.thresholds.minimum_system_samples,
        )
    }

    fn evaluate_vram_pressure(
        &self,
        input: &DiagnosisInput<'_>,
    ) -> Result<Option<Diagnostic>, DiagnosticError> {
        let mean_utilization =
            mean_optional_percentage(input.system_samples, |sample| sample.vram_utilization);
        let Some(mean_utilization) = mean_utilization else {
            return Ok(None);
        };

        let mut support = Vec::new();
        if mean_utilization >= self.thresholds.vram_utilization {
            support.push(supporting(
                "memory.vram_utilization",
                70,
                format!("Mean VRAM utilization is {mean_utilization:.1}%"),
            ));
        }
        if has_high_telemetry_overlap(
            input.stutters,
            input.system_samples,
            |sample| sample.vram_utilization,
            self.thresholds.vram_utilization,
        ) {
            support.push(supporting(
                "memory.vram_stutter_overlap",
                30,
                String::from("High VRAM utilization overlapped a detected stutter window"),
            ));
        }

        make_diagnostic(
            DiagnosticCategory::VramPressure,
            "vram_pressure.v1",
            support,
            Vec::new(),
            self.thresholds,
            input.system_samples.len(),
            self.thresholds.minimum_system_samples,
        )
    }

    fn evaluate_background_interference(
        &self,
        input: &DiagnosisInput<'_>,
    ) -> Result<Option<Diagnostic>, DiagnosticError> {
        let total_events = input.stutters.len();
        if total_events == 0 {
            return Ok(None);
        }

        let sampling_interval_ns = median_positive_timestamp_delta(input.system_samples);
        let margin_ns = sampling_interval_ns.unwrap_or(0);
        let mut aligned_events = 0_usize;
        let mut peak_background = None;

        for event in input.stutters {
            let window_start = event.start_timestamp_ns.saturating_sub(margin_ns);
            let window_end = event.end_timestamp_ns.saturating_add(margin_ns);
            let event_peak = input
                .system_samples
                .iter()
                .filter(|sample| {
                    sample.timestamp_ns >= window_start && sample.timestamp_ns <= window_end
                })
                .flat_map(|sample| sample.background_activity.iter())
                .map(|activity| activity.cpu_utilization)
                .max_by(|a, b| a.total_cmp(b));

            if let Some(event_peak) = event_peak {
                peak_background = max_f64(peak_background, Some(event_peak));
                if event_peak >= self.thresholds.background_cpu_utilization {
                    aligned_events = aligned_events.saturating_add(1);
                }
            }
        }

        let Some(peak_background) = peak_background else {
            return Ok(None);
        };

        let alignment = format!("{aligned_events}/{total_events}");
        let mut support = vec![supporting(
            "background.aligned_bursts",
            70,
            format!(
                "{aligned_events} of {total_events} stutter events overlapped background CPU activity at or above the configured threshold"
            ),
        )];

        if aligned_events > 0 {
            support.push(supporting(
                "background.cpu_burst",
                30,
                format!("Peak redacted background CPU activity was {peak_background:.1}%"),
            ));
        }

        let mut contradiction_evidence = Vec::new();
        if (aligned_events as f64)
            < self.thresholds.background_min_aligned_fraction * total_events as f64
        {
            contradiction_evidence.push(contradicting(
                "background.low_alignment",
                40,
                format!(
                    "Only {aligned_events} of {total_events} stutter events overlapped background CPU activity; most stutter events are not explained by background load"
                ),
            ));
        }

        let result = make_diagnostic(
            DiagnosticCategory::BackgroundInterference,
            "background_interference.v3",
            support,
            contradiction_evidence,
            self.thresholds,
            input.system_samples.len(),
            self.thresholds.minimum_system_samples,
        )?;

        if let Some(mut diagnostic) = result {
            diagnostic
                .parameters
                .push((String::from("aligned_events"), alignment));
            diagnostic
                .parameters
                .push((String::from("total_events"), total_events.to_string()));
            if aligned_events == 0 {
                return Ok(None);
            }
            return Ok(Some(diagnostic));
        }

        Ok(None)
    }

    fn unknown_diagnostic(&self, input: DiagnosisInput<'_>) -> Diagnostic {
        Diagnostic {
            category: DiagnosticCategory::Unknown,
            rule_id: String::from("unknown.insufficient_evidence.v1"),
            evidence_score: 0,
            supporting_evidence: vec![supporting(
                "unknown",
                100,
                format!(
                    "No rule reached the activation score with {} frame samples and {} system samples",
                    input.frames.len(),
                    input.system_samples.len()
                ),
            )],
            contradicting_evidence: Vec::new(),
            parameters: threshold_parameters(self.thresholds),
            sample_count: input.frames.len().max(input.system_samples.len()),
            minimum_sample_count: self
                .thresholds
                .minimum_frame_samples
                .min(self.thresholds.minimum_system_samples),
        }
    }
}

/// Errors raised by invalid diagnosis-engine thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DiagnosticError {
    /// A threshold was invalid.
    #[error("invalid diagnosis threshold: {0}")]
    InvalidThreshold(&'static str),
}

fn validate_thresholds(thresholds: RuleThresholds) -> Result<(), DiagnosticError> {
    if thresholds.minimum_frame_samples == 0 {
        return Err(DiagnosticError::InvalidThreshold("minimum_frame_samples"));
    }
    if thresholds.minimum_system_samples == 0 {
        return Err(DiagnosticError::InvalidThreshold("minimum_system_samples"));
    }
    if !(0.0..=1.0).contains(&thresholds.cpu_frame_share) || thresholds.cpu_frame_share == 0.0 {
        return Err(DiagnosticError::InvalidThreshold("cpu_frame_share"));
    }
    if !(0.0..=1.0).contains(&thresholds.gpu_frame_share) || thresholds.gpu_frame_share == 0.0 {
        return Err(DiagnosticError::InvalidThreshold("gpu_frame_share"));
    }
    if !(0.0..=100.0).contains(&thresholds.thread_utilization)
        || thresholds.thread_utilization == 0.0
    {
        return Err(DiagnosticError::InvalidThreshold("thread_utilization"));
    }
    if !(0.0..=100.0).contains(&thresholds.gpu_utilization) || thresholds.gpu_utilization == 0.0 {
        return Err(DiagnosticError::InvalidThreshold("gpu_utilization"));
    }
    if !(0.0..=120.0).contains(&thresholds.gpu_temperature_c) || thresholds.gpu_temperature_c <= 0.0
    {
        return Err(DiagnosticError::InvalidThreshold("gpu_temperature_c"));
    }
    if !(0.0..=100.0).contains(&thresholds.gpu_clock_drop_percent)
        || thresholds.gpu_clock_drop_percent <= 0.0
        || !thresholds.gpu_clock_drop_percent.is_finite()
    {
        return Err(DiagnosticError::InvalidThreshold("gpu_clock_drop_percent"));
    }
    if !(0.0..=100.0).contains(&thresholds.background_cpu_utilization)
        || thresholds.background_cpu_utilization == 0.0
    {
        return Err(DiagnosticError::InvalidThreshold(
            "background_cpu_utilization",
        ));
    }
    if !(0.0..=1.0).contains(&thresholds.background_min_aligned_fraction)
        || thresholds.background_min_aligned_fraction <= 0.0
        || !thresholds.background_min_aligned_fraction.is_finite()
    {
        return Err(DiagnosticError::InvalidThreshold(
            "background_min_aligned_fraction",
        ));
    }
    if !(0.0..=100.0).contains(&thresholds.ram_utilization) || thresholds.ram_utilization == 0.0 {
        return Err(DiagnosticError::InvalidThreshold("ram_utilization"));
    }
    if !(0.0..=100.0).contains(&thresholds.vram_utilization) || thresholds.vram_utilization == 0.0 {
        return Err(DiagnosticError::InvalidThreshold("vram_utilization"));
    }
    if thresholds.frame_pacing_ratio <= 1.0 || !thresholds.frame_pacing_ratio.is_finite() {
        return Err(DiagnosticError::InvalidThreshold("frame_pacing_ratio"));
    }
    if thresholds.frame_pacing_min_stutter_events == 0 {
        return Err(DiagnosticError::InvalidThreshold(
            "frame_pacing_min_stutter_events",
        ));
    }
    if thresholds.frame_pacing_min_events_per_minute <= 0.0
        || !thresholds.frame_pacing_min_events_per_minute.is_finite()
    {
        return Err(DiagnosticError::InvalidThreshold(
            "frame_pacing_min_events_per_minute",
        ));
    }
    if thresholds.activation_score == 0 || thresholds.activation_score > 100 {
        return Err(DiagnosticError::InvalidThreshold("activation_score"));
    }
    Ok(())
}

fn mean_frame_share(
    frames: &[FrameSample],
    getter: impl Fn(&FrameSample) -> Option<f64>,
) -> Option<f64> {
    let values = frames
        .iter()
        .filter_map(|frame| getter(frame).map(|value| value / frame.frame_time_ms))
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn mean_peak_thread_utilization(samples: &[SystemSample]) -> Option<f64> {
    let values = samples
        .iter()
        .filter_map(|sample| {
            sample
                .process_threads
                .iter()
                .map(|thread| thread.cpu_utilization)
                .max_by(|a, b| a.total_cmp(b))
        })
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn mean_optional_percentage(
    samples: &[SystemSample],
    getter: impl Fn(&SystemSample) -> Option<f64>,
) -> Option<f64> {
    let values = samples.iter().filter_map(getter).collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn has_high_telemetry_overlap(
    stutters: &[StutterEvent],
    samples: &[SystemSample],
    getter: impl Fn(&SystemSample) -> Option<f64>,
    threshold: f64,
) -> bool {
    stutters.iter().any(|event| {
        samples.iter().any(|sample| {
            sample.timestamp_ns >= event.start_timestamp_ns
                && sample.timestamp_ns <= event.end_timestamp_ns
                && getter(sample).is_some_and(|value| value >= threshold)
        })
    })
}

fn mean_gpu_utilization(samples: &[SystemSample]) -> Option<f64> {
    let values = samples
        .iter()
        .filter_map(|sample| sample.gpu_utilization)
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn stutter_events_per_minute(frames: &[FrameSample], stutters: &[StutterEvent]) -> Option<f64> {
    if stutters.is_empty() {
        return None;
    }
    let start = frames.first()?.timestamp_ns;
    let end = frames.last()?.timestamp_ns;
    let span_ns = end.checked_sub(start)?;
    if span_ns == 0 {
        return None;
    }
    let span_minutes = span_ns as f64 / 60_000_000_000.0;
    Some(stutters.len() as f64 / span_minutes)
}

fn median_positive_timestamp_delta(samples: &[SystemSample]) -> Option<u64> {
    let mut deltas = samples
        .windows(2)
        .filter_map(|window| window[1].timestamp_ns.checked_sub(window[0].timestamp_ns))
        .filter(|delta| *delta > 0)
        .collect::<Vec<_>>();
    if deltas.is_empty() {
        return None;
    }
    deltas.sort_unstable();
    Some(deltas[deltas.len() / 2])
}

fn max_f64(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    match (left, right) {
        (None, value) | (value, None) => value,
        (Some(left), Some(right)) => Some(left.max(right)),
    }
}

fn supporting(id: &str, weight: u16, statement: String) -> EvidenceItem {
    EvidenceItem {
        evidence_id: String::from(id),
        polarity: EvidencePolarity::Supporting,
        statement,
        weight,
    }
}

fn contradicting(id: &str, weight: u16, statement: String) -> EvidenceItem {
    EvidenceItem {
        evidence_id: String::from(id),
        polarity: EvidencePolarity::Contradicting,
        statement,
        weight,
    }
}

fn make_diagnostic(
    category: DiagnosticCategory,
    rule_id: &str,
    supporting: Vec<EvidenceItem>,
    contradicting: Vec<EvidenceItem>,
    thresholds: RuleThresholds,
    sample_count: usize,
    minimum_sample_count: usize,
) -> Result<Option<Diagnostic>, DiagnosticError> {
    let support_weight: u16 = supporting.iter().map(|item| item.weight).sum();
    let contradiction_weight: u16 = contradicting.iter().map(|item| item.weight).sum();
    let final_score = support_weight.saturating_sub(contradiction_weight).min(100) as u8;
    if final_score < thresholds.activation_score {
        return Ok(None);
    }
    let diagnostic = Diagnostic::new(
        category,
        String::from(rule_id),
        final_score,
        supporting,
        contradicting,
        threshold_parameters(thresholds),
        sample_count,
        minimum_sample_count,
    )
    .map_err(|_| DiagnosticError::InvalidThreshold("diagnostic score"))?;
    Ok(Some(diagnostic))
}

fn threshold_parameters(thresholds: RuleThresholds) -> Vec<(String, String)> {
    vec![
        (
            String::from("minimum_frame_samples"),
            thresholds.minimum_frame_samples.to_string(),
        ),
        (
            String::from("minimum_system_samples"),
            thresholds.minimum_system_samples.to_string(),
        ),
        (
            String::from("cpu_frame_share"),
            thresholds.cpu_frame_share.to_string(),
        ),
        (
            String::from("gpu_frame_share"),
            thresholds.gpu_frame_share.to_string(),
        ),
        (
            String::from("thread_utilization"),
            thresholds.thread_utilization.to_string(),
        ),
        (
            String::from("gpu_utilization"),
            thresholds.gpu_utilization.to_string(),
        ),
        (
            String::from("gpu_temperature_c"),
            thresholds.gpu_temperature_c.to_string(),
        ),
        (
            String::from("gpu_clock_drop_percent"),
            thresholds.gpu_clock_drop_percent.to_string(),
        ),
        (
            String::from("background_cpu_utilization"),
            thresholds.background_cpu_utilization.to_string(),
        ),
        (
            String::from("background_min_aligned_fraction"),
            thresholds.background_min_aligned_fraction.to_string(),
        ),
        (
            String::from("ram_utilization"),
            thresholds.ram_utilization.to_string(),
        ),
        (
            String::from("vram_utilization"),
            thresholds.vram_utilization.to_string(),
        ),
        (
            String::from("frame_pacing_ratio"),
            thresholds.frame_pacing_ratio.to_string(),
        ),
        (
            String::from("frame_pacing_min_stutter_events"),
            thresholds.frame_pacing_min_stutter_events.to_string(),
        ),
        (
            String::from("frame_pacing_min_events_per_minute"),
            thresholds.frame_pacing_min_events_per_minute.to_string(),
        ),
        (
            String::from("activation_score"),
            thresholds.activation_score.to_string(),
        ),
    ]
}

fn category_rank(category: DiagnosticCategory) -> u8 {
    match category {
        DiagnosticCategory::CpuBound => 0,
        DiagnosticCategory::GpuBound => 1,
        DiagnosticCategory::FramePacing => 2,
        DiagnosticCategory::Thermal => 3,
        DiagnosticCategory::MemoryPressure => 4,
        DiagnosticCategory::VramPressure => 5,
        DiagnosticCategory::BackgroundInterference => 6,
        DiagnosticCategory::StreamingOverhead => 7,
        DiagnosticCategory::Unknown => 8,
        _ => 255,
    }
}

#[cfg(test)]
mod tests {
    use super::{DiagnosisInput, DiagnosticEngine, RuleThresholds};
    use rwp_analysis::{StatisticsConfig, compute_statistics};
    use rwp_core::BackgroundActivity;
    use rwp_core::{FrameSample, ProcessThreadSample, StutterEvent, SystemSample};

    fn frames(cpu_busy_ms: f64, gpu_time_ms: f64) -> Result<Vec<FrameSample>, rwp_core::CoreError> {
        (0..60)
            .map(|index| {
                FrameSample::new(index * 10_000_000, 10.0).and_then(|frame| {
                    frame.with_telemetry(
                        Some(cpu_busy_ms),
                        None,
                        Some(gpu_time_ms),
                        None,
                        None,
                        None,
                    )
                })
            })
            .collect()
    }

    #[test]
    fn contradicting_evidence_has_contradicting_polarity() -> Result<(), Box<dyn std::error::Error>>
    {
        let frame_data = frames(9.0, 9.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let system = (0..10)
            .map(|index| {
                let mut sample = SystemSample::new(index * 100_000_000);
                sample.process_threads = vec![ProcessThreadSample::new(1, 95.0)?];
                Ok::<_, rwp_core::CoreError>(sample)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let engine = DiagnosticEngine::new(RuleThresholds {
            activation_score: 1,
            ..RuleThresholds::default()
        });
        let result = engine.diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &[],
            system_samples: &system,
        })?;
        let cpu = result
            .iter()
            .find(|d| d.category == rwp_core::DiagnosticCategory::CpuBound)
            .ok_or("CPU diagnostic missing")?;
        assert!(
            cpu.contradicting_evidence
                .iter()
                .all(|item| { item.polarity == rwp_core::EvidencePolarity::Contradicting })
        );
        Ok(())
    }

    #[test]
    fn background_overlap_without_load_is_not_interference()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(1.0, 1.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let mut sample = SystemSample::new(300_000_000);
        sample.background_activity = vec![BackgroundActivity::new(1, 0.0)?];
        let system = (0..10)
            .map(|index| {
                if index == 3 {
                    Ok(sample.clone())
                } else {
                    Ok(SystemSample::new(index * 100_000_000))
                }
            })
            .collect::<Result<Vec<_>, rwp_core::CoreError>>()?;
        let stutters = vec![StutterEvent {
            start_timestamp_ns: 300_000_000,
            end_timestamp_ns: 300_000_000,
            peak_frame_time_ms: 40.0,
            baseline_frame_time_ms: 10.0,
            candidate_frame_count: 1,
            detector_parameters: Vec::new(),
        }];
        let engine = DiagnosticEngine::new(RuleThresholds::default());
        let result = engine.diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &stutters,
            system_samples: &system,
        })?;
        assert!(
            !result
                .iter()
                .any(|d| { d.category == rwp_core::DiagnosticCategory::BackgroundInterference })
        );
        Ok(())
    }

    #[test]
    fn thermal_fires_when_hot_and_clock_drops() -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(1.0, 1.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let mut system = Vec::new();
        for index in 0..10_u64 {
            let mut sample = SystemSample::new(index * 100_000_000);
            if index < 8 {
                sample.gpu_temperature_c = Some(80.0);
                sample.gpu_clock_mhz = Some(1800);
            } else {
                sample.gpu_temperature_c = Some(90.0);
                sample.gpu_clock_mhz = Some(1400);
            }
            system.push(sample);
        }

        let engine = DiagnosticEngine::new(RuleThresholds::default());
        let result = engine.diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &[],
            system_samples: &system,
        })?;
        let thermal = result
            .iter()
            .find(|diagnostic| diagnostic.category == rwp_core::DiagnosticCategory::Thermal)
            .ok_or("thermal diagnostic missing")?;
        assert_eq!(thermal.rule_id, "thermal.v2");
        Ok(())
    }

    #[test]
    fn thermal_requires_clock_drop_in_addition_to_high_temperature()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(1.0, 1.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let mut system = Vec::new();
        for index in 0..10_u64 {
            let mut sample = SystemSample::new(index * 100_000_000);
            sample.gpu_temperature_c = Some(90.0);
            sample.gpu_clock_mhz = Some(1800);
            system.push(sample);
        }

        let engine = DiagnosticEngine::new(RuleThresholds::default());
        let result = engine.diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &[],
            system_samples: &system,
        })?;

        assert!(
            !result
                .iter()
                .any(|diagnostic| diagnostic.category == rwp_core::DiagnosticCategory::Thermal)
        );
        Ok(())
    }

    #[test]
    fn thermal_does_not_fire_below_minimum_system_samples() -> Result<(), Box<dyn std::error::Error>>
    {
        let frame_data = frames(1.0, 1.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let mut system = Vec::new();
        for index in 0..5_u64 {
            let mut sample = SystemSample::new(index * 100_000_000);
            if index < 3 {
                sample.gpu_temperature_c = Some(80.0);
                sample.gpu_clock_mhz = Some(1800);
            } else {
                sample.gpu_temperature_c = Some(90.0);
                sample.gpu_clock_mhz = Some(1400);
            }
            system.push(sample);
        }

        let result = DiagnosticEngine::new(RuleThresholds::default()).diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &[],
            system_samples: &system,
        })?;

        assert!(
            !result
                .iter()
                .any(|diagnostic| diagnostic.category == rwp_core::DiagnosticCategory::Thermal)
        );
        Ok(())
    }

    #[test]
    fn one_stutter_event_is_insufficient_for_frame_pacing() -> Result<(), Box<dyn std::error::Error>>
    {
        let frame_data = frames(1.0, 1.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let stutters = vec![StutterEvent {
            start_timestamp_ns: 300_000_000,
            end_timestamp_ns: 300_000_000,
            peak_frame_time_ms: 40.0,
            baseline_frame_time_ms: 10.0,
            candidate_frame_count: 1,
            detector_parameters: Vec::new(),
        }];

        let engine = DiagnosticEngine::new(RuleThresholds::default());
        let result = engine.diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &stutters,
            system_samples: &[],
        })?;

        assert!(
            !result
                .iter()
                .any(|diagnostic| diagnostic.category == rwp_core::DiagnosticCategory::FramePacing)
        );
        Ok(())
    }

    #[test]
    fn gpu_time_without_gpu_busy_does_not_activate_gpu_bound()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = (0..60_u64)
            .map(|index| {
                FrameSample::new(index * 10_000_000, 10.0).and_then(|frame| {
                    frame.with_telemetry(Some(1.0), None, Some(9.0), None, None, None)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;

        let engine = DiagnosticEngine::new(RuleThresholds {
            activation_score: 30,
            ..RuleThresholds::default()
        });
        let result = engine.diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &[],
            system_samples: &[],
        })?;

        assert!(
            !result
                .iter()
                .any(|diagnostic| diagnostic.category == rwp_core::DiagnosticCategory::GpuBound)
        );
        Ok(())
    }

    #[test]
    fn gpu_busy_activates_gpu_bound_with_explicit_v2_provenance()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = (0..60_u64)
            .map(|index| {
                FrameSample::new(index * 10_000_000, 10.0).and_then(|frame| {
                    frame.with_telemetry(Some(1.0), None, Some(1.0), Some(9.0), None, None)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;

        let engine = DiagnosticEngine::new(RuleThresholds {
            activation_score: 30,
            ..RuleThresholds::default()
        });
        let result = engine.diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &[],
            system_samples: &[],
        })?;

        let gpu = result
            .iter()
            .find(|diagnostic| diagnostic.category == rwp_core::DiagnosticCategory::GpuBound)
            .ok_or("GPU diagnostic missing")?;
        assert_eq!(gpu.rule_id, "gpu_bound.v2");
        Ok(())
    }

    #[test]
    fn cpu_frame_share_contradicts_gpu_bound() -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = (0..60_u64)
            .map(|index| {
                FrameSample::new(index * 10_000_000, 10.0).and_then(|frame| {
                    frame.with_telemetry(Some(9.0), None, Some(1.0), Some(9.0), None, None)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let system_samples = (0..10_u64)
            .map(|index| {
                let mut sample = SystemSample::new(index * 100_000_000);
                sample.gpu_utilization = Some(95.0);
                sample
            })
            .collect::<Vec<_>>();
        let engine = DiagnosticEngine::new(RuleThresholds {
            activation_score: 1,
            ..RuleThresholds::default()
        });
        let result = engine.diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &[],
            system_samples: &system_samples,
        })?;
        let gpu = result
            .iter()
            .find(|diagnostic| diagnostic.category == rwp_core::DiagnosticCategory::GpuBound)
            .ok_or("GPU diagnostic missing")?;

        assert_eq!(gpu.evidence_score, 30);
        assert_eq!(gpu.contradicting_evidence.len(), 1);
        assert_eq!(
            gpu.contradicting_evidence[0].evidence_id,
            "cpu.strong_share"
        );
        assert_eq!(
            gpu.contradicting_evidence[0].polarity,
            rwp_core::EvidencePolarity::Contradicting
        );
        Ok(())
    }

    #[test]
    fn gpu_frame_share_contradicts_cpu_bound() -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(9.0, 9.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let system_samples = (0..10_u64)
            .map(|index| {
                let mut sample = SystemSample::new(index * 100_000_000);
                sample.process_threads = vec![ProcessThreadSample::new(1, 95.0)?];
                Ok::<_, rwp_core::CoreError>(sample)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let engine = DiagnosticEngine::new(RuleThresholds {
            activation_score: 1,
            ..RuleThresholds::default()
        });
        let result = engine.diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &[],
            system_samples: &system_samples,
        })?;
        let cpu = result
            .iter()
            .find(|diagnostic| diagnostic.category == rwp_core::DiagnosticCategory::CpuBound)
            .ok_or("CPU diagnostic missing")?;

        assert_eq!(cpu.evidence_score, 30);
        assert_eq!(cpu.contradicting_evidence.len(), 1);
        assert_eq!(
            cpu.contradicting_evidence[0].evidence_id,
            "gpu.strong_share"
        );
        assert_eq!(
            cpu.contradicting_evidence[0].polarity,
            rwp_core::EvidencePolarity::Contradicting
        );
        Ok(())
    }

    #[test]
    fn detects_cpu_bound_when_cpu_and_target_thread_are_high()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(9.0, 2.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let system = (0..10)
            .map(|index| {
                let mut sample = SystemSample::new(index * 100_000_000);
                sample.process_threads = vec![ProcessThreadSample::new(1, 95.0)?];
                Ok::<_, rwp_core::CoreError>(sample)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let stutters = vec![StutterEvent {
            start_timestamp_ns: 300_000_000,
            end_timestamp_ns: 300_000_000,
            peak_frame_time_ms: 40.0,
            baseline_frame_time_ms: 10.0,
            candidate_frame_count: 1,
            detector_parameters: Vec::new(),
        }];
        let engine = DiagnosticEngine::new(RuleThresholds {
            minimum_frame_samples: 60,
            minimum_system_samples: 10,
            ..RuleThresholds::default()
        });
        let result = engine.diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &stutters,
            system_samples: &system,
        })?;
        assert!(
            result
                .iter()
                .any(|diagnostic| diagnostic.category == rwp_core::DiagnosticCategory::CpuBound)
        );
        Ok(())
    }

    #[test]
    fn cpu_bound_does_not_fire_when_cpu_share_and_thread_utilization_are_low()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(1.0, 1.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let system = (0..10)
            .map(|index| Ok(SystemSample::new(index * 100_000_000)))
            .collect::<Result<Vec<_>, rwp_core::CoreError>>()?;

        let result = DiagnosticEngine::new(RuleThresholds::default()).diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &[],
            system_samples: &system,
        })?;

        assert!(
            !result.iter().any(|diagnostic| {
                diagnostic.category == rwp_core::DiagnosticCategory::CpuBound
            })
        );
        Ok(())
    }

    #[test]
    fn detects_ram_pressure_from_high_normalized_utilization()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(1.0, 1.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let system = (0..10)
            .map(|index| {
                let mut sample = SystemSample::new(index * 100_000_000);
                sample.ram_utilization = Some(95.0);
                Ok::<_, rwp_core::CoreError>(sample)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let engine = DiagnosticEngine::new(RuleThresholds {
            minimum_frame_samples: 60,
            minimum_system_samples: 10,
            ..RuleThresholds::default()
        });
        let result = engine.diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &[],
            system_samples: &system,
        })?;
        assert!(result.iter().any(|diagnostic| {
            diagnostic.category == rwp_core::DiagnosticCategory::MemoryPressure
        }));
        Ok(())
    }

    #[test]
    fn detects_vram_pressure_from_high_normalized_utilization()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(1.0, 1.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let system = (0..10)
            .map(|index| {
                let mut sample = SystemSample::new(index * 100_000_000);
                sample.vram_utilization = Some(95.0);
                Ok::<_, rwp_core::CoreError>(sample)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let engine = DiagnosticEngine::new(RuleThresholds {
            minimum_frame_samples: 60,
            minimum_system_samples: 10,
            ..RuleThresholds::default()
        });
        let result = engine.diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &[],
            system_samples: &system,
        })?;
        assert!(result.iter().any(|diagnostic| {
            diagnostic.category == rwp_core::DiagnosticCategory::VramPressure
        }));
        Ok(())
    }

    #[test]
    fn background_interference_fires_for_aligned_cpu_burst_and_records_provenance()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(1.0, 1.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let system = (0..10)
            .map(|index| {
                let mut sample = SystemSample::new(index * 100_000_000);
                if index == 3 {
                    sample.background_activity = vec![BackgroundActivity::new(7, 80.0)?];
                }
                Ok::<_, rwp_core::CoreError>(sample)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let stutters = vec![StutterEvent {
            start_timestamp_ns: 300_000_000,
            end_timestamp_ns: 300_000_000,
            peak_frame_time_ms: 40.0,
            baseline_frame_time_ms: 10.0,
            candidate_frame_count: 1,
            detector_parameters: Vec::new(),
        }];

        let result = DiagnosticEngine::new(RuleThresholds::default()).diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &stutters,
            system_samples: &system,
        })?;
        let diagnosis = result
            .iter()
            .find(|diagnostic| {
                diagnostic.category == rwp_core::DiagnosticCategory::BackgroundInterference
            })
            .ok_or("background interference diagnostic missing")?;

        assert_eq!(diagnosis.rule_id, "background_interference.v3");
        assert_eq!(diagnosis.evidence_score, 100);
        assert!(diagnosis.contradicting_evidence.is_empty());
        assert!(
            diagnosis
                .parameters
                .iter()
                .any(|(key, value)| { key == "aligned_events" && value == "1/1" })
        );
        assert!(
            diagnosis
                .parameters
                .iter()
                .any(|(key, value)| { key == "total_events" && value == "1" })
        );
        assert!(diagnosis.supporting_evidence.iter().any(|item| {
            item.evidence_id == "background.cpu_burst"
                && item.polarity == rwp_core::EvidencePolarity::Supporting
        }));
        Ok(())
    }

    #[test]
    fn background_interference_records_contradicting_low_alignment()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(1.0, 1.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let system = (0..10_u64)
            .map(|index| {
                let mut sample = SystemSample::new(index * 100_000_000);
                let utilization = if index == 3 { 80.0 } else { 5.0 };
                sample.background_activity = vec![BackgroundActivity::new(7, utilization)?];
                Ok::<_, rwp_core::CoreError>(sample)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let stutters = [300_000_000_u64, 600_000_000, 900_000_000]
            .into_iter()
            .map(|timestamp_ns| StutterEvent {
                start_timestamp_ns: timestamp_ns,
                end_timestamp_ns: timestamp_ns,
                peak_frame_time_ms: 40.0,
                baseline_frame_time_ms: 10.0,
                candidate_frame_count: 1,
                detector_parameters: Vec::new(),
            })
            .collect::<Vec<_>>();

        let result = DiagnosticEngine::new(RuleThresholds::default()).diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &stutters,
            system_samples: &system,
        })?;
        let diagnosis = result
            .iter()
            .find(|diagnostic| {
                diagnostic.category == rwp_core::DiagnosticCategory::BackgroundInterference
            })
            .ok_or("background interference diagnostic missing")?;

        assert_eq!(diagnosis.evidence_score, 60);
        assert_eq!(diagnosis.contradicting_evidence.len(), 1);
        assert_eq!(
            diagnosis.contradicting_evidence[0].evidence_id,
            "background.low_alignment"
        );
        assert_eq!(
            diagnosis.contradicting_evidence[0].polarity,
            rwp_core::EvidencePolarity::Contradicting
        );
        assert!(
            diagnosis
                .parameters
                .iter()
                .any(|(key, value)| { key == "aligned_events" && value == "1/3" })
        );
        Ok(())
    }

    #[test]
    fn background_activity_outside_stutter_windows_does_not_activate()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(1.0, 1.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let mut system = (0..10_u64)
            .map(|index| Ok::<_, rwp_core::CoreError>(SystemSample::new(index * 100_000_000)))
            .collect::<Result<Vec<_>, _>>()?;
        system[7].background_activity = vec![BackgroundActivity::new(7, 90.0)?];
        let stutters = vec![StutterEvent {
            start_timestamp_ns: 300_000_000,
            end_timestamp_ns: 300_000_000,
            peak_frame_time_ms: 40.0,
            baseline_frame_time_ms: 10.0,
            candidate_frame_count: 1,
            detector_parameters: Vec::new(),
        }];

        let result = DiagnosticEngine::new(RuleThresholds::default()).diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &stutters,
            system_samples: &system,
        })?;

        assert!(!result.iter().any(|diagnostic| {
            diagnostic.category == rwp_core::DiagnosticCategory::BackgroundInterference
        }));
        Ok(())
    }

    #[test]
    fn telemetry_rules_do_not_fire_below_minimum_system_samples()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(1.0, 1.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let mut system = Vec::new();
        for index in 0..5_u64 {
            let mut sample = SystemSample::new(index * 100_000_000);
            sample.ram_utilization = Some(95.0);
            sample.vram_utilization = Some(95.0);
            sample.background_activity = vec![BackgroundActivity::new(7, 80.0)?];
            system.push(sample);
        }
        let stutters = vec![StutterEvent {
            start_timestamp_ns: 300_000_000,
            end_timestamp_ns: 300_000_000,
            peak_frame_time_ms: 40.0,
            baseline_frame_time_ms: 10.0,
            candidate_frame_count: 1,
            detector_parameters: Vec::new(),
        }];

        let result = DiagnosticEngine::new(RuleThresholds::default()).diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &stutters,
            system_samples: &system,
        })?;

        assert!(!result.iter().any(|diagnostic| {
            matches!(
                diagnostic.category,
                rwp_core::DiagnosticCategory::MemoryPressure
                    | rwp_core::DiagnosticCategory::VramPressure
                    | rwp_core::DiagnosticCategory::BackgroundInterference
                    | rwp_core::DiagnosticCategory::Thermal
            )
        }));
        Ok(())
    }

    #[test]
    fn frame_rules_do_not_fire_below_minimum_frame_samples()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(9.0, 9.0)?;
        let frame_data = frame_data[..30].to_vec();
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let system = (0..10_u64)
            .map(|index| {
                let mut sample = SystemSample::new(index * 100_000_000);
                sample.process_threads = vec![ProcessThreadSample::new(1, 95.0)?];
                Ok::<_, rwp_core::CoreError>(sample)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let engine = DiagnosticEngine::new(RuleThresholds {
            activation_score: 1,
            ..RuleThresholds::default()
        });

        let result = engine.diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &[],
            system_samples: &system,
        })?;

        assert!(!result.iter().any(|diagnostic| {
            matches!(
                diagnostic.category,
                rwp_core::DiagnosticCategory::CpuBound
                    | rwp_core::DiagnosticCategory::GpuBound
                    | rwp_core::DiagnosticCategory::FramePacing
            )
        }));
        Ok(())
    }

    #[test]
    fn frame_pacing_fires_when_tail_ratio_and_event_rate_are_sufficient()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = (0..60_u64)
            .map(|index| {
                let frame_time_ms = if matches!(index, 10 | 30 | 50) {
                    40.0
                } else {
                    10.0
                };
                FrameSample::new(index * 10_000_000, frame_time_ms)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let stutters = [100_000_000_u64, 300_000_000, 500_000_000]
            .into_iter()
            .map(|timestamp_ns| StutterEvent {
                start_timestamp_ns: timestamp_ns,
                end_timestamp_ns: timestamp_ns,
                peak_frame_time_ms: 40.0,
                baseline_frame_time_ms: 10.0,
                candidate_frame_count: 1,
                detector_parameters: Vec::new(),
            })
            .collect::<Vec<_>>();

        let result = DiagnosticEngine::new(RuleThresholds::default()).diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &stutters,
            system_samples: &[],
        })?;
        let diagnosis = result
            .iter()
            .find(|diagnostic| diagnostic.category == rwp_core::DiagnosticCategory::FramePacing)
            .ok_or("frame pacing diagnostic missing")?;

        assert_eq!(diagnosis.rule_id, "frame_pacing.v2");
        assert!(diagnosis.supporting_evidence.iter().any(|item| {
            item.evidence_id == "frame_pacing.event_rate"
                && item.polarity == rwp_core::EvidencePolarity::Supporting
        }));
        assert!(diagnosis.supporting_evidence.iter().any(|item| {
            item.evidence_id == "frame_pacing.tail_ratio"
                && item.polarity == rwp_core::EvidencePolarity::Supporting
        }));
        Ok(())
    }

    #[test]
    fn memory_and_vram_pressure_do_not_fire_for_absent_or_low_utilization()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(1.0, 1.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;

        for (ram_utilization, vram_utilization) in [(None, None), (Some(45.0), Some(40.0))] {
            let system = (0..10)
                .map(|index| {
                    let mut sample = SystemSample::new(index * 100_000_000);
                    sample.ram_utilization = ram_utilization;
                    sample.vram_utilization = vram_utilization;
                    Ok::<_, rwp_core::CoreError>(sample)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let result =
                DiagnosticEngine::new(RuleThresholds::default()).diagnose(DiagnosisInput {
                    statistics: &statistics,
                    frames: &frame_data,
                    stutters: &[],
                    system_samples: &system,
                })?;

            assert!(!result.iter().any(|diagnostic| {
                diagnostic.category == rwp_core::DiagnosticCategory::MemoryPressure
                    || diagnostic.category == rwp_core::DiagnosticCategory::VramPressure
            }));
        }
        Ok(())
    }

    #[test]
    fn returns_unknown_when_sample_counts_are_met_but_optional_telemetry_is_absent()
    -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = (0..60_u64)
            .map(|index| FrameSample::new(index * 10_000_000, 16.7))
            .collect::<Result<Vec<_>, _>>()?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let system_samples = (0..10_u64)
            .map(|index| Ok(SystemSample::new(index * 100_000_000)))
            .collect::<Result<Vec<_>, rwp_core::CoreError>>()?;

        let result = DiagnosticEngine::new(RuleThresholds::default()).diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &[],
            system_samples: &system_samples,
        })?;

        assert_eq!(result.len(), 1);
        let unknown = &result[0];
        assert_eq!(unknown.category, rwp_core::DiagnosticCategory::Unknown);
        assert_eq!(unknown.rule_id, "unknown.insufficient_evidence.v1");
        assert_eq!(unknown.evidence_score, 0);
        assert!(unknown.contradicting_evidence.is_empty());
        assert_eq!(unknown.sample_count, 60);
        assert_eq!(unknown.minimum_sample_count, 10);
        assert!(
            result
                .iter()
                .all(|diagnostic| diagnostic.category == rwp_core::DiagnosticCategory::Unknown)
        );
        Ok(())
    }

    #[test]
    fn returns_unknown_when_data_is_insufficient() -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(1.0, 1.0)?;
        let frame_data = frame_data[..30].to_vec();
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;
        let engine = DiagnosticEngine::new(RuleThresholds {
            minimum_frame_samples: 61,
            ..RuleThresholds::default()
        });
        let result = engine.diagnose(DiagnosisInput {
            statistics: &statistics,
            frames: &frame_data,
            stutters: &[],
            system_samples: &[],
        })?;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].category, rwp_core::DiagnosticCategory::Unknown);
        Ok(())
    }

    #[test]
    fn rejects_invalid_background_alignment_fraction() -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = frames(1.0, 1.0)?;
        let frame_times = frame_data
            .iter()
            .map(|frame| frame.frame_time_ms)
            .collect::<Vec<_>>();
        let statistics = compute_statistics(
            &frame_times,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
        )?;

        for invalid in [0.0, -0.5, 1.5, f64::NAN] {
            let engine = DiagnosticEngine::new(RuleThresholds {
                background_min_aligned_fraction: invalid,
                ..RuleThresholds::default()
            });
            let result = engine.diagnose(DiagnosisInput {
                statistics: &statistics,
                frames: &frame_data,
                stutters: &[],
                system_samples: &[],
            });
            assert!(matches!(
                result,
                Err(super::DiagnosticError::InvalidThreshold(
                    "background_min_aligned_fraction"
                ))
            ));
        }
        Ok(())
    }
}
