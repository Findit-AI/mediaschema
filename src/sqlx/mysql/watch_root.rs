//! MySQL row shape for `WatchRoot`. `Location` flattened to `location_volume`
//! (BINARY(16)) + `location_path` (TEXT) + `location_path_hash` (BINARY(32),
//! SHA-256) — MySQL can't UNIQUE-index TEXT, so the hash backs the unique key.

use std::vec::Vec;

use sha2::{Digest, Sha256};

use crate::{
  domain::{Location, Uuid7, WatchRoot, WatchRootError},
  sqlx::{
    dto::{bytes_to_uuid7, millis_to_timestamp, timestamp_to_millis, uuid7_to_uuid},
    mysql::leaves::{scan_status_from_i16, scan_status_to_i16},
    SqlxError,
  },
};

/// MySQL row shape for [`WatchRoot`].
///
/// Identity / FK columns are `BINARY(16)` (`Vec<u8>`). `Location` is flattened
/// to `location_volume` (BINARY(16)) + `location_path` (TEXT) +
/// `location_path_hash` (BINARY(32), SHA-256). Timestamps are `BIGINT`
/// ms-since-epoch. Boolean fields are `TINYINT` (0/1). The `walk_status`
/// discriminant is `0=Ok, 1=Partial, 2=Failed`; `NULL` = absent.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MySqlWatchRootRow {
  pub id: Vec<u8>,
  /// `Location::Local` volume identity.
  pub location_volume: Vec<u8>,
  /// `Location::Local` path components joined by `/`.
  pub location_path: String,
  /// SHA-256 of `location_path` (32 bytes); backs the
  /// `UNIQUE (location_volume, location_path_hash)` natural-key index.
  pub location_path_hash: Vec<u8>,
  pub recursive: i8,
  pub enabled: i8,
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

impl From<&WatchRoot<Uuid7>> for MySqlWatchRootRow {
  fn from(w: &WatchRoot<Uuid7>) -> Self {
    let volume = match w.location_ref() {
      Location::Local(l) => *l.volume_ref(),
    };
    // Build the canonical path once and hash THAT string — guarantees
    // `location_path_hash == SHA-256(location_path)` on the row.
    let location_path = location_path(w.location_ref());
    let location_path_hash = Sha256::digest(location_path.as_bytes()).to_vec();
    Self {
      id: uuid7_to_uuid(*w.id_ref()).as_bytes().to_vec(),
      location_volume: uuid7_to_uuid(volume).as_bytes().to_vec(),
      location_path,
      location_path_hash,
      recursive: i8::from(w.is_recursive()),
      enabled: i8::from(w.is_enabled()),
      added_at_ms: timestamp_to_millis(*w.added_at_ref()),
      last_walked_at_ms: w.last_walked_at_ref().map(|t| timestamp_to_millis(*t)),
      walk_status: w.walk_status_ref().copied().map(scan_status_to_i16),
    }
  }
}

impl TryFrom<MySqlWatchRootRow> for WatchRoot<Uuid7> {
  type Error = SqlxError;

  fn try_from(r: MySqlWatchRootRow) -> Result<Self, Self::Error> {
    let id = bytes_to_uuid7(&r.id)?;
    let volume = bytes_to_uuid7(&r.location_volume)?;
    let added_at = millis_to_timestamp(r.added_at_ms)?;
    // The hash carries no domain information; the domain reconstructs from
    // `location_volume` + `location_path` only. Verify width for sanity.
    if r.location_path_hash.len() != 32 {
      return Err(SqlxError::InvalidChecksum(format!(
        "WatchRoot.location_path_hash: expected 32 bytes, got {}",
        r.location_path_hash.len()
      )));
    }
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
      w = w.with_walk_status(Some(scan_status_from_i16(s)?));
    }
    Ok(w)
  }
}

/// Borrowed view of [`MySqlWatchRootRow`] — zero-copy decode from `&'r Row`.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MySqlWatchRootRowRef<'r> {
  pub id: &'r [u8],
  pub location_volume: &'r [u8],
  pub location_path: &'r str,
  pub location_path_hash: &'r [u8],
  pub recursive: i8,
  pub enabled: i8,
  pub added_at_ms: i64,
  pub last_walked_at_ms: Option<i64>,
  pub walk_status: Option<i16>,
}

impl MySqlWatchRootRow {
  /// Cheap borrow — produces a [`MySqlWatchRootRowRef`] referencing `self`.
  pub fn as_ref(&self) -> MySqlWatchRootRowRef<'_> {
    MySqlWatchRootRowRef {
      id: &self.id,
      location_volume: &self.location_volume,
      location_path: &self.location_path,
      location_path_hash: &self.location_path_hash,
      recursive: self.recursive,
      enabled: self.enabled,
      added_at_ms: self.added_at_ms,
      last_walked_at_ms: self.last_walked_at_ms,
      walk_status: self.walk_status,
    }
  }
}

impl<'r> TryFrom<MySqlWatchRootRowRef<'r>> for WatchRoot<Uuid7> {
  type Error = SqlxError;

  fn try_from(r: MySqlWatchRootRowRef<'r>) -> Result<Self, Self::Error> {
    let id = bytes_to_uuid7(r.id)?;
    let volume = bytes_to_uuid7(r.location_volume)?;
    let added_at = millis_to_timestamp(r.added_at_ms)?;
    // Hash not needed for domain reconstruction; verify width only.
    if r.location_path_hash.len() != 32 {
      return Err(SqlxError::InvalidChecksum(format!(
        "WatchRoot.location_path_hash: expected 32 bytes, got {}",
        r.location_path_hash.len()
      )));
    }
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
  use sha2::{Digest, Sha256};

  #[test]
  fn mysql_watch_root_roundtrip() {
    let vol = Uuid7::new();
    let loc = Location::try_local_uuid7(vol, ["Movies", "2024"]).unwrap();
    let w = WatchRoot::try_new(
      Uuid7::new(),
      loc,
      Timestamp::from_millisecond(1_700_000_000_000).unwrap(),
    )
    .unwrap()
    .with_enabled(true)
    .with_walk_status(Some(ScanStatus::Failed));
    let row: MySqlWatchRootRow = (&w).into();
    assert_eq!(row.location_path, "Movies/2024");
    assert_eq!(
      row.location_path_hash,
      Sha256::digest(b"Movies/2024").to_vec()
    );
    let w2: WatchRoot<Uuid7> = row.clone().try_into().unwrap();
    assert_eq!(w, w2);
  }
}
