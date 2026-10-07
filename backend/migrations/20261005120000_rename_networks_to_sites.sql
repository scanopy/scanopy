-- Rename the Network entity to Site: its table, the three tables keyed on it, every `network_id`
-- column, and the constraints and indexes named after them.
--
-- Every statement is a rename in place, so every row moves with its table: live rows, SCD2
-- history (`valid_to IS NOT NULL`) and snapshot rows (`snapshot_id IS NOT NULL`) alike, with
-- their `valid_from`, `valid_to`, `lineage_id` and `snapshot_id` untouched. No value is dropped.
--
--   networks                      -> sites
--   network_credentials           -> site_credentials
--   user_network_access           -> user_site_access
--   user_api_key_network_access   -> user_api_key_site_access
--   <table>.network_id            -> <table>.site_id   (21 tables, listed below)
--   invites.network_ids           -> invites.site_ids
--
-- JSONB values that name the entity (entity_tags, organizations.onboarding/plan/notifications,
-- discovery.integration_targets) are rewritten by 20261005120001, batched.
--
-- Deploy-Mode: downtime. An older server reads `networks` and `network_id` by name, so it cannot
-- run against this schema, and this server cannot run against the old one. Each
-- `squawk-ignore` below marks a statement whose unsafety that window is what makes acceptable.

SET lock_timeout = '5s';
SET statement_timeout = '5s';

-- Tables
-- squawk-ignore renaming-table
ALTER TABLE networks RENAME TO sites;
-- squawk-ignore renaming-table
ALTER TABLE network_credentials RENAME TO site_credentials;
-- squawk-ignore renaming-table
ALTER TABLE user_network_access RENAME TO user_site_access;
-- squawk-ignore renaming-table
ALTER TABLE user_api_key_network_access RENAME TO user_api_key_site_access;

-- Columns
-- squawk-ignore renaming-column
ALTER TABLE api_keys RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE bindings RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE daemons RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE dependencies RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE discovery RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE hosts RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE interface_neighbor_candidates RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE interface_neighbor_hosts RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE interface_neighbor_interfaces RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE interfaces RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE ip_addresses RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE ports RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE services RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE shares RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE snapshots RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE subnets RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE topologies RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE vlans RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE site_credentials RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE user_site_access RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE user_api_key_site_access RENAME COLUMN network_id TO site_id;
-- squawk-ignore renaming-column
ALTER TABLE invites RENAME COLUMN network_ids TO site_ids;

-- Constraints (a primary key or unique constraint renames its index with it)
ALTER TABLE sites RENAME CONSTRAINT networks_pkey TO sites_pkey;
ALTER TABLE site_credentials RENAME CONSTRAINT network_credentials_pkey TO site_credentials_pkey;
ALTER TABLE site_credentials RENAME CONSTRAINT network_credentials_network_id_fkey TO site_credentials_site_id_fkey;
ALTER TABLE site_credentials RENAME CONSTRAINT network_credentials_credential_id_fkey TO site_credentials_credential_id_fkey;
ALTER TABLE user_site_access RENAME CONSTRAINT user_network_access_pkey TO user_site_access_pkey;
ALTER TABLE user_site_access RENAME CONSTRAINT user_network_access_user_id_network_id_key TO user_site_access_user_id_site_id_key;
ALTER TABLE user_site_access RENAME CONSTRAINT user_network_access_network_id_fkey TO user_site_access_site_id_fkey;
ALTER TABLE user_site_access RENAME CONSTRAINT user_network_access_user_id_fkey TO user_site_access_user_id_fkey;
ALTER TABLE user_api_key_site_access RENAME CONSTRAINT user_api_key_network_access_pkey TO user_api_key_site_access_pkey;
ALTER TABLE user_api_key_site_access RENAME CONSTRAINT user_api_key_network_access_api_key_id_network_id_key TO user_api_key_site_access_api_key_id_site_id_key;
ALTER TABLE user_api_key_site_access RENAME CONSTRAINT user_api_key_network_access_network_id_fkey TO user_api_key_site_access_site_id_fkey;
ALTER TABLE user_api_key_site_access RENAME CONSTRAINT user_api_key_network_access_api_key_id_fkey TO user_api_key_site_access_api_key_id_fkey;
ALTER TABLE api_keys RENAME CONSTRAINT api_keys_network_id_fkey TO api_keys_site_id_fkey;
ALTER TABLE bindings RENAME CONSTRAINT bindings_network_id_fkey TO bindings_site_id_fkey;
ALTER TABLE daemons RENAME CONSTRAINT daemons_network_id_fkey TO daemons_site_id_fkey;
-- `dependencies` was once `groups`; its foreign key kept the old name until now.
ALTER TABLE dependencies RENAME CONSTRAINT groups_network_id_fkey TO dependencies_site_id_fkey;
ALTER TABLE discovery RENAME CONSTRAINT discovery_network_id_fkey TO discovery_site_id_fkey;
ALTER TABLE hosts RENAME CONSTRAINT hosts_network_id_fkey TO hosts_site_id_fkey;
ALTER TABLE interface_neighbor_candidates RENAME CONSTRAINT interface_neighbor_candidates_network_id_fkey TO interface_neighbor_candidates_site_id_fkey;
ALTER TABLE interface_neighbor_hosts RENAME CONSTRAINT interface_neighbor_hosts_network_id_fkey TO interface_neighbor_hosts_site_id_fkey;
ALTER TABLE interface_neighbor_interfaces RENAME CONSTRAINT interface_neighbor_interfaces_network_id_fkey TO interface_neighbor_interfaces_site_id_fkey;
ALTER TABLE interfaces RENAME CONSTRAINT interfaces_network_id_fkey TO interfaces_site_id_fkey;
ALTER TABLE ip_addresses RENAME CONSTRAINT ip_addresses_network_id_fkey TO ip_addresses_site_id_fkey;
ALTER TABLE ports RENAME CONSTRAINT ports_network_id_fkey TO ports_site_id_fkey;
ALTER TABLE services RENAME CONSTRAINT services_network_id_fkey TO services_site_id_fkey;
ALTER TABLE shares RENAME CONSTRAINT shares_network_id_fkey TO shares_site_id_fkey;
ALTER TABLE snapshots RENAME CONSTRAINT snapshots_network_id_fkey TO snapshots_site_id_fkey;
ALTER TABLE subnets RENAME CONSTRAINT subnets_network_id_fkey TO subnets_site_id_fkey;
ALTER TABLE topologies RENAME CONSTRAINT topologies_network_id_fkey TO topologies_site_id_fkey;
ALTER TABLE vlans RENAME CONSTRAINT vlans_network_id_fkey TO vlans_site_id_fkey;

-- Indexes
ALTER INDEX idx_networks_owner_organization RENAME TO idx_sites_owner_organization;
ALTER INDEX idx_user_network_access_network RENAME TO idx_user_site_access_site;
ALTER INDEX idx_user_network_access_user RENAME TO idx_user_site_access_user;
ALTER INDEX idx_user_api_key_network_access_network RENAME TO idx_user_api_key_site_access_site;
ALTER INDEX idx_user_api_key_network_access_key RENAME TO idx_user_api_key_site_access_key;
ALTER INDEX idx_api_keys_network RENAME TO idx_api_keys_site;
ALTER INDEX idx_bindings_network RENAME TO idx_bindings_site;
ALTER INDEX idx_daemons_network RENAME TO idx_daemons_site;
ALTER INDEX idx_groups_network RENAME TO idx_dependencies_site;
ALTER INDEX idx_discovery_network RENAME TO idx_discovery_site;
ALTER INDEX idx_hosts_network RENAME TO idx_hosts_site;
ALTER INDEX idx_interface_neighbor_candidates_network RENAME TO idx_interface_neighbor_candidates_site;
ALTER INDEX idx_interfaces_network RENAME TO idx_interfaces_site;
ALTER INDEX idx_ip_addresses_network RENAME TO idx_ip_addresses_site;
ALTER INDEX idx_ip_addresses_network_mac RENAME TO idx_ip_addresses_site_mac;
ALTER INDEX idx_ports_network RENAME TO idx_ports_site;
ALTER INDEX idx_services_network RENAME TO idx_services_site;
ALTER INDEX idx_shares_network RENAME TO idx_shares_site;
ALTER INDEX idx_snapshots_network_taken_at RENAME TO idx_snapshots_site_taken_at;
ALTER INDEX idx_subnets_network RENAME TO idx_subnets_site;
ALTER INDEX idx_topologies_network RENAME TO idx_topologies_site;
ALTER INDEX idx_vlans_network RENAME TO idx_vlans_site;
ALTER INDEX idx_vlans_network_number_live RENAME TO idx_vlans_site_number_live;

COMMENT ON COLUMN sites.organization_id IS 'The organization that owns and pays for this site';
COMMENT ON TABLE organizations IS 'Organizations that own sites and have Stripe subscriptions';
