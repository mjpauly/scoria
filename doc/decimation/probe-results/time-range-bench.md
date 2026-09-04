# Time range vs spatial index routing

2026-08-27, on bench4.db (the 2026-08-26 export, 2,563,861 points, with
gx/gy and level indexes L4-L16 from cell-index-levels.py). Viewport
400x800 CSS px centered on the densest 152 m cell; time ranges chosen
around the day with the most points inside the z10 view. Desktop, macOS
system SQLite 3.37, warm cache, min of 3. Script: time-range-bench.py.

Implemented in FilteredQuery::fetch_decimated_result_with_db
(back/database/location.rs). Spatial mode: `Route::Time` when the time
range holds under `TIME_PATH_MAX_ROWS` (300k) rows, else the grid walk.
Temporal mode: two LIMIT-bounded index-only counts, rows in range and
all-time points in view (cap `TEMPORAL_COUNT_CAP`, 1M), and the smaller
wins; both over the cap goes to the time route when a range is set (a
sequential scan in rowid order: 275 ms vs 591 grid / 1 s longitude band
on a world view, optimized desktop), else the grid. `lngtimeindex` is
dropped by the grid_indexes migration: on every shape measured it was within
~30% of the grid or slower, winning only when the view captures nearly
all of its longitude band.

## Background

The app's time filters were wrapped in `likelihood(..., 1.0)` so the
planner would stay on `lngtimeindex` and not use the unique `timestamp`
autoindex. That was right when the longitude index was the only spatial
option, but it made every time-restricted query pay the longitude band
scan: 55 ms for a one-day count that the timestamp index answers in 3 ms.
The bench below uses bare comparisons for the time path ("today" columns
are the planner's choice on the bare comparison, i.e. the timestamp index
when a range is set).

## Temporal decimation: count (`id % 10`) and decimated fetch, ms

| zoom | range | rows in range | in view | count: ts index | grid CTE | fetch: ts index | grid |
|---|---|---|---|---|---|---|---|
| z6 | day | 9.6k | 9.6k | 0.3 | 54 | 17 | 107 |
| z6 | year | 707k | 563k | 36 | 55 | 44 | 99 |
| z6 | all | 2.56M | 1.9M | 133 (lng index) | 56 | 88 | 102 |
| z10 | day | 9.6k | 8.1k | 0.3 | 39 | 15 | 80 |
| z10 | month | 61k | 39k | 3.0 | 40 | 20 | 79 |
| z10 | year | 707k | 372k | 36 | 40 | 47 | 78 |
| z10 | all | 2.56M | 1.4M | 132 (lng index) | 41 | 94 | 81 |
| z13 | all | 2.56M | 931k | 133 (lng index) | 28 | 103 | 67 |
| z16 | year | 707k | 75k | 37 | 11 | 57 | 41 |
| z16 | all | 2.56M | 334k | 133 (lng index) | 14 | 127 | 47 |

The grid count with the edge-cell `OR` (exact bounds) costs the same as
without it (within 1 ms), so the grid count is exact and needs no
sampling. Per-`cy`-row seeks via a recursive CTE beat a `cy BETWEEN`
band scan by ~25% at every zoom.

## Temporal grid filter level, count / fetch ms by index level

| zoom | L8 | L10 | L12 |
|---|---|---|---|
| z4 | 76 / 85 (27k rows) | 96 / 106 | 167 / 178 |
| z6 | 67 / 77 (6.7k rows) | 72 / 83 | 93 / 104 |
| z10 | 48 / 62 (417 rows) | 48 / 64 | 50 / 65 |
| z13 | 35 / 50 (53 rows) | 33 / 51 | 33 / 50 |
| z16 | 17 / 32 (7 rows) | 14 / 33 | 13 / 33 |
| z18 | 26 / 28 (3 rows) | 13 / 29 | 10 / 30 |

Coarser levels win while the rectangle is many cells across (fewer row
seeks); once it is a handful of cells across, most of it is edge cells
whose rows must be read for the exact bounds, and the finer level wins.
`grid::filter_level` takes the coarsest level with at least 16 cells
across the rectangle's smaller side.

## Spatial decimation: walk vs timestamp index + GROUP BY cell, ms

| zoom | range | rows in range | in view | walk (time-filtered) | ts index bucket |
|---|---|---|---|---|---|
| z6 | week | 23k | 20k | 168 | 3.6 |
| z6 | year | 707k | 563k | 176 | 146 |
| z6 | all | 2.56M | 1.9M | 188 | 565 |
| z10 | day | 9.6k | 8.1k | 132 | 1.9 (under the trigger anyway) |
| z10 | week | 23k | 18k | 136 | 4.4 |
| z10 | month | 61k | 39k | 136 | 9.7 |
| z10 | year | 707k | 372k | 147 | 120 |
| z10 | all | 2.56M | 1.4M | 152 | 495 |
| z13 | month | 61k | 27k | 47 | 9.6 |
| z13 | year | 707k | 251k | 127 | 107 |
| z16 | year | 707k | 75k | 48 | 58 |
| z16 | all | 2.56M | 334k | 153 | 226 |

The walk costs ~3-4 us per occupied index cell in view, independent of
the time range; the time path costs ~150-200 ns per row in range. The
crossover is ~700-800k rows at z6-z13 and ~150k at z16. Occupied cells
in view has no cheap estimator (counting them is the walk), so the
route is a single threshold on rows in range, counted on the timestamp
index with a LIMIT: one constant is within 2x of optimal at every zoom
here, and both paths cost ~100-150 ms at the crossover.

## Findings

- Time-restricted queries should use the timestamp index. Day-to-month
  ranges go from 130-170 ms (walk) or 55 ms (longitude band) to 2-20 ms.
- With no time range (or a very wide one), temporal mode benefits from
  the grid as a 2-D filter: the count goes from 130 ms to 14-56 ms and
  the fetch improves 1.1-2.7x, more at higher zoom.
- A separate timestamp index is unnecessary: `timestamp` is unique and
  has an autoindex.
- Rows-in-range is counted with `LIMIT 300001` on the index, not from
  id ranges: ids need not be time-ordered after an import of older data.
