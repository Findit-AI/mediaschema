CREATE TABLE IF NOT EXISTS watch_root (
    id                uuid    NOT NULL PRIMARY KEY,
    location_volume   uuid    NOT NULL,
    location_path     text    NOT NULL,
    recursive         boolean NOT NULL DEFAULT false,
    enabled           boolean NOT NULL DEFAULT false,
    added_at_ms       bigint  NOT NULL,
    last_walked_at_ms bigint,
    walk_status       smallint
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_watch_root_path ON watch_root(location_volume, location_path);
