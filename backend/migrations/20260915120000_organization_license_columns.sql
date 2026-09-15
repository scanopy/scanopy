-- License state for orgs on a self-hosted plan bought through the cloud app.
--
-- license_paid_through: single source of license expiry. Set to the trial end
--   when a self-hosted trial starts and to the invoice's service-period end
--   when a self-hosted invoice is paid. Keys and entitlements expire 7 days
--   after it (plus the 7-day grace window).
-- license_last_checked_in_at: last time an instance fetched an entitlement.
-- license_key_version: embedded in online keys; regenerating the key bumps it
--   so a leaked key stops receiving entitlements.
--
-- Expand-only: nullable columns and a constant default are metadata-only
-- changes in PG11+, so no table rewrite.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE organizations
    ADD COLUMN license_paid_through timestamptz,
    ADD COLUMN license_last_checked_in_at timestamptz,
    ADD COLUMN license_key_version bigint NOT NULL DEFAULT 0;
