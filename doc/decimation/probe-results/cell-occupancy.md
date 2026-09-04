# Spatial cell occupancy of the real dataset

2026-08-26. Full export (src/app/stem/db, 917,766 points), whole
dataset, Mercator grid with 2^k divisions of the world width. Cell
sizes are at the equator; at the data's ~35-40 deg latitudes they are
~20% smaller on the ground. Query in scratch; expression is the same
cast() grid the bucketed query uses, applied to Mercator x/y.

| level | cell    | occupied cells | N / cells | cells with 1 pt | pts in cells >= 100      |
|-------|---------|----------------|-----------|-----------------|--------------------------|
| 2^18  | 152 m   |  53,966        | 17.0      | 21,765          | 58% (1,085 cells; max 135k) |
| 2^20  |  38 m   | 159,724        |  5.7      | 84,410          | 38% (706 cells; max 50k)  |
| 2^22  | 9.6 m   | 375,007        |  2.4      |                 |                          |
| 2^23  | 4.8 m   | 500,149        |  1.8      | 414,355         | 16% (312 cells; max 15.7k) |
| 2^24  | 2.4 m   | 604,035        |  1.5      |                 |                          |
| 2^25  | 1.2 m   | 689,564        |  1.3      | 632,938         | 8%                       |
| 2^28  | 15 cm   | 836,208        |  1.1      |                 |                          |

Reading: the data is track-like, not dwell-like. Location services
deliver points on movement, so density is proportional to distance
travelled; ~45% of points are alone in their 4.8 m cell and only 16%
sit in cells with >= 100 points. Dwells exist (home is 135k points at
152 m) but spread over tens of meters. The collapse ratio grows
linearly with cell width (the d~1 scaling from the earlier probe),
with no jitter scale at which it steps.

Consequences for the spatial-decimation query design
(memory-limits.md discussion, 2026-08-26):

- A single mid-level (~5 m) cell-key index would cut per-cell seeks
  under 2x, no better than an ordered covering scan; a real cull from
  one level needs 40-150 m cells, which puts every zoom from ~z10
  inward back on a scan. Rejected.
- Per-level expression indexes `(morton_key >> 2k, timestamp)` with a
  loose index scan remain the only O(cells-on-screen) option; the cheap
  first step is a stored Morton key plus an ordered covering scan
  grouped in Rust (removes the GROUP BY sorter and per-row table
  reads).
- At the default 0.5 px pitch a z14 view keeps ~66% of in-bounds
  points; spatial decimation only thins materially from city zoom out.
- Temporal thinning is uniform per unit time, which on
  movement-triggered data erases dwells; spatial keeps one point per
  cell per dwell. This is the concrete case for the Spatial default.

Rerun 2026-08-26 on the newer 2,563,861-point export: 152 m 157,923
cells (16.2x), 38 m 397,814 (6.4x), 4.8 m 1,198,590 (2.1x), 1.2 m
1,830,263 (1.4x). Same shape.
