-- Add migration script here
CREATE TABLE IF NOT EXISTS location
(
    id          INTEGER PRIMARY KEY NOT NULL,
    lat         REAL                NOT NULL,
    lon         REAL                NOT NULL,
    accuracy    REAL                NOT NULL,
    speed       REAL                NOT NULL,
    course      REAL                NOT NULL,
    datetime    DATETIME            NOT NULL
);
