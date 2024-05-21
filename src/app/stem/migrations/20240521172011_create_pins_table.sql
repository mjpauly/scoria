-- User-saved map pins (locations).

CREATE TABLE IF NOT EXISTS pins (
    id                          INTEGER NOT NULL PRIMARY KEY,
    lng                         REAL NOT NULL,
    lat                         REAL NOT NULL,
    name                        TEXT NOT NULL,
    icon                        TEXT NOT NULL,
    lists                       TEXT NOT NULL,
    tags                        TEXT NOT NULL,
    boundary                    TEXT
) STRICT;
