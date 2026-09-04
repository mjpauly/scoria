# Cell index levels vs view zoom

Implemented 2026-08-27 with levels 8/10/12: migrations/
20260827000000_grid_cells.sql, back/database/grid.rs,
FilteredQuery::fetch_bucketed_grid in location.rs.

2026-08-26 (rerun on the 2026-08-26 export: 2,563,861 points, 342 MB;
the first pass on the 918k export gave the same shape at ~1/3 the
absolute times). Follow-up to cell-index-bench.md. Scratch schema: u32
Mercator grid columns gx/gy, virtual generated cells
`cy{L} = gy >> (22-L)`, `cx{L} = gx >> (22-L)` (0.5 px cells at view
zoom L) and an index `(cy{L}, cx{L}, timestamp)` per level
L in {4,6,8,10,12,14,16}; queries at every view zoom z0-z18 (z0-5 added 2026-08-27, latitude
clamped to +-85 for Mercator), viewport
400x800 CSS px centered on the densest 152 m cell. Script
cell-index-levels.py, raw numbers cell-index-levels.json (per-level
query and row-fetch ms), plot cell-index-levels.jpg. Desktop,
optimized SQLite 3.37, warm cache, min of 3, one baseline for all
levels.

Mismatch handling: when the query cell is coarser than the index cell
(zoom < level), the loose walk enumerates every occupied index cell in
view and groups them by query cell. When the query cell is finer
(zoom > level), the index can only serve as a 2-D bounds filter and
the grouping needs the row coordinates, so the row-read path returns.

Speedup = today's degree-cell GROUP BY time / (new query + row fetch
by id). Bold = view zoom equals index level.

| zoom | N in view | cells on screen | today (ms) | L4 | L6 | L8 | L10 | L12 | L14 | L16 |
|---|---|---|---|---|---|---|---|---|---|---|
| z0 | 2.32M | 0k | 2581 | 14.6x | 12.2x | 8.8x | 5.4x | 3.0x | 1.9x | 1.4x |
| z1 | 2.26M | 1k | 2571 | 14.3x | 12.3x | 8.8x | 5.5x | 3.0x | 1.9x | 1.4x |
| z2 | 2.22M | 2k | 2583 | 14.5x | 12.4x | 9.0x | 5.7x | 3.2x | 1.9x | 1.4x |
| z3 | 2.11M | 3k | 2549 | 14.9x | 13.1x | 9.5x | 6.1x | 3.3x | 2.0x | 1.4x |
| z4 | 2.03M | 5k | 2478 | **14.5x** | 13.5x | 10.3x | 6.5x | 3.5x | 2.0x | 1.4x |
| z5 | 2.02M | 7k | 2471 | 4.0x | 13.6x | 10.8x | 6.6x | 3.5x | 2.1x | 1.4x |
| z6 | 1.89M | 10k | 2301 | 3.7x | **13.3x** | 11.2x | 7.3x | 3.8x | 2.1x | 1.4x |
| z7 | 1.86M | 15k | 2336 | 3.6x | 3.7x | 11.1x | 7.8x | 4.0x | 2.2x | 1.5x |
| z8 | 1.73M | 24k | 2247 | 3.4x | 3.5x | **10.2x** | 7.6x | 4.1x | 2.3x | 1.6x |
| z9 | 1.63M | 34k | 2192 | 3.4x | 3.4x | 3.4x | 7.6x | 4.5x | 2.4x | 1.6x |
| z10 | 1.40M | 38k | 1957 | 3.3x | 3.3x | 3.2x | **8.2x** | 5.2x | 2.7x | 1.7x |
| z11 | 1.30M | 57k | 1894 | 3.0x | 3.0x | 2.9x | 2.6x | 5.0x | 2.9x | 1.7x |
| z12 | 1.15M | 87k | 1793 | 2.7x | 2.7x | 2.6x | 2.3x | **4.5x** | 2.8x | 1.7x |
| z13 | 0.93M | 121k | 1623 | 2.3x | 2.4x | 2.3x | 2.1x | 1.7x | 2.6x | 1.8x |
| z14 | 0.67M | 128k | 1326 | 2.2x | 2.2x | 2.2x | 2.0x | 1.7x | **2.7x** | 1.8x |
| z15 | 0.47M | 114k | 1002 | 1.9x | 2.0x | 1.9x | 1.8x | 1.6x | 1.4x | 1.8x |
| z16 | 0.33M | 84k | 743 | 2.0x | 2.0x | 2.0x | 1.8x | 1.6x | 1.4x | **2.3x** |
| z17 | 0.28M | 80k | 629 | 1.8x | 1.9x | 1.9x | 1.7x | 1.5x | 1.4x | 1.4x |
| z18 | 0.25M | 100k | 615 | 1.6x | 1.6x | 1.6x | 1.6x | 1.4x | 1.3x | 1.3x |

Disk on this db: gx/gy +7%; each level index ~54 MB = +16% of the
342 MB base (~21 B/row); all seven levels took the file to 744 MB.

Findings:

- At the matched level the walk is 8-16x faster than today's scan
  (query only) and the total speedup is 13x at z6, 10-11x at z7-8, 8x
  at z10, 4.5-5x at z11-12, then 2-2.7x from z13 out where the row
  fetch for 80-130k on-screen cells (200-300 ms) is the
  index-independent floor. Absolute times: today 2.3 s at z6-8 and
  1.6-1.8 s at z12-13; with the matched index 170-210 ms and 400-610
  ms respectively.
- Coarser query than index (left of the square in the plot): decays
  slowly (a z10 index still gives 7.3x at z6) since the occupied index
  cells in view saturate once the view covers the data.
- Finer query than index (right of the square): drops at once to a
  flat plateau that is higher for coarser indexes and saturates at z8
  (L4/L6/L8 identical: query-only ~4x at z9-18; L10 ~3.3x, L12 ~2.9x,
  L14 ~2.6x, L16 ~2.4x). Likely mechanism: in that direction every
  qualifying index entry costs a table row read for gx/gy; within an
  index cell entries are in timestamp order, nearly rowid order, so
  the reads run sequentially through the table b-tree, and coarser
  cells mean longer sequential runs, at the floor by 153 m cells.
  Finer cells fragment the reads. The finer index's cy stripe is
  tighter (fewer wasted entries) yet slower, so read order is the
  remaining difference. Today's query is worse than all of them: the
  longitude-stripe index visits rows in longitude order (random
  w.r.t. rowid) and its sorter carries full rows.
- A free, index-less part of that plateau: carrying only
  `id, max(timestamp)` through the existing GROUP BY instead of `*`
  and fetching rows by id afterwards measured 1.5-1.8x at zoomed-out
  views on the 918k db (719 -> 410 ms at z6), neutral by z18.
- Each index level has a narrow band of maximal use (view zoom equal
  to it, or coarser zooms with no coarser index). Recommendation on
  this data (user decision 2026-08-27): z8 + z10 + z12 (+48% disk)
  covers z0-10 at 9-11x and z11-12 at 4.5-5x; nothing finer is worth
  its disk since the row fetch dominates from z13 out, and nothing
  coarser earns its space (see z0-5 below).
- z0-5 (2026-08-27): the matched-level ceiling flattens at ~14-15x
  (L4 at z0-4, L6 at z5-6); below z6 the view already holds nearly
  all data (2.0-2.3M of 2.56M), so N stops growing and the walk cost
  (~160-210 ms for L4/L6) is the floor. An L8 index gives 9-11x
  there, L10 5.5-6.6x. Gains from L4/L6 over L8 are 1.3-1.6x on
  views (continent/world) that hold the whole dataset and are rarely
  dwelt on; not worth +16% each.
- Battery: the per-query CPU time is what the walk cuts (10x fewer
  b-tree operations at the matched level, and it is the only work
  that scales with N); the price is 3 extra index inserts per logged
  point, negligible against the GPS fix that produced it. So the
  indexes are a net energy win, not just a latency one.
- The row-fetch floor (~2.4 us/row here) is the next target if z13+
  matters: fetch only the columns the tileset needs rather than `*`.

R-tree note: SQLite's R*Tree answers "which points lie in this
rectangle" scale-invariantly, but the query here is an aggregate
(newest point per cell), and an R-tree returns every point in the
view to be grouped, i.e. O(N in view), the same as today's scan with
only the 2-D-filter gain (the longitude stripe is already within
~2-20% of the true in-bounds count, see the stripe column in the
json). The per-level index wins because (cell, timestamp) order lets
the walk skip all but one row per cell, which needs the cell grid in
the key. A structure that is both scale-invariant and aggregate-aware
is a quadtree/mipmap of per-node newest points, which is the
side-table design rejected earlier for being invalid under time-range
and user filters.
