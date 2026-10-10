# Statistical Methodology

All frame-time statistics operate on valid frame intervals x_i in milliseconds after warm-up trimming and source-semantic filtering.

## Definitions

mean = sum(x_i) / N

average_fps = 1000 * N / sum(x_i)

median = P50

sample variance for N >= 2 = sum((x_i - mean)^2) / (N - 1)

standard deviation = sqrt(sample variance)

## Percentiles

M1 uses nearest-rank percentiles. After ascending sort:

k = ceil(p * N)

P_p = x_(max(1, min(N, k)))

Required: P0.1, P1, P5, P10, P50, P95, P99, P99.9.

P0.1 is a lower-tail frame-time metric and must not be labeled 0.1% low FPS.

## Low FPS

Two non-ambiguous families are reported.

Percentile-frame-time family:
- 1% low (P99-FPS) = 1000 / P99(frame_time)
- 0.1% low (P99.9-FPS) = 1000 / P99.9(frame_time)

Worst-frame-average family:
- select max(1, floor(0.01*N)) largest frame times and average their reciprocal FPS values;
- select max(1, floor(0.001*N)) largest frame times for the 0.1% metric.

## Stutter

Default detector:
- half-window = 15 frames;
- multiplier = 2.0;
- absolute threshold = 30 ms;
- minimum neighbors = 7;
- merge gap = 50 ms.

For frame i, baseline is the nearest-rank P50 of its valid symmetric neighbors excluding i.

A candidate requires both frame_time > multiplier * baseline and frame_time > absolute threshold.

Candidate timestamps within merge_gap_ms are grouped into one event.

Dropped or repeated frame semantics are never inferred from a frame-time spike alone; source semantics remain an adapter responsibility.

## Sample size

P99.9 is marked less reliable below 1000 frames by default. This is a reliability warning, not a statistical invalidation.
