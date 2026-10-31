-- no-transaction
--
-- Proxmox VE's unreached-guest warnings become every hypervisor's.
--
-- `ProxmoxGuestOutsideScan`, `ProxmoxGuestBehindNode` and `ProxmoxGuestIsolated` are now
-- `GuestOutsideScan`, `GuestBehindHypervisor` and `GuestIsolated`, and carry the integration that
-- reported the guest. A stored `discovery_warnings` row is rewritten in place:
--
--   code                          the new code
--   details.code                  the new code
--   details.integration           "Proxmox", since only Proxmox produced them
--   details.node → .hypervisor    on `ProxmoxGuestBehindNode`, the key renamed, its value kept
--
-- Nothing is dropped. Warnings inside a discovery's historical `run_type` keep their old shape:
-- the daemon type reads both through serde aliases, so they need no rewrite.
--
-- Batched at 1000 rows with a COMMIT per batch (hence `-- no-transaction`). Re-running from any
-- partial state finishes the job: each batch selects only rows still under an old code.

SET lock_timeout = '5s';
SET statement_timeout = '0';

DO $$
DECLARE
    batch UUID[];
BEGIN
    PERFORM set_config('lock_timeout', '5s', false);
    PERFORM set_config('statement_timeout', '0', false);

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
END $$;
