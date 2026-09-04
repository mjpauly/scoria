<!-- Baseline for the memory probe regression mode (doc/decimation/memory-limits.md, "Probe harness"). Measures the retired geojson map path; kept as the record of why the 10k cap was load-bearing. -->

Environment: MacBook (Apple Silicon), macOS Darwin 24.5.0; chrome-headless-shell 152.0.7977.54 (swiftshader GL); 2026-08-24.
RSS is swiftshader-inflated vs real GPUs; post-GC heap is the portable signal. k slopes are per byte of geojson string, regressed across N per style.

# Memory probe sweep

| style | N | points MB | lines MB | peak RSS d MB | settled RSS d MB | page heap MB | worker heap MB |
|---|---|---|---|---|---|---|---|
| points | 10000 | 1.2 | 0.0 | 738.9 | 458.5 | 10.7 | 59.0 |
| points | 30000 | 3.5 | 0.0 | 1198.8 | 608.0 | 10.7 | 163.7 |
| points | 100000 | 11.7 | 0.0 | 2861.7 | 1104.1 | 10.8 | 551.9 |
| points | 300000 | 35.1 | 0.0 | 3706.4 | 2188.6 | 10.7 | 1742.6 |
| lines | 10000 | 1.2 | 1.6 | 760.0 | 498.1 | 10.9 | 84.5 |
| lines | 30000 | 3.5 | 4.9 | 1195.6 | 711.6 | 10.8 | 237.3 |
| lines | 100000 | 11.7 | 16.4 | 3488.3 | 1490.2 | 10.8 | 747.7 |
| lines | 300000 | 35.1 | 49.2 | 4369.8 | 2783.5 | 10.8 | 2071.6 |
| cmap | 10000 | 1.3 | 1.8 | 776.5 | 557.4 | 12.0 | 90.6 |
| cmap | 30000 | 4.0 | 5.4 | 1284.5 | 778.4 | 12.0 | 231.4 |
| cmap | 100000 | 13.4 | 18.1 | 3416.5 | 1525.9 | 12.1 | 707.8 |
| cmap | 300000 | 40.2 | 54.3 | 4164.2 | 2445.7 | 12.0 | 1882.3 |

## Per-style rates (regression across N)

k = bytes of browser memory per byte of geojson string. Runs where
the renderer crashed or the maplibre worker died (memory exhaustion;
their byte counts and heaps are invalid) are excluded.

| style | k (peak RSS) | k (settled RSS) | k (post-GC heap) | str bytes/pt | str bytes/line |
|---|---|---|---|---|---|
| points | 82.1 | 50.4 | 49.8 | 117 | 0 |
| lines | 42.2 | 27.5 | 24.3 | 117 | 164 |
| cmap | 34.8 | 20.0 | 19.5 | 134 | 181 |
