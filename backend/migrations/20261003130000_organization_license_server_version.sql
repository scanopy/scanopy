-- Highest server version a check-in with the organization's online license
-- key has reported, as semver text.
--
-- Self-hosted servers send their version on each entitlement check-in. Several
-- servers can share a key, so the column only ever rises. NULL until a server
-- new enough to send its version checks in; there is nothing to backfill.
--
-- Expand-only: a nullable column is a metadata-only change in PG11+.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE organizations
    ADD COLUMN license_server_version text;
