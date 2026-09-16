-- Issued-at stamp for an org's online license key.
--
-- Online key claims carry `iat`, so minting from `now()` produced a different
-- key string on every copy. Holding the stamp per organization makes the mint
-- deterministic: the same claims sign to a byte-identical key, and the key
-- changes only when it is regenerated (which also moves this stamp).
--
-- Stores no key material: without the signing key the stamp mints nothing.
--
-- Expand-only: a nullable column is a metadata-only change in PG11+.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE organizations
    ADD COLUMN license_key_issued_at timestamptz;
