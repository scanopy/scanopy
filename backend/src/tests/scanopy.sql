--
-- PostgreSQL database dump
--

\restrict GjfVS29iX7gdYI3bTiabVD86psO000c6LT9aH6F8eV5v3WvA81n24P3blseLg6I

-- Dumped from database version 17.11
-- Dumped by pg_dump version 17.11

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

ALTER TABLE IF EXISTS ONLY public.vlans DROP CONSTRAINT IF EXISTS vlans_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.vlans DROP CONSTRAINT IF EXISTS vlans_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.vlans DROP CONSTRAINT IF EXISTS vlans_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.vlans DROP CONSTRAINT IF EXISTS vlans_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.vlans DROP CONSTRAINT IF EXISTS vlans_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.users DROP CONSTRAINT IF EXISTS users_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_site_access DROP CONSTRAINT IF EXISTS user_site_access_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_site_access DROP CONSTRAINT IF EXISTS user_site_access_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_key_site_access DROP CONSTRAINT IF EXISTS user_api_key_site_access_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_key_site_access DROP CONSTRAINT IF EXISTS user_api_key_site_access_api_key_id_fkey;
ALTER TABLE IF EXISTS ONLY public.topologies DROP CONSTRAINT IF EXISTS topologies_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.tags DROP CONSTRAINT IF EXISTS tags_tag_group_id_fkey;
ALTER TABLE IF EXISTS ONLY public.tags DROP CONSTRAINT IF EXISTS tags_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.tags DROP CONSTRAINT IF EXISTS tags_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.tag_groups DROP CONSTRAINT IF EXISTS tag_groups_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_virtualization_service_id_fkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.subnet_vlans DROP CONSTRAINT IF EXISTS subnet_vlans_vlan_id_fkey;
ALTER TABLE IF EXISTS ONLY public.subnet_vlans DROP CONSTRAINT IF EXISTS subnet_vlans_subnet_id_fkey;
ALTER TABLE IF EXISTS ONLY public.subnet_vlans DROP CONSTRAINT IF EXISTS subnet_vlans_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.snapshots DROP CONSTRAINT IF EXISTS snapshots_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.snapshots DROP CONSTRAINT IF EXISTS snapshots_created_by_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.site_credentials DROP CONSTRAINT IF EXISTS site_credentials_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.site_credentials DROP CONSTRAINT IF EXISTS site_credentials_credential_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_topology_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_created_by_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_virtualization_service_id_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.sites DROP CONSTRAINT IF EXISTS organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.license_instances DROP CONSTRAINT IF EXISTS license_instances_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ip_addresses DROP CONSTRAINT IF EXISTS ip_addresses_subnet_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ip_addresses DROP CONSTRAINT IF EXISTS ip_addresses_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ip_addresses DROP CONSTRAINT IF EXISTS ip_addresses_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ip_addresses DROP CONSTRAINT IF EXISTS ip_addresses_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.ip_addresses DROP CONSTRAINT IF EXISTS ip_addresses_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ip_addresses DROP CONSTRAINT IF EXISTS ip_addresses_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_created_by_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_native_vlan_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_ip_address_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.interface_neighbor_interfaces DROP CONSTRAINT IF EXISTS interface_neighbor_interfaces_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interface_neighbor_interfaces DROP CONSTRAINT IF EXISTS interface_neighbor_interfaces_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interface_neighbor_interfaces DROP CONSTRAINT IF EXISTS interface_neighbor_interfaces_neighbor_interface_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interface_neighbor_interfaces DROP CONSTRAINT IF EXISTS interface_neighbor_interfaces_interface_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interface_neighbor_hosts DROP CONSTRAINT IF EXISTS interface_neighbor_hosts_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interface_neighbor_hosts DROP CONSTRAINT IF EXISTS interface_neighbor_hosts_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interface_neighbor_hosts DROP CONSTRAINT IF EXISTS interface_neighbor_hosts_neighbor_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interface_neighbor_hosts DROP CONSTRAINT IF EXISTS interface_neighbor_hosts_interface_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interface_neighbor_candidates DROP CONSTRAINT IF EXISTS interface_neighbor_candidates_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interface_neighbor_candidates DROP CONSTRAINT IF EXISTS interface_neighbor_candidates_interface_id_fkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_virtualization_service_id_fkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_virtualization_interface_id_fkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.host_credentials DROP CONSTRAINT IF EXISTS host_credentials_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.host_credentials DROP CONSTRAINT IF EXISTS host_credentials_credential_id_fkey;
ALTER TABLE IF EXISTS ONLY public.dependency_members DROP CONSTRAINT IF EXISTS group_bindings_group_id_fkey;
ALTER TABLE IF EXISTS ONLY public.dependency_members DROP CONSTRAINT IF EXISTS group_bindings_binding_id_fkey;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_tag_id_fkey;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery_warnings DROP CONSTRAINT IF EXISTS discovery_warnings_subnet_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery_warnings DROP CONSTRAINT IF EXISTS discovery_warnings_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery_warnings DROP CONSTRAINT IF EXISTS discovery_warnings_remote_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery_warnings DROP CONSTRAINT IF EXISTS discovery_warnings_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery_warnings DROP CONSTRAINT IF EXISTS discovery_warnings_discovery_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery_warnings DROP CONSTRAINT IF EXISTS discovery_warnings_credential_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_daemon_id_fkey;
ALTER TABLE IF EXISTS ONLY public.dependency_members DROP CONSTRAINT IF EXISTS dependency_members_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.dependency_members DROP CONSTRAINT IF EXISTS dependency_members_service_id_fkey;
ALTER TABLE IF EXISTS ONLY public.dependencies DROP CONSTRAINT IF EXISTS dependencies_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.dependencies DROP CONSTRAINT IF EXISTS dependencies_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_api_key_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemon_interfaced_subnets DROP CONSTRAINT IF EXISTS daemon_interfaced_subnets_subnet_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemon_interfaced_subnets DROP CONSTRAINT IF EXISTS daemon_interfaced_subnets_daemon_id_fkey;
ALTER TABLE IF EXISTS ONLY public.credentials DROP CONSTRAINT IF EXISTS credentials_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_snapshot_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_service_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_port_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_last_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_ip_address_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_first_discovery_fk;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_daemon_id_fkey;
DROP TRIGGER IF EXISTS reassign_daemons_before_user_delete ON public.users;
DROP INDEX IF EXISTS public.idx_vlans_snapshot_id;
DROP INDEX IF EXISTS public.idx_vlans_site_number_live;
DROP INDEX IF EXISTS public.idx_vlans_site;
DROP INDEX IF EXISTS public.idx_vlans_organization;
DROP INDEX IF EXISTS public.idx_vlans_live;
DROP INDEX IF EXISTS public.idx_vlans_lineage;
DROP INDEX IF EXISTS public.idx_vlans_as_of;
DROP INDEX IF EXISTS public.idx_users_password_reset_token;
DROP INDEX IF EXISTS public.idx_users_organization;
DROP INDEX IF EXISTS public.idx_users_oidc_provider_subject;
DROP INDEX IF EXISTS public.idx_users_email_verification_token;
DROP INDEX IF EXISTS public.idx_users_email_unsubscribe_token;
DROP INDEX IF EXISTS public.idx_users_email_lower;
DROP INDEX IF EXISTS public.idx_user_site_access_user;
DROP INDEX IF EXISTS public.idx_user_site_access_site;
DROP INDEX IF EXISTS public.idx_user_api_keys_user;
DROP INDEX IF EXISTS public.idx_user_api_keys_org;
DROP INDEX IF EXISTS public.idx_user_api_keys_key;
DROP INDEX IF EXISTS public.idx_user_api_key_site_access_site;
DROP INDEX IF EXISTS public.idx_user_api_key_site_access_key;
DROP INDEX IF EXISTS public.idx_topologies_site;
DROP INDEX IF EXISTS public.idx_tags_tag_group_id;
DROP INDEX IF EXISTS public.idx_tags_snapshot_id;
DROP INDEX IF EXISTS public.idx_tags_organization;
DROP INDEX IF EXISTS public.idx_tags_org_name_live;
DROP INDEX IF EXISTS public.idx_tags_live;
DROP INDEX IF EXISTS public.idx_tags_lineage;
DROP INDEX IF EXISTS public.idx_tags_as_of;
DROP INDEX IF EXISTS public.idx_tag_groups_org_system;
DROP INDEX IF EXISTS public.idx_tag_groups_org_name;
DROP INDEX IF EXISTS public.idx_subnets_virtualization_service_id;
DROP INDEX IF EXISTS public.idx_subnets_snapshot_id;
DROP INDEX IF EXISTS public.idx_subnets_site;
DROP INDEX IF EXISTS public.idx_subnets_live;
DROP INDEX IF EXISTS public.idx_subnets_lineage;
DROP INDEX IF EXISTS public.idx_subnets_as_of;
DROP INDEX IF EXISTS public.idx_subnet_vlans_vlan;
DROP INDEX IF EXISTS public.idx_subnet_vlans_unique_live;
DROP INDEX IF EXISTS public.idx_subnet_vlans_subnet;
DROP INDEX IF EXISTS public.idx_subnet_vlans_snapshot_id;
DROP INDEX IF EXISTS public.idx_subnet_vlans_live;
DROP INDEX IF EXISTS public.idx_subnet_vlans_lineage;
DROP INDEX IF EXISTS public.idx_subnet_vlans_as_of;
DROP INDEX IF EXISTS public.idx_snapshots_site_taken_at;
DROP INDEX IF EXISTS public.idx_sites_owner_organization;
DROP INDEX IF EXISTS public.idx_shares_topology;
DROP INDEX IF EXISTS public.idx_shares_site;
DROP INDEX IF EXISTS public.idx_shares_enabled;
DROP INDEX IF EXISTS public.idx_services_virtualization_service_id;
DROP INDEX IF EXISTS public.idx_services_snapshot_id;
DROP INDEX IF EXISTS public.idx_services_site;
DROP INDEX IF EXISTS public.idx_services_live;
DROP INDEX IF EXISTS public.idx_services_lineage;
DROP INDEX IF EXISTS public.idx_services_host_position;
DROP INDEX IF EXISTS public.idx_services_host_id;
DROP INDEX IF EXISTS public.idx_services_as_of;
DROP INDEX IF EXISTS public.idx_release_announcements_version;
DROP INDEX IF EXISTS public.idx_ports_unique_live;
DROP INDEX IF EXISTS public.idx_ports_snapshot_id;
DROP INDEX IF EXISTS public.idx_ports_site;
DROP INDEX IF EXISTS public.idx_ports_number;
DROP INDEX IF EXISTS public.idx_ports_live;
DROP INDEX IF EXISTS public.idx_ports_lineage;
DROP INDEX IF EXISTS public.idx_ports_host;
DROP INDEX IF EXISTS public.idx_ports_as_of;
DROP INDEX IF EXISTS public.idx_organizations_stripe_customer;
DROP INDEX IF EXISTS public.idx_license_instances_organization;
DROP INDEX IF EXISTS public.idx_ip_addresses_unique_live;
DROP INDEX IF EXISTS public.idx_ip_addresses_subnet;
DROP INDEX IF EXISTS public.idx_ip_addresses_snapshot_id;
DROP INDEX IF EXISTS public.idx_ip_addresses_site_mac;
DROP INDEX IF EXISTS public.idx_ip_addresses_site;
DROP INDEX IF EXISTS public.idx_ip_addresses_live;
DROP INDEX IF EXISTS public.idx_ip_addresses_lineage;
DROP INDEX IF EXISTS public.idx_ip_addresses_host_mac;
DROP INDEX IF EXISTS public.idx_ip_addresses_host;
DROP INDEX IF EXISTS public.idx_ip_addresses_as_of;
DROP INDEX IF EXISTS public.idx_invites_organization;
DROP INDEX IF EXISTS public.idx_invites_expires_at;
DROP INDEX IF EXISTS public.idx_interfaces_snapshot_id;
DROP INDEX IF EXISTS public.idx_interfaces_site;
DROP INDEX IF EXISTS public.idx_interfaces_mac_address;
DROP INDEX IF EXISTS public.idx_interfaces_live;
DROP INDEX IF EXISTS public.idx_interfaces_lineage;
DROP INDEX IF EXISTS public.idx_interfaces_ip_address;
DROP INDEX IF EXISTS public.idx_interfaces_host_name_live;
DROP INDEX IF EXISTS public.idx_interfaces_host_if_index;
DROP INDEX IF EXISTS public.idx_interfaces_host;
DROP INDEX IF EXISTS public.idx_interfaces_as_of;
DROP INDEX IF EXISTS public.idx_interface_neighbor_interfaces_snapshot_id;
DROP INDEX IF EXISTS public.idx_interface_neighbor_interfaces_neighbor;
DROP INDEX IF EXISTS public.idx_interface_neighbor_interfaces_natural_key;
DROP INDEX IF EXISTS public.idx_interface_neighbor_interfaces_live;
DROP INDEX IF EXISTS public.idx_interface_neighbor_interfaces_lineage;
DROP INDEX IF EXISTS public.idx_interface_neighbor_interfaces_as_of;
DROP INDEX IF EXISTS public.idx_interface_neighbor_hosts_snapshot_id;
DROP INDEX IF EXISTS public.idx_interface_neighbor_hosts_neighbor;
DROP INDEX IF EXISTS public.idx_interface_neighbor_hosts_natural_key;
DROP INDEX IF EXISTS public.idx_interface_neighbor_hosts_live;
DROP INDEX IF EXISTS public.idx_interface_neighbor_hosts_lineage;
DROP INDEX IF EXISTS public.idx_interface_neighbor_hosts_as_of;
DROP INDEX IF EXISTS public.idx_interface_neighbor_candidates_site;
DROP INDEX IF EXISTS public.idx_interface_neighbor_candidates_interface;
DROP INDEX IF EXISTS public.idx_hosts_virtualization_service_id;
DROP INDEX IF EXISTS public.idx_hosts_virtualization_interface_id;
DROP INDEX IF EXISTS public.idx_hosts_snapshot_id;
DROP INDEX IF EXISTS public.idx_hosts_site;
DROP INDEX IF EXISTS public.idx_hosts_live;
DROP INDEX IF EXISTS public.idx_hosts_lineage;
DROP INDEX IF EXISTS public.idx_hosts_chassis_id;
DROP INDEX IF EXISTS public.idx_hosts_as_of;
DROP INDEX IF EXISTS public.idx_entity_tags_unique_live;
DROP INDEX IF EXISTS public.idx_entity_tags_tag_id;
DROP INDEX IF EXISTS public.idx_entity_tags_snapshot_id;
DROP INDEX IF EXISTS public.idx_entity_tags_live;
DROP INDEX IF EXISTS public.idx_entity_tags_lineage;
DROP INDEX IF EXISTS public.idx_entity_tags_entity;
DROP INDEX IF EXISTS public.idx_entity_tags_as_of;
DROP INDEX IF EXISTS public.idx_discovery_warnings_subnet;
DROP INDEX IF EXISTS public.idx_discovery_warnings_site;
DROP INDEX IF EXISTS public.idx_discovery_warnings_remote_host;
DROP INDEX IF EXISTS public.idx_discovery_warnings_host;
DROP INDEX IF EXISTS public.idx_discovery_warnings_discovery;
DROP INDEX IF EXISTS public.idx_discovery_warnings_credential;
DROP INDEX IF EXISTS public.idx_discovery_site;
DROP INDEX IF EXISTS public.idx_discovery_daemon;
DROP INDEX IF EXISTS public.idx_dependency_members_unique_live;
DROP INDEX IF EXISTS public.idx_dependency_members_snapshot_id;
DROP INDEX IF EXISTS public.idx_dependency_members_service;
DROP INDEX IF EXISTS public.idx_dependency_members_live;
DROP INDEX IF EXISTS public.idx_dependency_members_lineage;
DROP INDEX IF EXISTS public.idx_dependency_members_dependency;
DROP INDEX IF EXISTS public.idx_dependency_members_binding;
DROP INDEX IF EXISTS public.idx_dependency_members_as_of;
DROP INDEX IF EXISTS public.idx_dependencies_snapshot_id;
DROP INDEX IF EXISTS public.idx_dependencies_site;
DROP INDEX IF EXISTS public.idx_dependencies_live;
DROP INDEX IF EXISTS public.idx_dependencies_lineage;
DROP INDEX IF EXISTS public.idx_dependencies_as_of;
DROP INDEX IF EXISTS public.idx_daemons_site;
DROP INDEX IF EXISTS public.idx_daemons_api_key;
DROP INDEX IF EXISTS public.idx_daemon_interfaced_subnets_subnet_id;
DROP INDEX IF EXISTS public.idx_daemon_host_id;
DROP INDEX IF EXISTS public.idx_credentials_type;
DROP INDEX IF EXISTS public.idx_credentials_org;
DROP INDEX IF EXISTS public.idx_bindings_snapshot_id;
DROP INDEX IF EXISTS public.idx_bindings_site;
DROP INDEX IF EXISTS public.idx_bindings_service;
DROP INDEX IF EXISTS public.idx_bindings_port;
DROP INDEX IF EXISTS public.idx_bindings_live;
DROP INDEX IF EXISTS public.idx_bindings_lineage;
DROP INDEX IF EXISTS public.idx_bindings_ip_address;
DROP INDEX IF EXISTS public.idx_bindings_as_of;
DROP INDEX IF EXISTS public.idx_api_keys_site;
DROP INDEX IF EXISTS public.idx_api_keys_key;
DROP INDEX IF EXISTS public.idx_api_keys_daemon_id;
ALTER TABLE IF EXISTS ONLY tower_sessions.session DROP CONSTRAINT IF EXISTS session_pkey;
ALTER TABLE IF EXISTS ONLY public.vlans DROP CONSTRAINT IF EXISTS vlans_pkey;
ALTER TABLE IF EXISTS ONLY public.users DROP CONSTRAINT IF EXISTS users_pkey;
ALTER TABLE IF EXISTS ONLY public.user_site_access DROP CONSTRAINT IF EXISTS user_site_access_user_id_site_id_key;
ALTER TABLE IF EXISTS ONLY public.user_site_access DROP CONSTRAINT IF EXISTS user_site_access_pkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_pkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_key_key;
ALTER TABLE IF EXISTS ONLY public.user_api_key_site_access DROP CONSTRAINT IF EXISTS user_api_key_site_access_pkey;
ALTER TABLE IF EXISTS ONLY public.user_api_key_site_access DROP CONSTRAINT IF EXISTS user_api_key_site_access_api_key_id_site_id_key;
ALTER TABLE IF EXISTS ONLY public.topologies DROP CONSTRAINT IF EXISTS topologies_pkey;
ALTER TABLE IF EXISTS ONLY public.tags DROP CONSTRAINT IF EXISTS tags_pkey;
ALTER TABLE IF EXISTS ONLY public.tag_groups DROP CONSTRAINT IF EXISTS tag_groups_pkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_pkey;
ALTER TABLE IF EXISTS ONLY public.subnet_vlans DROP CONSTRAINT IF EXISTS subnet_vlans_pkey;
ALTER TABLE IF EXISTS ONLY public.snapshots DROP CONSTRAINT IF EXISTS snapshots_pkey;
ALTER TABLE IF EXISTS ONLY public.sites DROP CONSTRAINT IF EXISTS sites_pkey;
ALTER TABLE IF EXISTS ONLY public.site_credentials DROP CONSTRAINT IF EXISTS site_credentials_pkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_pkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_pkey;
ALTER TABLE IF EXISTS ONLY public.server_instance DROP CONSTRAINT IF EXISTS server_instance_pkey;
ALTER TABLE IF EXISTS ONLY public.release_announcements DROP CONSTRAINT IF EXISTS release_announcements_pkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_pkey;
ALTER TABLE IF EXISTS ONLY public.organizations DROP CONSTRAINT IF EXISTS organizations_pkey;
ALTER TABLE IF EXISTS ONLY public.license_instances DROP CONSTRAINT IF EXISTS license_instances_pkey;
ALTER TABLE IF EXISTS ONLY public.ip_addresses DROP CONSTRAINT IF EXISTS ip_addresses_pkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_pkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_pkey;
ALTER TABLE IF EXISTS ONLY public.interface_neighbor_interfaces DROP CONSTRAINT IF EXISTS interface_neighbor_interfaces_pkey;
ALTER TABLE IF EXISTS ONLY public.interface_neighbor_hosts DROP CONSTRAINT IF EXISTS interface_neighbor_hosts_pkey;
ALTER TABLE IF EXISTS ONLY public.interface_neighbor_candidates DROP CONSTRAINT IF EXISTS interface_neighbor_candidates_pkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_pkey;
ALTER TABLE IF EXISTS ONLY public.host_credentials DROP CONSTRAINT IF EXISTS host_credentials_pkey;
ALTER TABLE IF EXISTS ONLY public.dependencies DROP CONSTRAINT IF EXISTS groups_pkey;
ALTER TABLE IF EXISTS ONLY public.dependency_members DROP CONSTRAINT IF EXISTS group_bindings_pkey;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_pkey;
ALTER TABLE IF EXISTS ONLY public.discovery_warnings DROP CONSTRAINT IF EXISTS discovery_warnings_pkey;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_pkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_pkey;
ALTER TABLE IF EXISTS ONLY public.daemon_interfaced_subnets DROP CONSTRAINT IF EXISTS daemon_interfaced_subnets_pkey;
ALTER TABLE IF EXISTS ONLY public.credentials DROP CONSTRAINT IF EXISTS credentials_pkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_pkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_pkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_key_key;
ALTER TABLE IF EXISTS ONLY public._sqlx_migrations DROP CONSTRAINT IF EXISTS _sqlx_migrations_pkey;
DROP TABLE IF EXISTS tower_sessions.session;
DROP TABLE IF EXISTS public.vlans;
DROP TABLE IF EXISTS public.users;
DROP TABLE IF EXISTS public.user_site_access;
DROP TABLE IF EXISTS public.user_api_keys;
DROP TABLE IF EXISTS public.user_api_key_site_access;
DROP TABLE IF EXISTS public.topologies;
DROP TABLE IF EXISTS public.tags;
DROP TABLE IF EXISTS public.tag_groups;
DROP TABLE IF EXISTS public.subnets;
DROP TABLE IF EXISTS public.subnet_vlans;
DROP TABLE IF EXISTS public.snapshots;
DROP TABLE IF EXISTS public.sites;
DROP TABLE IF EXISTS public.site_credentials;
DROP TABLE IF EXISTS public.shares;
DROP TABLE IF EXISTS public.services;
DROP TABLE IF EXISTS public.server_instance;
DROP TABLE IF EXISTS public.release_announcements;
DROP TABLE IF EXISTS public.ports;
DROP TABLE IF EXISTS public.organizations;
DROP TABLE IF EXISTS public.license_instances;
DROP TABLE IF EXISTS public.ip_addresses;
DROP TABLE IF EXISTS public.invites;
DROP TABLE IF EXISTS public.interfaces;
DROP TABLE IF EXISTS public.interface_neighbor_interfaces;
DROP TABLE IF EXISTS public.interface_neighbor_hosts;
DROP TABLE IF EXISTS public.interface_neighbor_candidates;
DROP TABLE IF EXISTS public.hosts;
DROP TABLE IF EXISTS public.host_credentials;
DROP TABLE IF EXISTS public.entity_tags;
DROP TABLE IF EXISTS public.discovery_warnings;
DROP TABLE IF EXISTS public.discovery;
DROP TABLE IF EXISTS public.dependency_members;
DROP TABLE IF EXISTS public.dependencies;
DROP TABLE IF EXISTS public.daemons;
DROP TABLE IF EXISTS public.daemon_interfaced_subnets;
DROP TABLE IF EXISTS public.credentials;
DROP TABLE IF EXISTS public.bindings;
DROP TABLE IF EXISTS public.api_keys;
DROP TABLE IF EXISTS public._sqlx_migrations;
DROP FUNCTION IF EXISTS public.reassign_daemons_on_user_delete();
DROP EXTENSION IF EXISTS pgcrypto;
DROP SCHEMA IF EXISTS tower_sessions;
--
-- Name: tower_sessions; Type: SCHEMA; Schema: -; Owner: postgres
--

CREATE SCHEMA tower_sessions;


ALTER SCHEMA tower_sessions OWNER TO postgres;

--
-- Name: pgcrypto; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;


--
-- Name: EXTENSION pgcrypto; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pgcrypto IS 'cryptographic functions';


--
-- Name: reassign_daemons_on_user_delete(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.reassign_daemons_on_user_delete() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE
    new_owner_id UUID;
BEGIN
    SELECT id INTO new_owner_id
    FROM users
    WHERE organization_id = OLD.organization_id
      AND permissions = 'Owner'
      AND id != OLD.id
    ORDER BY created_at ASC
    LIMIT 1;

    IF new_owner_id IS NOT NULL THEN
        UPDATE daemons
        SET user_id = new_owner_id
        WHERE user_id = OLD.id;
    END IF;

    RETURN OLD;
END;
$$;


ALTER FUNCTION public.reassign_daemons_on_user_delete() OWNER TO postgres;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: _sqlx_migrations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public._sqlx_migrations (
    version bigint NOT NULL,
    description text NOT NULL,
    installed_on timestamp with time zone DEFAULT now() NOT NULL,
    success boolean NOT NULL,
    checksum bytea NOT NULL,
    execution_time bigint NOT NULL
);


ALTER TABLE public._sqlx_migrations OWNER TO postgres;

--
-- Name: api_keys; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.api_keys (
    id uuid NOT NULL,
    key text NOT NULL,
    site_id uuid NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used timestamp with time zone,
    expires_at timestamp with time zone,
    is_enabled boolean DEFAULT true NOT NULL,
    plaintext text,
    daemon_id uuid
);


ALTER TABLE public.api_keys OWNER TO postgres;

--
-- Name: bindings; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.bindings (
    id uuid NOT NULL,
    site_id uuid NOT NULL,
    service_id uuid NOT NULL,
    binding_type text NOT NULL,
    ip_address_id uuid,
    port_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid,
    CONSTRAINT bindings_binding_type_check CHECK ((binding_type = ANY (ARRAY['IPAddress'::text, 'Port'::text]))),
    CONSTRAINT valid_binding CHECK ((((binding_type = 'IPAddress'::text) AND (ip_address_id IS NOT NULL) AND (port_id IS NULL)) OR ((binding_type = 'Port'::text) AND (port_id IS NOT NULL))))
);


ALTER TABLE public.bindings OWNER TO postgres;

--
-- Name: credentials; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.credentials (
    id uuid NOT NULL,
    organization_id uuid NOT NULL,
    name text NOT NULL,
    credential_type jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    daemon_os text,
    description text
);


ALTER TABLE public.credentials OWNER TO postgres;

--
-- Name: daemon_interfaced_subnets; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.daemon_interfaced_subnets (
    daemon_id uuid NOT NULL,
    subnet_id uuid NOT NULL
);


ALTER TABLE public.daemon_interfaced_subnets OWNER TO postgres;

--
-- Name: daemons; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.daemons (
    id uuid NOT NULL,
    site_id uuid NOT NULL,
    host_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    last_seen timestamp with time zone,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    mode text DEFAULT '"Push"'::text,
    url text NOT NULL,
    name text,
    version text,
    user_id uuid NOT NULL,
    api_key_id uuid,
    is_unreachable boolean DEFAULT false NOT NULL,
    standby boolean DEFAULT false NOT NULL,
    standby_cleared_at timestamp with time zone,
    os text
);


ALTER TABLE public.daemons OWNER TO postgres;

--
-- Name: dependencies; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.dependencies (
    id uuid NOT NULL,
    site_id uuid NOT NULL,
    name text NOT NULL,
    description text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    source jsonb NOT NULL,
    color text NOT NULL,
    edge_style text DEFAULT '"SmoothStep"'::text,
    dependency_type text NOT NULL,
    member_type text DEFAULT 'Bindings'::text NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.dependencies OWNER TO postgres;

--
-- Name: dependency_members; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.dependency_members (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    dependency_id uuid NOT NULL,
    binding_id uuid,
    "position" integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    service_id uuid NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.dependency_members OWNER TO postgres;

--
-- Name: discovery; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.discovery (
    id uuid NOT NULL,
    site_id uuid NOT NULL,
    daemon_id uuid NOT NULL,
    run_type jsonb NOT NULL,
    discovery_type jsonb NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    scan_count integer DEFAULT 0 NOT NULL,
    force_full_scan boolean DEFAULT false NOT NULL,
    integration_targets jsonb DEFAULT '[]'::jsonb NOT NULL
);


ALTER TABLE public.discovery OWNER TO postgres;

--
-- Name: discovery_warnings; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.discovery_warnings (
    id uuid NOT NULL,
    discovery_id uuid NOT NULL,
    site_id uuid NOT NULL,
    code text NOT NULL,
    credential_id uuid,
    host_id uuid,
    remote_host_id uuid,
    subnet_id uuid,
    details jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.discovery_warnings OWNER TO postgres;

--
-- Name: entity_tags; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.entity_tags (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    entity_id uuid NOT NULL,
    entity_type character varying(50) NOT NULL,
    tag_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.entity_tags OWNER TO postgres;

--
-- Name: host_credentials; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.host_credentials (
    host_id uuid NOT NULL,
    credential_id uuid NOT NULL,
    ip_address_ids uuid[]
);


ALTER TABLE public.host_credentials OWNER TO postgres;

--
-- Name: hosts; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.hosts (
    id uuid NOT NULL,
    site_id uuid NOT NULL,
    name text NOT NULL,
    hostname text,
    description text,
    source jsonb NOT NULL,
    virtualization_metadata jsonb,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    sys_descr text,
    sys_object_id text,
    sys_location text,
    sys_contact text,
    management_url text,
    chassis_id text,
    manufacturer text,
    model text,
    serial_number text,
    sys_name text,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid,
    virtualization_service_id uuid,
    firmware_revision text,
    sys_descr_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    sys_object_id_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    sys_location_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    sys_contact_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    management_url_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    chassis_id_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    sys_name_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    manufacturer_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    model_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    serial_number_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    firmware_revision_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    name_source jsonb DEFAULT '"Manual"'::jsonb NOT NULL,
    software_revision text,
    software_revision_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    hostname_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    os jsonb,
    os_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    virtualization_interface_id uuid,
    asset_tag text,
    asset_tag_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL
);


ALTER TABLE public.hosts OWNER TO postgres;

--
-- Name: COLUMN hosts.sys_descr; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.sys_descr IS 'SNMP sysDescr.0 - full system description';


--
-- Name: COLUMN hosts.sys_object_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.sys_object_id IS 'SNMP sysObjectID.0 - vendor OID for device identification';


--
-- Name: COLUMN hosts.sys_location; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.sys_location IS 'SNMP sysLocation.0 - physical location';


--
-- Name: COLUMN hosts.sys_contact; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.sys_contact IS 'SNMP sysContact.0 - admin contact info';


--
-- Name: COLUMN hosts.management_url; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.management_url IS 'URL for device management interface (manual or discovered)';


--
-- Name: COLUMN hosts.chassis_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.chassis_id IS 'LLDP lldpLocChassisId - globally unique device identifier for deduplication';


--
-- Name: interface_neighbor_candidates; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.interface_neighbor_candidates (
    id uuid NOT NULL,
    site_id uuid NOT NULL,
    interface_id uuid NOT NULL,
    lldp_chassis_id jsonb,
    lldp_port_id jsonb,
    lldp_sys_name text,
    lldp_port_desc text,
    lldp_mgmt_addr inet,
    lldp_sys_desc text,
    cdp_device_id text,
    cdp_port_id text,
    cdp_platform text,
    cdp_address inet,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    evidence_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL
);


ALTER TABLE public.interface_neighbor_candidates OWNER TO postgres;

--
-- Name: interface_neighbor_hosts; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.interface_neighbor_hosts (
    id uuid NOT NULL,
    site_id uuid NOT NULL,
    interface_id uuid NOT NULL,
    neighbor_host_id uuid NOT NULL,
    neighbor_seen_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.interface_neighbor_hosts OWNER TO postgres;

--
-- Name: interface_neighbor_interfaces; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.interface_neighbor_interfaces (
    id uuid NOT NULL,
    site_id uuid NOT NULL,
    interface_id uuid NOT NULL,
    neighbor_interface_id uuid NOT NULL,
    neighbor_seen_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.interface_neighbor_interfaces OWNER TO postgres;

--
-- Name: interfaces; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.interfaces (
    id uuid NOT NULL,
    host_id uuid NOT NULL,
    site_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    if_index integer,
    if_descr text,
    if_alias text,
    if_type integer,
    speed_bps bigint,
    admin_status integer,
    oper_status integer,
    mac_address macaddr,
    ip_address_id uuid,
    if_name text,
    fdb_macs jsonb,
    native_vlan_id uuid,
    vlan_ids jsonb,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid,
    mac_address_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    ip_configured boolean DEFAULT false NOT NULL,
    interface_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    fdb_macs_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    vlan_membership_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL
);


ALTER TABLE public.interfaces OWNER TO postgres;

--
-- Name: TABLE interfaces; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON TABLE public.interfaces IS 'SNMP ifTable entries - physical/logical interfaces on network devices';


--
-- Name: COLUMN interfaces.if_index; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.if_index IS 'SNMP ifIndex - stable identifier within device';


--
-- Name: COLUMN interfaces.if_descr; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.if_descr IS 'SNMP ifDescr - interface description (e.g., GigabitEthernet0/1)';


--
-- Name: COLUMN interfaces.if_alias; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.if_alias IS 'SNMP ifAlias - user-configured description';


--
-- Name: COLUMN interfaces.if_type; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.if_type IS 'SNMP ifType - IANAifType integer (6=ethernet, 24=loopback, etc.)';


--
-- Name: COLUMN interfaces.speed_bps; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.speed_bps IS 'Interface speed from ifSpeed/ifHighSpeed in bits per second';


--
-- Name: COLUMN interfaces.admin_status; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.admin_status IS 'SNMP ifAdminStatus: 1=up, 2=down, 3=testing';


--
-- Name: COLUMN interfaces.oper_status; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.oper_status IS 'SNMP ifOperStatus: 1=up, 2=down, 3=testing, 4=unknown, 5=dormant, 6=notPresent, 7=lowerLayerDown';


--
-- Name: COLUMN interfaces.ip_address_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.interfaces.ip_address_id IS 'FK to Interface entity when this ifEntry has an IP address (must be on same host)';


--
-- Name: invites; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.invites (
    id uuid NOT NULL,
    organization_id uuid NOT NULL,
    permissions text NOT NULL,
    site_ids uuid[] NOT NULL,
    url text NOT NULL,
    created_by uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    send_to text
);


ALTER TABLE public.invites OWNER TO postgres;

--
-- Name: ip_addresses; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.ip_addresses (
    id uuid NOT NULL,
    site_id uuid NOT NULL,
    host_id uuid NOT NULL,
    subnet_id uuid NOT NULL,
    ip_address inet NOT NULL,
    mac_address macaddr,
    name text,
    "position" integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid,
    mac_address_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    ip_address_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL
);


ALTER TABLE public.ip_addresses OWNER TO postgres;

--
-- Name: TABLE ip_addresses; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON TABLE public.ip_addresses IS 'IP addresses assigned to hosts on subnets';


--
-- Name: license_instances; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.license_instances (
    id uuid NOT NULL,
    organization_id uuid NOT NULL,
    instance_id uuid,
    server_version text,
    last_seen_ip inet,
    non_production boolean,
    first_seen_at timestamp with time zone NOT NULL,
    last_seen_at timestamp with time zone NOT NULL,
    active_since timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.license_instances OWNER TO postgres;

--
-- Name: organizations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.organizations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name text NOT NULL,
    stripe_customer_id text,
    plan jsonb NOT NULL,
    plan_status text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    onboarding jsonb DEFAULT '[]'::jsonb,
    has_payment_method boolean DEFAULT false NOT NULL,
    trial_end_date timestamp with time zone,
    use_case text,
    last_paused_at timestamp with time zone,
    trial_extended_used boolean DEFAULT false NOT NULL,
    last_downgrade_at timestamp with time zone,
    last_downgrade_from_plan jsonb,
    last_discount_at timestamp with time zone,
    discount_save_offer_percent_off bigint,
    discount_save_offer_active_until timestamp with time zone,
    next_renewal_at timestamp with time zone,
    notifications jsonb DEFAULT '{}'::jsonb NOT NULL,
    license_paid_through timestamp with time zone,
    license_checkin_at timestamp with time zone,
    license_key_version bigint DEFAULT 0 NOT NULL,
    license_entitlement text,
    license_entitlement_at timestamp with time zone,
    license_key_issued_at timestamp with time zone,
    license_key_type text,
    bills_by_invoice boolean DEFAULT false NOT NULL,
    license_server_version text
);


ALTER TABLE public.organizations OWNER TO postgres;

--
-- Name: TABLE organizations; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON TABLE public.organizations IS 'Organizations that own sites and have Stripe subscriptions';


--
-- Name: COLUMN organizations.plan; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.organizations.plan IS 'The current billing plan for the organization (e.g., Community, Pro)';


--
-- Name: ports; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.ports (
    id uuid NOT NULL,
    site_id uuid NOT NULL,
    host_id uuid NOT NULL,
    port_number integer NOT NULL,
    protocol text NOT NULL,
    port_type text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid,
    port_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    CONSTRAINT ports_port_number_check CHECK (((port_number >= 0) AND (port_number <= 65535))),
    CONSTRAINT ports_protocol_check CHECK ((protocol = ANY (ARRAY['Tcp'::text, 'Udp'::text])))
);


ALTER TABLE public.ports OWNER TO postgres;

--
-- Name: release_announcements; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.release_announcements (
    id uuid NOT NULL,
    version text NOT NULL,
    email_sent_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.release_announcements OWNER TO postgres;

--
-- Name: server_instance; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.server_instance (
    id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.server_instance OWNER TO postgres;

--
-- Name: services; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.services (
    id uuid NOT NULL,
    site_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    name text NOT NULL,
    host_id uuid NOT NULL,
    service_definition text NOT NULL,
    virtualization_metadata jsonb,
    source jsonb NOT NULL,
    "position" integer DEFAULT 0 NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid,
    virtualization_service_id uuid
);


ALTER TABLE public.services OWNER TO postgres;

--
-- Name: shares; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.shares (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    topology_id uuid NOT NULL,
    site_id uuid NOT NULL,
    created_by uuid NOT NULL,
    name text NOT NULL,
    is_enabled boolean DEFAULT true NOT NULL,
    expires_at timestamp with time zone,
    password_hash text,
    allowed_domains text[],
    options jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    enabled_views jsonb
);


ALTER TABLE public.shares OWNER TO postgres;

--
-- Name: site_credentials; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.site_credentials (
    site_id uuid NOT NULL,
    credential_id uuid NOT NULL
);


ALTER TABLE public.site_credentials OWNER TO postgres;

--
-- Name: sites; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.sites (
    id uuid NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    organization_id uuid NOT NULL,
    stale_after_hours bigint
);


ALTER TABLE public.sites OWNER TO postgres;

--
-- Name: COLUMN sites.organization_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.sites.organization_id IS 'The organization that owns and pays for this site';


--
-- Name: snapshots; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.snapshots (
    id uuid NOT NULL,
    site_id uuid NOT NULL,
    taken_at timestamp with time zone NOT NULL,
    created_by_user_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.snapshots OWNER TO postgres;

--
-- Name: subnet_vlans; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.subnet_vlans (
    id uuid NOT NULL,
    subnet_id uuid NOT NULL,
    vlan_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.subnet_vlans OWNER TO postgres;

--
-- Name: subnets; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.subnets (
    id uuid NOT NULL,
    site_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    cidr text NOT NULL,
    name text NOT NULL,
    description text,
    subnet_type text NOT NULL,
    source jsonb NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid,
    virtualization_service_id uuid,
    cidr_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL
);


ALTER TABLE public.subnets OWNER TO postgres;

--
-- Name: tag_groups; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.tag_groups (
    id uuid NOT NULL,
    organization_id uuid NOT NULL,
    name text NOT NULL,
    description text,
    group_type text NOT NULL,
    entity_types text[],
    ordered boolean DEFAULT true NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.tag_groups OWNER TO postgres;

--
-- Name: tags; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.tags (
    id uuid NOT NULL,
    organization_id uuid NOT NULL,
    name text NOT NULL,
    description text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    color text NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    snapshot_id uuid,
    icon text,
    tag_group_id uuid,
    "position" integer
);


ALTER TABLE public.tags OWNER TO postgres;

--
-- Name: topologies; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.topologies (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    site_id uuid NOT NULL,
    options jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.topologies OWNER TO postgres;

--
-- Name: user_api_key_site_access; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.user_api_key_site_access (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    api_key_id uuid NOT NULL,
    site_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.user_api_key_site_access OWNER TO postgres;

--
-- Name: user_api_keys; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.user_api_keys (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    key text NOT NULL,
    user_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    permissions text DEFAULT 'Viewer'::text NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used timestamp with time zone,
    expires_at timestamp with time zone,
    is_enabled boolean DEFAULT true NOT NULL
);


ALTER TABLE public.user_api_keys OWNER TO postgres;

--
-- Name: user_site_access; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.user_site_access (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    site_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.user_site_access OWNER TO postgres;

--
-- Name: users; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.users (
    id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    password_hash text,
    oidc_provider text,
    oidc_subject text,
    oidc_linked_at timestamp with time zone,
    email text NOT NULL,
    organization_id uuid NOT NULL,
    permissions text DEFAULT 'Member'::text NOT NULL,
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL,
    terms_accepted_at timestamp with time zone,
    email_verified boolean DEFAULT false NOT NULL,
    email_verification_token text,
    email_verification_expires timestamp with time zone,
    password_reset_token text,
    password_reset_expires timestamp with time zone,
    pending_email text,
    email_settings jsonb DEFAULT '{"discovery_digest": true}'::jsonb NOT NULL,
    session_epoch bigint DEFAULT 0 NOT NULL,
    display_settings jsonb DEFAULT '{}'::jsonb NOT NULL,
    email_unsubscribe_token uuid,
    release_notes_seen_version text,
    terms_notice_version date
);


ALTER TABLE public.users OWNER TO postgres;

--
-- Name: COLUMN users.organization_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.users.organization_id IS 'The single organization this user belongs to';


--
-- Name: COLUMN users.permissions; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.users.permissions IS 'User role within their organization: Owner, Member, Viewer';


--
-- Name: vlans; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.vlans (
    id uuid NOT NULL,
    vlan_number smallint NOT NULL,
    name text NOT NULL,
    description text,
    site_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    source jsonb DEFAULT '"Manual"'::jsonb NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    valid_from timestamp with time zone DEFAULT now() NOT NULL,
    valid_to timestamp with time zone,
    lineage_id uuid,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    last_discovery_id uuid,
    first_discovery_id uuid,
    snapshot_id uuid
);


ALTER TABLE public.vlans OWNER TO postgres;

--
-- Name: session; Type: TABLE; Schema: tower_sessions; Owner: postgres
--

CREATE TABLE tower_sessions.session (
    id text NOT NULL,
    data bytea NOT NULL,
    expiry_date timestamp with time zone NOT NULL
);


ALTER TABLE tower_sessions.session OWNER TO postgres;

--
-- Data for Name: _sqlx_migrations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public._sqlx_migrations (version, description, installed_on, success, checksum, execution_time) FROM stdin;
20251006215000	users	2026-10-15 03:33:39.908293+00	t	\\x4f13ce14ff67ef0b7145987c7b22b588745bf9fbb7b673450c26a0f2f9a36ef8ca980e456c8d77cfb1b2d7a4577a64d7	-1
20251006215100	networks	2026-10-15 03:33:39.91353+00	t	\\xeaa5a07a262709f64f0c59f31e25519580c79e2d1a523ce72736848946a34b17dd9adc7498eaf90551af6b7ec6d4e0e3	-1
20251006215151	create_hosts	2026-10-15 03:33:39.918351+00	t	\\x6ec7487074c0724932d21df4cf1ed66645313cf62c159a7179e39cbc261bcb81a24f7933a0e3cf58504f2a90fc5c1962	-1
20251006215155	create_subnets	2026-10-15 03:33:39.922319+00	t	\\xefb5b25742bd5f4489b67351d9f2494a95f307428c911fd8c5f475bfb03926347bdc269bbd048d2ddb06336945b27926	-1
20251006215201	create_groups	2026-10-15 03:33:39.926387+00	t	\\x0a7032bf4d33a0baf020e905da865cde240e2a09dda2f62aa535b2c5d4b26b20be30a3286f1b5192bd94cd4a5dbb5bcd	-1
20251006215204	create_daemons	2026-10-15 03:33:39.930605+00	t	\\xcfea93403b1f9cf9aac374711d4ac72d8a223e3c38a1d2a06d9edb5f94e8a557debac3668271f8176368eadc5105349f	-1
20251006215212	create_services	2026-10-15 03:33:39.935714+00	t	\\xd5b07f82fc7c9da2782a364d46078d7d16b5c08df70cfbf02edcfe9b1b24ab6024ad159292aeea455f15cfd1f4740c1d	-1
20251029193448	user-auth	2026-10-15 03:33:39.945256+00	t	\\xfde8161a8db89d51eeade7517d90a41d560f19645620f2298f78f116219a09728b18e91251ae31e46a47f6942d5a9032	-1
20251030044828	daemon_api	2026-10-15 03:33:39.947386+00	t	\\x181eb3541f51ef5b038b2064660370775d1b364547a214a20dde9c9d4bb95a1c273cd4525ef29e61fa65a3eb4fee0400	-1
20251030170438	host-hide	2026-10-15 03:33:39.948845+00	t	\\x87c6fda7f8456bf610a78e8e98803158caa0e12857c5bab466a5bb0004d41b449004a68e728ca13f17e051f662a15454	-1
20251102224919	create_discovery	2026-10-15 03:33:39.960045+00	t	\\xb32a04abb891aba48f92a059fae7341442355ca8e4af5d109e28e2a4f79ee8e11b2a8f40453b7f6725c2dd6487f26573	-1
20251106235621	normalize-daemon-cols	2026-10-15 03:33:39.962094+00	t	\\x5b137118d506e2708097c432358bf909265b3cf3bacd662b02e2c81ba589a9e0100631c7801cffd9c57bb10a6674fb3b	-1
20251107034459	api_keys	2026-10-15 03:33:39.969961+00	t	\\x3133ec043c0c6e25b6e55f7da84cae52b2a72488116938a2c669c8512c2efe72a74029912bcba1f2a2a0a8b59ef01dde	-1
20251107222650	oidc-auth	2026-10-15 03:33:39.995332+00	t	\\xd349750e0298718cbcd98eaff6e152b3fb45c3d9d62d06eedeb26c75452e9ce1af65c3e52c9f2de4bd532939c2f31096	-1
20251110181948	orgs-billing	2026-10-15 03:33:40.006294+00	t	\\x5bbea7a2dfc9d00213bd66b473289ddd66694eff8a4f3eaab937c985b64c5f8c3ad2d64e960afbb03f335ac6766687aa	-1
20251113223656	group-enhancements	2026-10-15 03:33:40.007697+00	t	\\xbe0699486d85df2bd3edc1f0bf4f1f096d5b6c5070361702c4d203ec2bb640811be88bb1979cfe51b40805ad84d1de65	-1
20251117032720	daemon-mode	2026-10-15 03:33:40.009099+00	t	\\xdd0d899c24b73d70e9970e54b2c748d6b6b55c856ca0f8590fe990da49cc46c700b1ce13f57ff65abd6711f4bd8a6481	-1
20251118143058	set-default-plan	2026-10-15 03:33:40.010571+00	t	\\xd19142607aef84aac7cfb97d60d29bda764d26f513f2c72306734c03cec2651d23eee3ce6cacfd36ca52dbddc462f917	-1
20251118225043	save-topology	2026-10-15 03:33:40.018822+00	t	\\x011a594740c69d8d0f8b0149d49d1b53cfbf948b7866ebd84403394139cb66a44277803462846b06e762577adc3e61a3	-1
20251123232748	network-permissions	2026-10-15 03:33:40.021966+00	t	\\x161be7ae5721c06523d6488606f1a7b1f096193efa1183ecdd1c2c9a4a9f4cad4884e939018917314aaf261d9a3f97ae	-1
20251125001342	billing-updates	2026-10-15 03:33:40.023286+00	t	\\xa235d153d95aeb676e3310a52ccb69dfbd7ca36bba975d5bbca165ceeec7196da12119f23597ea5276c364f90f23db1e	-1
20251128035448	org-onboarding-status	2026-10-15 03:33:40.025958+00	t	\\x1d7a7e9bf23b5078250f31934d1bc47bbaf463ace887e7746af30946e843de41badfc2b213ed64912a18e07b297663d8	-1
20251129180942	nfs-consolidate	2026-10-15 03:33:40.027419+00	t	\\xb38f41d30699a475c2b967f8e43156f3b49bb10341bddbde01d9fb5ba805f6724685e27e53f7e49b6c8b59e29c74f98e	-1
20251206052641	discovery-progress	2026-10-15 03:33:40.029143+00	t	\\x9d433b7b8c58d0d5437a104497e5e214febb2d1441a3ad7c28512e7497ed14fb9458e0d4ff786962a59954cb30da1447	-1
20251206202200	plan-fix	2026-10-15 03:33:40.030162+00	t	\\x242f6699dbf485cf59a8d1b8cd9d7c43aeef635a9316be815a47e15238c5e4af88efaa0daf885be03572948dc0c9edac	-1
20251207061341	daemon-url	2026-10-15 03:33:40.032689+00	t	\\x01172455c4f2d0d57371d18ef66d2ab3b7a8525067ef8a86945c616982e6ce06f5ea1e1560a8f20dadcd5be2223e6df1	-1
20251210045929	tags	2026-10-15 03:33:40.040758+00	t	\\xe3dde83d39f8552b5afcdc1493cddfeffe077751bf55472032bc8b35fc8fc2a2caa3b55b4c2354ace7de03c3977982db	-1
20251210175035	terms	2026-10-15 03:33:40.042007+00	t	\\xe47f0cf7aba1bffa10798bede953da69fd4bfaebf9c75c76226507c558a3595c6bfc6ac8920d11398dbdf3b762769992	-1
20251213025048	hash-keys	2026-10-15 03:33:40.051039+00	t	\\xfc7cbb8ce61f0c225322297f7459dcbe362242b9001c06cb874b7f739cea7ae888d8f0cfaed6623bcbcb9ec54c8cd18b	-1
20251214050638	scanopy	2026-10-15 03:33:40.052752+00	t	\\x0108bb39832305f024126211710689adc48d973ff66e5e59ff49468389b75c1ff95d1fbbb7bdb50e33ec1333a1f29ea6	-1
20251215215724	topo-scanopy-fix	2026-10-15 03:33:40.053617+00	t	\\xed88a4b71b3c9b61d46322b5053362e5a25a9293cd3c420c9df9fcaeb3441254122b8a18f58c297f535c842b8a8b0a38	-1
20251217153736	category_rename	2026-10-15 03:33:40.055786+00	t	\\x03af7ec905e11a77e25038a3c272645da96014da7c50c585a25cea3f9a7579faba3ff45114a5e589d144c9550ba42421	-1
20251218053111	invite-persistence	2026-10-15 03:33:40.061016+00	t	\\x21d12f48b964acfd600f88e70ceb14abd9cf2a8a10db2eae2a6d8f44cf7d20749f93293631e6123e92b7c3c1793877c2	-1
20251219211216	create_shares	2026-10-15 03:33:40.067794+00	t	\\x036485debd3536f9e58ead728f461b925585911acf565970bf3b2ab295b12a2865606d6a56d334c5641dcd42adeb3d68	-1
20251220170928	permissions-cleanup	2026-10-15 03:33:40.069404+00	t	\\x632f7b6702b494301e0d36fd3b900686b1a7f9936aef8c084b5880f1152b8256a125566e2b5ac40216eaadd3c4c64a03	-1
20251220180000	commercial-to-community	2026-10-15 03:33:40.070363+00	t	\\x26fc298486c225f2f01271d611418377c403183ae51daf32fef104ec07c027f2017d138910c4fbfb5f49819a5f4194d6	-1
20251221010000	cleanup_subnet_type	2026-10-15 03:33:40.071333+00	t	\\xb521121f3fd3a10c0de816977ac2a2ffb6118f34f8474ffb9058722abc0dc4cf5cbec83bc6ee49e79a68e6b715087f40	-1
20251221020000	remove_host_target	2026-10-15 03:33:40.072528+00	t	\\x77b5f8872705676ca81a5704bd1eaee90b9a52b404bdaa27a23da2ffd4858d3e131680926a5a00ad2a0d7a24ba229046	-1
20251221030000	user_network_access	2026-10-15 03:33:40.079308+00	t	\\x5c23f5bb6b0b8ca699a17eee6730c4197a006ca21fecc79136a5e5697b9211a81b4cd08ceda70dace6a26408d021ff3a	-1
20251221040000	interfaces_table	2026-10-15 03:33:40.088251+00	t	\\xf7977b6f1e7e5108c614397d03a38c9bd9243fdc422575ec29610366a0c88f443de2132185878d8e291f06a50a8c3244	-1
20251221050000	ports_table	2026-10-15 03:33:40.096342+00	t	\\xdf72f9306b405be7be62c39003ef38408115e740b120f24e8c78b8e136574fff7965c52023b3bc476899613fa5f4fe35	-1
20251221060000	bindings_table	2026-10-15 03:33:40.105871+00	t	\\x933648a724bd179c7f47305e4080db85342d48712cde39374f0f88cde9d7eba8fe5fafba360937331e2a8178dec420c4	-1
20251221070000	group_bindings	2026-10-15 03:33:40.112225+00	t	\\x697475802f6c42e38deee6596f4ba786b09f7b7cd91742fbc5696dd0f9b3ddfce90dd905153f2b1a9e82f959f5a88302	-1
20251222020000	tag_cascade_delete	2026-10-15 03:33:40.114272+00	t	\\xabfb48c0da8522f5c8ea6d482eb5a5f4562ed41f6160a5915f0fd477c7dd0517aa84760ef99ab3a5db3e0f21b0c69b5f	-1
20251223232524	network_remove_default	2026-10-15 03:33:40.115598+00	t	\\x7099fe4e52405e46269d7ce364050da930b481e72484ad3c4772fd2911d2d505476d659fa9f400c63bc287512d033e18	-1
20251225100000	color_enum	2026-10-15 03:33:40.117028+00	t	\\x62cecd9d79a49835a3bea68a7959ab62aa0c1aaa7e2940dec6a7f8a714362df3649f0c1f9313672d9268295ed5a1cfa9	-1
20251227010000	topology_snapshot_migration	2026-10-15 03:33:40.12137+00	t	\\xc042591d254869c0e79c8b52a9ede680fd26f094e2c385f5f017e115f5e3f31ad155f4885d095344f2642ebb70755d54	-1
20251228010000	user_api_keys	2026-10-15 03:33:40.131699+00	t	\\xa41adb558a5b9d94a4e17af3f16839b83f7da072dbeac9251b12d8a84c7bec6df008009acf246468712a975bb36bb5f5	-1
20251230160000	daemon_version_and_maintainer	2026-10-15 03:33:40.134979+00	t	\\xafed3d9f00adb8c1b0896fb663af801926c218472a0a197f90ecdaa13305a78846a9e15af0043ec010328ba533fca68f	-1
20260103000000	service_position	2026-10-15 03:33:40.137678+00	t	\\x19d00e8c8b300d1c74d721931f4d771ec7bc4e06db0d6a78126e00785586fdc4bcff5b832eeae2fce0cb8d01e12a7fb5	-1
20260106000000	interface_mac_index	2026-10-15 03:33:40.139694+00	t	\\xa26248372a1e31af46a9c6fbdaef178982229e2ceeb90cc6a289d5764f87a38747294b3adf5f21276b5d171e42bdb6ac	-1
20260106204402	entity_tags_junction	2026-10-15 03:33:40.15126+00	t	\\xf73c604f9f0b8db065d990a861684b0dbd62c3ef9bead120c68431c933774de56491a53f021e79f09801680152f5a08a	-1
20260108033856	fix_entity_tags_json_format	2026-10-15 03:33:40.152999+00	t	\\x197eaa063d4f96dd0e897ad8fd96cc1ba9a54dda40a93a5c12eac14597e4dea4c806dd0a527736fb5807b7a8870d9916	-1
20260110000000	email_verification	2026-10-15 03:33:40.156413+00	t	\\xb8da8433f58ba4ce846b9fa0c2551795747a8473ad10266b19685504847458ea69d27a0ce430151cfb426f5f5fb6ac3a	-1
20260114145808	daemon_user_fk_set_null	2026-10-15 03:33:40.157844+00	t	\\x57b060be9fc314d7c5851c75661ca8269118feea6cf7ee9c61b147a0e117c4d39642cf0d1acdf7a723a9a76066c1b8ff	-1
20260116010000	snmp_credentials	2026-10-15 03:33:40.165138+00	t	\\x6f3971cf194d56883c61fa795406a8ab568307ed86544920d098b32a6a1ebb7effcb5ec38a70fdc9b617eff92d63d51e	-1
20260116020000	host_snmp_fields	2026-10-15 03:33:40.169465+00	t	\\xf2f088c13ab0dd34e1cb1e5327b0b4137440b0146e5ce1e78b8d2dfa05d9b5a12a328eeb807988453a8a43ad8a1c95ba	-1
20260116030000	if_entries	2026-10-15 03:33:40.18137+00	t	\\xa58391708f8b21901ab9250af528f638a6055462f70ffddfd7c451433aacdabd62825546fa8be108f23a3cae78b8ae28	-1
20260116100000	daemon_api_key_link	2026-10-15 03:33:40.184903+00	t	\\x41088aa314ab173344a6b416280721806b2f296a32a8d8cae58c7e5717f389fe599134ed03980ed97e4b7659e99c4f82	-1
20260131190000	add_hubspot_company_id	2026-10-15 03:33:40.186088+00	t	\\x4326f95f4954e176157c1c3e034074a3e5c44da4d60bbd7a9e4b6238c9ef52a30f8b38d3c887864b6e4c1163dc062beb	-1
20260201021238	fix_service_acronym_capitalization	2026-10-15 03:33:40.187534+00	t	\\x88b010ac8f0223d880ea6a730f11dc6d27fa5de9d8747de3431e46d59f1dbf2f72ae4a87c2e52c32152549f5c1f96bb2	-1
20260204004436	add_entity_tags_to_topology	2026-10-15 03:33:40.188761+00	t	\\x3eff1a1490e77065ec861ef1b9aad8c55de0170106a42720f7931b3929b179122b16e44390b2652771bf91bba32a7757	-1
20260205120000	billing_overhaul	2026-10-15 03:33:40.190842+00	t	\\xbf850cfa0c40a3c65f574efd15fd55a4b702296203d28077a09d1c22076fee8601f2b78345aef370ab9163657de767ab	-1
20260205183207	rename_hubspot_to_brevo	2026-10-15 03:33:40.192131+00	t	\\x4678a7d80215e5eafb5e80af0daa20e2868a3b4f2112e88cb1b2b9efc87d63de3fb96c133f359b224c658789ae4b0d13	-1
20260221120000	add_plan_limit_notifications	2026-10-15 03:33:40.193594+00	t	\\xef770dac07e1d80888832f33184dc46c1d3b8185b91c507cb404468d6ad8c29cacf455178801c67aa27b6a626d3ad82d	-1
20260222120000	add_pending_email	2026-10-15 03:33:40.195381+00	t	\\xddd220f7602c44548d56849c0a8d081ecd1da1383374a11e3e227c7d9becb73a49f5e5bb09ed65901c16df4c16e913e5	-1
20260301120000	add_if_name_to_if_entries	2026-10-15 03:33:40.196536+00	t	\\xc9fc0a2b77ecbf0e1d5ab292c4fe162a26113468c878dfd26a3c63d89c0ee1957ca328ecfe25c611867a0e73780f0cb6	-1
20260306002816	cleanup_standby	2026-10-15 03:33:40.197651+00	t	\\x01b0c236a8a4d0d97f0f633b18f8cbdb92b6d72063289989b90a1b7b6b303e65e0557eb09927b2580dcb7e8ee5966c75	-1
20260309120000	add_org_use_case	2026-10-15 03:33:40.198797+00	t	\\xdb8c8a2f0f9416ba3b687fc75453d7c12c50a6f386b4784d21bd6adfc4a4a7556c637c25cf116118402bbd12c0d5aafe	-1
20260313120000	snmp_extended_discovery	2026-10-15 03:33:40.200587+00	t	\\xc4e72539099de1b830d87a169bfbabba4b8fb378a3c4c4a1dfca698adf3e403d750040d784c26d9fa343be2908064c9d	-1
20260315120000	universal_credentials	2026-10-15 03:33:40.217225+00	t	\\x87dc6f39202e81d5555df78a9d056b143f11bd22e6d7f483065f605e242a360902c72c4d5a49717e7fcc24a366bb5ff5	-1
20260315120001	discovery_scan_settings	2026-10-15 03:33:40.218684+00	t	\\xe9da183fdd8e04e574f553f61f6f33efa046cdae38c846c8077b06c5260446fb4aa39da2449bda7f1d8cf3aa9f16e158	-1
20260315120002	backfill_org_created_milestone	2026-10-15 03:33:40.219714+00	t	\\x14f886a19773cd2263d86f88479be460d21f071d5212e3789c5c40b6415c293fc7d06c7b138351cc42108f89a14fe745	-1
20260316120000	fix_jsonb_null_if_entries	2026-10-15 03:33:40.221344+00	t	\\x65c358069710f7f86d6a3e257e658c2f241cc376433c3a0317b0ec9e1876a66f9738cb65c6ab1a5c197fe40d5aa2aa2b	-1
20260319120000	rename_snmp_to_snmpv2c	2026-10-15 03:33:40.222445+00	t	\\xdce5c9461f402e1672607078b2c571f0eb30b51d46f8e9414d8909efb40693f543e49e560cb7d703db274515043aa08e	-1
20260321120000	add_discovery_scan_count	2026-10-15 03:33:40.224453+00	t	\\x6c8201ab453a51632176d534c6604e0818e28a8a4a153e33e254f4dac0f9b67c9db394082cb663ff1b25941229cf96fc	-1
20260329120000	backfill_subnet_virtualization	2026-10-15 03:33:40.22658+00	t	\\xeac50ded27603dbb5e8773604a52143c9fa8654263e7dd12d3d128ce972c2feed84600e36b2e7a79525b58c44d2ad9d3	-1
20260402120000	rename_topology_node_types	2026-10-15 03:33:40.227482+00	t	\\xc4ba06868add823f83ff1948091bdfe17dbdde80bbec6fe2cf8da2b3689aeeebbe9e9de01b1292bff3c98a74d9e6279f	-1
20260403120000	topology_grouping_rules	2026-10-15 03:33:40.228874+00	t	\\x00799da1206d7c3b3c3db90b7d14437cc054ed2d7273020342e562c619a671e008ff4fdf0365170440b392956949e730	-1
20260405120000	rename_groups_to_dependencies	2026-10-15 03:33:40.236544+00	t	\\x9ce895b456366bf6e54316b22cabd2803aa542dd3733fffa680f0a3af5c4c55a612c5ee511371206921869b7f271c35b	-1
20260406120000	add_tag_is_application_group	2026-10-15 03:33:40.237803+00	t	\\xb7a71e5fdd96ca46c9c7577003309050a93bc53ad192ac5df78e7621f3ed64f07fb29b4658f17af55732cf6dfb7958c2	-1
20260406130000	add_vlans	2026-10-15 03:33:40.247145+00	t	\\x5b3e5d10578d90b5175e5718a28d7147a21b99af2fb3e0ed171d20ee8fd8838c290f648dafdd3b72ef60ff487f7f2494	-1
20260409000000	add_vlans_to_topologies	2026-10-15 03:33:40.24884+00	t	\\x5e0b9dc670580ceec3aa6eae005a39f98733fc27dc574b7f3922f4297813facd5d610af953dfec13e09d0b99eceb3865	-1
20260410000000	rename_interfaces_and_if_entries	2026-10-15 03:33:40.255207+00	t	\\x07f54a59869f458f41f45d75f250aee26b20a426f1ec29930606841770194d6aea0e9e6253a6375fbeebcf9b49121224	-1
20260414000000	add_share_enabled_views	2026-10-15 03:33:40.256351+00	t	\\xc56514355a5977c3242e728e7f5a2533e7b4a5cf8a7ce7757e412e51f1ad85e96d65c13ccd96d050be4a07799b9aef57	-1
20260415120000	rename_onboarding_first_group_created	2026-10-15 03:33:40.257327+00	t	\\x2c17035835d3ead105b76d98688c0b7bd328abdaf9f721d70d057c8afdf438819e93da56707deea5b469b81a7b84d5d7	-1
20260417000000	reindex_interfaces_identity	2026-10-15 03:33:40.260947+00	t	\\x10701e13bc3d838e2ec4a856555ebf338173792f220c405996d3c77e7987e9806798ca0328eb6259e4a62b7e05665b25	-1
20260418000000	add_standby_cleared_at_to_daemons	2026-10-15 03:33:40.261925+00	t	\\x547807de451d015a4ce1438796d5b95e2b98043c521015a21239f6778d10a8d3bf7d8b14e278e09aa0105f1935ad4181	-1
20260501000000	add_organization_billing_flags	2026-10-15 03:33:40.264142+00	t	\\x2de34c4af667d4cd8bc263c27f0526a4a2132022e2eb71ae94fd89edebbfb40cda840055a94e89a21925a317cdff285f	-1
20260502000000	scd2_add_columns	2026-10-15 03:33:40.27702+00	t	\\xe78a73574d86320c0de7fcd43682ee3cdd436dc64d1371f98cb67ef0cdf33097df6e8a92c9a26c5702795144103b085e	-1
20260502000001	scd2_add_discovery_fks_not_valid	2026-10-15 03:33:40.287378+00	t	\\xb49114eb6f8d77cb0c5062619da5c829e5bd65898710efb6207d2804be3677a95eebe88d41d21ae8cba21344fcd63f97	-1
20260502000002	scd2_validate_discovery_fks	2026-10-15 03:33:40.295903+00	t	\\x500201842b9f486d1397c0f0f3ee1a36bc101440d4d97fe0f0bfc971672f1f53f7945aed93c34e2e53d2d9cb913419f1	-1
20260502000003	scd2_backfill_with_metadata	2026-10-15 03:33:40.299864+00	t	\\xd299c1dbf7ec284995dc8a5fcf4b264e4f1ff970085a8631e6d00aa0bc6d6708a5e41d515bb9299c22ef128a958e68e9	-1
20260502000004	scd2_partial_unique_indexes	2026-10-15 03:33:40.3207+00	t	\\xbc30e554c37ec3ca72baa7fc634920257909944dc841231a966845fd4c5bc27ae20606c98867934c872e8883bd56efa1	-1
20260502000005	scd2_drop_old_unique_constraints	2026-10-15 03:33:40.324038+00	t	\\x660c0237299796c6e70a21780d529453a2ac8645f7b122a0b8aa4f362ff780de8849f53a66b71b3ea29ecc66877f47ad	-1
20260502000006	scd2_supporting_indexes	2026-10-15 03:33:40.389777+00	t	\\x4394aaf6951c4b7048f66334fb8c550c55d4cdc06beb29eb7f086a717ee9f26ddeb28e73a7b298633ef517f6c0f84ee0	-1
20260502000007	entitysource_metadata_strip	2026-10-15 03:33:40.392795+00	t	\\x1cfd1a00b100ca0b007be6e8b55f67dde412108f299f94b419a80af45b74b75fdead95637dccb13fdd57fed2f1132446	-1
20260502100000	add_user_email_settings	2026-10-15 03:33:40.394368+00	t	\\xe886b44ca9d3d6461d82b573f617391b8b5f22f18b8113a333aa8c275545ec6455060f68ecbfdd17af86fccc9a500d7a	-1
20260502120000	create_snapshots_table	2026-10-15 03:33:40.398476+00	t	\\x18f829b255b8e11e385903e51182f04fb835aa78ef0090865ce7743f747e4aaaa7b1a7b806a377e9b12af6205aed527a	-1
20260502120001	add_snapshot_id_fks	2026-10-15 03:33:40.40791+00	t	\\x0a080ceaa2922301464b30ea7f25e1f0de5493d15424d07a8dd1f2b4bf1880cf2d25f64d5bf1863f705a1557d867afa2	-1
20260502120002	add_snapshot_id_indexes	2026-10-15 03:33:40.429805+00	t	\\xb7514ff0cf637241d21e90b48214a3a3a5076d0142c99aca39479d2a82cf35e1abdbce8ad5663ca8272d5f74004ae5f9	-1
20260502120003	topology_snapshots_backfill	2026-10-15 03:33:40.434587+00	t	\\x5435da11acf8a9fc37e0520a5b2a959fce84b16080b2a5b67adcf249dea07ae533f858c56f87542152cfc271933c1428	-1
20260502120004	drop_legacy_topology_columns	2026-10-15 03:33:40.443669+00	t	\\xfe45e73be308e6267840664a662cd679fb43469a11f06a168b8b8cfc64b6e86eb10d948271b67a0541481dcc60447769	-1
20260627120000	add_discovery_integration_targets	2026-10-15 03:33:40.44586+00	t	\\x9adf9ea4c36868a3b86067214f8550b8e866aff5d3d6c6ab3d71d1a15ea5e4be0b4349db6d15308fefbceca8403f0cbb	-1
20260703120000	daemon_interfaced_subnets_junction	2026-10-15 03:33:40.450729+00	t	\\xf1399bf8104b1d9cff6a7f66c6040829f01dd64614294295f0ecc27412acc19fd6dd20eff66ce9eadf0cf0c0e8c4353e	-1
20260703120001	drop_discovery_pending_credential_ids	2026-10-15 03:33:40.452331+00	t	\\xc2cfe6f450d000ae36149b042ebcfed938c01487d78ca56a125d72bbeca0de0548824a2396761b3dc647dc80e2e43e80	-1
20260705120000	add_user_session_epoch	2026-10-15 03:33:40.453839+00	t	\\xb296babe8e50ecdd1f50770e12d4234b47ed41cdd826c545debf5f94226672cc2d6fd28eef0483a5fdc8837f5d280d88	-1
20260706120000	drop_credentials_target_ips_and_daemons_capabilities	2026-10-15 03:33:40.455756+00	t	\\xc544c88df68334ac0b3ec180da7ba2030837707f9e3cf2b75b4d56012646c24f8f60f094cc00f05c0f446a2a845e5e98	-1
20260716120000	api_keys_daemon_id	2026-10-15 03:33:40.458069+00	t	\\x484c3b2553bcf81c95aabce4be70b1e2048e097529e04a176a0b75c2edf28ecd0892c959bbd8e8166f621a9dc2dba0dc	-1
20260716120001	api_keys_daemon_id_validate	2026-10-15 03:33:40.459621+00	t	\\x723190451e98f9394b2134dccb89f8eeeb15f6a5644fb88cb24e9e0e7a4ebf5a0c455659930ce2d3096e20c004d7da89	-1
20260716120002	api_keys_daemon_id_unique	2026-10-15 03:33:40.461942+00	t	\\xf788be5a414917ec5c8b9040f85acd352ae3f580dab7e8833db5df19b7d2cf3b43b8a021c03a5082da27eca3bb7a07f0	-1
20260722120000	add_networks_stale_after_hours	2026-10-15 03:33:40.46299+00	t	\\x308d9e04ef943560d3e420c59a3c8017064d59bd28cb63f99c4b562124e2fa038630e71405171901d731420bd9de4e2f	-1
20260727120000	rename_plan_limit_notifications_to_notifications	2026-10-15 03:33:40.464602+00	t	\\x2447c735b8ac6885ae4341738294392f367abcc7c0f5e857bb08523dc5d5a8eccf77db5462b531370a0795135757098d	-1
20260728120000	rename_stale_if_entries_constraints	2026-10-15 03:33:40.467584+00	t	\\x577c54966f8c146a6e5101eb8a2f7f006ef7899ba95ec876be10970b3483b622b254d00dc9c9582efb1862404315764b	-1
20260803120000	drop_organizations_plan_limit_notifications	2026-10-15 03:33:40.469335+00	t	\\xc4c9b44a4cce6cf419cffcbbeade3d5eb119b51f321bdcd3c51a1046d8ec8df81d32f6e76c6472eefdcfae9899ef3107	-1
20260803120001	virtualization_service_id_column	2026-10-15 03:33:40.477253+00	t	\\xe07f6fee751430c94141be6d7ba17390f08f60bdcd01a65072708382b27e9543a9f27fb0cc76bb824940794d6ea67553	-1
20260819120000	hosts_name_source	2026-10-15 03:33:40.481919+00	t	\\x6526859d261bf724e6ac3bb2a51c8588764aab15ecdfad964e06e98bb59f05a753ea2950bdce40cbf7f945d157a230aa	-1
20260824120000	interfaces_neighbor_seen_at	2026-10-15 03:33:40.483653+00	t	\\x70a26934dcc110484dc95100cbc96879504a9bc0ae0fb12ca0effe032816e3282e255d90145c4d2c6bfe2eea351cb4c3	-1
20260826120000	subnets_cidr_source	2026-10-15 03:33:40.486474+00	t	\\x1d8c6a9258f1fd07a08a268cddf5f164bb3b8bb0af34d6d167e480d50b649cbdc5a7b89aa419a218266e9aa1b3d3b497	-1
20260827120000	hosts_firmware_revision	2026-10-15 03:33:40.489067+00	t	\\xa9b3a89986c699c70232d37f5909ab5fecd9c4d2c8708a11b2cad4775494a12080c6664964f942b0079a883f3ecd6ea2	-1
20260827130000	interfaces_unknown_port_facts	2026-10-15 03:33:40.490742+00	t	\\xfa35944166a6c60219a07a6792cd3abbaf909ba78e5494482d73d3d10ddacea618845822dc3dc875542772af742ac38d	-1
20260828120000	hosts_attribute_sources	2026-10-15 03:33:40.510612+00	t	\\x29220b8fb99e5ffb55ca71ea3c05f375214dd77d247d4a54dc7855c5654dbfbbbefceafeb5ddf10710c27085f0a866c4	-1
20260828120001	mac_address_sources	2026-10-15 03:33:40.512758+00	t	\\x39ea28fed133ae2c317bd5fad54e4d16f402ee895b75a6290322e75b434f5502aeddc9f6359d7e95625bb0c0d4ca1d4e	-1
20260828130000	hosts_software_revision	2026-10-15 03:33:40.515091+00	t	\\x83cdafd5691f1bc5fc2345a696ec9fd5c75700693f0011c65d352276d806a6f207f335189153ee8e0fa3fb7e60c18234	-1
20260828140000	ip_addresses_network_mac_index	2026-10-15 03:33:40.517749+00	t	\\x838b853bbb63bcfe0d0976f1c7289960e3133563151c47904ddda0ce5a0078bdc4af9ce07de63d3c024d89b831995535	-1
20260903120000	interfaces_if_descr_optional	2026-10-15 03:33:40.518951+00	t	\\x54495748b0271deebaf7cb06e9e4da9241563e83a162e4bfffaa9e571516223565ab239fcc8bb51fac9e37cbb7625757	-1
20260903120001	interface_neighbor_tables	2026-10-15 03:33:40.542162+00	t	\\xa8352a3ed9b845d96229adcb3e3eb8e796d22a8b6aa794bbc0d2ecafcbf76d68aa59a6e7713f134a57af0acceca3865a	-1
20260908120000	interfaces_ip_configured	2026-10-15 03:33:40.543935+00	t	\\x2467c0b912e78e9cac3818ffdec4157356f6b4a3c96e87a0171d88496439943eacbafefc7a4b51a2b68cc08bbbd29656	-1
20261005120001	rename_networks_to_sites_backfill	2026-10-15 03:33:40.591534+00	t	\\x085c1f2e6ec6b548e7f47fb092519f5f8889629e6198b30bd4a91e4e1d6b5e502513efb213c0601abf444d1f39183f81	-1
20261006120000	host_asset_tag	2026-10-15 03:33:40.593463+00	t	\\x1a9ef01df34f62321f6fdc26f4f75d952000cc366b54217a3107132643258e44957033c5f37b3618ff8d895d01eae918	-1
20261006120100	tag_group_and_icon	2026-10-15 03:33:40.594758+00	t	\\x3bad6e7e42b3a23b29a94462ddefba434ba2e7e92d1d9464db16644d40e45b97d01a83716203f5cda27d7bb5623dd436	-1
20261009120000	user_email_unsubscribe_token	2026-10-15 03:33:40.596302+00	t	\\x41aa0b879e77d264d1d6fc17c65bebe7e2d7796b9785ad79a85b928e23cc93c3cb8a0532ecbc8d72e34fc6e202d25ce1	-1
20261009120001	user_email_unsubscribe_token_index	2026-10-15 03:33:40.598925+00	t	\\x3cdfe6e7f6fe847ff4f18c7ee1d2430ee9833e48c6aa4eb3585c873529bb007b8786c26e6bad1c193ee8cf0dc0b23f0c	-1
20261009120100	tag_groups	2026-10-15 03:33:40.627304+00	t	\\xbdae7fe6cd73793312edbf2c987d3425f1e2c88c9fa98717a9e21550503734aaf67b3a1b804e145269962cd361aaa16a	-1
20261011120000	backfill_interface_credential_milestone	2026-10-15 03:33:40.628809+00	t	\\xfcad57a2a59537e0c9033a6ff906abf7115789554bc06c00c0caa388c220a6a5d3dc880c7aec0df9c6f561b3b94d7477	-1
20261011130000	release_notes	2026-10-15 03:33:40.632402+00	t	\\x46e5e77f7da9f2a01cb042045c01461eeb40b09477a5d2a40f302582b17e2b02e40f6e9dbe40ed1b24e667227ba3a2b3	-1
20261011130001	merge_and_seed_default_credentials	2026-10-15 03:33:40.649743+00	t	\\xc5fc006e4c216dd10154418fd2d52cf3e58b70039498b21a2fc74a1b3e8d9ce34af25da2b6bc197d0c7dff4f9ca910eb	-1
20261012120000	collection_sources	2026-10-15 03:33:40.653328+00	t	\\xfa0a6238fae38ae0c45b9657d540271cf8201a4981d6a16d84c856e8ba192e312b22472db6e2f734e1cd0a56eae723de	-1
20260914120000	hosts_hostname_source	2026-10-15 03:33:40.547043+00	t	\\xa57a5dc26c000ec5b0b4aaf66486018f637b1f1228ae994dafa06a653b5afae3d8dd2bb1264f5695c50053d656bccdc2	-1
20260915120000	organization_license_columns	2026-10-15 03:33:40.548896+00	t	\\xd46a3e083f8179128aae52f68541627b57131d66c93d84c2ad0002a7bf6c025f757345490b47d810511d49cd9528525d	-1
20260915120001	organizations_license_entitlement	2026-10-15 03:33:40.550159+00	t	\\x1da434d08d6afb9f0bbe4165f112cae53d3046c8578d4feb88d5f3ea0fa0649cd4bf2a69ac6c2c24978a561a6333fcb9	-1
20260916120000	organization_license_key_issued_at	2026-10-15 03:33:40.551254+00	t	\\x99d66ef126e64532e4e87c1a99b0ac630a2031b1b4c9ba41cbe257b8e3a83e47ec0fe1360a409b5cbb57401326d7a9d3	-1
20260917120000	organization_license_key_type	2026-10-15 03:33:40.552409+00	t	\\xac327476c465df672532ce71bfbe03a0d92ff5cc3053fae9a15fa24100308cda6b08e118267c3c1009118eeb90f46038	-1
20260917130000	organization_bills_by_invoice	2026-10-15 03:33:40.5539+00	t	\\x33bf32d3b3edce4fe6803c326785210f322b9cdfd065440098b53424df7042ba90da1ac6f99ade774f894da1b67b0979	-1
20260930120000	clear_zero_mac_addresses	2026-10-15 03:33:40.55854+00	t	\\xef8416f55b61953f00f1fd249a04c82d63d0d8d510e9db6cae96f79db7a70a61dfc5a22029a6878e74da05e60b63b7dd	-1
20261001120000	add_user_display_settings	2026-10-15 03:33:40.560012+00	t	\\xd7f9afb0762c40085ced713496cf39e9778c4257196e1ba02a0feec1af67eab309df78ce5b753ebde6d250f6fbf3ef32	-1
20261001120001	credential_daemon_os	2026-10-15 03:33:40.561875+00	t	\\xe679b2c2caea51e608200d064b81b02f1f4cee20cfa60a929db29b87666ba8a5d01e5d0739a2b9953f401f4171c11c9d	-1
20261002120000	daemon_os	2026-10-15 03:33:40.562999+00	t	\\x931ef66a80cd751db47512b5f9397a5d7df21aff77cbf28c5a190c69b27ce6040d2e2f2ed3c08e85faf13d28e5cba25f	-1
20261003120000	hosts_os	2026-10-15 03:33:40.565121+00	t	\\xb8a3b3a3e6ec9e21cc64468fa5b691dba7d39556aab7cd28fee01a21fcbde0782952b4b73cbdbd334a17c554769fbdce	-1
20261003130000	organization_license_server_version	2026-10-15 03:33:40.566199+00	t	\\xe2f1bf9692ed8bb1a08a1ae13b53f632fd5f2112d53225bbf13332af26639b9518edb28daa4b3ed98bf71ed4b1002d34	-1
20261004120000	host_virtualization_interface	2026-10-15 03:33:40.568631+00	t	\\x79224f29c669331336f577a5c9f7e5f674dec637f1a879e9c4016e8705af2e921e63cf4ab178cb21af1026317d6e57be	-1
20261004120001	host_virtualization_interface_validate	2026-10-15 03:33:40.570509+00	t	\\xf03d5b6826e7aca5a5efe1ab57878591bfd12ac5c097b1d71e5539348cf9a5e794c2ef19c5fa3ee8aac039920b9f1f4e	-1
20261004120002	host_virtualization_interface_index	2026-10-15 03:33:40.572882+00	t	\\x64e39691a98e9ed761812e1bfdeaa7b8d9aa13eb0401a39fb40dd36f21222ac47dad7f1b4ff9465a3cd0da1af42039ee	-1
20261005120000	rename_networks_to_sites	2026-10-15 03:33:40.58527+00	t	\\x5e08911ddc549e9d522eece781caa57fe42ee4cb6f7d16fb25b63bf6c7f95e897d2cbd12db345040f5a4035d420ace43	-1
20261012130000	discovery_warnings	2026-10-15 03:33:40.669071+00	t	\\xc760d410eaa7d4d14214bf602cd9ba7423c1a328b71768d79bcef0cee51b9b685c37438f74091fb9c3afd6483a98e46c	-1
20261013120000	license_instances	2026-10-15 03:33:40.675152+00	t	\\x86796dc3b14fc5805b84f6c6fd92fa8aaa560a004ea4ca8f4affa76f55b830f8d2403393cddfcf215070e84d499bb5cb	-1
20261013140000	terms_notice_version	2026-10-15 03:33:40.676448+00	t	\\x624bbb290511d7491b8816b152b06194417047e2b9b19eeb8b1d954ec17e6fee77c73d9fd19fb74d62f8c44d66bb1a93	-1
\.


--
-- Data for Name: api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.api_keys (id, key, site_id, name, created_at, updated_at, last_used, expires_at, is_enabled, plaintext, daemon_id) FROM stdin;
e932eb5d-117d-4ebf-ad71-25dc834d62c9	e5b4e187ffcb847add46fddc2b95c478d7b0a55450249ab328a59609d13c5706	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	Compat Test API Key	2026-10-15 03:41:41.936657+00	2026-10-15 03:41:41.936657+00	2026-10-15 03:42:02.654534+00	\N	t	\N	\N
0236a02c-226b-4a23-a285-b2a449c86e8d	328a8d8f7f297f12728027f2c14f04a7c0ad55d008f5f46f3aea65766e925ca0	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	scanopy-daemon-serverpoll API Key	2026-10-15 03:34:44.014415+00	2026-10-15 03:34:44.014415+00	2026-10-15 03:42:18.173635+00	\N	t	scp_d_fE5amMb5stpFm8Ch2LwxyFpKOtQSW2N0	96d7b351-8ddb-4403-96f9-d50e23f473ee
c443357f-2470-4ed1-9ccd-07ca0e16723a	c16adaac9db178dad5b2436a647c29fff4604a55bd5b8092da2b75dcaad18ad3	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	scanopy-daemon API Key	2026-10-15 03:33:45.704366+00	2026-10-15 03:33:45.704366+00	2026-10-15 03:42:23.298036+00	\N	t	\N	d199ce69-450c-4a64-a87b-d7b6470c432b
\.


--
-- Data for Name: bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.bindings (id, site_id, service_id, binding_type, ip_address_id, port_id, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
252b1078-0d64-4cbd-a889-00d02445629d	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	0b62457e-a3d6-4712-b086-b55ab8eceb97	Port	618bc887-da03-48ca-8c41-542c0d06e3fa	fda4149b-1b8f-4143-a8fd-6326626f2c98	2026-03-06 07:04:08.184024+00	2026-03-06 07:04:08.184024+00	2026-10-15 03:42:06.907721+00	\N	\N	2026-10-15 03:42:06.907722+00	\N	\N	\N
\.


--
-- Data for Name: credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.credentials (id, organization_id, name, credential_type, created_at, updated_at, daemon_os, description) FROM stdin;
a93bbe5d-146c-4264-a367-191f44da54f6	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	SNMP (public)	{"type": "SnmpV2c", "community": {"mode": "Inline", "value": "public"}}	2026-10-15 03:33:45.610186+00	2026-10-15 03:33:45.610186+00	\N	Assigned to every new site. Tries the SNMPv2c community "public" on every host.
75c2934d-993a-4079-8155-e3648a9e5925	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Docker socket	{"type": "DockerSocket"}	2026-10-15 03:33:45.61947+00	2026-10-15 03:33:45.61947+00	\N	Added to every new daemon. Reads the local Docker socket at its default path.
cba9d1c4-5409-4cd7-9dc5-1301515e7f53	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Podman socket	{"type": "PodmanSocket"}	2026-10-15 03:33:45.622279+00	2026-10-15 03:33:45.622279+00	\N	Added to every new daemon. Reads the local Podman socket at its default path.
\.


--
-- Data for Name: daemon_interfaced_subnets; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemon_interfaced_subnets (daemon_id, subnet_id) FROM stdin;
96d7b351-8ddb-4403-96f9-d50e23f473ee	571af076-9a61-4b37-a74f-ba5254746f48
96d7b351-8ddb-4403-96f9-d50e23f473ee	a764172f-790f-4870-b9fe-aac03274c53c
d199ce69-450c-4a64-a87b-d7b6470c432b	571af076-9a61-4b37-a74f-ba5254746f48
d199ce69-450c-4a64-a87b-d7b6470c432b	a764172f-790f-4870-b9fe-aac03274c53c
\.


--
-- Data for Name: daemons; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemons (id, site_id, host_id, created_at, last_seen, updated_at, mode, url, name, version, user_id, api_key_id, is_unreachable, standby, standby_cleared_at, os) FROM stdin;
d199ce69-450c-4a64-a87b-d7b6470c432b	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	6943745c-a092-47e4-9991-74ba816a3f6a	2026-10-15 03:33:45.720719+00	2026-10-15 03:42:23.305175+00	2026-10-15 03:33:45.720719+00	"daemon_poll"		scanopy-daemon	0.18.2	ad97c597-a289-4837-a404-079bb7efe966	c443357f-2470-4ed1-9ccd-07ca0e16723a	f	f	\N	"linux"
96d7b351-8ddb-4403-96f9-d50e23f473ee	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	71257747-2cda-461d-be2e-37ddacd1f60c	2026-10-15 03:34:44.020848+00	2026-10-15 03:42:25.482004+00	2026-10-15 03:34:44.020848+00	"server_poll"	http://daemon-serverpoll:60074	scanopy-daemon-serverpoll	0.18.2	ad97c597-a289-4837-a404-079bb7efe966	0236a02c-226b-4a23-a285-b2a449c86e8d	f	f	2026-10-15 03:42:25.482006+00	"linux"
\.


--
-- Data for Name: dependencies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.dependencies (id, site_id, name, description, created_at, updated_at, source, color, edge_style, dependency_type, member_type, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
39711915-977f-4e5b-8704-a410edf874d3	4b74f32e-3f98-4de2-863f-ffff5fe88fe2		\N	2026-10-15 03:41:41.340487+00	2026-10-15 03:41:41.340487+00	{"type": "Manual"}	Yellow	"SmoothStep"	RequestPath	Services	2026-10-15 03:41:41.340487+00	\N	\N	\N
\.


--
-- Data for Name: dependency_members; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.dependency_members (id, dependency_id, binding_id, "position", created_at, service_id, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
\.


--
-- Data for Name: discovery; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.discovery (id, site_id, daemon_id, run_type, discovery_type, name, created_at, updated_at, scan_count, force_full_scan, integration_targets) FROM stdin;
43f80b66-bfe8-408d-a2dc-24b2b2ca0569	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "c10d2eae-1267-4023-997e-5a51fdf97281", "started_at": "2026-03-11T06:32:25.836913871Z", "finished_at": "2026-03-11T06:32:25.836988451Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:44.489560197Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:32:25.836913+00	2026-10-15 03:41:44.491889+00	0	f	[]
a0e17c57-7a55-48b1-9b19-012a790e9e9b	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Scheduled", "enabled": true, "last_run": "2026-10-15T03:33:47.988855681Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "6943745c-a092-47e4-9991-74ba816a3f6a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:33:45.725372+00	2026-10-15 03:34:43.688522+00	1	f	[]
760f2ebf-e803-4e95-a994-5c8c0556d20f	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 100, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "be760657-cb8b-4175-8484-00ea5633e0a3", "started_at": "2026-10-15T03:33:48.001242310Z", "finished_at": "2026-10-15T03:34:43.658285428Z", "discovery_id": "a0e17c57-7a55-48b1-9b19-012a790e9e9b", "daemon_version": "0.18.2", "discovery_type": {"type": "Unified", "host_id": "6943745c-a092-47e4-9991-74ba816a3f6a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:34:43.681562697Z", "hosts_discovered": 6, "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "6943745c-a092-47e4-9991-74ba816a3f6a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:33:48.001242+00	2026-10-15 03:34:43.68784+00	0	f	[]
953ad9f1-7e91-4dc7-8ac0-d01bebbcfef4	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "969d115c-0112-4ba3-8757-bb1ece340112", "started_at": "2026-03-11T06:32:27.172078644Z", "finished_at": "2026-03-11T06:32:27.172135060Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:44.523040544Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:32:27.172078+00	2026-10-15 03:41:44.525034+00	0	f	[]
cdc97978-e7ff-4945-ad4e-f7adf064d3fa	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "abba33fb-bf1f-4e8b-985f-6a2d0b5d0380", "started_at": "2026-03-11T06:32:27.975947389Z", "finished_at": "2026-03-11T06:32:27.976013693Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:44.548283870Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:32:27.975947+00	2026-10-15 03:41:44.55037+00	0	f	[]
ce4403ee-148f-4efd-9d5f-bcbf0659cd17	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	96d7b351-8ddb-4403-96f9-d50e23f473ee	{"type": "Scheduled", "enabled": true, "last_run": "2026-10-15T03:35:10.724560858Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:34:44.02306+00	2026-10-15 03:38:11.273334+00	1	f	[]
0584cb5a-fbfc-4aba-bf5b-fb2b0fd15e78	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	96d7b351-8ddb-4403-96f9-d50e23f473ee	{"type": "AdHoc", "last_run": "2026-10-15T03:35:12.471374406Z"}	{"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	ServerPoll Integration Test Discovery	2026-10-15 03:35:12.463694+00	2026-10-15 03:41:41.302307+00	1	f	[]
59d14eab-837b-41f1-92e8-2c28760e7946	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 100, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "6845dcc8-6ea0-47af-9ce3-054b8cf1f667", "started_at": "2026-01-23T03:40:38.920476295Z", "finished_at": "2026-01-23T03:40:38.980206587Z", "discovery_id": null, "daemon_version": "0.17.17", "discovery_type": {"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}, "last_update_at": "2026-10-15T03:41:56.993912434Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}	Self Report — My Site	2026-01-23 03:40:38.920476+00	2026-10-15 03:41:56.99604+00	0	f	[]
d7167cce-7c08-4383-abf7-0f7e3ef7e701	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	96d7b351-8ddb-4403-96f9-d50e23f473ee	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 100, "warnings": [], "daemon_id": "96d7b351-8ddb-4403-96f9-d50e23f473ee", "session_id": "ae2ef896-c9ee-4c7d-92d0-2e940d717f51", "started_at": "2026-10-15T03:35:10.733740412Z", "finished_at": "2026-10-15T03:38:11.240059923Z", "discovery_id": "ce4403ee-148f-4efd-9d5f-bcbf0659cd17", "daemon_version": "0.18.2", "discovery_type": {"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:38:11.268042047Z", "hosts_discovered": 6, "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:35:10.73374+00	2026-10-15 03:38:11.272677+00	0	f	[]
077ba46d-b91d-4aaf-a7b0-2c2718688935	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	96d7b351-8ddb-4403-96f9-d50e23f473ee	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 100, "warnings": [], "daemon_id": "96d7b351-8ddb-4403-96f9-d50e23f473ee", "session_id": "26c0ac92-5d7b-41b7-969c-9e07d17bd1be", "started_at": "2026-10-15T03:38:40.733230767Z", "finished_at": "2026-10-15T03:41:41.272373769Z", "discovery_id": "0584cb5a-fbfc-4aba-bf5b-fb2b0fd15e78", "daemon_version": "0.18.2", "discovery_type": {"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:41.297251941Z", "hosts_discovered": 6, "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:38:40.73323+00	2026-10-15 03:41:41.301338+00	0	f	[]
fcb8f40a-42c4-4269-98d2-a80a3fdf0d92	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "0ffe5496-d64e-41a0-ab86-ff28eed28819", "started_at": "2026-03-11T06:32:26.369555086Z", "finished_at": "2026-03-11T06:32:26.369642510Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:44.508871409Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:32:26.369555+00	2026-10-15 03:41:44.510942+00	0	f	[]
32ed5e80-a39a-4944-a12b-9404a37e2e25	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "10fdd8f4-03b6-44ea-adb6-27e74136b365", "started_at": "2026-03-11T06:32:27.440078826Z", "finished_at": "2026-03-11T06:32:27.440151903Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:44.531585426Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:32:27.440078+00	2026-10-15 03:41:44.5334+00	0	f	[]
080efd5b-0334-43dc-9821-44e6c9a02d90	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "4273fe18-d031-4977-950e-ad5a49d2ea8e", "started_at": "2026-03-11T06:32:27.708165946Z", "finished_at": "2026-03-11T06:32:27.708228433Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:44.539585142Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:32:27.708165+00	2026-10-15 03:41:44.541703+00	0	f	[]
386cbf7c-2d7e-4b33-99e9-039a96da64fe	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "27ff5779-08b0-4970-aebd-04649d27c725", "started_at": "2026-03-11T06:32:28.243891377Z", "finished_at": "2026-03-11T06:32:28.243957151Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:44.556923536Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:32:28.243891+00	2026-10-15 03:41:44.559087+00	0	f	[]
af9fb8fd-829a-4bc1-ad92-43c363f0710a	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "857fbace-0358-4642-8ca0-b14bb999f353", "started_at": "2026-03-11T06:32:28.512463426Z", "finished_at": "2026-03-11T06:32:28.512545810Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:44.565390762Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:32:28.512463+00	2026-10-15 03:41:44.56747+00	0	f	[]
cdd6252a-cc2c-4e3b-9c5c-40636885d63f	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "3b0868af-d558-45e3-b688-2cf55472b6ee", "started_at": "2026-03-11T06:32:30.393154209Z", "finished_at": "2026-03-11T06:32:30.393220914Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:44.621100425Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:32:30.393154+00	2026-10-15 03:41:44.623297+00	0	f	[]
673d3f15-1e0c-45e7-aea7-0a85a0a9799f	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 100, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "7f54ecab-3771-4583-a7d0-f12569030e17", "started_at": "2026-01-23T03:48:00.469451847Z", "finished_at": "2026-01-23T03:48:00.553983097Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}, "last_update_at": "2026-10-15T03:41:48.600912086Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}	Self Report — My Site	2026-01-23 03:48:00.469451+00	2026-10-15 03:41:48.602977+00	0	f	[]
6fb6a198-7d4d-4d0c-a71c-fabf4abaea0e	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "e0ea6622-5f78-4cc3-b1f9-257946754dd8", "started_at": "2026-03-09T05:29:17.642983004Z", "finished_at": "2026-03-09T05:29:17.643045281Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:42:00.768908781Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-09 05:29:17.642983+00	2026-10-15 03:42:00.770872+00	0	f	[]
b236b9ed-3cfb-4865-9337-c45a0a8ae896	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "22ea697f-065c-4d2e-a81f-0809764aad01", "started_at": "2026-03-11T06:32:28.783225595Z", "finished_at": "2026-03-11T06:32:28.783284155Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:44.573835842Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:32:28.783225+00	2026-10-15 03:41:44.575806+00	0	f	[]
68c0cce1-e672-4ee3-aff6-5b2508b9fda5	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "bbe36108-56c4-4d82-9087-e577842ff202", "started_at": "2026-03-11T06:32:29.052254842Z", "finished_at": "2026-03-11T06:32:29.052318772Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:44.582847657Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:32:29.052254+00	2026-10-15 03:41:44.585072+00	0	f	[]
8f8db8d4-c92b-4c83-a4bf-f72ba8ecec8e	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "59fba01e-fefd-4c66-9ef2-a85c0e76a811", "started_at": "2026-03-11T06:32:29.322736685Z", "finished_at": "2026-03-11T06:32:29.322811406Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:44.591668143Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:32:29.322736+00	2026-10-15 03:41:44.593789+00	0	f	[]
2297bcd1-708b-42dd-acc6-9ebe8c4364b5	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 100, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "f2d5414b-3369-4280-929d-5422dd11a4b4", "started_at": "2026-01-23T03:55:10.297725045Z", "finished_at": "2026-01-23T03:55:10.328241754Z", "discovery_id": null, "daemon_version": "0.14.4", "discovery_type": {"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}, "last_update_at": "2026-10-15T03:41:45.349073559Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}	Self Report — My Site	2026-01-23 03:55:10.297725+00	2026-10-15 03:41:45.351293+00	0	f	[]
7bec180e-9429-4d4b-82f6-ef8665544e7d	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "ec9dc330-67d0-4ce5-94d8-506859c74940", "started_at": "2026-03-11T06:32:29.590854940Z", "finished_at": "2026-03-11T06:32:29.590913560Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:44.599713773Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:32:29.590854+00	2026-10-15 03:41:44.601694+00	0	f	[]
24cfb418-e450-490c-8e23-80441168f85f	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "5e741620-3de9-476c-86e7-7ce656d0a5a8", "started_at": "2026-03-11T06:32:29.860793225Z", "finished_at": "2026-03-11T06:32:29.860865761Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:44.607609751Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:32:29.860793+00	2026-10-15 03:41:44.609661+00	0	f	[]
d56d6d54-c070-4ca0-b91f-0bc4a1eb4b63	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 0, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "5bd5e93e-a606-4c6e-b159-b04879bdd801", "started_at": "2026-03-15T19:48:14.853886058Z", "finished_at": "2026-03-15T19:48:14.853942173Z", "discovery_id": null, "daemon_version": "0.14.19", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-15T03:41:55.952858485Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-15 19:48:14.853886+00	2026-10-15 03:41:55.955649+00	0	f	[]
2cc64117-861e-4e0e-b701-5860b55af252	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 100, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "c8c6530c-aaff-4d7c-a872-ba4457906d77", "started_at": "2026-01-05T04:59:56.611843173Z", "finished_at": "2026-01-05T04:59:56.687605923Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}, "last_update_at": "2026-10-15T03:42:05.812317747Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}	Self Report — My Site	2026-01-05 04:59:56.611843+00	2026-10-15 03:42:05.814281+00	0	f	[]
935ef03a-a075-4a68-bb47-d11353ae2721	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	d199ce69-450c-4a64-a87b-d7b6470c432b	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "4b74f32e-3f98-4de2-863f-ffff5fe88fe2", "progress": 100, "warnings": [], "daemon_id": "d199ce69-450c-4a64-a87b-d7b6470c432b", "session_id": "b64df0ca-f173-4ba8-a48b-edf7e372974a", "started_at": "2026-01-07T19:14:20.155782339Z", "finished_at": "2026-01-07T19:14:20.224698505Z", "discovery_id": null, "daemon_version": "0.17.19", "discovery_type": {"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}, "last_update_at": "2026-10-15T03:42:02.714270103Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}	Self Report — My Site	2026-01-07 19:14:20.155782+00	2026-10-15 03:42:02.716402+00	0	f	[]
\.


--
-- Data for Name: discovery_warnings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.discovery_warnings (id, discovery_id, site_id, code, credential_id, host_id, remote_host_id, subnet_id, details, created_at) FROM stdin;
81a68b78-2c76-4f4d-a3e2-b6be67a2384f	43f80b66-bfe8-408d-a2dc-24b2b2ca0569	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	OutdatedDaemonFormat	\N	\N	\N	\N	{"code": "OutdatedDaemonFormat", "daemon_version": "0.14.18"}	2026-10-15 03:41:44.492987+00
7f8776d8-68ba-458e-b880-64b16513390b	2297bcd1-708b-42dd-acc6-9ebe8c4364b5	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	OutdatedDaemonFormat	\N	\N	\N	\N	{"code": "OutdatedDaemonFormat", "daemon_version": "0.14.4"}	2026-10-15 03:41:45.352297+00
eaf9acc4-4237-4cfb-9c3c-9a940c30ad3a	673d3f15-1e0c-45e7-aea7-0a85a0a9799f	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	OutdatedDaemonFormat	\N	\N	\N	\N	{"code": "OutdatedDaemonFormat", "daemon_version": "0.15.2"}	2026-10-15 03:41:48.604026+00
277211e5-d115-4c98-87e9-578f151b7ecf	59d14eab-837b-41f1-92e8-2c28760e7946	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	OutdatedDaemonFormat	\N	\N	\N	\N	{"code": "OutdatedDaemonFormat", "daemon_version": "0.17.17"}	2026-10-15 03:41:56.996978+00
527b0b06-d8c1-4c6c-af96-b1dc7edd61d8	935ef03a-a075-4a68-bb47-d11353ae2721	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	OutdatedDaemonFormat	\N	\N	\N	\N	{"code": "OutdatedDaemonFormat", "daemon_version": "0.17.19"}	2026-10-15 03:42:02.717383+00
7728b185-632d-429e-b84a-33a87ae6e72f	2cc64117-861e-4e0e-b701-5860b55af252	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	OutdatedDaemonFormat	\N	\N	\N	\N	{"code": "OutdatedDaemonFormat", "daemon_version": "0.17.2"}	2026-10-15 03:42:05.815186+00
\.


--
-- Data for Name: entity_tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.entity_tags (id, entity_id, entity_type, tag_id, created_at, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
d01969cd-5f22-4c34-825f-e09ac29973a5	2504af58-e67a-4234-ae29-f655788cd9e4	"Service"	f6800baf-0640-4350-bc8b-0f9e83faf072	2026-10-15 03:41:41.335886+00	2026-10-15 03:41:41.335886+00	\N	\N	\N
\.


--
-- Data for Name: host_credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.host_credentials (host_id, credential_id, ip_address_ids) FROM stdin;
\.


--
-- Data for Name: hosts; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.hosts (id, site_id, name, hostname, description, source, virtualization_metadata, created_at, updated_at, sys_descr, sys_object_id, sys_location, sys_contact, management_url, chassis_id, manufacturer, model, serial_number, sys_name, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id, virtualization_service_id, firmware_revision, sys_descr_source, sys_object_id_source, sys_location_source, sys_contact_source, management_url_source, chassis_id_source, sys_name_source, manufacturer_source, model_source, serial_number_source, firmware_revision_source, name_source, software_revision, software_revision_source, hostname_source, os, os_source, virtualization_interface_id, asset_tag, asset_tag_source) FROM stdin;
07ca9676-4995-4fd4-b0a3-c60b0b356658	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	a955a2a0d33b	a955a2a0d33b	Scanopy daemon	{"type": "Discovery"}	null	2026-10-15 03:42:06.909659+00	2026-10-15 03:42:06.909659+00	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-10-15 03:42:06.909659+00	\N	\N	2026-10-15 03:42:06.909659+00	\N	\N	\N	\N	\N	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	\N	"Unspecified"	"Unspecified"	\N	"Unspecified"	\N	\N	"Unspecified"
\.


--
-- Data for Name: interface_neighbor_candidates; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.interface_neighbor_candidates (id, site_id, interface_id, lldp_chassis_id, lldp_port_id, lldp_sys_name, lldp_port_desc, lldp_mgmt_addr, lldp_sys_desc, cdp_device_id, cdp_port_id, cdp_platform, cdp_address, created_at, evidence_source) FROM stdin;
\.


--
-- Data for Name: interface_neighbor_hosts; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.interface_neighbor_hosts (id, site_id, interface_id, neighbor_host_id, neighbor_seen_at, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
\.


--
-- Data for Name: interface_neighbor_interfaces; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.interface_neighbor_interfaces (id, site_id, interface_id, neighbor_interface_id, neighbor_seen_at, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
\.


--
-- Data for Name: interfaces; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.interfaces (id, host_id, site_id, created_at, updated_at, if_index, if_descr, if_alias, if_type, speed_bps, admin_status, oper_status, mac_address, ip_address_id, if_name, fdb_macs, native_vlan_id, vlan_ids, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id, mac_address_source, ip_configured, interface_source, fdb_macs_source, vlan_membership_source) FROM stdin;
\.


--
-- Data for Name: invites; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.invites (id, organization_id, permissions, site_ids, url, created_by, created_at, updated_at, expires_at, send_to) FROM stdin;
\.


--
-- Data for Name: ip_addresses; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ip_addresses (id, site_id, host_id, subnet_id, ip_address, mac_address, name, "position", created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id, mac_address_source, ip_address_source) FROM stdin;
618bc887-da03-48ca-8c41-542c0d06e3fa	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	07ca9676-4995-4fd4-b0a3-c60b0b356658	a764172f-790f-4870-b9fe-aac03274c53c	172.25.0.4	b2:2b:55:c5:16:3d	eth0	0	2026-10-15 03:42:06.909659+00	2026-10-15 03:42:06.909659+00	2026-10-15 03:42:06.909659+00	\N	\N	2026-10-15 03:42:06.909659+00	\N	\N	\N	"Unspecified"	"Unspecified"
\.


--
-- Data for Name: license_instances; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.license_instances (id, organization_id, instance_id, server_version, last_seen_ip, non_production, first_seen_at, last_seen_at, active_since, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: organizations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.organizations (id, name, stripe_customer_id, plan, plan_status, created_at, updated_at, onboarding, has_payment_method, trial_end_date, use_case, last_paused_at, trial_extended_used, last_downgrade_at, last_downgrade_from_plan, last_discount_at, discount_save_offer_percent_off, discount_save_offer_active_until, next_renewal_at, notifications, license_paid_through, license_checkin_at, license_key_version, license_entitlement, license_entitlement_at, license_key_issued_at, license_key_type, bills_by_invoice, license_server_version) FROM stdin;
ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	My Organization	\N	{"rate": "Month", "type": "Community", "base_cents": 0, "host_cents": null, "seat_cents": null, "site_cents": null, "trial_days": 0, "included_orgs": null, "included_hosts": null, "included_seats": null, "included_sites": null}	active	2026-10-15 03:33:45.587357+00	2026-10-15 03:33:45.587357+00	["OrgCreated", "OnboardingModalCompleted", "FirstDaemonRegistered", "FirstHostDiscovered", "FirstDiscoveryCompleted", "FirstTagCreated", "FirstDependencyCreated", "FirstUserApiKeyCreated", "SecondSiteCreated"]	f	\N	other	\N	f	\N	null	\N	\N	\N	\N	{"hosts": "None", "seats": "None", "sites": "None", "sunset_notified_floor": null, "license_over_instance_limit": false, "license_silence_reported_for": null, "airgap_expiry_notified_through": null}	\N	\N	0	\N	\N	\N	\N	f	\N
\.


--
-- Data for Name: ports; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ports (id, site_id, host_id, port_number, protocol, port_type, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id, port_source) FROM stdin;
fda4149b-1b8f-4143-a8fd-6326626f2c98	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	07ca9676-4995-4fd4-b0a3-c60b0b356658	60073	Tcp	Custom	2026-10-15 03:42:06.909659+00	2026-10-15 03:42:06.909659+00	2026-10-15 03:42:06.909659+00	\N	\N	2026-10-15 03:42:06.909659+00	\N	\N	\N	"Unspecified"
\.


--
-- Data for Name: release_announcements; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.release_announcements (id, version, email_sent_at, created_at, updated_at) FROM stdin;
ef971651-e9da-4c80-842d-678de553c1d5	0.18.2	\N	2026-10-15 03:33:40.762334+00	2026-10-15 03:33:40.762334+00
\.


--
-- Data for Name: server_instance; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.server_instance (id, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: services; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.services (id, site_id, created_at, updated_at, name, host_id, service_definition, virtualization_metadata, source, "position", valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id, virtualization_service_id) FROM stdin;
0b62457e-a3d6-4712-b086-b55ab8eceb97	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	2026-10-15 03:42:06.909659+00	2026-10-15 03:42:06.909659+00	Scanopy Daemon	07ca9676-4995-4fd4-b0a3-c60b0b356658	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}}	0	2026-10-15 03:42:06.909659+00	\N	\N	2026-10-15 03:42:06.909659+00	\N	\N	\N	\N
\.


--
-- Data for Name: shares; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.shares (id, topology_id, site_id, created_by, name, is_enabled, expires_at, password_hash, allowed_domains, options, created_at, updated_at, enabled_views) FROM stdin;
\.


--
-- Data for Name: site_credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.site_credentials (site_id, credential_id) FROM stdin;
4b74f32e-3f98-4de2-863f-ffff5fe88fe2	a93bbe5d-146c-4264-a367-191f44da54f6
\.


--
-- Data for Name: sites; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.sites (id, name, created_at, updated_at, organization_id, stale_after_hours) FROM stdin;
4b74f32e-3f98-4de2-863f-ffff5fe88fe2	My Site	2026-10-15 03:33:45.638991+00	2026-10-15 03:33:45.638991+00	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	\N
\.


--
-- Data for Name: snapshots; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.snapshots (id, site_id, taken_at, created_by_user_id, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: subnet_vlans; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.subnet_vlans (id, subnet_id, vlan_id, created_at, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
\.


--
-- Data for Name: subnets; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.subnets (id, site_id, created_at, updated_at, cidr, name, description, subnet_type, source, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id, virtualization_service_id, cidr_source) FROM stdin;
571af076-9a61-4b37-a74f-ba5254746f48	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	2026-10-15 03:42:10.690187+00	2026-10-15 03:42:10.690187+00	"127.0.0.0/8"	127.0.0.0/8	\N	Loopback	{"type": "Discovery"}	2026-10-15 03:42:10.690187+00	\N	\N	2026-10-15 03:42:23.294448+00	\N	\N	\N	\N	"DaemonSelfReport"
a764172f-790f-4870-b9fe-aac03274c53c	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	1970-01-01 00:00:00+00	2026-03-06 07:04:08.161423+00	"172.25.0.0/28"	172.25.0.0/28	\N	Lan	{"type": "Discovery"}	1970-01-01 00:00:00+00	\N	\N	2026-10-15 03:42:23.294482+00	\N	\N	\N	\N	"DaemonSelfReport"
b802fe53-c474-4f40-a4fb-4bb6445084e2	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	2026-10-15 03:42:24.74285+00	2026-10-15 03:42:24.74285+00	"10.1.0.0/24"	Blocked Subnet	\N	Lan	{"type": "Manual"}	2026-10-15 03:42:24.74285+00	\N	\N	2026-10-15 03:42:24.74285+00	\N	\N	\N	\N	"Manual"
\.


--
-- Data for Name: tag_groups; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tag_groups (id, organization_id, name, description, group_type, entity_types, ordered, created_at, updated_at) FROM stdin;
1debfe1c-00cb-41c3-a09a-417fa2aa8a27	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Application	What each entity is part of. A service inherits its host's application tag, and the Application view groups by it.	Application	\N	f	2026-10-15 03:33:45.629922+00	2026-10-15 03:33:45.629922+00
7f119b1b-8fc8-4867-83d9-b2228e83e113	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Status	Where an asset is in its lifecycle.	Status	\N	t	2026-10-15 03:33:45.631319+00	2026-10-15 03:33:45.631319+00
d851c33c-a4c8-4d15-b501-56e1cf3cd4bf	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Visibility	Entities holding a tag of this group are left out of every topology.	Visibility	\N	f	2026-10-15 03:33:45.632245+00	2026-10-15 03:33:45.632245+00
6835afe1-14f9-4b69-9f46-810a5e9db4a3	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Environment	Which stage of delivery an asset serves.	Custom	\N	t	2026-10-15 03:33:45.663904+00	2026-10-15 03:33:45.663904+00
194bea94-fcc5-4562-ac91-3c88cd6b5500	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Criticality	How much an outage of the asset costs.	Custom	\N	t	2026-10-15 03:33:45.664609+00	2026-10-15 03:33:45.664609+00
5e9a010f-f522-4911-b03f-438b6eb45bc1	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Management	Who administers the asset.	Custom	\N	t	2026-10-15 03:33:45.665311+00	2026-10-15 03:33:45.665311+00
\.


--
-- Data for Name: tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tags (id, organization_id, name, description, created_at, updated_at, color, valid_from, valid_to, lineage_id, snapshot_id, icon, tag_group_id, "position") FROM stdin;
42c05340-b114-4076-8dd6-539c4cb71961	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Hidden	Left out of topology.	2026-10-15 03:33:45.633385+00	2026-10-15 03:33:45.633385+00	Gray	2026-10-15 03:33:45.633385+00	\N	\N	\N	eye-off	d851c33c-a4c8-4d15-b501-56e1cf3cd4bf	\N
7e67e192-443c-452d-add6-c4ef952dba90	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Planned	Ordered or scheduled, not yet in service	2026-10-15 03:33:45.668077+00	2026-10-15 03:33:45.668077+00	Blue	2026-10-15 03:33:45.668077+00	\N	\N	\N	calendar-clock	7f119b1b-8fc8-4867-83d9-b2228e83e113	1
8595ec63-a40b-495d-96b2-7cb1b64fc673	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Active	In service	2026-10-15 03:33:45.668086+00	2026-10-15 03:33:45.668086+00	Green	2026-10-15 03:33:45.668086+00	\N	\N	\N	circle-check	7f119b1b-8fc8-4867-83d9-b2228e83e113	2
2cffe76a-231b-427e-93a5-4f3fcc3bd77e	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Decommissioning	Being retired; still on the network	2026-10-15 03:33:45.668088+00	2026-10-15 03:33:45.668088+00	Orange	2026-10-15 03:33:45.668088+00	\N	\N	\N	hourglass	7f119b1b-8fc8-4867-83d9-b2228e83e113	3
edc762b2-738b-49d6-8d32-7651566ae5e7	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Decommissioned	Retired from service; kept for the record	2026-10-15 03:33:45.668089+00	2026-10-15 03:33:45.668089+00	Gray	2026-10-15 03:33:45.668089+00	\N	\N	\N	archive	7f119b1b-8fc8-4867-83d9-b2228e83e113	4
7acdea78-4bf4-4bdc-b04c-5ec628dd2872	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Production	Serves live users and business traffic	2026-10-15 03:33:45.668093+00	2026-10-15 03:33:45.668093+00	Red	2026-10-15 03:33:45.668093+00	\N	\N	\N	server	6835afe1-14f9-4b69-9f46-810a5e9db4a3	1
25772be4-f90a-4323-80fe-0d946ab6cd7d	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Staging	A pre-release copy of production for final testing	2026-10-15 03:33:45.668095+00	2026-10-15 03:33:45.668095+00	Amber	2026-10-15 03:33:45.668095+00	\N	\N	\N	flask-conical	6835afe1-14f9-4b69-9f46-810a5e9db4a3	2
d8011dba-83d3-4854-9358-9e57e3eb3da8	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Development	Used to build and test changes	2026-10-15 03:33:45.668096+00	2026-10-15 03:33:45.668096+00	Blue	2026-10-15 03:33:45.668096+00	\N	\N	\N	code	6835afe1-14f9-4b69-9f46-810a5e9db4a3	3
e0ab1db5-8bba-4664-bba4-681482375f7b	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Lab	Test bench or experimental equipment	2026-10-15 03:33:45.668097+00	2026-10-15 03:33:45.668097+00	Purple	2026-10-15 03:33:45.668097+00	\N	\N	\N	test-tube	6835afe1-14f9-4b69-9f46-810a5e9db4a3	4
4c816f19-2ee6-4a12-9909-e299c9d9a876	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Critical	An outage stops the business	2026-10-15 03:33:45.6681+00	2026-10-15 03:33:45.6681+00	Red	2026-10-15 03:33:45.6681+00	\N	\N	\N	siren	194bea94-fcc5-4562-ac91-3c88cd6b5500	1
a9f9659d-250c-4a09-b2df-82250bc65d76	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	High	An outage disrupts many people	2026-10-15 03:33:45.668101+00	2026-10-15 03:33:45.668101+00	Orange	2026-10-15 03:33:45.668101+00	\N	\N	\N	arrow-up	194bea94-fcc5-4562-ac91-3c88cd6b5500	2
52e6f1f1-8f07-4fc3-9990-e62f271f67a1	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Medium	An outage disrupts some people	2026-10-15 03:33:45.668102+00	2026-10-15 03:33:45.668102+00	Yellow	2026-10-15 03:33:45.668102+00	\N	\N	\N	minus	194bea94-fcc5-4562-ac91-3c88cd6b5500	3
f55f3e84-becd-4c1c-8656-6176cf828240	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Low	An outage has little impact	2026-10-15 03:33:45.668103+00	2026-10-15 03:33:45.668103+00	Gray	2026-10-15 03:33:45.668103+00	\N	\N	\N	arrow-down	194bea94-fcc5-4562-ac91-3c88cd6b5500	4
086422c6-6844-4d1f-aaaf-e7d6ca31beb0	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Managed	Administered by your IT team	2026-10-15 03:33:45.668107+00	2026-10-15 03:33:45.668107+00	Green	2026-10-15 03:33:45.668107+00	\N	\N	\N	shield-check	5e9a010f-f522-4911-b03f-438b6eb45bc1	1
a6713952-3331-4f09-bb61-d4d4d8b9aaee	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Unmanaged	Administered by no one, such as a personal device	2026-10-15 03:33:45.668108+00	2026-10-15 03:33:45.668108+00	Orange	2026-10-15 03:33:45.668108+00	\N	\N	\N	shield-question-mark	5e9a010f-f522-4911-b03f-438b6eb45bc1	2
e7e25360-1076-45cb-88aa-4ea7acaabf3a	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Third-party	Administered by a vendor or another organization	2026-10-15 03:33:45.668109+00	2026-10-15 03:33:45.668109+00	Blue	2026-10-15 03:33:45.668109+00	\N	\N	\N	handshake	5e9a010f-f522-4911-b03f-438b6eb45bc1	3
f6800baf-0640-4350-bc8b-0f9e83faf072	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Integration Test Tag	\N	2026-10-15 03:41:41.313769+00	2026-10-15 03:41:41.313769+00	Yellow	2026-10-15 03:41:41.313769+00	\N	\N	\N	\N	\N	\N
7015ba39-7c0d-43bd-9943-e546ba0643af	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Test Tag	\N	2026-10-15 03:42:23.839454+00	2026-10-15 03:42:23.839454+00	Yellow	2026-10-15 03:42:23.839454+00	2026-10-15 03:42:23.861116+00	3a651c32-39ab-4e39-b7dc-32fec5d1364c	\N	\N	\N	\N
3a651c32-39ab-4e39-b7dc-32fec5d1364c	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Updated Tag	\N	2026-10-15 03:42:23.839454+00	2026-10-15 03:42:23.839454+00	Yellow	2026-10-15 03:42:23.861116+00	2026-10-15 03:42:23.875335+00	\N	\N	\N	\N	\N
\.


--
-- Data for Name: topologies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.topologies (id, site_id, options, created_at, updated_at) FROM stdin;
562fe257-b279-4bab-b75a-0f3aecc19fd8	4b74f32e-3f98-4de2-863f-ffff5fe88fe2	{"local": {"tag_filter": {}, "bundle_edges": true, "show_minimap": true, "no_fade_edges": false, "hide_edge_types": {"L3Logical": ["Hypervisor", "SameContainer"], "Workloads": ["RequestPath", "HubAndSpoke", "PhysicalLink", "NeighborLink"], "L2Physical": ["SameHost"], "Application": ["ContainerRuntime"]}}, "request": {"element_sort": {}, "element_rules": [{"id": "fa7dae40-7ce1-4fce-912d-4e3c3ea1c0d2", "rule": "ByTrunkPort"}, {"id": "985f7633-bef4-4212-a32d-8f531b0fb284", "rule": "ByVLAN"}, {"id": "adb74368-d1a0-471e-a26e-14b729c098fc", "rule": "ByPortOpStatus"}, {"id": "17640ca9-ad33-4cd6-866b-fc1b60ee1642", "rule": {"ByServiceCategory": {"title": "Infrastructure", "categories": ["NetworkCore", "NetworkAccess", "NetworkAppliance", "RemoteAccess", "Workstation", "Mobile", "IoT", "Printer", "OpenPorts", "NetworkIdentities"], "is_infra_rule": true}}}, {"id": "ddbae8ef-a84f-4a75-b481-5383c09d1b3c", "rule": {"ByTag": {"title": null, "tag_ids": []}}}, {"id": "a4e22f2d-95a9-41e9-b620-f48c5321d0ce", "rule": "ByHypervisor"}, {"id": "d4cdf68e-e2db-43f9-b189-15848aca60ba", "rule": "ByContainerRuntime"}, {"id": "0709d807-3b93-43f8-bb32-39d77ac46789", "rule": "ByStack"}], "hide_entities": {}, "container_rules": {"L3Logical": [{"id": "63fcdd79-7b7d-4730-a215-4044aa401b60", "rule": "BySubnet"}, {"id": "4ee6c36b-eba6-4e59-897b-0de96a2815f7", "rule": "MergeContainerBridges"}], "Workloads": [{"id": "4d0506c8-5913-4dae-b12f-874ae88dc443", "rule": "ByHost"}], "L2Physical": [{"id": "4d0506c8-5913-4dae-b12f-874ae88dc443", "rule": "ByHost"}], "Application": [{"id": "0f29eddd-1287-407c-93dc-a6afec2c23b5", "rule": {"ByApplication": {"tag_ids": []}}}]}, "hide_metadata_values": {"L3Logical": {"Service": {"Category": ["OpenPorts"]}}, "Workloads": {"Service": {"Category": ["OpenPorts"]}}, "L2Physical": {"Service": {"Category": ["OpenPorts"]}, "Interface": {"LinkState": ["Unlinked"]}}, "Application": {"Service": {"Category": ["OpenPorts"]}}}}}	2026-10-15 03:33:45.659329+00	2026-10-15 03:33:45.659329+00
\.


--
-- Data for Name: user_api_key_site_access; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_api_key_site_access (id, api_key_id, site_id, created_at) FROM stdin;
\.


--
-- Data for Name: user_api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_api_keys (id, key, user_id, organization_id, permissions, name, created_at, updated_at, last_used, expires_at, is_enabled) FROM stdin;
\.


--
-- Data for Name: user_site_access; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_site_access (id, user_id, site_id, created_at) FROM stdin;
\.


--
-- Data for Name: users; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.users (id, created_at, updated_at, password_hash, oidc_provider, oidc_subject, oidc_linked_at, email, organization_id, permissions, tags, terms_accepted_at, email_verified, email_verification_token, email_verification_expires, password_reset_token, password_reset_expires, pending_email, email_settings, session_epoch, display_settings, email_unsubscribe_token, release_notes_seen_version, terms_notice_version) FROM stdin;
ad97c597-a289-4837-a404-079bb7efe966	2026-10-15 03:33:45.623496+00	2026-10-15 03:33:45.623496+00	$argon2id$v=19$m=19456,t=2,p=1$VzDd9pC3nrnQCdKH7QQRww$rYriCLwW1bXyJlaJZJ/TlqhnPAE7CNTnz3VIVQXcd5c	\N	\N	\N	user@gmail.com	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Owner	{}	\N	t	\N	\N	\N	\N	\N	{"daemon_alerts": true, "product_updates": true, "trial_and_usage": true, "discovery_digest": true, "product_onboarding": true}	0	{"clock": "browser_default", "time_zone": null, "date_order": "browser_default", "timestamps": "relative", "week_start": "monday", "table_density": "compact"}	\N	0.18.2	\N
02f7a947-8014-4316-8cf8-62cc4627025a	2026-10-15 03:42:24.463593+00	2026-10-15 03:42:24.463593+00	\N	\N	\N	\N	user@example.com	ca9bf2ab-265e-40d6-9be3-ce5fc525e8cc	Owner	{}	\N	f	\N	\N	\N	\N	\N	{"daemon_alerts": true, "product_updates": true, "trial_and_usage": true, "discovery_digest": true, "product_onboarding": true}	0	{"clock": "browser_default", "time_zone": null, "date_order": "browser_default", "timestamps": "relative", "week_start": "monday", "table_density": "compact"}	\N	\N	\N
\.


--
-- Data for Name: vlans; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.vlans (id, vlan_number, name, description, site_id, organization_id, source, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
\.


--
-- Data for Name: session; Type: TABLE DATA; Schema: tower_sessions; Owner: postgres
--

COPY tower_sessions.session (id, data, expiry_date) FROM stdin;
GmxBxnF_P6magcLuhUxOMQ	\\x93c410314e4c85eec2819aa93f7f71c6416c1a82a7757365725f6964d92461643937633539372d613238392d343833372d613430342d303739626237656665393636ad73657373696f6e5f65706f63680099cd07eacd012703212dce2dd144a0000000	2026-10-22 03:33:45.768689+00
9Ujn3-_VlhiM4ei1NlYztg	\\x93c410b6335636b5e8e18c1896d5efdfe748f583ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92434326137613635342d646563662d343065372d623037302d623266353962653438333732a87573655f63617365a56f74686572a7757365725f6964d92461643937633539372d613238392d343833372d613430342d303739626237656665393636ad73657373696f6e5f65706f63680099cd07eacd0127032929ce37b0fb4a000000	2026-10-22 03:41:41.934345+00
xNzkKv1CYaN_grEpLbv2RA	\\x93c41044f6bb2d29b1827fa36142fd2ae4dcc483ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92435313933636163642d373936652d346462392d393931302d343936383963336531343461a87573655f63617365a56f74686572a7757365725f6964d92461643937633539372d613238392d343833372d613430342d303739626237656665393636ad73657373696f6e5f65706f63680099cd07eacd0127032a07ce0eb6985c000000	2026-10-22 03:42:07.246847+00
mUCIbdc1K6yXNRjoZ8H1Cw	\\x93c4100bf5c167e8183597ac2b35d76d88409983ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92461363564643233392d366631622d346139612d626433322d613639663239646532633438a87573655f63617365a56f74686572ad73657373696f6e5f65706f636800a7757365725f6964d92461643937633539372d613238392d343833372d613430342d30373962623765666539363699cd07eacd0127032a17ce236c041c000000	2026-10-22 03:42:23.594281+00
\.


--
-- Name: _sqlx_migrations _sqlx_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public._sqlx_migrations
    ADD CONSTRAINT _sqlx_migrations_pkey PRIMARY KEY (version);


--
-- Name: api_keys api_keys_key_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_key_key UNIQUE (key);


--
-- Name: api_keys api_keys_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_pkey PRIMARY KEY (id);


--
-- Name: bindings bindings_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_pkey PRIMARY KEY (id);


--
-- Name: credentials credentials_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.credentials
    ADD CONSTRAINT credentials_pkey PRIMARY KEY (id);


--
-- Name: daemon_interfaced_subnets daemon_interfaced_subnets_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemon_interfaced_subnets
    ADD CONSTRAINT daemon_interfaced_subnets_pkey PRIMARY KEY (daemon_id, subnet_id);


--
-- Name: daemons daemons_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_pkey PRIMARY KEY (id);


--
-- Name: discovery discovery_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery
    ADD CONSTRAINT discovery_pkey PRIMARY KEY (id);


--
-- Name: discovery_warnings discovery_warnings_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery_warnings
    ADD CONSTRAINT discovery_warnings_pkey PRIMARY KEY (id);


--
-- Name: entity_tags entity_tags_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_pkey PRIMARY KEY (id);


--
-- Name: dependency_members group_bindings_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependency_members
    ADD CONSTRAINT group_bindings_pkey PRIMARY KEY (id);


--
-- Name: dependencies groups_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependencies
    ADD CONSTRAINT groups_pkey PRIMARY KEY (id);


--
-- Name: host_credentials host_credentials_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.host_credentials
    ADD CONSTRAINT host_credentials_pkey PRIMARY KEY (host_id, credential_id);


--
-- Name: hosts hosts_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_pkey PRIMARY KEY (id);


--
-- Name: interface_neighbor_candidates interface_neighbor_candidates_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interface_neighbor_candidates
    ADD CONSTRAINT interface_neighbor_candidates_pkey PRIMARY KEY (id);


--
-- Name: interface_neighbor_hosts interface_neighbor_hosts_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interface_neighbor_hosts
    ADD CONSTRAINT interface_neighbor_hosts_pkey PRIMARY KEY (id);


--
-- Name: interface_neighbor_interfaces interface_neighbor_interfaces_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interface_neighbor_interfaces
    ADD CONSTRAINT interface_neighbor_interfaces_pkey PRIMARY KEY (id);


--
-- Name: interfaces interfaces_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_pkey PRIMARY KEY (id);


--
-- Name: invites invites_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.invites
    ADD CONSTRAINT invites_pkey PRIMARY KEY (id);


--
-- Name: ip_addresses ip_addresses_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ip_addresses
    ADD CONSTRAINT ip_addresses_pkey PRIMARY KEY (id);


--
-- Name: license_instances license_instances_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.license_instances
    ADD CONSTRAINT license_instances_pkey PRIMARY KEY (id);


--
-- Name: organizations organizations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.organizations
    ADD CONSTRAINT organizations_pkey PRIMARY KEY (id);


--
-- Name: ports ports_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_pkey PRIMARY KEY (id);


--
-- Name: release_announcements release_announcements_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.release_announcements
    ADD CONSTRAINT release_announcements_pkey PRIMARY KEY (id);


--
-- Name: server_instance server_instance_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.server_instance
    ADD CONSTRAINT server_instance_pkey PRIMARY KEY (id);


--
-- Name: services services_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_pkey PRIMARY KEY (id);


--
-- Name: shares shares_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_pkey PRIMARY KEY (id);


--
-- Name: site_credentials site_credentials_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.site_credentials
    ADD CONSTRAINT site_credentials_pkey PRIMARY KEY (site_id, credential_id);


--
-- Name: sites sites_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.sites
    ADD CONSTRAINT sites_pkey PRIMARY KEY (id);


--
-- Name: snapshots snapshots_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.snapshots
    ADD CONSTRAINT snapshots_pkey PRIMARY KEY (id);


--
-- Name: subnet_vlans subnet_vlans_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnet_vlans
    ADD CONSTRAINT subnet_vlans_pkey PRIMARY KEY (id);


--
-- Name: subnets subnets_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_pkey PRIMARY KEY (id);


--
-- Name: tag_groups tag_groups_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tag_groups
    ADD CONSTRAINT tag_groups_pkey PRIMARY KEY (id);


--
-- Name: tags tags_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_pkey PRIMARY KEY (id);


--
-- Name: topologies topologies_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.topologies
    ADD CONSTRAINT topologies_pkey PRIMARY KEY (id);


--
-- Name: user_api_key_site_access user_api_key_site_access_api_key_id_site_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_site_access
    ADD CONSTRAINT user_api_key_site_access_api_key_id_site_id_key UNIQUE (api_key_id, site_id);


--
-- Name: user_api_key_site_access user_api_key_site_access_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_site_access
    ADD CONSTRAINT user_api_key_site_access_pkey PRIMARY KEY (id);


--
-- Name: user_api_keys user_api_keys_key_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_keys
    ADD CONSTRAINT user_api_keys_key_key UNIQUE (key);


--
-- Name: user_api_keys user_api_keys_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_keys
    ADD CONSTRAINT user_api_keys_pkey PRIMARY KEY (id);


--
-- Name: user_site_access user_site_access_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_site_access
    ADD CONSTRAINT user_site_access_pkey PRIMARY KEY (id);


--
-- Name: user_site_access user_site_access_user_id_site_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_site_access
    ADD CONSTRAINT user_site_access_user_id_site_id_key UNIQUE (user_id, site_id);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: vlans vlans_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.vlans
    ADD CONSTRAINT vlans_pkey PRIMARY KEY (id);


--
-- Name: session session_pkey; Type: CONSTRAINT; Schema: tower_sessions; Owner: postgres
--

ALTER TABLE ONLY tower_sessions.session
    ADD CONSTRAINT session_pkey PRIMARY KEY (id);


--
-- Name: idx_api_keys_daemon_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_api_keys_daemon_id ON public.api_keys USING btree (daemon_id) WHERE (daemon_id IS NOT NULL);


--
-- Name: idx_api_keys_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_api_keys_key ON public.api_keys USING btree (key);


--
-- Name: idx_api_keys_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_api_keys_site ON public.api_keys USING btree (site_id);


--
-- Name: idx_bindings_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_as_of ON public.bindings USING btree (site_id, valid_from, valid_to);


--
-- Name: idx_bindings_ip_address; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_ip_address ON public.bindings USING btree (ip_address_id);


--
-- Name: idx_bindings_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_lineage ON public.bindings USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_bindings_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_live ON public.bindings USING btree (site_id) WHERE (valid_to IS NULL);


--
-- Name: idx_bindings_port; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_port ON public.bindings USING btree (port_id);


--
-- Name: idx_bindings_service; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_service ON public.bindings USING btree (service_id);


--
-- Name: idx_bindings_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_site ON public.bindings USING btree (site_id);


--
-- Name: idx_bindings_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_snapshot_id ON public.bindings USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_credentials_org; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_credentials_org ON public.credentials USING btree (organization_id);


--
-- Name: idx_credentials_type; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_credentials_type ON public.credentials USING btree (((credential_type ->> 'type'::text)));


--
-- Name: idx_daemon_host_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_daemon_host_id ON public.daemons USING btree (host_id);


--
-- Name: idx_daemon_interfaced_subnets_subnet_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_daemon_interfaced_subnets_subnet_id ON public.daemon_interfaced_subnets USING btree (subnet_id);


--
-- Name: idx_daemons_api_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_daemons_api_key ON public.daemons USING btree (api_key_id) WHERE (api_key_id IS NOT NULL);


--
-- Name: idx_daemons_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_daemons_site ON public.daemons USING btree (site_id);


--
-- Name: idx_dependencies_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependencies_as_of ON public.dependencies USING btree (site_id, valid_from, valid_to);


--
-- Name: idx_dependencies_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependencies_lineage ON public.dependencies USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_dependencies_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependencies_live ON public.dependencies USING btree (site_id) WHERE (valid_to IS NULL);


--
-- Name: idx_dependencies_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependencies_site ON public.dependencies USING btree (site_id);


--
-- Name: idx_dependencies_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependencies_snapshot_id ON public.dependencies USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_dependency_members_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependency_members_as_of ON public.dependency_members USING btree (dependency_id, valid_from, valid_to);


--
-- Name: idx_dependency_members_binding; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependency_members_binding ON public.dependency_members USING btree (binding_id) WHERE (binding_id IS NOT NULL);


--
-- Name: idx_dependency_members_dependency; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependency_members_dependency ON public.dependency_members USING btree (dependency_id);


--
-- Name: idx_dependency_members_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependency_members_lineage ON public.dependency_members USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_dependency_members_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependency_members_live ON public.dependency_members USING btree (dependency_id) WHERE (valid_to IS NULL);


--
-- Name: idx_dependency_members_service; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependency_members_service ON public.dependency_members USING btree (service_id);


--
-- Name: idx_dependency_members_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_dependency_members_snapshot_id ON public.dependency_members USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_dependency_members_unique_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_dependency_members_unique_live ON public.dependency_members USING btree (dependency_id, service_id) WHERE (valid_to IS NULL);


--
-- Name: idx_discovery_daemon; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_daemon ON public.discovery USING btree (daemon_id);


--
-- Name: idx_discovery_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_site ON public.discovery USING btree (site_id);


--
-- Name: idx_discovery_warnings_credential; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_warnings_credential ON public.discovery_warnings USING btree (credential_id) WHERE (credential_id IS NOT NULL);


--
-- Name: idx_discovery_warnings_discovery; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_warnings_discovery ON public.discovery_warnings USING btree (discovery_id);


--
-- Name: idx_discovery_warnings_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_warnings_host ON public.discovery_warnings USING btree (host_id) WHERE (host_id IS NOT NULL);


--
-- Name: idx_discovery_warnings_remote_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_warnings_remote_host ON public.discovery_warnings USING btree (remote_host_id) WHERE (remote_host_id IS NOT NULL);


--
-- Name: idx_discovery_warnings_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_warnings_site ON public.discovery_warnings USING btree (site_id);


--
-- Name: idx_discovery_warnings_subnet; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_warnings_subnet ON public.discovery_warnings USING btree (subnet_id) WHERE (subnet_id IS NOT NULL);


--
-- Name: idx_entity_tags_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_as_of ON public.entity_tags USING btree (entity_id, entity_type, valid_from, valid_to);


--
-- Name: idx_entity_tags_entity; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_entity ON public.entity_tags USING btree (entity_id, entity_type);


--
-- Name: idx_entity_tags_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_lineage ON public.entity_tags USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_entity_tags_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_live ON public.entity_tags USING btree (entity_id, entity_type) WHERE (valid_to IS NULL);


--
-- Name: idx_entity_tags_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_snapshot_id ON public.entity_tags USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_entity_tags_tag_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_tag_id ON public.entity_tags USING btree (tag_id);


--
-- Name: idx_entity_tags_unique_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_entity_tags_unique_live ON public.entity_tags USING btree (entity_id, entity_type, tag_id) WHERE (valid_to IS NULL);


--
-- Name: idx_hosts_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_as_of ON public.hosts USING btree (site_id, valid_from, valid_to);


--
-- Name: idx_hosts_chassis_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_chassis_id ON public.hosts USING btree (chassis_id);


--
-- Name: idx_hosts_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_lineage ON public.hosts USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_hosts_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_live ON public.hosts USING btree (site_id) WHERE (valid_to IS NULL);


--
-- Name: idx_hosts_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_site ON public.hosts USING btree (site_id);


--
-- Name: idx_hosts_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_snapshot_id ON public.hosts USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_hosts_virtualization_interface_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_virtualization_interface_id ON public.hosts USING btree (virtualization_interface_id) WHERE (virtualization_interface_id IS NOT NULL);


--
-- Name: idx_hosts_virtualization_service_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_virtualization_service_id ON public.hosts USING btree (virtualization_service_id) WHERE (virtualization_service_id IS NOT NULL);


--
-- Name: idx_interface_neighbor_candidates_interface; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interface_neighbor_candidates_interface ON public.interface_neighbor_candidates USING btree (interface_id);


--
-- Name: idx_interface_neighbor_candidates_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interface_neighbor_candidates_site ON public.interface_neighbor_candidates USING btree (site_id);


--
-- Name: idx_interface_neighbor_hosts_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interface_neighbor_hosts_as_of ON public.interface_neighbor_hosts USING btree (site_id, valid_from, valid_to);


--
-- Name: idx_interface_neighbor_hosts_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interface_neighbor_hosts_lineage ON public.interface_neighbor_hosts USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_interface_neighbor_hosts_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interface_neighbor_hosts_live ON public.interface_neighbor_hosts USING btree (site_id) WHERE (valid_to IS NULL);


--
-- Name: idx_interface_neighbor_hosts_natural_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_interface_neighbor_hosts_natural_key ON public.interface_neighbor_hosts USING btree (interface_id, neighbor_host_id) WHERE (valid_to IS NULL);


--
-- Name: idx_interface_neighbor_hosts_neighbor; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interface_neighbor_hosts_neighbor ON public.interface_neighbor_hosts USING btree (neighbor_host_id);


--
-- Name: idx_interface_neighbor_hosts_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interface_neighbor_hosts_snapshot_id ON public.interface_neighbor_hosts USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_interface_neighbor_interfaces_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interface_neighbor_interfaces_as_of ON public.interface_neighbor_interfaces USING btree (site_id, valid_from, valid_to);


--
-- Name: idx_interface_neighbor_interfaces_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interface_neighbor_interfaces_lineage ON public.interface_neighbor_interfaces USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_interface_neighbor_interfaces_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interface_neighbor_interfaces_live ON public.interface_neighbor_interfaces USING btree (site_id) WHERE (valid_to IS NULL);


--
-- Name: idx_interface_neighbor_interfaces_natural_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_interface_neighbor_interfaces_natural_key ON public.interface_neighbor_interfaces USING btree (interface_id, neighbor_interface_id) WHERE (valid_to IS NULL);


--
-- Name: idx_interface_neighbor_interfaces_neighbor; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interface_neighbor_interfaces_neighbor ON public.interface_neighbor_interfaces USING btree (neighbor_interface_id);


--
-- Name: idx_interface_neighbor_interfaces_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interface_neighbor_interfaces_snapshot_id ON public.interface_neighbor_interfaces USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_interfaces_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_as_of ON public.interfaces USING btree (site_id, valid_from, valid_to);


--
-- Name: idx_interfaces_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_host ON public.interfaces USING btree (host_id);


--
-- Name: idx_interfaces_host_if_index; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_host_if_index ON public.interfaces USING btree (host_id, if_index);


--
-- Name: idx_interfaces_host_name_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_interfaces_host_name_live ON public.interfaces USING btree (host_id, if_name) WHERE ((if_name IS NOT NULL) AND (valid_to IS NULL));


--
-- Name: idx_interfaces_ip_address; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_ip_address ON public.interfaces USING btree (ip_address_id);


--
-- Name: idx_interfaces_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_lineage ON public.interfaces USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_interfaces_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_live ON public.interfaces USING btree (site_id) WHERE (valid_to IS NULL);


--
-- Name: idx_interfaces_mac_address; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_mac_address ON public.interfaces USING btree (mac_address);


--
-- Name: idx_interfaces_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_site ON public.interfaces USING btree (site_id);


--
-- Name: idx_interfaces_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_snapshot_id ON public.interfaces USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_invites_expires_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_invites_expires_at ON public.invites USING btree (expires_at);


--
-- Name: idx_invites_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_invites_organization ON public.invites USING btree (organization_id);


--
-- Name: idx_ip_addresses_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_as_of ON public.ip_addresses USING btree (site_id, valid_from, valid_to);


--
-- Name: idx_ip_addresses_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_host ON public.ip_addresses USING btree (host_id);


--
-- Name: idx_ip_addresses_host_mac; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_host_mac ON public.ip_addresses USING btree (host_id, mac_address) WHERE (mac_address IS NOT NULL);


--
-- Name: idx_ip_addresses_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_lineage ON public.ip_addresses USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_ip_addresses_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_live ON public.ip_addresses USING btree (site_id) WHERE (valid_to IS NULL);


--
-- Name: idx_ip_addresses_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_site ON public.ip_addresses USING btree (site_id);


--
-- Name: idx_ip_addresses_site_mac; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_site_mac ON public.ip_addresses USING btree (site_id, mac_address) WHERE (mac_address IS NOT NULL);


--
-- Name: idx_ip_addresses_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_snapshot_id ON public.ip_addresses USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_ip_addresses_subnet; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ip_addresses_subnet ON public.ip_addresses USING btree (subnet_id);


--
-- Name: idx_ip_addresses_unique_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_ip_addresses_unique_live ON public.ip_addresses USING btree (host_id, subnet_id, ip_address) WHERE (valid_to IS NULL);


--
-- Name: idx_license_instances_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_license_instances_organization ON public.license_instances USING btree (organization_id);


--
-- Name: idx_organizations_stripe_customer; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_organizations_stripe_customer ON public.organizations USING btree (stripe_customer_id);


--
-- Name: idx_ports_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_as_of ON public.ports USING btree (site_id, valid_from, valid_to);


--
-- Name: idx_ports_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_host ON public.ports USING btree (host_id);


--
-- Name: idx_ports_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_lineage ON public.ports USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_ports_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_live ON public.ports USING btree (site_id) WHERE (valid_to IS NULL);


--
-- Name: idx_ports_number; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_number ON public.ports USING btree (port_number);


--
-- Name: idx_ports_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_site ON public.ports USING btree (site_id);


--
-- Name: idx_ports_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_snapshot_id ON public.ports USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_ports_unique_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_ports_unique_live ON public.ports USING btree (host_id, port_number, protocol) WHERE (valid_to IS NULL);


--
-- Name: idx_release_announcements_version; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_release_announcements_version ON public.release_announcements USING btree (version);


--
-- Name: idx_services_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_as_of ON public.services USING btree (site_id, valid_from, valid_to);


--
-- Name: idx_services_host_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_host_id ON public.services USING btree (host_id);


--
-- Name: idx_services_host_position; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_host_position ON public.services USING btree (host_id, "position");


--
-- Name: idx_services_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_lineage ON public.services USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_services_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_live ON public.services USING btree (site_id) WHERE (valid_to IS NULL);


--
-- Name: idx_services_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_site ON public.services USING btree (site_id);


--
-- Name: idx_services_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_snapshot_id ON public.services USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_services_virtualization_service_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_virtualization_service_id ON public.services USING btree (virtualization_service_id) WHERE (virtualization_service_id IS NOT NULL);


--
-- Name: idx_shares_enabled; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_shares_enabled ON public.shares USING btree (is_enabled) WHERE (is_enabled = true);


--
-- Name: idx_shares_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_shares_site ON public.shares USING btree (site_id);


--
-- Name: idx_shares_topology; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_shares_topology ON public.shares USING btree (topology_id);


--
-- Name: idx_sites_owner_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_sites_owner_organization ON public.sites USING btree (organization_id);


--
-- Name: idx_snapshots_site_taken_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_snapshots_site_taken_at ON public.snapshots USING btree (site_id, taken_at DESC);


--
-- Name: idx_subnet_vlans_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnet_vlans_as_of ON public.subnet_vlans USING btree (subnet_id, valid_from, valid_to);


--
-- Name: idx_subnet_vlans_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnet_vlans_lineage ON public.subnet_vlans USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_subnet_vlans_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnet_vlans_live ON public.subnet_vlans USING btree (subnet_id) WHERE (valid_to IS NULL);


--
-- Name: idx_subnet_vlans_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnet_vlans_snapshot_id ON public.subnet_vlans USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_subnet_vlans_subnet; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnet_vlans_subnet ON public.subnet_vlans USING btree (subnet_id);


--
-- Name: idx_subnet_vlans_unique_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_subnet_vlans_unique_live ON public.subnet_vlans USING btree (subnet_id, vlan_id) WHERE (valid_to IS NULL);


--
-- Name: idx_subnet_vlans_vlan; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnet_vlans_vlan ON public.subnet_vlans USING btree (vlan_id);


--
-- Name: idx_subnets_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnets_as_of ON public.subnets USING btree (site_id, valid_from, valid_to);


--
-- Name: idx_subnets_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnets_lineage ON public.subnets USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_subnets_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnets_live ON public.subnets USING btree (site_id) WHERE (valid_to IS NULL);


--
-- Name: idx_subnets_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnets_site ON public.subnets USING btree (site_id);


--
-- Name: idx_subnets_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnets_snapshot_id ON public.subnets USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_subnets_virtualization_service_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnets_virtualization_service_id ON public.subnets USING btree (virtualization_service_id) WHERE (virtualization_service_id IS NOT NULL);


--
-- Name: idx_tag_groups_org_name; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_tag_groups_org_name ON public.tag_groups USING btree (organization_id, name);


--
-- Name: idx_tag_groups_org_system; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_tag_groups_org_system ON public.tag_groups USING btree (organization_id, group_type) WHERE (group_type <> 'Custom'::text);


--
-- Name: idx_tags_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tags_as_of ON public.tags USING btree (organization_id, valid_from, valid_to);


--
-- Name: idx_tags_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tags_lineage ON public.tags USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_tags_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tags_live ON public.tags USING btree (organization_id) WHERE (valid_to IS NULL);


--
-- Name: idx_tags_org_name_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_tags_org_name_live ON public.tags USING btree (organization_id, name) WHERE (valid_to IS NULL);


--
-- Name: idx_tags_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tags_organization ON public.tags USING btree (organization_id);


--
-- Name: idx_tags_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tags_snapshot_id ON public.tags USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: idx_tags_tag_group_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tags_tag_group_id ON public.tags USING btree (tag_group_id);


--
-- Name: idx_topologies_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_topologies_site ON public.topologies USING btree (site_id);


--
-- Name: idx_user_api_key_site_access_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_key_site_access_key ON public.user_api_key_site_access USING btree (api_key_id);


--
-- Name: idx_user_api_key_site_access_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_key_site_access_site ON public.user_api_key_site_access USING btree (site_id);


--
-- Name: idx_user_api_keys_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_keys_key ON public.user_api_keys USING btree (key);


--
-- Name: idx_user_api_keys_org; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_keys_org ON public.user_api_keys USING btree (organization_id);


--
-- Name: idx_user_api_keys_user; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_keys_user ON public.user_api_keys USING btree (user_id);


--
-- Name: idx_user_site_access_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_site_access_site ON public.user_site_access USING btree (site_id);


--
-- Name: idx_user_site_access_user; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_site_access_user ON public.user_site_access USING btree (user_id);


--
-- Name: idx_users_email_lower; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_users_email_lower ON public.users USING btree (lower(email));


--
-- Name: idx_users_email_unsubscribe_token; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_users_email_unsubscribe_token ON public.users USING btree (email_unsubscribe_token);


--
-- Name: idx_users_email_verification_token; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_users_email_verification_token ON public.users USING btree (email_verification_token) WHERE (email_verification_token IS NOT NULL);


--
-- Name: idx_users_oidc_provider_subject; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_users_oidc_provider_subject ON public.users USING btree (oidc_provider, oidc_subject) WHERE ((oidc_provider IS NOT NULL) AND (oidc_subject IS NOT NULL));


--
-- Name: idx_users_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_users_organization ON public.users USING btree (organization_id);


--
-- Name: idx_users_password_reset_token; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_users_password_reset_token ON public.users USING btree (password_reset_token) WHERE (password_reset_token IS NOT NULL);


--
-- Name: idx_vlans_as_of; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_vlans_as_of ON public.vlans USING btree (site_id, valid_from, valid_to);


--
-- Name: idx_vlans_lineage; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_vlans_lineage ON public.vlans USING btree (lineage_id) WHERE (valid_to IS NOT NULL);


--
-- Name: idx_vlans_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_vlans_live ON public.vlans USING btree (site_id) WHERE (valid_to IS NULL);


--
-- Name: idx_vlans_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_vlans_organization ON public.vlans USING btree (organization_id);


--
-- Name: idx_vlans_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_vlans_site ON public.vlans USING btree (site_id);


--
-- Name: idx_vlans_site_number_live; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_vlans_site_number_live ON public.vlans USING btree (site_id, vlan_number) WHERE (valid_to IS NULL);


--
-- Name: idx_vlans_snapshot_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_vlans_snapshot_id ON public.vlans USING btree (snapshot_id) WHERE (snapshot_id IS NOT NULL);


--
-- Name: users reassign_daemons_before_user_delete; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER reassign_daemons_before_user_delete BEFORE DELETE ON public.users FOR EACH ROW EXECUTE FUNCTION public.reassign_daemons_on_user_delete();


--
-- Name: api_keys api_keys_daemon_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_daemon_id_fkey FOREIGN KEY (daemon_id) REFERENCES public.daemons(id) ON DELETE CASCADE;


--
-- Name: api_keys api_keys_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: bindings bindings_ip_address_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_ip_address_id_fkey FOREIGN KEY (ip_address_id) REFERENCES public.ip_addresses(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: bindings bindings_port_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_port_id_fkey FOREIGN KEY (port_id) REFERENCES public.ports(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_service_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_service_id_fkey FOREIGN KEY (service_id) REFERENCES public.services(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: credentials credentials_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.credentials
    ADD CONSTRAINT credentials_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: daemon_interfaced_subnets daemon_interfaced_subnets_daemon_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemon_interfaced_subnets
    ADD CONSTRAINT daemon_interfaced_subnets_daemon_id_fkey FOREIGN KEY (daemon_id) REFERENCES public.daemons(id) ON DELETE CASCADE;


--
-- Name: daemon_interfaced_subnets daemon_interfaced_subnets_subnet_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemon_interfaced_subnets
    ADD CONSTRAINT daemon_interfaced_subnets_subnet_id_fkey FOREIGN KEY (subnet_id) REFERENCES public.subnets(id) ON DELETE CASCADE;


--
-- Name: daemons daemons_api_key_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_api_key_id_fkey FOREIGN KEY (api_key_id) REFERENCES public.api_keys(id) ON DELETE SET NULL;


--
-- Name: daemons daemons_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: daemons daemons_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: dependencies dependencies_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependencies
    ADD CONSTRAINT dependencies_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: dependencies dependencies_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependencies
    ADD CONSTRAINT dependencies_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: dependency_members dependency_members_service_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependency_members
    ADD CONSTRAINT dependency_members_service_id_fkey FOREIGN KEY (service_id) REFERENCES public.services(id) ON DELETE CASCADE;


--
-- Name: dependency_members dependency_members_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependency_members
    ADD CONSTRAINT dependency_members_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: discovery discovery_daemon_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery
    ADD CONSTRAINT discovery_daemon_id_fkey FOREIGN KEY (daemon_id) REFERENCES public.daemons(id) ON DELETE CASCADE;


--
-- Name: discovery discovery_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery
    ADD CONSTRAINT discovery_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: discovery_warnings discovery_warnings_credential_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery_warnings
    ADD CONSTRAINT discovery_warnings_credential_id_fkey FOREIGN KEY (credential_id) REFERENCES public.credentials(id) ON DELETE SET NULL;


--
-- Name: discovery_warnings discovery_warnings_discovery_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery_warnings
    ADD CONSTRAINT discovery_warnings_discovery_id_fkey FOREIGN KEY (discovery_id) REFERENCES public.discovery(id) ON DELETE CASCADE;


--
-- Name: discovery_warnings discovery_warnings_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery_warnings
    ADD CONSTRAINT discovery_warnings_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE SET NULL;


--
-- Name: discovery_warnings discovery_warnings_remote_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery_warnings
    ADD CONSTRAINT discovery_warnings_remote_host_id_fkey FOREIGN KEY (remote_host_id) REFERENCES public.hosts(id) ON DELETE SET NULL;


--
-- Name: discovery_warnings discovery_warnings_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery_warnings
    ADD CONSTRAINT discovery_warnings_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: discovery_warnings discovery_warnings_subnet_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery_warnings
    ADD CONSTRAINT discovery_warnings_subnet_id_fkey FOREIGN KEY (subnet_id) REFERENCES public.subnets(id) ON DELETE SET NULL;


--
-- Name: entity_tags entity_tags_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: entity_tags entity_tags_tag_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_tag_id_fkey FOREIGN KEY (tag_id) REFERENCES public.tags(id) ON DELETE CASCADE;


--
-- Name: dependency_members group_bindings_binding_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependency_members
    ADD CONSTRAINT group_bindings_binding_id_fkey FOREIGN KEY (binding_id) REFERENCES public.bindings(id) ON DELETE CASCADE;


--
-- Name: dependency_members group_bindings_group_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.dependency_members
    ADD CONSTRAINT group_bindings_group_id_fkey FOREIGN KEY (dependency_id) REFERENCES public.dependencies(id) ON DELETE CASCADE;


--
-- Name: host_credentials host_credentials_credential_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.host_credentials
    ADD CONSTRAINT host_credentials_credential_id_fkey FOREIGN KEY (credential_id) REFERENCES public.credentials(id) ON DELETE CASCADE;


--
-- Name: host_credentials host_credentials_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.host_credentials
    ADD CONSTRAINT host_credentials_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: hosts hosts_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: hosts hosts_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: hosts hosts_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: hosts hosts_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: hosts hosts_virtualization_interface_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_virtualization_interface_id_fkey FOREIGN KEY (virtualization_interface_id) REFERENCES public.interfaces(id) ON DELETE SET NULL;


--
-- Name: hosts hosts_virtualization_service_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_virtualization_service_id_fkey FOREIGN KEY (virtualization_service_id) REFERENCES public.services(id) ON DELETE SET NULL;


--
-- Name: interface_neighbor_candidates interface_neighbor_candidates_interface_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interface_neighbor_candidates
    ADD CONSTRAINT interface_neighbor_candidates_interface_id_fkey FOREIGN KEY (interface_id) REFERENCES public.interfaces(id) ON DELETE CASCADE;


--
-- Name: interface_neighbor_candidates interface_neighbor_candidates_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interface_neighbor_candidates
    ADD CONSTRAINT interface_neighbor_candidates_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: interface_neighbor_hosts interface_neighbor_hosts_interface_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interface_neighbor_hosts
    ADD CONSTRAINT interface_neighbor_hosts_interface_id_fkey FOREIGN KEY (interface_id) REFERENCES public.interfaces(id) ON DELETE CASCADE;


--
-- Name: interface_neighbor_hosts interface_neighbor_hosts_neighbor_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interface_neighbor_hosts
    ADD CONSTRAINT interface_neighbor_hosts_neighbor_host_id_fkey FOREIGN KEY (neighbor_host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: interface_neighbor_hosts interface_neighbor_hosts_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interface_neighbor_hosts
    ADD CONSTRAINT interface_neighbor_hosts_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: interface_neighbor_hosts interface_neighbor_hosts_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interface_neighbor_hosts
    ADD CONSTRAINT interface_neighbor_hosts_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: interface_neighbor_interfaces interface_neighbor_interfaces_interface_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interface_neighbor_interfaces
    ADD CONSTRAINT interface_neighbor_interfaces_interface_id_fkey FOREIGN KEY (interface_id) REFERENCES public.interfaces(id) ON DELETE CASCADE;


--
-- Name: interface_neighbor_interfaces interface_neighbor_interfaces_neighbor_interface_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interface_neighbor_interfaces
    ADD CONSTRAINT interface_neighbor_interfaces_neighbor_interface_id_fkey FOREIGN KEY (neighbor_interface_id) REFERENCES public.interfaces(id) ON DELETE CASCADE;


--
-- Name: interface_neighbor_interfaces interface_neighbor_interfaces_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interface_neighbor_interfaces
    ADD CONSTRAINT interface_neighbor_interfaces_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: interface_neighbor_interfaces interface_neighbor_interfaces_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interface_neighbor_interfaces
    ADD CONSTRAINT interface_neighbor_interfaces_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: interfaces interfaces_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_ip_address_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_ip_address_id_fkey FOREIGN KEY (ip_address_id) REFERENCES public.ip_addresses(id) ON DELETE SET NULL;


--
-- Name: interfaces interfaces_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: interfaces interfaces_native_vlan_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_native_vlan_id_fkey FOREIGN KEY (native_vlan_id) REFERENCES public.vlans(id) ON DELETE SET NULL;


--
-- Name: interfaces interfaces_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: invites invites_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.invites
    ADD CONSTRAINT invites_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: invites invites_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.invites
    ADD CONSTRAINT invites_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: ip_addresses ip_addresses_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ip_addresses
    ADD CONSTRAINT ip_addresses_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: ip_addresses ip_addresses_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ip_addresses
    ADD CONSTRAINT ip_addresses_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: ip_addresses ip_addresses_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ip_addresses
    ADD CONSTRAINT ip_addresses_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: ip_addresses ip_addresses_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ip_addresses
    ADD CONSTRAINT ip_addresses_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: ip_addresses ip_addresses_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ip_addresses
    ADD CONSTRAINT ip_addresses_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: ip_addresses ip_addresses_subnet_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ip_addresses
    ADD CONSTRAINT ip_addresses_subnet_id_fkey FOREIGN KEY (subnet_id) REFERENCES public.subnets(id) ON DELETE CASCADE;


--
-- Name: license_instances license_instances_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.license_instances
    ADD CONSTRAINT license_instances_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: sites organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.sites
    ADD CONSTRAINT organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: ports ports_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: ports ports_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: ports ports_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: ports ports_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: ports ports_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: services services_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: services services_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: services services_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: services services_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: services services_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: services services_virtualization_service_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_virtualization_service_id_fkey FOREIGN KEY (virtualization_service_id) REFERENCES public.services(id) ON DELETE SET NULL;


--
-- Name: shares shares_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: shares shares_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: shares shares_topology_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_topology_id_fkey FOREIGN KEY (topology_id) REFERENCES public.topologies(id) ON DELETE CASCADE;


--
-- Name: site_credentials site_credentials_credential_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.site_credentials
    ADD CONSTRAINT site_credentials_credential_id_fkey FOREIGN KEY (credential_id) REFERENCES public.credentials(id) ON DELETE CASCADE;


--
-- Name: site_credentials site_credentials_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.site_credentials
    ADD CONSTRAINT site_credentials_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: snapshots snapshots_created_by_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.snapshots
    ADD CONSTRAINT snapshots_created_by_user_id_fkey FOREIGN KEY (created_by_user_id) REFERENCES public.users(id) ON DELETE SET NULL;


--
-- Name: snapshots snapshots_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.snapshots
    ADD CONSTRAINT snapshots_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: subnet_vlans subnet_vlans_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnet_vlans
    ADD CONSTRAINT subnet_vlans_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: subnet_vlans subnet_vlans_subnet_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnet_vlans
    ADD CONSTRAINT subnet_vlans_subnet_id_fkey FOREIGN KEY (subnet_id) REFERENCES public.subnets(id) ON DELETE CASCADE;


--
-- Name: subnet_vlans subnet_vlans_vlan_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnet_vlans
    ADD CONSTRAINT subnet_vlans_vlan_id_fkey FOREIGN KEY (vlan_id) REFERENCES public.vlans(id) ON DELETE CASCADE;


--
-- Name: subnets subnets_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: subnets subnets_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: subnets subnets_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: subnets subnets_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: subnets subnets_virtualization_service_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_virtualization_service_id_fkey FOREIGN KEY (virtualization_service_id) REFERENCES public.services(id) ON DELETE SET NULL;


--
-- Name: tag_groups tag_groups_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tag_groups
    ADD CONSTRAINT tag_groups_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: tags tags_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: tags tags_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- Name: tags tags_tag_group_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_tag_group_id_fkey FOREIGN KEY (tag_group_id) REFERENCES public.tag_groups(id) ON DELETE SET NULL;


--
-- Name: topologies topologies_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.topologies
    ADD CONSTRAINT topologies_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: user_api_key_site_access user_api_key_site_access_api_key_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_site_access
    ADD CONSTRAINT user_api_key_site_access_api_key_id_fkey FOREIGN KEY (api_key_id) REFERENCES public.user_api_keys(id) ON DELETE CASCADE;


--
-- Name: user_api_key_site_access user_api_key_site_access_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_site_access
    ADD CONSTRAINT user_api_key_site_access_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: user_api_keys user_api_keys_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_keys
    ADD CONSTRAINT user_api_keys_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: user_api_keys user_api_keys_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_keys
    ADD CONSTRAINT user_api_keys_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: user_site_access user_site_access_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_site_access
    ADD CONSTRAINT user_site_access_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: user_site_access user_site_access_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_site_access
    ADD CONSTRAINT user_site_access_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: users users_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: vlans vlans_first_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.vlans
    ADD CONSTRAINT vlans_first_discovery_fk FOREIGN KEY (first_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: vlans vlans_last_discovery_fk; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.vlans
    ADD CONSTRAINT vlans_last_discovery_fk FOREIGN KEY (last_discovery_id) REFERENCES public.discovery(id) ON DELETE SET NULL;


--
-- Name: vlans vlans_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.vlans
    ADD CONSTRAINT vlans_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: vlans vlans_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.vlans
    ADD CONSTRAINT vlans_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: vlans vlans_snapshot_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.vlans
    ADD CONSTRAINT vlans_snapshot_id_fkey FOREIGN KEY (snapshot_id) REFERENCES public.snapshots(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict GjfVS29iX7gdYI3bTiabVD86psO000c6LT9aH6F8eV5v3WvA81n24P3blseLg6I

