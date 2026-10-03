--
-- A host's operating system, with the source that produced it.
--
-- `os` holds a JSON object: a `family` (one of `HostOsFamily`) and optionally `name`, `version`,
-- `edition`, `codename` and `kernel_version`. It is written by the daemon reading its own host, by
-- an SSH credential's script, and by matching SNMP `sysDescr` / `sysObjectID` against the
-- vendored Recog fingerprints. jsonb because the value is a struct, not a string; `NULL` when no
-- source has named an OS.
--
-- The `_source` sibling follows every other discovered attribute (20260828120000): the same
-- `Unspecified` default, displaced by the first real reading.
--
-- No backfill. Stored `sys_descr` values are not re-matched here; each host gets an OS from its
-- next scan, from whichever source reads or matches one. Nothing is dropped.
--
-- Additive and prod-safe: both are `ADD COLUMN` with a non-volatile default, which is
-- metadata-only on PG11+ (no rewrite). Older servers ignore both columns.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE hosts ADD COLUMN IF NOT EXISTS os JSONB;
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS os_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;
