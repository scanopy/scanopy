-- no-transaction
--
-- Rewrite the stored JSON values that name the Network entity, after 20261005120000 renamed its
-- tables and columns. Each value moves to its new name; none is dropped.
--
--   entity_tags.entity_type              '"Network"'                 -> '"Site"'
--   organizations.onboarding             "SecondNetworkCreated"      -> "SecondSiteCreated"
--   organizations.plan,
--     organizations.last_downgrade_from_plan
--                                        included_networks, network_cents
--                                                                    -> included_sites, site_cents
--   organizations.notifications          networks                    -> sites
--   discovery.integration_targets[]      {"scope": "Network"}        -> {"scope": "Site"}
--   discovery.run_type.results           network_id                  -> site_id
--
-- `entity_tags` is SCD2: the update covers live, closed and snapshot rows, and touches no
-- validity or lineage column. `run_type.results` is the final progress of a Historical run.
--
-- Only the plan type still accepts the old key names, because Stripe subscription metadata
-- carries them. Every other value here is read under its new name only.
--
-- Batched at 1000 rows with a COMMIT per batch (hence `-- no-transaction`), keyset-paginated by
-- `id`. Re-running is harmless: rewritten rows no longer match.

SET lock_timeout = '5s';
SET statement_timeout = '0';

DO $$
DECLARE
    last_id UUID := '00000000-0000-0000-0000-000000000000';
    batch UUID[];
BEGIN
    LOOP
        SELECT array_agg(id ORDER BY id)
          INTO batch
          FROM (SELECT id FROM entity_tags
                 WHERE id > last_id AND entity_type = '"Network"'
                 ORDER BY id LIMIT 1000) t;

        EXIT WHEN batch IS NULL;

        UPDATE entity_tags SET entity_type = '"Site"' WHERE id = ANY(batch);

        last_id := batch[array_length(batch, 1)];
        COMMIT;
    END LOOP;
END $$;

DO $$
DECLARE
    last_id UUID := '00000000-0000-0000-0000-000000000000';
    batch UUID[];
BEGIN
    LOOP
        SELECT array_agg(id ORDER BY id)
          INTO batch
          FROM (SELECT id FROM organizations WHERE id > last_id ORDER BY id LIMIT 1000) t;

        EXIT WHEN batch IS NULL;

        UPDATE organizations o
           SET onboarding = CASE
                   WHEN o.onboarding @> '["SecondNetworkCreated"]'::jsonb THEN (
                       SELECT jsonb_agg(
                                  CASE WHEN e = '"SecondNetworkCreated"'::jsonb
                                       THEN '"SecondSiteCreated"'::jsonb ELSE e END
                                  ORDER BY ord)
                         FROM jsonb_array_elements(o.onboarding) WITH ORDINALITY AS a(e, ord))
                   ELSE o.onboarding
               END,
               plan = CASE
                   WHEN o.plan ? 'included_networks' OR o.plan ? 'network_cents' THEN
                       (o.plan - 'included_networks' - 'network_cents')
                       || jsonb_build_object(
                              'included_sites', o.plan -> 'included_networks',
                              'site_cents', o.plan -> 'network_cents')
                   ELSE o.plan
               END,
               last_downgrade_from_plan = CASE
                   WHEN o.last_downgrade_from_plan ? 'included_networks'
                        OR o.last_downgrade_from_plan ? 'network_cents' THEN
                       (o.last_downgrade_from_plan - 'included_networks' - 'network_cents')
                       || jsonb_build_object(
                              'included_sites', o.last_downgrade_from_plan -> 'included_networks',
                              'site_cents', o.last_downgrade_from_plan -> 'network_cents')
                   ELSE o.last_downgrade_from_plan
               END,
               notifications = CASE
                   WHEN o.notifications ? 'networks' THEN
                       (o.notifications - 'networks')
                       || jsonb_build_object('sites', o.notifications -> 'networks')
                   ELSE o.notifications
               END
         WHERE o.id = ANY(batch);

        last_id := batch[array_length(batch, 1)];
        COMMIT;
    END LOOP;
END $$;

DO $$
DECLARE
    last_id UUID := '00000000-0000-0000-0000-000000000000';
    batch UUID[];
BEGIN
    LOOP
        SELECT array_agg(id ORDER BY id)
          INTO batch
          FROM (SELECT id FROM discovery
                 WHERE id > last_id AND integration_targets @> '[{"scope": "Network"}]'::jsonb
                 ORDER BY id LIMIT 1000) t;

        EXIT WHEN batch IS NULL;

        UPDATE discovery d
           SET integration_targets = (
                   SELECT jsonb_agg(
                              CASE WHEN t ->> 'scope' = 'Network'
                                   THEN jsonb_set(t, '{scope}', '"Site"'::jsonb) ELSE t END
                              ORDER BY ord)
                     FROM jsonb_array_elements(d.integration_targets) WITH ORDINALITY AS a(t, ord))
         WHERE d.id = ANY(batch);

        last_id := batch[array_length(batch, 1)];
        COMMIT;
    END LOOP;
END $$;

DO $$
DECLARE
    last_id UUID := '00000000-0000-0000-0000-000000000000';
    batch UUID[];
BEGIN
    LOOP
        SELECT array_agg(id ORDER BY id)
          INTO batch
          FROM (SELECT id FROM discovery
                 WHERE id > last_id AND run_type -> 'results' ? 'network_id'
                 ORDER BY id LIMIT 1000) t;

        EXIT WHEN batch IS NULL;

        UPDATE discovery
           SET run_type = jsonb_set(run_type #- '{results,network_id}',
                                    '{results,site_id}',
                                    run_type -> 'results' -> 'network_id')
         WHERE id = ANY(batch);

        last_id := batch[array_length(batch, 1)];
        COMMIT;
    END LOOP;
END $$;
