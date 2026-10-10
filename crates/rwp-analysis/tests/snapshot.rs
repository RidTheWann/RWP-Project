#![allow(missing_docs)]

use rwp_analysis::{StatisticsConfig, compute_statistics};

#[test]
fn statistics_snapshot_is_stable() -> Result<(), Box<dyn std::error::Error>> {
    let values = [8.0, 8.0, 8.0, 16.7, 16.8, 20.0, 33.4, 50.0];
    let stats = compute_statistics(
        &values,
        StatisticsConfig {
            p999_reliability_threshold: 8,
        },
    )?;
    let rendered = format!(
        "sample_count={}\nmean_frame_time_ms={:.6}\nmedian_frame_time_ms={:.6}\naverage_fps={:.6}\nvariance_ms2={:?}\nstdev_ms={:?}\np0_1_ms={:.6}\np1_ms={:.6}\np50_ms={:.6}\np95_ms={:.6}\np99_ms={:.6}\np99_9_ms={:.6}\npercentile_1_low_fps={:.6}\npercentile_0_1_low_fps={:.6}\nmean_fps_of_worst_1pct_frames={:.6}\nmean_fps_of_worst_0_1pct_frames={:.6}\np999_reliable={}",
        stats.sample_count,
        stats.mean_frame_time_ms,
        stats.median_frame_time_ms,
        stats.average_fps,
        stats.variance_ms2,
        stats.stdev_ms,
        stats.percentiles.p0_1_ms,
        stats.percentiles.p1_ms,
        stats.percentiles.p50_ms,
        stats.percentiles.p95_ms,
        stats.percentiles.p99_ms,
        stats.percentiles.p99_9_ms,
        stats.low_fps.percentile_1_low_fps,
        stats.low_fps.percentile_0_1_low_fps,
        stats.low_fps.mean_fps_of_worst_1pct_frames,
        stats.low_fps.mean_fps_of_worst_0_1pct_frames,
        stats.p999_reliable,
    );
    insta::assert_snapshot!(rendered, @r###"sample_count=8
mean_frame_time_ms=20.112500
median_frame_time_ms=16.700000
average_fps=49.720323
variance_ms2=Some(218.94125)
stdev_ms=Some(14.796663475256846)
p0_1_ms=8.000000
p1_ms=8.000000
p50_ms=16.700000
p95_ms=50.000000
p99_ms=50.000000
p99_9_ms=50.000000
percentile_1_low_fps=20.000000
percentile_0_1_low_fps=20.000000
mean_fps_of_worst_1pct_frames=20.000000
mean_fps_of_worst_0_1pct_frames=20.000000
p999_reliable=true"###);
    Ok(())
}
