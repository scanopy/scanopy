-- no-transaction
--
-- A scan's subnet targets, and the subnets a historical run swept, move out of
-- `discovery.discovery_type->'subnet_ids'`, a JSONB id list with no referential integrity, into a
-- junction with a foreign key. Expand-only: the JSONB key stays, and the server reads the lists
-- from the junction.
--
-- A deleted subnet used to leave its id in the list. The daemon skips ids its site no longer
-- holds, so a scan whose every target was deleted scanned nothing and completed clean. With
-- `ON DELETE CASCADE` the target goes with its subnet, and so does its place in the runs that
-- swept it.
--
-- Every scan sweeps the subnets its daemon has an interface on (`daemon_interfaced_subnets`). A
-- scan's rows hold only the routed subnets swept on top of them, and a scan with no rows sweeps
-- the interfaced subnets alone. A run's rows hold everything it swept, interfaced subnets
-- included.
--
-- Backfill: every discovery of a type with subnet targets gets one row per listed id whose subnet
-- still exists on its site. Not carried:
--   - Ids whose subnet is gone, or on another site. The daemon already skipped them, so they
--     targeted nothing, and a run that listed them swept nothing there.
--   - On a scan configuration, ids of its daemon's interfaced subnets. The scan keeps sweeping
--     them as interfaced subnets. A scan whose list left one of them out now sweeps it too, and a
--     scan whose list was empty (`[]`, which swept nothing) sweeps them again.
-- A run whose list is null recorded none, and gets no rows.
--
-- Historical runs are batched at 1000 with a COMMIT per batch (hence `-- no-transaction`),
-- keyset-paginated by `id`. Re-running is harmless: every statement is idempotent.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

CREATE TABLE IF NOT EXISTS discovery_subnets (
    discovery_id UUID NOT NULL REFERENCES discovery(id) ON DELETE CASCADE,
    subnet_id UUID NOT NULL REFERENCES subnets(id) ON DELETE CASCADE,
    PRIMARY KEY (discovery_id, subnet_id)
);

-- The reverse lookup a subnet delete cascades through. The PK prefix serves the forward one.
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_discovery_subnets_subnet_id ON discovery_subnets (subnet_id);

-- Scan configurations: a handful per site, so one statement. It reads every discovery row,
-- historical runs included, to pick them out.
SET statement_timeout = '120s';

INSERT INTO discovery_subnets (discovery_id, subnet_id)
SELECT DISTINCT d.id, s.id
FROM discovery d
    CROSS JOIN LATERAL jsonb_array_elements_text(d.discovery_type -> 'subnet_ids') AS elem(subnet_id)
    JOIN subnets s ON s.id = elem.subnet_id::uuid AND s.site_id = d.site_id
    LEFT JOIN daemon_interfaced_subnets dis ON dis.daemon_id = d.daemon_id AND dis.subnet_id = s.id
WHERE d.run_type ->> 'type' != 'Historical'
    AND d.discovery_type ->> 'type' IN ('Network', 'Unified')
    AND jsonb_typeof(d.discovery_type -> 'subnet_ids') = 'array'
    AND dis.subnet_id IS NULL
ON CONFLICT DO NOTHING;

-- Historical runs, in batches.
SET statement_timeout = '0';

DO $$
DECLARE
    last_id UUID := '00000000-0000-0000-0000-000000000000';
    batch UUID[];
BEGIN
    LOOP
        SELECT array_agg(id ORDER BY id)
          INTO batch
          FROM (SELECT id FROM discovery
                 WHERE id > last_id
                   AND run_type ->> 'type' = 'Historical'
                   AND discovery_type ->> 'type' IN ('Network', 'Unified')
                   AND jsonb_typeof(discovery_type -> 'subnet_ids') = 'array'
                 ORDER BY id LIMIT 1000) t;

        EXIT WHEN batch IS NULL;

        INSERT INTO discovery_subnets (discovery_id, subnet_id)
        SELECT DISTINCT d.id, s.id
        FROM discovery d
            CROSS JOIN LATERAL jsonb_array_elements_text(d.discovery_type -> 'subnet_ids') AS elem(subnet_id)
            JOIN subnets s ON s.id = elem.subnet_id::uuid AND s.site_id = d.site_id
        WHERE d.id = ANY(batch)
        ON CONFLICT DO NOTHING;

        last_id := batch[array_length(batch, 1)];
        COMMIT;
    END LOOP;
END $$;
