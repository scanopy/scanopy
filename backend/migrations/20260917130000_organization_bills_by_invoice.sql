-- Whether an organization's subscription is billed by sent invoice.
--
-- `has_payment_method` means a card is on file, and the paths that ask Stripe
-- directly (end trial, plan change) mean the same by it. An organization
-- paying against a purchase order has no card, so it needs its own signal:
-- the payment prompts read either, and nothing has to call Stripe to render
-- the sidebar nudge or the no-payment-method banner.
--
-- Written by the org subscriber: true when an invoice is finalized, false when
-- a card is attached (which switches the subscription back to automatic
-- charges) or the subscription is cancelled.
--
-- Expand-only: a new column with a default is a metadata-only change in
-- PG11+, and an older container simply never reads it.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE organizations
    ADD COLUMN bills_by_invoice boolean NOT NULL DEFAULT false;
