# Per-level cell index benchmark (loose index scan vs GROUP BY scan)

2026-08-26. Prototype for the spatial-decimation query redesign
discussed in memory-limits.md: store u32 Mercator grid coords (gx, gy)
per row, add virtual generated columns `cy{z} = gy >> (22-z)`,
`cx{z} = gx >> (22-z)` (0.5 px cells at view zoom z) and an index
`(cy{z}, cx{z}, timestamp)` per level, then answer "latest point per
cell in view" with a loose index scan (recursive CTE seeking to the
next occupied cell inside the cell rectangle, then one seek per cell
for its newest row). Script: cell-index-bench.py, run against a scratch
copy of the then-current export (917,766 pts; superseded by the
2.56M-point rerun in cell-index-levels.md); desktop, optimized SQLite 3.37,
warm cache, min of 3. Viewport 400x800 CSS px (iPhone-ish) centered on
the densest 152 m cell (135k pts).

| zoom | N in bounds | A: today (degree-cell GROUP BY) | B: same, int cells | C: loose scan | C + row fetch | speedup |
|------|-------------|------|------|-----|-----|------|
| z9   | 628k | 666 ms | 653 ms | 65 ms | 106 ms | 6.3x |
| z11  | 476k | 580 ms | 576 ms | 66 ms | 143 ms | 4.1x |
| z13  | 373k | 546 ms | 554 ms | 91 ms | 249 ms | 2.2x |
| z15  | 213k | 377 ms | 358 ms | 66 ms | 192 ms | 2.0x |

Cell counts agree across A/B/C within ~0.3%.

Findings:
- A == B: the cast expression is free; materializing cell coords
  (the earlier idea) would gain nothing. The scan + GROUP BY sorter is
  the whole cost.
- The loose scan is O(occupied cells), nearly flat in N (65-91 ms),
  6-10x under the scan at every zoom. The remaining cost is fetching
  the returned rows (O(cells on screen), 2 us/row), which dominates at
  z13/z15 with 60-80k cells in view and is the new floor.
- Disk: +7% for gx/gy, +13% per level index (~17 B/row); four levels
  (z9/11/13/15) = +60%, built in ~3 s total including the backfill.

Implementation notes for the real thing: inline the level literal in
the SQL (an expression index / generated column only matches the
identical expression; a bound parameter does not), or rely on the
generated-column names as here; put time-range and user filters into
the per-cell newest-row seek (walk backwards until a row passes);
antimeridian views need two x ranges; zooms between indexed levels use
the next-finer index with a small sub-cell walk; verify plans with
EXPLAIN QUERY PLAN / INDEXED BY in tests.
