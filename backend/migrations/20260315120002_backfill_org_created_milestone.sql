-- Backfill OrgCreated milestone for legacy orgs that were migrated with empty onboarding arrays.
-- Every org that exists has, by definition, been created.
UPDATE organizations
SET onboarding = onboarding || '["OrgCreated"]'::JSONB
WHERE NOT (onboarding @> '["OrgCreated"]'::JSONB);
