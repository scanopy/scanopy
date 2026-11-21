-- A scan's subnet targets move out of `discovery.discovery_type->'subnet_ids'`, a JSONB id list
-- with no referential integrity, into a junction with a foreign key. Expand-only: the JSONB key
-- stays, and the server reads targets from the junction.
--
-- A deleted subnet used to leave its id in the list. The daemon skips ids its site no longer
-- holds, so a scan whose every target was deleted scanned nothing and completed clean. With
-- `ON DELETE CASCADE` the target goes with its subnet, and a scan with no target rows reads as
-- targeting none, which covers the subnets its daemon has an interface on.
--
-- Backfill: every scan configuration (not a historical run) of a type with subnet targets gets one
-- row per listed id whose subnet still exists on the scan's site. Not carried:
--   - Ids whose subnet is gone, or on another site. The daemon already skipped them, so they
--     targeted nothing. A scan left with no rows now covers its daemon's interfaced subnets
--     instead of nothing.
--   - Historical runs. A run's `discovery_type` records what it scanned, and stays as recorded.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

CREATE TABLE discovery_subnets (
    discovery_id UUID NOT NULL REFERENCES discovery(id) ON DELETE CASCADE,
    subnet_id UUID NOT NULL REFERENCES subnets(id) ON DELETE CASCADE,
    PRIMARY KEY (discovery_id, subnet_id)
);

-- The reverse lookup a subnet delete cascades through. The PK prefix serves the forward one.
CREATE INDEX idx_discovery_subnets_subnet_id ON discovery_subnets (subnet_id);

-- Reads every discovery row, historical runs included, to pick out the configurations.
SET statement_timeout = '120s';

INSERT INTO discovery_subnets (discovery_id, subnet_id)
SELECT DISTINCT d.id, s.id
FROM discovery d
    CROSS JOIN LATERAL jsonb_array_elements_text(d.discovery_type -> 'subnet_ids') AS elem(subnet_id)
    JOIN subnets s ON s.id = elem.subnet_id::uuid AND s.site_id = d.site_id
WHERE d.run_type ->> 'type' != 'Historical'
    AND d.discovery_type ->> 'type' IN ('Network', 'Unified')
    AND jsonb_typeof(d.discovery_type -> 'subnet_ids') = 'array'
ON CONFLICT DO NOTHING;
