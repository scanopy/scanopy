-- Two credential columns from the SSH and Wake-on-LAN work (feat/ssh-wol-credentials). Unreleased,
-- so later changes were made to this migration in place rather than in new ones.
--
-- daemon_os: the OS of the daemons that read a credential's files and sockets, stored as JSON text
-- like daemons.mode. Set only on credentials that read something on the daemon (a file-mode
-- secret or value, a daemon-side SSH script, a socket path) and NULL on the rest, where it means
-- nothing. Credential paths are validated for it, and a daemon on another OS cannot use the
-- credential. Backfill: every credential path before this column was a Unix path, so existing
-- credentials that read a daemon-side path get "Unix"; the others stay NULL. Credentials are few
-- per organization, so one UPDATE stays well under the batching threshold.
--
-- description: free-text notes, like the other entities' descriptions. Nullable, no default, no
-- backfill: there is nothing to carry forward.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE credentials ADD COLUMN IF NOT EXISTS daemon_os TEXT;
ALTER TABLE credentials ADD COLUMN IF NOT EXISTS description TEXT;

UPDATE credentials
SET daemon_os = '"Unix"'
WHERE daemon_os IS NULL
  AND (
    jsonb_path_exists(credential_type, '$.** ? (@.mode == "FilePath" || @.mode == "DaemonFile")')
    OR credential_type ->> 'socket_path' IS NOT NULL
  );
