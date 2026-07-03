//! Postgres row shape for `WatchRoot`. `Location` flattened to
//! `location_volume` (uuid) + `location_path` (text, components joined `/`).

use uuid::Uuid;

use crate::{
  domain::{Location, Uuid7, WatchRoot, WatchRootError},
  sqlx::{
    dto::{millis_to_timestamp, timestamp_to_millis, uuid7_to_uuid, uuid_to_uuid7},
    postgres::leaves::{scan_status_from_i16, scan_status_to_i16},
    SqlxError,
  },
};

/// Postgres row shape for [`WatchRoot`].
///
/// Identity / FK columns are native `uuid` (`uuid::Uuid`). `Location` is
/// flattened to `location_volume` (uuid) + `location_path` (text, components
/// joined `/`). Timestamps are `BIGINT` ms-since-epoch. Boolean fields are
/// native `boolean`. The `walk_status` discriminant is `0=Ok, 1=Partial,
/// 2=Failed`; `NULL` = absent.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct PgWatchRootRow {
  pub id: Uuid,
  pub location_volume: Uuid,
  /// `Location::Local` path components joined by `/`.
  pub location_path: String,
  pub recursive: bool,
  pub enabled: bool,
  pub added_at_ms: i64,
  pub last_walked_at_ms: Option<i64>,
  /// `ScanStatus` discriminant: 0=Ok, 1=Partial, 2=Failed. NULL = absent.
  pub walk_status: Option<i16>,
}

/// Join a `Location`'s path components with `/` for storage.
fn location_path(location: &Location<Uuid7>) -> String {
  match location {
    Location::Local(local) => local
      .components_slice()
      .iter()
      .map(AsRef::as_ref)
      .collect::<Vec<&str>>()
      .join("/"),
  }
}

impl From<&WatchRoot<Uuid7>> for PgWatchRootRow {
  fn from(w: &WatchRoot<Uuid7>) -> Self {
    let volume = match w.location_ref() {
      Location::Local(l) => *l.volume_ref(),
    };
    Self {
      id: uuid7_to_uuid(*w.id_ref()),
      location_volume: uuid7_to_uuid(volume),
      location_path: location_path(w.location_ref()),
      recursive: w.is_recursive(),
      enabled: w.is_enabled(),
      added_at_ms: timestamp_to_millis(*w.added_at_ref()),
      last_walked_at_ms: w.last_walked_at_ref().map(|t| timestamp_to_millis(*t)),
      walk_status: w.walk_status_ref().copied().map(scan_status_to_i16),
    }
  }
}

impl TryFrom<PgWatchRootRow> for WatchRoot<Uuid7> {
  type Error = SqlxError;

  fn try_from(r: PgWatchRootRow) -> Result<Self, Self::Error> {
    let id = uuid_to_uuid7(r.id)?;
    let volume = uuid_to_uuid7(r.location_volume)?;
    let added_at = millis_to_timestamp(r.added_at_ms)?;
    let location = Location::try_local_uuid7(volume, r.location_path.split('/'))
      .map_err(|e| SqlxError::DomainConstructorRejected(format!("WatchRoot.location: {e}")))?;
    let mut w = WatchRoot::try_new(id, location, added_at)
      .map_err(|e: WatchRootError| SqlxError::DomainConstructorRejected(e.to_string()))?
      .with_recursive(r.recursive)
      .with_enabled(r.enabled);
    if let Some(ms) = r.last_walked_at_ms {
      w = w.with_last_walked_at(Some(millis_to_timestamp(ms)?));
    }
    if let Some(s) = r.walk_status {
      w = w.with_walk_status(Some(scan_status_from_i16(s)?));
    }
    Ok(w)
  }
}

/// Borrowed view of [`PgWatchRootRow`] — zero-copy decode from `&'r Row`.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct PgWatchRootRowRef<'r> {
  pub id: Uuid,
  pub location_volume: Uuid,
  pub location_path: &'r str,
  pub recursive: bool,
  pub enabled: bool,
  pub added_at_ms: i64,
  pub last_walked_at_ms: Option<i64>,
  pub walk_status: Option<i16>,
}

impl PgWatchRootRow {
  /// Cheap borrow — produces a [`PgWatchRootRowRef`] referencing `self`.
  pub fn as_ref(&self) -> PgWatchRootRowRef<'_> {
    PgWatchRootRowRef {
      id: self.id,
      location_volume: self.location_volume,
      location_path: &self.location_path,
      recursive: self.recursive,
      enabled: self.enabled,
      added_at_ms: self.added_at_ms,
      last_walked_at_ms: self.last_walked_at_ms,
      walk_status: self.walk_status,
    }
  }
}

impl<'r> TryFrom<PgWatchRootRowRef<'r>> for WatchRoot<Uuid7> {
  type Error = SqlxError;

  fn try_from(r: PgWatchRootRowRef<'r>) -> Result<Self, Self::Error> {
    let id = uuid_to_uuid7(r.id)?;
    let volume = uuid_to_uuid7(r.location_volume)?;
    let added_at = millis_to_timestamp(r.added_at_ms)?;
    let location = Location::try_local_uuid7(volume, r.location_path.split('/'))
      .map_err(|e| SqlxError::DomainConstructorRejected(format!("WatchRoot.location: {e}")))?;
    let mut w = WatchRoot::try_new(id, location, added_at)
      .map_err(|e: WatchRootError| SqlxError::DomainConstructorRejected(e.to_string()))?
      .with_recursive(r.recursive)
      .with_enabled(r.enabled);
    if let Some(ms) = r.last_walked_at_ms {
      w = w.with_last_walked_at(Some(millis_to_timestamp(ms)?));
    }
    if let Some(s) = r.walk_status {
      w = w.with_walk_status(Some(scan_status_from_i16(s)?));
    }
    Ok(w)
  }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::{Location, ScanStatus};
  use jiff::Timestamp;

  #[test]
  fn pg_watch_root_roundtrip() {
    let vol = Uuid7::new();
    let loc = Location::try_local_uuid7(vol, ["Movies", "2024"]).unwrap();
    let w = WatchRoot::try_new(
      Uuid7::new(),
      loc,
      Timestamp::from_millisecond(1_700_000_000_000).unwrap(),
    )
    .unwrap()
    .with_recursive(true)
    .with_enabled(true)
    .with_walk_status(Some(ScanStatus::Ok));
    let row: PgWatchRootRow = (&w).into();
    assert_eq!(row.location_path, "Movies/2024");
    assert!(row.recursive);
    let w2: WatchRoot<Uuid7> = row.clone().try_into().unwrap();
    assert_eq!(w, w2);
    let w3: WatchRoot<Uuid7> = row.as_ref().try_into().unwrap();
    assert_eq!(w, w3);
  }
}
