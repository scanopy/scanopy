SET lock_timeout = '5s';
SET statement_timeout = '5s';

-- Latest entitlement an online license key fetched from Scanopy Cloud, and when
-- the cloud last answered. Instance-level state, written to every org row so it
-- survives restarts while the cloud is unreachable.
ALTER TABLE organizations ADD COLUMN license_entitlement TEXT;
ALTER TABLE organizations ADD COLUMN license_checked_at TIMESTAMPTZ;
