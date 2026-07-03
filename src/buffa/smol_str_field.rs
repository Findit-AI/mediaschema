//! [`buffa::ProtoString`] support for [`smol_str::SmolStr`] — the crate-local
//! newtype the buffa-generated wire layer uses for every proto `string` field.
//!
//! buffa 0.8 made owned string types pluggable via [`buffa::ProtoString`] and
//! dropped the `StringRepr::SmolStr` preset. The orphan rule forbids
//! implementing `ProtoString` on the foreign `smol_str::SmolStr` directly, so
//! this `#[repr(transparent)]` newtype is the bridge — crate-local because it is
//! a `repeated string` element (`Location.components`), whose `ReflectElement`
//! impls the orphan rule forbids for a foreign type.

use buffa::{DecodeError, ProtoString, WirePayload};
// Under `feature = "alloc"` (no std), `String` isn't in the prelude — pull it
// in via the `extern crate alloc as std` alias declared in `lib.rs`. Under
// `feature = "std"` it comes from the std prelude automatically; the cfg
// keeps the import a no-op there.
#[cfg(all(not(feature = "std"), feature = "alloc"))]
use std::string::String;

/// A [`buffa::ProtoString`] newtype around [`smol_str::SmolStr`].
#[derive(Clone, PartialEq, Eq, Default, Debug, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "json", serde(transparent))]
#[repr(transparent)]
pub struct SmolStrField(pub smol_str::SmolStr);

const _: () = {
  assert!(core::mem::size_of::<SmolStrField>() == core::mem::size_of::<smol_str::SmolStr>());
  assert!(core::mem::align_of::<SmolStrField>() == core::mem::align_of::<smol_str::SmolStr>());
};

impl SmolStrField {
  /// Borrow as `&str`.
  #[inline]
  #[must_use]
  pub fn as_str(&self) -> &str { self.0.as_str() }
}
impl core::ops::Deref for SmolStrField {
  type Target = str;
  #[inline]
  fn deref(&self) -> &str { self.0.as_str() }
}
impl AsRef<str> for SmolStrField {
  #[inline]
  fn as_ref(&self) -> &str { self.0.as_str() }
}
impl From<String> for SmolStrField {
  #[inline]
  fn from(s: String) -> Self { Self(smol_str::SmolStr::from(s)) }
}
impl From<&str> for SmolStrField {
  #[inline]
  fn from(s: &str) -> Self { Self(smol_str::SmolStr::from(s)) }
}
impl From<smol_str::SmolStr> for SmolStrField {
  #[inline]
  fn from(s: smol_str::SmolStr) -> Self { Self(s) }
}
impl From<SmolStrField> for smol_str::SmolStr {
  #[inline]
  fn from(s: SmolStrField) -> Self { s.0 }
}
impl From<&SmolStrField> for smol_str::SmolStr {
  #[inline]
  fn from(s: &SmolStrField) -> Self { s.0.clone() }
}
impl ProtoString for SmolStrField {
  #[inline]
  fn from_wire(payload: WirePayload<'_>) -> Result<Self, DecodeError> {
    Ok(Self(smol_str::SmolStr::from(payload.to_str()?)))
  }
}
