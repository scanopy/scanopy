-- Remove probe_raw_socket_ports from discovery_type JSONB (moved to scan_settings inside Unified variant)
UPDATE discovery
SET discovery_type = discovery_type - 'probe_raw_socket_ports'
WHERE discovery_type ? 'probe_raw_socket_ports';

-- Drop scan_settings column (never shipped; now lives inside DiscoveryType::Unified)
ALTER TABLE discovery DROP COLUMN IF EXISTS scan_settings;
