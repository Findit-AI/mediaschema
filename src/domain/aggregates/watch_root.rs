//! `WatchRoot` — a folder/prefix the monitor watches for changes.
//!
//! Folder-scoped (a `Location`: volume + within-volume path), distinct from
//! the volume-scoped `WatchedLocation`. The thing the `watch --dir` verb
//! creates and the daemon reloads + resumes on startup. `Location` is
//! `#[non_exhaustive]` (object-storage variants later), so a bucket prefix is
//! a future `WatchRoot` with no shape change. See `schema/watch_root.md`.

use derive_more::{IsVariant, TryUnwrap, Unwrap};
use jiff::Timestamp;

use crate::domain::{Location, ScanStatus, Uuid7};

/// A monitored folder/prefix.
///
/// Generic over `Id` (default [`Uuid7`]). **No `Default`** — a nil id + empty
/// location is not a real watch. Construct via [`WatchRoot::try_new`]; fields
/// are private (accessors + `with_*`/`set_*` builders below).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WatchRoot<Id = Uuid7> {
  id: Id,
  /// The watched location — volume + within-volume path. Validated at
  /// `Location` construction (non-empty path; nil volume rejected for `Uuid7`).
  location: Location<Id>,
  recursive: bool,
  enabled: bool,
  added_at: Timestamp,
  last_walked_at: Option<Timestamp>,
  walk_status: Option<ScanStatus>,
}

impl WatchRoot<Uuid7> {
  /// Validating constructor. Rejects a nil `id` (the `location` is already
  /// validated by [`Location::try_local_uuid7`]). Defaults `recursive=false`,
  /// `enabled=false`; caller flips via `with_*`.
  pub fn try_new(
    id: Uuid7,
    location: Location<Uuid7>,
    added_at: Timestamp,
  ) -> Result<Self, WatchRootError> {
    if id.is_nil() {
      return Err(WatchRootError::NilId);
    }
    Ok(Self {
      id,
      location,
      recursive: false,
      enabled: false,
      added_at,
      last_walked_at: None,
      walk_status: None,
    })
  }
}

impl<Id> WatchRoot<Id> {
  /// Canonical identity.
  #[inline(always)]
  pub const fn id_ref(&self) -> &Id {
    &self.id
  }

  /// The watched location.
  #[inline(always)]
  pub const fn location_ref(&self) -> &Location<Id> {
    &self.location
  }

  /// Descend subdirectories.
  #[inline(always)]
  pub const fn is_recursive(&self) -> bool {
    self.recursive
  }

  /// Actively watched (vs paused).
  #[inline(always)]
  pub const fn is_enabled(&self) -> bool {
    self.enabled
  }

  /// When this watch was configured.
  #[inline(always)]
  pub const fn added_at_ref(&self) -> &Timestamp {
    &self.added_at
  }

  /// Last full reconcile walk.
  #[inline(always)]
  pub const fn last_walked_at_ref(&self) -> Option<&Timestamp> {
    self.last_walked_at.as_ref()
  }

  /// Status of that walk.
  #[inline(always)]
  pub const fn walk_status_ref(&self) -> Option<&ScanStatus> {
    self.walk_status.as_ref()
  }

  /// Builder: replace `recursive`.
  #[inline(always)]
  #[must_use]
  pub const fn with_recursive(mut self, recursive: bool) -> Self {
    self.recursive = recursive;
    self
  }

  /// Builder: replace `enabled`.
  #[inline(always)]
  #[must_use]
  pub const fn with_enabled(mut self, enabled: bool) -> Self {
    self.enabled = enabled;
    self
  }

  /// Builder: replace `last_walked_at`.
  #[inline(always)]
  #[must_use]
  pub fn with_last_walked_at(mut self, t: Option<Timestamp>) -> Self {
    self.last_walked_at = t;
    self
  }

  /// Builder: replace `walk_status`.
  #[inline(always)]
  #[must_use]
  pub fn with_walk_status(mut self, s: Option<ScanStatus>) -> Self {
    self.walk_status = s;
    self
  }

  /// In-place mutator for `recursive`.
  #[inline(always)]
  pub const fn set_recursive(&mut self, recursive: bool) -> &mut Self {
    self.recursive = recursive;
    self
  }

  /// In-place mutator for `enabled`.
  #[inline(always)]
  pub const fn set_enabled(&mut self, enabled: bool) -> &mut Self {
    self.enabled = enabled;
    self
  }

  /// In-place mutator for `last_walked_at`.
  #[inline(always)]
  pub fn set_last_walked_at(&mut self, t: Option<Timestamp>) -> &mut Self {
    self.last_walked_at = t;
    self
  }

  /// In-place mutator for `walk_status`.
  #[inline(always)]
  pub fn set_walk_status(&mut self, s: Option<ScanStatus>) -> &mut Self {
    self.walk_status = s;
    self
  }
}

/// Error returned when [`WatchRoot::try_new`] cannot uphold the non-nil-id
/// invariant. Unit-only enum; derives `IsVariant` plus `Unwrap`/`TryUnwrap`
/// with shared-ref + mut-ref accessor flavours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IsVariant, Unwrap, TryUnwrap, thiserror::Error)]
#[unwrap(ref, ref_mut)]
#[try_unwrap(ref, ref_mut)]
#[non_exhaustive]
pub enum WatchRootError {
  /// The supplied `id` was the nil sentinel — not a real identity.
  #[error("WatchRoot id must not be the nil UUID")]
  NilId,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
  use super::*;
  use crate::domain::Location;
  use jiff::Timestamp as JiffTimestamp;

  fn loc() -> Location<Uuid7> {
    Location::try_local_uuid7(Uuid7::new(), ["Movies", "2024"]).unwrap()
  }

  #[test]
  fn try_new_rejects_nil_id() {
    // Uuid7::nil() is pub(crate); this test lives in-crate so it can name it.
    let e = WatchRoot::try_new(Uuid7::nil(), loc(), JiffTimestamp::default());
    assert_eq!(e.unwrap_err(), WatchRootError::NilId);
  }

  #[test]
  fn builders_roundtrip() {
    let w = WatchRoot::try_new(Uuid7::new(), loc(), JiffTimestamp::default())
      .unwrap()
      .with_recursive(true)
      .with_enabled(true)
      .with_walk_status(Some(ScanStatus::Ok));
    assert!(w.is_recursive());
    assert!(w.is_enabled());
    assert_eq!(w.walk_status_ref(), Some(&ScanStatus::Ok));
    assert!(w.location_ref().is_local());
  }
}
