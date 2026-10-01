-- The OS of the daemons that read a credential's files and sockets (SSH and Wake-on-LAN work,
-- feat/ssh-wol-credentials). Credential paths are validated for it, and a daemon on another OS
-- skips the credential with a warning.
--
-- Expand only. A constant default is a metadata-only change on PostgreSQL 11+, so existing rows
-- read as "Unix" without a rewrite or a backfill: every credential path before this column was a
-- Unix path. Stored as JSON text, like daemons.mode.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE credentials ADD COLUMN IF NOT EXISTS daemon_os TEXT NOT NULL DEFAULT '"Unix"';
