-- Fix JSONB literal null vs SQL NULL in if_entries
-- JSONB 'null' is a value, not absence — IS NULL checks fail against it
UPDATE if_entries SET lldp_chassis_id = NULL WHERE lldp_chassis_id = 'null'::jsonb;
UPDATE if_entries SET lldp_port_id = NULL WHERE lldp_port_id = 'null'::jsonb;
UPDATE if_entries SET fdb_macs = NULL WHERE fdb_macs = 'null'::jsonb;
