-- Add an index on (longitude, timestamp) so view-bounded queries are fast and
-- feel responsive. Including the timestamp as a second column makes queries
-- that are also time-bounded faster. Space-bounded responsiveness is higher
-- priority because 1) it is easier to be selective with the view bounds than
-- it is with the time range, 2) changing view bounds is more interactive,
-- and 3) large space range but small time range queries usually capture
-- stationary locations which have lots of points, and aren't very selective
-- anyways.
CREATE INDEX lngtimeindex ON location(longitude, timestamp);
