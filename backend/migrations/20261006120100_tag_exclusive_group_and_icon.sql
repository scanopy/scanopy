-- Exclusive tag sets and tag icons.
--
-- `exclusive_group` names the set a tag belongs to; an entity holds at most one tag of a set.
-- The built-in application set stays in `is_application`, so existing application tags need no
-- backfill: they read back as the application set. `icon` is a lucide icon name.
--
-- Both columns are nullable and added without a default, so this is a metadata-only change.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE tags
    ADD COLUMN exclusive_group TEXT,
    ADD COLUMN icon TEXT;
