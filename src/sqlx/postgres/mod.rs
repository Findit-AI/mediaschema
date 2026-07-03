//! PostgreSQL row mapping. `Uuid7` rides as a native `uuid` column
//! (`uuid::Uuid` in sqlx). `FileChecksum` rides as `BYTEA`. Nested VOs
//! ride as `JSONB` columns — the row struct reads them as `String` for
//! a portable mapping shape; queries selecting those columns should
//! cast as `column::text` so sqlx decodes them via the `String` type
//! adapter. Wall-clock columns are `BIGINT` milliseconds-since-epoch
//! (matches the cross-backend convention; native `TIMESTAMPTZ` is a
//! tracked follow-up).
//!
//! See the module-level [`super`] doc for the cross-backend mapping
//! conventions and current coverage scope.

#[cfg(feature = "audio")]
#[cfg_attr(docsrs, doc(cfg(feature = "audio")))]
pub mod audio;
// `Attachment` is container-level (like `chapter` / `media`) — ungated.
pub mod attachment;
pub mod chapter;
// `Data` is container-level (like `chapter` / `media`) — ungated.
pub mod data;
pub mod leaves;
pub mod media;
pub mod media_file;
pub mod person;
#[cfg(feature = "subtitle")]
#[cfg_attr(docsrs, doc(cfg(feature = "subtitle")))]
pub mod subtitle;
#[cfg(feature = "video")]
#[cfg_attr(docsrs, doc(cfg(feature = "video")))]
pub mod video;
pub mod watch_root;

pub use attachment::{
  attachment_track_from_rows, PgAttachmentRow, PgAttachmentTrackIndexErrorRow,
  PgAttachmentTrackMetadataRow, PgAttachmentTrackRow,
};
#[cfg(feature = "audio")]
pub use audio::{
  audio_track_from_rows, sound_event_from_row, PgAudioRow, PgAudioSegmentRow,
  PgAudioSegmentWordRow, PgAudioTrackIndexErrorRow, PgAudioTrackMetadataRow, PgAudioTrackRow,
  PgSoundEventRow,
};
pub use chapter::{chapter_from_rows, PgChapterMetadataRow, PgChapterRow};
pub use data::{
  data_track_from_rows, PgDataRow, PgDataTrackIndexErrorRow, PgDataTrackMetadataRow, PgDataTrackRow,
};
pub use leaves::{PgSceneAnnotationRow, PgSpeakerRow, PgUserTagRow, PgWatchedLocationRow};
pub use media::PgMediaRow;
pub use media_file::PgMediaFileRow;
pub use person::PgPersonRow;
#[cfg(feature = "subtitle")]
pub use subtitle::{
  subtitle_track_from_rows, PgSubtitleCueAssRow, PgSubtitleCueBaseRow, PgSubtitleCueLrcRow,
  PgSubtitleCueLrcWordRow, PgSubtitleCueVttRow, PgSubtitleRow, PgSubtitleTrackAssStyleRow,
  PgSubtitleTrackIndexErrorRow, PgSubtitleTrackLrcMetadataRow, PgSubtitleTrackMetadataRow,
  PgSubtitleTrackRow, PgSubtitleTrackVttRegionRow, PgSubtitleTrackVttStyleRow,
};
#[cfg(feature = "video")]
pub use video::{
  cover_keyframe_from_rows, cover_keyframe_rows_from, video_track_from_rows, PgCoverKeyframeRow,
  PgCoverKeyframeRows, PgKeyframeActionRow, PgKeyframeBarcodeRow, PgKeyframeBodyPose3DJointRow,
  PgKeyframeBodyPose3DRow, PgKeyframeBodyPoseJointRow, PgKeyframeBodyPoseRow,
  PgKeyframeClassificationRow, PgKeyframeColorRow, PgKeyframeDocumentSegmentRow,
  PgKeyframeFaceLandmarkPointRow, PgKeyframeFaceLandmarkRegionRow, PgKeyframeFaceLandmarksRow,
  PgKeyframeFaceRow, PgKeyframeHandPoseRow, PgKeyframeMaskRow, PgKeyframeObjectRow, PgKeyframeRow,
  PgKeyframeRows, PgKeyframeSaliencyRow, PgKeyframeSubjectRow, PgKeyframeTextDetectionRow,
  PgKeyframeVlmLabelRow, PgSceneRow, PgVideoRow, PgVideoTrackIndexErrorRow,
  PgVideoTrackMetadataRow, PgVideoTrackRow,
};
pub use watch_root::{PgWatchRootRow, PgWatchRootRowRef};

/// Canonical PostgreSQL DDL for the mediaschema tables this revision maps.
pub const SCHEMA_SQL: &str = include_str!("schema.sql");

/// Initial migration mirror of [`SCHEMA_SQL`].
pub const MIGRATION_0001_INIT: &str = include_str!("migrations/0001_init.sql");

/// Additive migration that creates the `watch_root` table.
pub const MIGRATION_0003_WATCH_ROOT: &str = include_str!("migrations/0003_watch_root.sql");

#[cfg(all(test, feature = "video"))]
mod schema_tests {
  use super::{MIGRATION_0001_INIT, MIGRATION_0003_WATCH_ROOT, SCHEMA_SQL};

  #[test]
  fn schema_has_thumbnail_table_and_keyframe_fk() {
    // The thumbnail FK target exists with its storage discriminator.
    assert!(SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS thumbnail"));
    assert!(SCHEMA_SQL.contains("kind      text   NOT NULL"));
    // Keyframe references the thumbnail by FK.
    assert!(SCHEMA_SQL.contains("thumbnail_id              uuid   NOT NULL"));
    assert!(SCHEMA_SQL.contains("idx_keyframe_thumbnail_id"));
    // The thumbnail table is declared BEFORE keyframe (FK target first).
    let thumb = SCHEMA_SQL
      .find("CREATE TABLE IF NOT EXISTS thumbnail")
      .expect("thumbnail table present");
    let keyframe = SCHEMA_SQL
      .find("CREATE TABLE IF NOT EXISTS keyframe (")
      .expect("keyframe table present");
    assert!(thumb < keyframe, "thumbnail must precede keyframe");
  }

  #[test]
  fn schema_has_cover_keyframe_and_role_and_nullable_scene_id() {
    // The cover keyframe is a distinct table parented by video_id.
    assert!(SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS cover_keyframe ("));
    assert!(SCHEMA_SQL.contains("idx_cover_keyframe_video_id"));
    assert!(SCHEMA_SQL.contains("idx_cover_keyframe_thumbnail_id"));
    // The video facet gains the cover FK.
    assert!(SCHEMA_SQL.contains("cover_keyframe_id      uuid"));
    // The scene keyframe gains a role column and its scene_id is now
    // nullable (the cover decision-b override).
    assert!(SCHEMA_SQL.contains("role                      text   NOT NULL DEFAULT 'scene'"));
    assert!(!SCHEMA_SQL.contains("scene_id                    uuid   NOT NULL"));
  }

  #[test]
  fn migration_mirror_matches_schema() {
    // 0001 is the frozen baseline; 0003 adds the watch_root table.
    assert!(MIGRATION_0003_WATCH_ROOT.contains("CREATE TABLE IF NOT EXISTS watch_root ("));
    assert!(SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS watch_root ("));
    // schema.sql is the exact composition of 0001 + separator + 0003.
    assert_eq!(
      SCHEMA_SQL,
      format!("{MIGRATION_0001_INIT}\n{MIGRATION_0003_WATCH_ROOT}")
    );
  }
}

#[cfg(test)]
mod data_schema_tests {
  use super::{MIGRATION_0001_INIT, MIGRATION_0003_WATCH_ROOT, SCHEMA_SQL};

  #[test]
  fn schema_has_data_cluster_tables() {
    assert!(SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS data ("));
    assert!(SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS data_track ("));
    assert!(SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS data_track_metadata ("));
    assert!(SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS data_track_index_error ("));
    assert!(SCHEMA_SQL.contains("idx_data_track_data_id"));
  }

  #[test]
  fn schema_has_attachment_cluster_tables() {
    assert!(SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS attachment ("));
    assert!(SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS attachment_track ("));
    assert!(SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS attachment_track_metadata ("));
    assert!(SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS attachment_track_index_error ("));
    assert!(SCHEMA_SQL.contains("idx_attachment_track_attachment_id"));
    assert!(SCHEMA_SQL.contains("blob_uri"));
  }

  #[test]
  fn media_wires_data_and_attachment_facet_fks() {
    assert!(SCHEMA_SQL.contains("idx_media_data"));
    assert!(SCHEMA_SQL.contains("idx_media_attachment"));
  }

  #[test]
  fn data_migration_mirror_matches_schema() {
    // 0001 is the frozen baseline; 0003 adds the watch_root table.
    assert!(MIGRATION_0003_WATCH_ROOT.contains("CREATE TABLE IF NOT EXISTS watch_root ("));
    assert!(SCHEMA_SQL.contains("CREATE TABLE IF NOT EXISTS watch_root ("));
    assert_eq!(
      SCHEMA_SQL,
      format!("{MIGRATION_0001_INIT}\n{MIGRATION_0003_WATCH_ROOT}")
    );
  }
}
