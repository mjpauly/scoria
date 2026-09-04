-- Web Mercator grid coordinates per point: gx/gy are u32 positions over the
-- world (2^32 divisions per axis, ~9 mm at the equator), computed in Rust at
-- insert. Existing rows are NULL until the resumable backfill in
-- database/grid.rs fills them, which database::open_db runs before the next
-- migration builds the per-level indexes (cy{L}, cx{L}, timestamp).
--
-- The virtual cells are the 0.5 px cell at view zoom L (2^(L+10) divisions
-- per axis, i.e. gx >> (22 - L)). With a (cell, timestamp) index the spatial
-- decimation query becomes a loose index scan, O(occupied cells on screen)
-- instead of O(points in view): 8-13x at the matched zoom on real data
-- (doc/decimation/probe-results/cell-index-levels.md). Levels 8/10/12 cover
-- world-to-neighborhood zooms; finer levels gain nothing since the row fetch
-- for the cells on screen dominates there. Virtual columns cost no storage;
-- each index is ~16% of the table.
ALTER TABLE location ADD COLUMN gx INTEGER;
ALTER TABLE location ADD COLUMN gy INTEGER;
ALTER TABLE location ADD COLUMN cy8 INTEGER GENERATED ALWAYS AS (gy >> 14) VIRTUAL;
ALTER TABLE location ADD COLUMN cx8 INTEGER GENERATED ALWAYS AS (gx >> 14) VIRTUAL;
ALTER TABLE location ADD COLUMN cy10 INTEGER GENERATED ALWAYS AS (gy >> 12) VIRTUAL;
ALTER TABLE location ADD COLUMN cx10 INTEGER GENERATED ALWAYS AS (gx >> 12) VIRTUAL;
ALTER TABLE location ADD COLUMN cy12 INTEGER GENERATED ALWAYS AS (gy >> 10) VIRTUAL;
ALTER TABLE location ADD COLUMN cx12 INTEGER GENERATED ALWAYS AS (gx >> 10) VIRTUAL;
