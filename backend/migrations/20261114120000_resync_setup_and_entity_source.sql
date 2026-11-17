-- no-transaction
--
-- Re-applies 20261108120000's data steps to rows written after it ran. v0.20.0 applied it on
-- 2026-11-13 and was rolled back to v0.19.6, which kept running on the migrated schema. v0.19.6
-- writes Setup milestones only to `organizations.onboarding`, and creates starter tags and seeded
-- credentials with the column default `source = {"type": "Manual"}`. Expand-only: nothing is
-- dropped or removed.
--
--   organizations.setup  gains every `onboarding` value it doesn't already hold. Values already in
--                        `setup` stay. `SetupItemDiscriminants::parse_recorded` reads old milestone
--                        names through serde aliases and drops duplicates, so an old name next to
--                        its new one reads as one item.
--   tags.source,         set to {"type": "System"} on rows still Manual that match 20261108120000's
--   credentials.source   starter-tag and seeded-credential rules, unchanged. `tags` is SCD2: every
--                        row version is updated (live, closed and snapshot), and no validity,
--                        lineage or snapshot column is touched.
--
-- Every step runs in batches of 1000 rows with a COMMIT per batch, keyset-paginated by `id`, and
-- is idempotent: a second run changes nothing.

SET lock_timeout = '5s';
SET statement_timeout = '0';

-- Milestones v0.19.6 wrote to `onboarding` only.
DO $$
DECLARE
    last_id UUID := '00000000-0000-0000-0000-000000000000';
    batch UUID[];
BEGIN
    LOOP
        SELECT array_agg(id ORDER BY id)
          INTO batch
          FROM (SELECT id FROM organizations
                 WHERE id > last_id
                 ORDER BY id LIMIT 1000) o;

        EXIT WHEN batch IS NULL;

        UPDATE organizations o
           SET setup = o.setup || (
                   SELECT jsonb_agg(DISTINCT v)
                     FROM jsonb_array_elements(o.onboarding) v
                    WHERE NOT o.setup @> jsonb_build_array(v))
         WHERE o.id = ANY(batch)
           AND jsonb_typeof(o.onboarding) = 'array'
           AND EXISTS (SELECT 1
                         FROM jsonb_array_elements(o.onboarding) v
                        WHERE NOT o.setup @> jsonb_build_array(v));

        last_id := batch[array_length(batch, 1)];
        COMMIT;
    END LOOP;
END $$;

-- Starter tags, by group and name (20261108120000's rule).
DO $$
DECLARE
    last_id UUID := '00000000-0000-0000-0000-000000000000';
    batch UUID[];
BEGIN
    LOOP
        SELECT array_agg(id ORDER BY id)
          INTO batch
          FROM (SELECT t.id
                  FROM tags t
                  JOIN tag_groups g ON g.id = t.tag_group_id
                 WHERE t.id > last_id
                   AND t.source->>'type' = 'Manual'
                   AND (
                        (g.group_type = 'Status'
                            AND t.name IN ('Planned', 'Active', 'Decommissioning', 'Decommissioned'))
                     OR (g.group_type = 'Custom' AND lower(g.name) = 'environment'
                            AND t.name IN ('Production', 'Staging', 'Development', 'Lab'))
                     OR (g.group_type = 'Custom' AND lower(g.name) = 'criticality'
                            AND t.name IN ('Critical', 'High', 'Medium', 'Low'))
                     OR (g.group_type = 'Custom' AND lower(g.name) = 'management'
                            AND t.name IN ('Managed', 'Unmanaged', 'Third-party'))
                     OR (g.group_type = 'Visibility'
                            AND t.name IN ('Hidden', 'Hidden (topology)'))
                   )
                 ORDER BY t.id LIMIT 1000) s;

        EXIT WHEN batch IS NULL;

        UPDATE tags SET source = '{"type": "System"}'::jsonb WHERE id = ANY(batch);

        last_id := batch[array_length(batch, 1)];
        COMMIT;
    END LOOP;
END $$;

-- Seeded default credentials, by name and type (20261108120000's rule).
DO $$
DECLARE
    last_id UUID := '00000000-0000-0000-0000-000000000000';
    batch UUID[];
BEGIN
    LOOP
        SELECT array_agg(id ORDER BY id)
          INTO batch
          FROM (SELECT id FROM credentials
                 WHERE id > last_id
                   AND source->>'type' = 'Manual'
                   AND (
                        (name = 'Docker socket' AND credential_type->>'type' = 'DockerSocket')
                     OR (name = 'Podman socket' AND credential_type->>'type' = 'PodmanSocket')
                     OR (name = 'SNMP (public)' AND credential_type->>'type' = 'SnmpCommunity')
                   )
                 ORDER BY id LIMIT 1000) c;

        EXIT WHEN batch IS NULL;

        UPDATE credentials SET source = '{"type": "System"}'::jsonb WHERE id = ANY(batch);

        last_id := batch[array_length(batch, 1)];
        COMMIT;
    END LOOP;
END $$;
