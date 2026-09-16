-- Let an interface record that it has no ifDescr, rather than inventing one.
--
-- PROFINET DCP identifies a device over raw Ethernet and carries no per-port description at
-- all -- unlike LLDP/CDP, which always has the far end's advertised port name to put here
-- (20260827130000_interfaces_unknown_port_facts.sql). `if_descr` was the one ifTable-shaped
-- column that migration left NOT NULL, on the reasoning that every producer up to then always had
-- a real value. DCP is the first that does not, and the alternative -- a fabricated literal
-- standing in for "we don't know" -- is exactly the `PROP_VIRTUAL` anti-pattern that migration's
-- own header argued against for other fields.
--
-- Catalog-only: DROP NOT NULL updates pg_attribute and neither rewrites nor scans the table,
-- which matters because `interfaces` carries SCD2 history rows. No backfill -- every existing row
-- keeps the value it has, and NULL means "no value exists" only for rows written from here on.
--
-- squawk flags this as ban-drop-not-null and it is right to: the currently released container
-- reads if_descr with `row.get::<String>`, which panics rather than errors on NULL, so an old and
-- a new container cannot both be live while NULL rows exist. This ships in a release cut as
-- `downtime` (org.scanopy.deploy_mode), so the two never coexist and the DDL and the first NULL
-- write go out together. The exclusion is registered per-file in
-- backend/scripts/lint-migrations.sh, not repo-wide. See the expand/contract ledger.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE interfaces
    ALTER COLUMN if_descr DROP NOT NULL;
