//! SQLite row shape for the `WatchRoot` aggregate. `Location` is flattened to
//! `location_volume` (16-byte BLOB) + `location_path` (TEXT, components joined
//! by `/`) — the same shape `media_file` uses.

use std::vec::Vec;

use crate::{
  domain::{Location, Uuid7, WatchRoot, WatchRootError},
  sqlx::{
    dto::{bytes_to_uuid7, millis_to_timestamp, timestamp_to_millis, uuid7_to_uuid},
    sqlite::leaves::{scan_status_from_i64, scan_status_to_i64},
    SqlxError,
  },
};

/// SQLite row shape for [`WatchRoot`].
///
/// Identity / FK columns are 16-byte `BLOB`s. `Location` is flattened to
/// `location_volume` (16-byte BLOB) + `location_path` (TEXT). Timestamps are
/// `INTEGER` ms-since-epoch. Boolean fields ride as `INTEGER` (0/1). The
/// `walk_status` discriminant is `0=Ok, 1=Partial, 2=Failed`; `NULL` = absent.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct SqliteWatchRootRow {
  pub id: Vec<u8>,
  /// `Location::Local` volume identity.
  pub location_volume: Vec<u8>,
  /// `Location::Local` path components joined by `/`.
  pub location_path: String,
  pub recursive: i64,
  pub enabled: i64,
  pub added_at_ms: i64,
  pub last_walked_at_ms: Option<i64>,
  /// `ScanStatus` discriminant: 0=Ok, 1=Partial, 2=Failed. NULL = absent.
  pub walk_status: Option<i64>,
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

impl From<&WatchRoot<Uuid7>> for SqliteWatchRootRow {
  fn from(w: &WatchRoot<Uuid7>) -> Self {
    let volume = match w.location_ref() {
      Location::Local(local) => *local.volume_ref(),
    };
    Self {
      id: uuid7_to_uuid(*w.id_ref()).as_bytes().to_vec(),
      location_volume: uuid7_to_uuid(volume).as_bytes().to_vec(),
      location_path: location_path(w.location_ref()),
      recursive: i64::from(w.is_recursive()),
      enabled: i64::from(w.is_enabled()),
      added_at_ms: timestamp_to_millis(*w.added_at_ref()),
      last_walked_at_ms: w.last_walked_at_ref().map(|t| timestamp_to_millis(*t)),
      walk_status: w.walk_status_ref().copied().map(scan_status_to_i64),
    }
  }
}

impl TryFrom<SqliteWatchRootRow> for WatchRoot<Uuid7> {
  type Error = SqlxError;

  fn try_from(r: SqliteWatchRootRow) -> Result<Self, Self::Error> {
    let id = bytes_to_uuid7(&r.id)?;
    let volume = bytes_to_uuid7(&r.location_volume)?;
    let added_at = millis_to_timestamp(r.added_at_ms)?;
    let location = Location::try_local_uuid7(volume, r.location_path.split('/'))
      .map_err(|e| SqlxError::DomainConstructorRejected(format!("WatchRoot.location: {e}")))?;
    let mut w = WatchRoot::try_new(id, location, added_at)
      .map_err(|e: WatchRootError| SqlxError::DomainConstructorRejected(e.to_string()))?
      .with_recursive(r.recursive != 0)
      .with_enabled(r.enabled != 0);
    if let Some(ms) = r.last_walked_at_ms {
      w = w.with_last_walked_at(Some(millis_to_timestamp(ms)?));
    }
    if let Some(s) = r.walk_status {
      w = w.with_walk_status(Some(scan_status_from_i64(s)?));
    }
    Ok(w)
  }
}

/// Borrowed view of [`SqliteWatchRootRow`] — zero-copy decode from `&'r Row`.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct SqliteWatchRootRowRef<'r> {
  pub id: &'r [u8],
  pub location_volume: &'r [u8],
  pub location_path: &'r str,
  pub recursive: i64,
  pub enabled: i64,
  pub added_at_ms: i64,
  pub last_walked_at_ms: Option<i64>,
  pub walk_status: Option<i64>,
}

impl SqliteWatchRootRow {
  /// Cheap borrow — produces a [`SqliteWatchRootRowRef`] referencing `self`.
  pub fn as_ref(&self) -> SqliteWatchRootRowRef<'_> {
    SqliteWatchRootRowRef {
      id: &self.id,
      location_volume: &self.location_volume,
      location_path: &self.location_path,
      recursive: self.recursive,
      enabled: self.enabled,
      added_at_ms: self.added_at_ms,
      last_walked_at_ms: self.last_walked_at_ms,
      walk_status: self.walk_status,
    }
  }
}

impl<'r> TryFrom<SqliteWatchRootRowRef<'r>> for WatchRoot<Uuid7> {
  type Error = SqlxError;

  fn try_from(r: SqliteWatchRootRowRef<'r>) -> Result<Self, Self::Error> {
    let id = bytes_to_uuid7(r.id)?;
    let volume = bytes_to_uuid7(r.location_volume)?;
    let added_at = millis_to_timestamp(r.added_at_ms)?;
    let location = Location::try_local_uuid7(volume, r.location_path.split('/'))
      .map_err(|e| SqlxError::DomainConstructorRejected(format!("WatchRoot.location: {e}")))?;
    let mut w = WatchRoot::try_new(id, location, added_at)
      .map_err(|e: WatchRootError| SqlxError::DomainConstructorRejected(e.to_string()))?
      .with_recursive(r.recursive != 0)
      .with_enabled(r.enabled != 0);
    if let Some(ms) = r.last_walked_at_ms {
      w = w.with_last_walked_at(Some(millis_to_timestamp(ms)?));
    }
    if let Some(s) = r.walk_status {
      w = w.with_walk_status(Some(scan_status_from_i64(s)?));
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
  use jiff::Timestamp as JiffTimestamp;

  #[test]
  fn watch_root_roundtrip() {
    let vol = Uuid7::new();
    let location = Location::try_local_uuid7(vol, ["Movies", "2024"]).unwrap();
    let w = WatchRoot::try_new(
      Uuid7::new(),
      location,
      JiffTimestamp::from_millisecond(1_700_000_000_000).unwrap(),
    )
    .unwrap()
    .with_recursive(true)
    .with_enabled(true)
    .with_walk_status(Some(ScanStatus::Partial));
    let row: SqliteWatchRootRow = (&w).into();
    assert_eq!(row.location_path, "Movies/2024");
    let w2: WatchRoot<Uuid7> = row.clone().try_into().unwrap();
    assert_eq!(w, w2);
    let w3: WatchRoot<Uuid7> = row.as_ref().try_into().unwrap();
    assert_eq!(w, w3);
  }
}
