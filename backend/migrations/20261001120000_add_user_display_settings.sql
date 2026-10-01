-- Per-user date and time display preferences, set from Settings → System.
-- '{}' reads as DisplaySettings::default() (every key is serde-defaulted),
-- which renders dates exactly as the UI did before this column existed.
--
-- ADD COLUMN ... DEFAULT with a constant is metadata-only in PG11+, so
-- this is safe on a populated table.

SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE users
    ADD COLUMN display_settings jsonb NOT NULL DEFAULT '{}'::jsonb;
