CREATE TABLE IF NOT EXISTS watch_root (
    id                 BINARY(16) NOT NULL,
    location_volume    BINARY(16) NOT NULL,
    location_path      TEXT       NOT NULL,
    location_path_hash BINARY(32) NOT NULL,
    recursive          TINYINT    NOT NULL DEFAULT 0,
    enabled            TINYINT    NOT NULL DEFAULT 0,
    added_at_ms        BIGINT     NOT NULL,
    last_walked_at_ms  BIGINT,
    walk_status        SMALLINT,
    PRIMARY KEY (id),
    UNIQUE KEY idx_watch_root_path        (location_volume, location_path_hash),
    KEY        idx_watch_root_path_prefix (location_volume, location_path)
);
