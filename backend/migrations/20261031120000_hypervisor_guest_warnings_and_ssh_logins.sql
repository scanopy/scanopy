-- no-transaction
--
-- 1. Proxmox VE's unreached-guest warnings become every hypervisor's.
--
--    `ProxmoxGuestOutsideScan`, `ProxmoxGuestBehindNode` and `ProxmoxGuestIsolated` are now
--    `GuestOutsideScan`, `GuestBehindHypervisor` and `GuestIsolated`, and carry the integration
--    that reported the guest. A stored `discovery_warnings` row is rewritten in place:
--
--      code                          the new code
--      details.code                  the new code
--      details.integration           "Proxmox", since only Proxmox produced them
--      details.node → .hypervisor    on `ProxmoxGuestBehindNode`, the key renamed, its value kept
--
--    Warnings inside a discovery's historical `run_type` keep their old shape: the daemon type
--    reads both through serde aliases, so they need no rewrite.
--
-- 2. An SSH credential splits into its login and its script.
--
--    `SshPassword` and `SshKey` held a login (port, username, password or key and passphrase,
--    host key fingerprint) and a script (script, target_os, timeout_seconds). They are now logins
--    only, and the script is an `SshScript` credential that names its login in the new
--    `credentials.login_credential_id`. For each SSH row still holding a script:
--
--      the row itself, same id       rewritten to `SshScript` with its script, target_os and
--                                     timeout_seconds, and `login_credential_id` set. Its id
--                                     keeps its site and host assignments, tags, discovery
--                                     targets, run results and warnings, so none is re-pointed.
--      a login row                    the row's type and login fields, its organization and
--                                     daemon_os, named "<name> login". Rows of one organization
--                                     whose login fields and daemon_os are identical share one
--                                     login, named after the oldest of them.
--
--    Nothing is dropped: every field of every SSH row lands on one of the two.
--
-- Released as `downtime` (org.scanopy.deploy_mode): an older container cannot read `SshScript`,
-- or a login row with no script, so old and new containers must not run together.
--
-- Batched at 1000 rows with a COMMIT per batch (hence `-- no-transaction`). Re-running from any
-- partial state finishes the job: each batch selects only rows still under an old code or still
-- holding a script, and a login identical to one already written is reused, never duplicated.

SET lock_timeout = '5s';
SET statement_timeout = '0';

ALTER TABLE credentials ADD COLUMN IF NOT EXISTS login_credential_id UUID;

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_credentials_login_credential_id
    ON credentials (login_credential_id);

-- NO ACTION, checked at the end of each statement, so deleting a login together with the
-- credentials that use it (an organization reset, a bulk delete) succeeds. The server refuses
-- deleting a login still in use before the key would.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'credentials_login_credential_id_fkey'
    ) THEN
        ALTER TABLE credentials
            ADD CONSTRAINT credentials_login_credential_id_fkey
            FOREIGN KEY (login_credential_id) REFERENCES credentials (id) NOT VALID;
    END IF;
END $$;

DO $$
DECLARE
    batch UUID[];
    source RECORD;
    login_type JSONB;
    login_id UUID;
BEGIN
    PERFORM set_config('lock_timeout', '5s', false);
    PERFORM set_config('statement_timeout', '0', false);

    -- 1. Hypervisor guest warnings.
    LOOP
        SELECT array_agg(id) INTO batch FROM (
            SELECT id FROM discovery_warnings
             WHERE code IN ('ProxmoxGuestOutsideScan', 'ProxmoxGuestBehindNode', 'ProxmoxGuestIsolated')
             LIMIT 1000
        ) t;
        EXIT WHEN batch IS NULL;

        UPDATE discovery_warnings w
           SET code = renamed.code,
               details = CASE
                   WHEN w.code = 'ProxmoxGuestBehindNode'
                       THEN (w.details - 'node')
                            || jsonb_build_object('hypervisor', w.details->'node')
                   ELSE w.details
               END
               || jsonb_build_object('code', renamed.code, 'integration', 'Proxmox')
          FROM (VALUES
                  ('ProxmoxGuestOutsideScan', 'GuestOutsideScan'),
                  ('ProxmoxGuestBehindNode', 'GuestBehindHypervisor'),
                  ('ProxmoxGuestIsolated', 'GuestIsolated')
               ) AS renamed (old_code, code)
         WHERE w.id = ANY (batch)
           AND w.code = renamed.old_code;
        COMMIT;
    END LOOP;

    -- 2. SSH logins. Oldest first, so the oldest of the rows sharing a login names it.
    LOOP
        SELECT array_agg(id) INTO batch FROM (
            SELECT id FROM credentials
             WHERE credential_type->>'type' IN ('SshPassword', 'SshKey')
               AND credential_type ? 'script'
             ORDER BY created_at, id
             LIMIT 1000
        ) t;
        EXIT WHEN batch IS NULL;

        FOR source IN
            SELECT id, organization_id, name, credential_type, daemon_os
              FROM credentials
             WHERE id = ANY (batch)
             ORDER BY created_at, id
        LOOP
            login_type := source.credential_type - 'script' - 'target_os' - 'timeout_seconds';

            SELECT id INTO login_id
              FROM credentials
             WHERE organization_id = source.organization_id
               AND credential_type = login_type
               AND daemon_os IS NOT DISTINCT FROM source.daemon_os
             ORDER BY created_at, id
             LIMIT 1;

            IF login_id IS NULL THEN
                login_id := gen_random_uuid();
                INSERT INTO credentials (
                    id, organization_id, name, description, credential_type, daemon_os,
                    created_at, updated_at
                ) VALUES (
                    login_id,
                    source.organization_id,
                    left(source.name, 94) || ' login',
                    'The SSH login the "' || left(source.name, 100)
                        || '" credential ran over before logins became credentials of their own.',
                    login_type,
                    source.daemon_os,
                    now(),
                    now()
                );
            END IF;

            UPDATE credentials
               SET credential_type = jsonb_build_object('type', 'SshScript')
                       || jsonb_strip_nulls(jsonb_build_object(
                              'script', source.credential_type->'script',
                              'target_os', source.credential_type->'target_os',
                              'timeout_seconds', source.credential_type->'timeout_seconds'
                          )),
                   login_credential_id = login_id,
                   updated_at = now()
             WHERE id = source.id;
        END LOOP;
        COMMIT;
    END LOOP;
END $$;

-- Validating a validated constraint is a no-op, so a re-run after a failure here is safe.
-- squawk-ignore prefer-robust-stmts
ALTER TABLE credentials VALIDATE CONSTRAINT credentials_login_credential_id_fkey;
