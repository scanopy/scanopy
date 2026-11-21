-- Contract for `organizations.onboarding` (see docs/db-expand-contract-ledger.md).
--
-- Expand (v0.20.0): 20261108120000 added `organizations.setup` and copied `onboarding` into it.
-- 20261114120000 re-copied it as a union after the rollback to v0.19.6. Since v0.20.0 the server
-- reads and writes only `setup`.
--
-- Nothing to backfill: `setup` holds every value. The milestone names without a serde alias on
-- `SetupItemDiscriminants` (OrgCreated, PlanSelected, the DaemonPrompt values, the First*Created
-- values, InviteSent, InviteAccepted) were dropped on purpose in v0.20.0.
--
-- Precondition: prod runs v0.20.0 and no v0.19.6 container remains. Storage names its columns
-- in `to_params` and v0.20.0 never names `onboarding`, so this drop is safe under a rolling
-- deploy.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

-- squawk-ignore ban-drop-column
ALTER TABLE organizations DROP COLUMN onboarding;
