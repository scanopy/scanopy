-- daemons.os: the operating system picked when the daemon was created in the create-daemon modal
-- (linux, macos, windows, freebsd), stored as JSON text like daemons.mode. Credentials that read
-- files or sockets on the daemon are refused for a daemon on another OS.
--
-- Nullable, no default, no backfill: the OS was never recorded before, so existing daemons stay
-- NULL and are never blocked. Nothing is dropped.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE daemons ADD COLUMN IF NOT EXISTS os TEXT;
