CREATE TABLE IF NOT EXISTS watch_root (
    id                BLOB    NOT NULL PRIMARY KEY,
    location_volume   BLOB    NOT NULL,
    location_path     TEXT    NOT NULL,   -- path components joined by '/'
    recursive         INTEGER NOT NULL DEFAULT 0,
    enabled           INTEGER NOT NULL DEFAULT 0,
    added_at_ms       INTEGER NOT NULL,
    last_walked_at_ms INTEGER,
    walk_status       INTEGER,
    UNIQUE (location_volume, location_path)
);
