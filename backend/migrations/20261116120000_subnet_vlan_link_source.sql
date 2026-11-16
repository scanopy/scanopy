-- Who linked a subnet to a VLAN: discovery, or a person.
--
-- Every link so far came from discovery, which rebuilds a subnet's whole set on each scan from
-- the native VLANs of the interfaces on it. A person can now link a subnet to a VLAN by hand, and
-- that rebuild would close the link on the next scan. Scans now rebuild only `Discovery` links,
-- and a subnet edit sets only `Manual` ones.
--
-- Every existing row came from discovery, so the default describes it correctly and nothing needs
-- backfilling. Live, closed and snapshot rows all take the default. Nothing is dropped.
--
-- Additive and prod-safe: `ADD COLUMN` with a constant default is metadata-only on PG11+ (no
-- rewrite).

SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE subnet_vlans
    ADD COLUMN source TEXT NOT NULL DEFAULT 'Discovery';
