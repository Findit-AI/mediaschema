-- mediaschema — PostgreSQL migration 0002: speaker voiceprint provenance
-- backend + host platform.
--
-- Additive, upgrade-safe follow-up to 0001. Adds the inference backend +
-- host platform that the voiceprint model ran on to the `speaker` table.
-- All four columns are nullable; NULL = not recorded (decodes to
-- Backend::Unspecified / empty Platform), so rows written before this
-- migration remain valid. `0001_init.sql` is left untouched so databases
-- that already applied it pick these columns up here instead.
--
-- PostgreSQL `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` is idempotent
-- (safe to re-run; no-ops if the column already exists — available since
-- PostgreSQL 9.6).

ALTER TABLE speaker ADD COLUMN IF NOT EXISTS voiceprint_provenance_backend             smallint;
ALTER TABLE speaker ADD COLUMN IF NOT EXISTS voiceprint_provenance_platform_os         text;
ALTER TABLE speaker ADD COLUMN IF NOT EXISTS voiceprint_provenance_platform_arch       text;
ALTER TABLE speaker ADD COLUMN IF NOT EXISTS voiceprint_provenance_platform_os_version text;
