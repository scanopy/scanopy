-- Two credential columns from the SSH and Wake-on-LAN work (feat/ssh-wol-credentials). Unreleased,
-- so the description column was added to this migration in place rather than in a new one.
--
-- daemon_os: the OS of the daemons that read a credential's files and sockets. Credential paths
-- are validated for it, and a daemon on another OS skips the credential with a warning. A constant
-- default is a metadata-only change on PostgreSQL 11+, so existing rows read as "Unix" without a
-- rewrite or a backfill: every credential path before this column was a Unix path. Stored as JSON
-- text, like daemons.mode.
--
-- description: free-text notes, like the other entities' descriptions. Nullable, no default, no
-- backfill: there is nothing to carry forward.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE credentials ADD COLUMN IF NOT EXISTS daemon_os TEXT NOT NULL DEFAULT '"Unix"';
ALTER TABLE credentials ADD COLUMN IF NOT EXISTS description TEXT;
