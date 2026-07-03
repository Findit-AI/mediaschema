# `WatchRoot<Id>` — a monitored folder/prefix  *(rev 1 — LOCKED, user-approved)*

## Domain meaning

A folder (or object-storage prefix) the **monitor watches for changes**. The
thing the `watch --dir` verb creates and the daemon reloads and resumes on
startup (reconcile-then-watch). Distinct from [`WatchedLocation`](watched_location.md):
`WatchedLocation` is volume-scoped (one entry per storage volume); `WatchRoot`
is **folder-scoped** — the monitored target is a full `Location` (volume +
within-volume path components). This allows watching `/Volumes/MyDrive/Movies`
independently of `/Volumes/MyDrive/Recordings` on the same volume.

`Location` is `#[non_exhaustive]` — object-storage bucket prefixes are a
future `WatchRoot` variant with no structural change to this schema.

## Folder-scoped (vs `WatchedLocation` volume-scoped)

A `WatchRoot` targets a **`Location`**: a volume UUID plus a non-empty sequence
of path components. Two `WatchRoot`s on the same volume at different paths are
fully representable and independent. The schema enforces uniqueness at the
`(location_volume, location_path)` pair — one watch per (volume, path).

The `enabled` flag allows pausing a watch without deleting it; `recursive`
controls whether subdirectories are descended.

## Fields

| Field              | Type                  | Default  | Notes                                                      |
|--------------------|-----------------------|----------|------------------------------------------------------------|
| `id`               | `Uuid7`               | —        | Stable row identity; nil rejected by `try_new`.            |
| `location`         | `Location<Uuid7>`     | —        | Volume + path; validated at `Location::try_local_uuid7`.   |
| `recursive`        | `bool`                | `false`  | Descend subdirectories.                                    |
| `enabled`          | `bool`                | `false`  | Actively watched vs paused.                                |
| `added_at`         | `Timestamp`           | —        | Wall-clock when the watch was configured.                  |
| `last_walked_at`   | `Option<Timestamp>`   | `None`   | When the last full reconcile walk completed.               |
| `walk_status`      | `Option<ScanStatus>`  | `None`   | Status of that walk (Ok / Partial / Failed).               |

## Invariants

- `id` must not be the nil UUID (enforced by `WatchRoot::try_new` → `WatchRootError::NilId`).
- `location` is validated by `Location::try_local_uuid7`: non-nil volume, non-empty path.
- No `Default` impl — a nil id + empty location is not a real watch root.

## Projection notes

Persisted as the `watch_root` table. `location` is **flattened** to two
columns — `location_volume BLOB` (16-byte UUID) + `location_path TEXT` (path
components joined by `/`) — mirroring the `media_file` shape. A
`UNIQUE(location_volume, location_path)` constraint enforces one watch per
(volume, path). SQLite indexes TEXT natively; the MySQL dialect adds a
`location_path_hash BINARY(32)` side-car for its InnoDB prefix-length
constraint.

Migration `0003_watch_root.sql` creates the table additively; `SCHEMA_SQL`
already includes it for fresh creates.

---

*Status: LOCKED (rev 1)*
