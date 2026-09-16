-- no-transaction
--
-- Give a host's hostname its own source, and stop storing identifiers as names.
--
-- Why: a host's title can come from two kinds of value. Identifiers (hostname, sysName, chassis
-- ID, address) are facts about the host with uses beyond naming it, and each has its own column.
-- Names (a person's override, a controller-assigned name, an mDNS instance label, a service guess)
-- exist only to name the host and live in `name`. Until now, writers also copied identifiers into
-- `name`: every hostname as `ReverseDns` whatever produced it, controller DHCP hostnames as
-- `Probe(..)`, and addresses as `OwnAddress`. The same value then showed twice with two different
-- provenances, and `hostname` itself had none.
--
-- The server no longer writes those copies and drops them from older daemons at ingest. This
-- clears the ones already stored so the display ladder falls through to the identifier's own rung:
--
--   OwnAddress name                              -> cleared (the address rung shows it)
--   ReverseDns / Probe name equal to an identifier -> cleared (hostname or sysName rung shows it)
--   ReverseDns / Probe name matching neither     -> kept at `Unspecified`, a placeholder. These are
--                                                   daemon provisioning and registration names,
--                                                   which yield to the daemon's own hostname.
--
-- A cleared name is the empty string at `Unspecified`, which is how an unnamed host is stored.
--
-- `hostname_source` defaults to `Unspecified`: the column cannot say which of the four producers
-- wrote an existing hostname, and the next discovery displaces `Unspecified` with a real source.
-- Hosts a person created get `Manual` for the hostname they typed, as 20260828120000 did for the
-- other create-form fields.
--
-- Batched at 1000 rows with a COMMIT per batch (hence `-- no-transaction`), keyset-paginated by
-- `id` as 20260828120000_hosts_attribute_sources.sql does, because `hosts` carries SCD2 history
-- rows. Every expression reads the row's pre-update values, so the three CASEs agree.
--
-- Additive and backward-compatible: an older server ignores the new column, and a cleared name is
-- a state it already handles. Re-running is harmless: cleared rows no longer match.

SET lock_timeout = '5s';
SET statement_timeout = '0';

ALTER TABLE hosts ADD COLUMN IF NOT EXISTS hostname_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;

DO $$
DECLARE
    last_id UUID := '00000000-0000-0000-0000-000000000000';
    batch UUID[];
BEGIN
    LOOP
        SELECT array_agg(id ORDER BY id)
          INTO batch
          FROM (SELECT id FROM hosts WHERE id > last_id ORDER BY id LIMIT 1000) t;

        EXIT WHEN batch IS NULL;

        UPDATE hosts h
           SET name = CASE
                   WHEN h.name_source = '"OwnAddress"'::jsonb THEN ''
                   WHEN (h.name_source = '"ReverseDns"'::jsonb OR h.name_source ? 'Probe')
                        AND (h.name = h.hostname OR h.name = h.sys_name) THEN ''
                   ELSE h.name
               END,
               name_source = CASE
                   WHEN h.name_source = '"OwnAddress"'::jsonb
                        OR h.name_source = '"ReverseDns"'::jsonb
                        OR h.name_source ? 'Probe' THEN '"Unspecified"'::jsonb
                   ELSE h.name_source
               END,
               hostname_source = CASE
                   WHEN h.source->>'type' = 'Manual' AND btrim(coalesce(h.hostname, '')) <> ''
                       THEN '"Manual"'::jsonb
                   ELSE h.hostname_source
               END
         WHERE h.id = ANY(batch);

        last_id := batch[array_length(batch, 1)];
        COMMIT;
    END LOOP;
END $$;
