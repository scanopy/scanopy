-- no-transaction
--
-- Clear the all-zero MAC from interfaces and IP addresses.
--
-- Why: `00:00:00:00:00:00` is what firmware reports for an interface with no hardware address.
-- Net-SNMP answers `lo`'s ifPhysAddress with six zero bytes, and older daemons stored that as the
-- interface's MAC. It is not an address, but stored as one it sorted every such host under
-- `00:00:00:00:00:00`, and a zero LLDP chassis id or FDB entry resolved to whichever host held it.
--
-- The server now reads a stored zero as no MAC and never writes one, and the daemon no longer
-- reports it. This clears the ones already stored, so SQL-side sorts and lookups agree with what
-- the application reads.
--
-- Every row is cleared, live, closed (`valid_to IS NOT NULL`) and snapshot alike. The value was
-- never an address, so no history is lost. A cleared MAC is `NULL` at `Unspecified`, which is how
-- an absent MAC is stored.
--
-- Batched at 1000 matching rows with a COMMIT per batch (hence `-- no-transaction`). Each pass
-- selects only rows still holding the zero MAC, so it touches nothing else and re-running is
-- harmless: cleared rows no longer match.

SET lock_timeout = '5s';
SET statement_timeout = '0';

DO $$
DECLARE
    batch UUID[];
BEGIN
    LOOP
        SELECT array_agg(id)
          INTO batch
          FROM (SELECT id FROM interfaces
                 WHERE mac_address = '00:00:00:00:00:00'::macaddr
                 LIMIT 1000) t;

        EXIT WHEN batch IS NULL;

        UPDATE interfaces
           SET mac_address = NULL,
               mac_address_source = '"Unspecified"'::jsonb
         WHERE id = ANY(batch);

        COMMIT;
    END LOOP;

    LOOP
        SELECT array_agg(id)
          INTO batch
          FROM (SELECT id FROM ip_addresses
                 WHERE mac_address = '00:00:00:00:00:00'::macaddr
                 LIMIT 1000) t;

        EXIT WHEN batch IS NULL;

        UPDATE ip_addresses
           SET mac_address = NULL,
               mac_address_source = '"Unspecified"'::jsonb
         WHERE id = ANY(batch);

        COMMIT;
    END LOOP;
END $$;
