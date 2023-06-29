-- Adds an additional column to the location table indicating whether the data
-- was imported.

ALTER TABLE location ADD COLUMN
    was_imported                INTEGER NOT NULL DEFAULT 0
;
