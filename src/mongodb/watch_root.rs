//! `WatchRoot` ↔ bson `Document` mapping. `Location` is a nested
//! `{kind, volume, components}` sub-document (via `util::location_*_bson`).

use ::bson::{Bson, Document};

use super::{
  error::MongoError,
  leaves::{scan_status_from_i64, scan_status_to_i32},
  util::{
    as_bool, as_i64, jiff_from_bson, jiff_to_bson, location_from_bson, location_to_bson, take,
    take_opt, uuid7_from_bson, uuid7_to_bson,
  },
};
use crate::domain::{Uuid7, WatchRoot};

impl From<&WatchRoot<Uuid7>> for Document {
  fn from(w: &WatchRoot<Uuid7>) -> Self {
    let mut d = Document::new();
    d.insert("_id", uuid7_to_bson(*w.id_ref()));
    d.insert("location", location_to_bson(w.location_ref()));
    d.insert("recursive", Bson::Boolean(w.is_recursive()));
    d.insert("enabled", Bson::Boolean(w.is_enabled()));
    d.insert("added_at", jiff_to_bson(*w.added_at_ref()));
    d.insert(
      "last_walked_at",
      w.last_walked_at_ref()
        .map(|t| jiff_to_bson(*t))
        .unwrap_or(Bson::Null),
    );
    d.insert(
      "walk_status",
      w.walk_status_ref()
        .map(|s| Bson::Int32(scan_status_to_i32(*s)))
        .unwrap_or(Bson::Null),
    );
    d
  }
}

impl TryFrom<Document> for WatchRoot<Uuid7> {
  type Error = MongoError;

  fn try_from(mut d: Document) -> Result<Self, Self::Error> {
    let id = uuid7_from_bson(take(&mut d, "_id")?, "_id")?;
    let location = location_from_bson(take(&mut d, "location")?, "location")?;
    let added_at = jiff_from_bson(take(&mut d, "added_at")?, "added_at")?;
    let mut w = WatchRoot::try_new(id, location, added_at)?; // MongoError: WatchRoot(#[from])
    if let Some(b) = take_opt(&mut d, "recursive") {
      w.set_recursive(as_bool(b, "recursive")?);
    }
    if let Some(b) = take_opt(&mut d, "enabled") {
      w.set_enabled(as_bool(b, "enabled")?);
    }
    if let Some(b) = take_opt(&mut d, "last_walked_at") {
      w.set_last_walked_at(Some(jiff_from_bson(b, "last_walked_at")?));
    }
    if let Some(b) = take_opt(&mut d, "walk_status") {
      let v = as_i64(b, "walk_status")?;
      w.set_walk_status(Some(scan_status_from_i64(v, "walk_status")?));
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
  fn mongo_watch_root_roundtrip() {
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
    .with_walk_status(Some(ScanStatus::Partial));
    let doc: Document = (&w).into();
    let w2: WatchRoot<Uuid7> = doc.try_into().unwrap();
    assert_eq!(w, w2);
  }
}
