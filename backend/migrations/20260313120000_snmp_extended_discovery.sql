-- Add hardware inventory fields to hosts (from ENTITY-MIB)
ALTER TABLE hosts ADD COLUMN manufacturer TEXT;
ALTER TABLE hosts ADD COLUMN model TEXT;
ALTER TABLE hosts ADD COLUMN serial_number TEXT;

-- Add sysName as a dedicated field (previously only used transiently for chassis_id)
ALTER TABLE hosts ADD COLUMN sys_name TEXT;

-- Add bridge FDB MAC list to if_entries (from dot1dTpFdbTable)
ALTER TABLE if_entries ADD COLUMN fdb_macs JSONB;
