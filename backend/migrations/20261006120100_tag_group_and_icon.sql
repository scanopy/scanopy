-- Tag groups and tag icons.
--
-- `tag_group` names the group a tag belongs to; an entity holds at most one tag of a group.
-- The built-in application group stays in `is_application`, so existing application tags need no
-- backfill: they read back as the application group. `icon` is a lucide icon name.
--
-- Both columns are nullable and added without a default, so this is a metadata-only change.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE tags
    ADD COLUMN tag_group TEXT,
    ADD COLUMN icon TEXT;
