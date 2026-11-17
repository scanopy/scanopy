-- The signup's answers beside `use_case`.
--
-- use_case_other: the text a signup typed for "Something else" as the use case.
-- intents:        what the signup wants Scanopy to do, as `SignupIntent` values. NULL means the
--                 org was never asked; an empty array means the signup skipped the question.
-- intent_other:   the text typed for "Something else" as an intent.
--
-- Existing organizations were never asked, so all three stay NULL; there is nothing to backfill.
--
-- Expand-only: nullable columns with no default are a metadata-only change in PG11+.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE organizations
    ADD COLUMN use_case_other text,
    ADD COLUMN intents text[],
    ADD COLUMN intent_other text;
