-- Which license key an organization currently has issued.
--
-- One key is live at a time. The tab shows whichever type is active, and the
-- server compares a request against this value to tell a re-read from a
-- switch: only a real switch retires the previous key by bumping the version.
-- NULL reads as the online key, which is what every existing org holds.
--
-- Expand-only: a nullable column is a metadata-only change in PG11+.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE organizations
    ADD COLUMN license_key_type text;
