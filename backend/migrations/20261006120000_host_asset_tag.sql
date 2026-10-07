-- The organization's asset tag on a host, with the provenance every other discovered attribute
-- carries: read from ENTITY-MIB `entPhysicalAssetID`, or typed in by a person (`"Manual"`), which
-- outranks the scan.
--
-- Additive only. `asset_tag` is nullable, and `asset_tag_source` takes a constant default, which
-- PostgreSQL records in the catalog without rewriting the table. A server still on the previous
-- release omits both columns from its INSERTs and gets NULL and "Unspecified", which is what it
-- would have meant. No backfill: no earlier column held this value.
SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE hosts ADD COLUMN IF NOT EXISTS asset_tag TEXT;
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS asset_tag_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;
