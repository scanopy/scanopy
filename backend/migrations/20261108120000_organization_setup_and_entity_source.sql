-- no-transaction
--
-- The org's Setup record, and who made each tag and credential. Expand-only: nothing is dropped.
--
-- `organizations.setup` holds the Setup item types an org has finished or dismissed. It starts as a
-- copy of `organizations.onboarding`, value for value. The server reads the milestone names it can
-- use through serde aliases on `SetupItemDiscriminants`:
--
--   FirstDaemonRegistered   -> InstallDaemon
--   FirstDiscoveryCompleted -> RunFirstScan
--   FirstTopologyRebuild    -> OpenTopology
--   ProfileCompleted        -> AnswerProfile
--   ReferralSourceCompleted -> AnswerReferralSource
--
-- and drops every other value (OrgCreated, OnboardingModalCompleted, PlanSelected, the DaemonPrompt
-- values, FirstHostDiscovered, SecondSiteCreated, the other First*Created values, InviteSent,
-- InviteAccepted) when it loads the row. The next write of the org stores the cleaned list.
-- `onboarding` is no longer read or written, and stays until the contract release drops it
-- (recorded in the expand/contract ledger).
--
-- `tags.source` and `credentials.source` say who made the row, in the JSON form `hosts.source`
-- uses: {"type": "System"} for a row Scanopy seeded, {"type": "Manual"} for one a person added.
-- Both columns default to Manual. Backfilled to System:
--
--   tags         a tag whose name and group are a starter tag's (`STARTER_TAG_GROUPS` in
--                shared/storage/seed_data.rs): Planned, Active, Decommissioning or Decommissioned in
--                the Status group; Production, Staging, Development or Lab in the custom group
--                named Environment; Critical, High, Medium or Low in Criticality; Managed,
--                Unmanaged or Third-party in Management. Also the Visibility group's Hidden tag
--                (named 'Hidden (topology)' where the org already had a tag called Hidden), which
--                the tag-groups migration and `hidden_tag` seed. Group names match without regard
--                to case, as `create_starter_tags` matches them. `tags` is SCD2: every row version
--                is updated (live, closed `valid_to IS NOT NULL` and snapshot `snapshot_id IS NOT
--                NULL` rows), and no validity, lineage or snapshot column is touched.
--   credentials  a credential whose name and `credential_type->>'type'` are a seeded default's
--                (`CredentialTypeDiscriminants::seeded_default`): 'Docker socket' / DockerSocket,
--                'Podman socket' / PodmanSocket, 'SNMP (public)' / SnmpCommunity.
--
-- A seeded tag or credential a person renamed no longer matches, and stays Manual.
--
-- Every update runs in batches of 1000 rows with a COMMIT per batch (hence `-- no-transaction`),
-- keyset-paginated by `id`. Each step is idempotent, so a run that fails part-way can be re-run.

SET lock_timeout = '5s';
SET statement_timeout = '0';

ALTER TABLE organizations ADD COLUMN IF NOT EXISTS setup JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE tags ADD COLUMN IF NOT EXISTS source JSONB NOT NULL DEFAULT '{"type": "Manual"}'::jsonb;
ALTER TABLE credentials ADD COLUMN IF NOT EXISTS source JSONB NOT NULL DEFAULT '{"type": "Manual"}'::jsonb;

-- Copy each org's milestones. `onboarding` was nullable (DEFAULT '[]', no NOT NULL).
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

        UPDATE organizations
           SET setup = COALESCE(onboarding, '[]'::jsonb)
         WHERE id = ANY(batch)
           AND setup = '[]'::jsonb;

        last_id := batch[array_length(batch, 1)];
        COMMIT;
    END LOOP;
END $$;

-- Starter tags, by group and name.
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

-- Seeded default credentials, by name and type.
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
