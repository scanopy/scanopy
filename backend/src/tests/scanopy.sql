--
-- PostgreSQL database dump
--

\restrict uPffam4jfSPMVEX8U71uOgdufbhRqxaJFDwm2XVpQTf8Ss7S4LTP7BzNamU1MIq

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
    asset_tag_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    matched_device_type text,
    matched_device_type_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    dismissed_at timestamp with time zone
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
    cidr_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    name_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL,
    subnet_type_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL
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
    snapshot_id uuid,
    name_source jsonb DEFAULT '"Unspecified"'::jsonb NOT NULL
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
20251006215000	users	2026-10-31 00:21:11.916732+00	t	\\x4f13ce14ff67ef0b7145987c7b22b588745bf9fbb7b673450c26a0f2f9a36ef8ca980e456c8d77cfb1b2d7a4577a64d7	-1
20251006215100	networks	2026-10-31 00:21:11.922294+00	t	\\xeaa5a07a262709f64f0c59f31e25519580c79e2d1a523ce72736848946a34b17dd9adc7498eaf90551af6b7ec6d4e0e3	-1
20251006215151	create_hosts	2026-10-31 00:21:11.926586+00	t	\\x6ec7487074c0724932d21df4cf1ed66645313cf62c159a7179e39cbc261bcb81a24f7933a0e3cf58504f2a90fc5c1962	-1
20251006215155	create_subnets	2026-10-31 00:21:11.930764+00	t	\\xefb5b25742bd5f4489b67351d9f2494a95f307428c911fd8c5f475bfb03926347bdc269bbd048d2ddb06336945b27926	-1
20251006215201	create_groups	2026-10-31 00:21:11.935158+00	t	\\x0a7032bf4d33a0baf020e905da865cde240e2a09dda2f62aa535b2c5d4b26b20be30a3286f1b5192bd94cd4a5dbb5bcd	-1
20251006215204	create_daemons	2026-10-31 00:21:11.939757+00	t	\\xcfea93403b1f9cf9aac374711d4ac72d8a223e3c38a1d2a06d9edb5f94e8a557debac3668271f8176368eadc5105349f	-1
20251006215212	create_services	2026-10-31 00:21:11.944982+00	t	\\xd5b07f82fc7c9da2782a364d46078d7d16b5c08df70cfbf02edcfe9b1b24ab6024ad159292aeea455f15cfd1f4740c1d	-1
20251029193448	user-auth	2026-10-31 00:21:11.954763+00	t	\\xfde8161a8db89d51eeade7517d90a41d560f19645620f2298f78f116219a09728b18e91251ae31e46a47f6942d5a9032	-1
20251030044828	daemon_api	2026-10-31 00:21:11.956642+00	t	\\x181eb3541f51ef5b038b2064660370775d1b364547a214a20dde9c9d4bb95a1c273cd4525ef29e61fa65a3eb4fee0400	-1
20251030170438	host-hide	2026-10-31 00:21:11.958041+00	t	\\x87c6fda7f8456bf610a78e8e98803158caa0e12857c5bab466a5bb0004d41b449004a68e728ca13f17e051f662a15454	-1
20251102224919	create_discovery	2026-10-31 00:21:11.96935+00	t	\\xb32a04abb891aba48f92a059fae7341442355ca8e4af5d109e28e2a4f79ee8e11b2a8f40453b7f6725c2dd6487f26573	-1
20251106235621	normalize-daemon-cols	2026-10-31 00:21:11.971697+00	t	\\x5b137118d506e2708097c432358bf909265b3cf3bacd662b02e2c81ba589a9e0100631c7801cffd9c57bb10a6674fb3b	-1
20251107034459	api_keys	2026-10-31 00:21:11.980518+00	t	\\x3133ec043c0c6e25b6e55f7da84cae52b2a72488116938a2c669c8512c2efe72a74029912bcba1f2a2a0a8b59ef01dde	-1
20251107222650	oidc-auth	2026-10-31 00:21:12.006337+00	t	\\xd349750e0298718cbcd98eaff6e152b3fb45c3d9d62d06eedeb26c75452e9ce1af65c3e52c9f2de4bd532939c2f31096	-1
20251110181948	orgs-billing	2026-10-31 00:21:12.017529+00	t	\\x5bbea7a2dfc9d00213bd66b473289ddd66694eff8a4f3eaab937c985b64c5f8c3ad2d64e960afbb03f335ac6766687aa	-1
20251113223656	group-enhancements	2026-10-31 00:21:12.019125+00	t	\\xbe0699486d85df2bd3edc1f0bf4f1f096d5b6c5070361702c4d203ec2bb640811be88bb1979cfe51b40805ad84d1de65	-1
20251117032720	daemon-mode	2026-10-31 00:21:12.021284+00	t	\\xdd0d899c24b73d70e9970e54b2c748d6b6b55c856ca0f8590fe990da49cc46c700b1ce13f57ff65abd6711f4bd8a6481	-1
20251118143058	set-default-plan	2026-10-31 00:21:12.022833+00	t	\\xd19142607aef84aac7cfb97d60d29bda764d26f513f2c72306734c03cec2651d23eee3ce6cacfd36ca52dbddc462f917	-1
20251118225043	save-topology	2026-10-31 00:21:12.031608+00	t	\\x011a594740c69d8d0f8b0149d49d1b53cfbf948b7866ebd84403394139cb66a44277803462846b06e762577adc3e61a3	-1
20251123232748	network-permissions	2026-10-31 00:21:12.034699+00	t	\\x161be7ae5721c06523d6488606f1a7b1f096193efa1183ecdd1c2c9a4a9f4cad4884e939018917314aaf261d9a3f97ae	-1
20251125001342	billing-updates	2026-10-31 00:21:12.035965+00	t	\\xa235d153d95aeb676e3310a52ccb69dfbd7ca36bba975d5bbca165ceeec7196da12119f23597ea5276c364f90f23db1e	-1
20251128035448	org-onboarding-status	2026-10-31 00:21:12.037835+00	t	\\x1d7a7e9bf23b5078250f31934d1bc47bbaf463ace887e7746af30946e843de41badfc2b213ed64912a18e07b297663d8	-1
20251129180942	nfs-consolidate	2026-10-31 00:21:12.039373+00	t	\\xb38f41d30699a475c2b967f8e43156f3b49bb10341bddbde01d9fb5ba805f6724685e27e53f7e49b6c8b59e29c74f98e	-1
20251206052641	discovery-progress	2026-10-31 00:21:12.041275+00	t	\\x9d433b7b8c58d0d5437a104497e5e214febb2d1441a3ad7c28512e7497ed14fb9458e0d4ff786962a59954cb30da1447	-1
20251206202200	plan-fix	2026-10-31 00:21:12.042462+00	t	\\x242f6699dbf485cf59a8d1b8cd9d7c43aeef635a9316be815a47e15238c5e4af88efaa0daf885be03572948dc0c9edac	-1
20251207061341	daemon-url	2026-10-31 00:21:12.045123+00	t	\\x01172455c4f2d0d57371d18ef66d2ab3b7a8525067ef8a86945c616982e6ce06f5ea1e1560a8f20dadcd5be2223e6df1	-1
20251210045929	tags	2026-10-31 00:21:12.054769+00	t	\\xe3dde83d39f8552b5afcdc1493cddfeffe077751bf55472032bc8b35fc8fc2a2caa3b55b4c2354ace7de03c3977982db	-1
20251210175035	terms	2026-10-31 00:21:12.056137+00	t	\\xe47f0cf7aba1bffa10798bede953da69fd4bfaebf9c75c76226507c558a3595c6bfc6ac8920d11398dbdf3b762769992	-1
20251213025048	hash-keys	2026-10-31 00:21:12.066045+00	t	\\xfc7cbb8ce61f0c225322297f7459dcbe362242b9001c06cb874b7f739cea7ae888d8f0cfaed6623bcbcb9ec54c8cd18b	-1
20251214050638	scanopy	2026-10-31 00:21:12.06782+00	t	\\x0108bb39832305f024126211710689adc48d973ff66e5e59ff49468389b75c1ff95d1fbbb7bdb50e33ec1333a1f29ea6	-1
20251215215724	topo-scanopy-fix	2026-10-31 00:21:12.06887+00	t	\\xed88a4b71b3c9b61d46322b5053362e5a25a9293cd3c420c9df9fcaeb3441254122b8a18f58c297f535c842b8a8b0a38	-1
20251217153736	category_rename	2026-10-31 00:21:12.071051+00	t	\\x03af7ec905e11a77e25038a3c272645da96014da7c50c585a25cea3f9a7579faba3ff45114a5e589d144c9550ba42421	-1
20251218053111	invite-persistence	2026-10-31 00:21:12.076428+00	t	\\x21d12f48b964acfd600f88e70ceb14abd9cf2a8a10db2eae2a6d8f44cf7d20749f93293631e6123e92b7c3c1793877c2	-1
20251219211216	create_shares	2026-10-31 00:21:12.083675+00	t	\\x036485debd3536f9e58ead728f461b925585911acf565970bf3b2ab295b12a2865606d6a56d334c5641dcd42adeb3d68	-1
20251220170928	permissions-cleanup	2026-10-31 00:21:12.085393+00	t	\\x632f7b6702b494301e0d36fd3b900686b1a7f9936aef8c084b5880f1152b8256a125566e2b5ac40216eaadd3c4c64a03	-1
20251220180000	commercial-to-community	2026-10-31 00:21:12.086526+00	t	\\x26fc298486c225f2f01271d611418377c403183ae51daf32fef104ec07c027f2017d138910c4fbfb5f49819a5f4194d6	-1
20251221010000	cleanup_subnet_type	2026-10-31 00:21:12.087702+00	t	\\xb521121f3fd3a10c0de816977ac2a2ffb6118f34f8474ffb9058722abc0dc4cf5cbec83bc6ee49e79a68e6b715087f40	-1
20251221020000	remove_host_target	2026-10-31 00:21:12.089058+00	t	\\x77b5f8872705676ca81a5704bd1eaee90b9a52b404bdaa27a23da2ffd4858d3e131680926a5a00ad2a0d7a24ba229046	-1
20251221030000	user_network_access	2026-10-31 00:21:12.096169+00	t	\\x5c23f5bb6b0b8ca699a17eee6730c4197a006ca21fecc79136a5e5697b9211a81b4cd08ceda70dace6a26408d021ff3a	-1
20251221040000	interfaces_table	2026-10-31 00:21:12.10624+00	t	\\xf7977b6f1e7e5108c614397d03a38c9bd9243fdc422575ec29610366a0c88f443de2132185878d8e291f06a50a8c3244	-1
20251221050000	ports_table	2026-10-31 00:21:12.115339+00	t	\\xdf72f9306b405be7be62c39003ef38408115e740b120f24e8c78b8e136574fff7965c52023b3bc476899613fa5f4fe35	-1
20251221060000	bindings_table	2026-10-31 00:21:12.125966+00	t	\\x933648a724bd179c7f47305e4080db85342d48712cde39374f0f88cde9d7eba8fe5fafba360937331e2a8178dec420c4	-1
20251221070000	group_bindings	2026-10-31 00:21:12.132478+00	t	\\x697475802f6c42e38deee6596f4ba786b09f7b7cd91742fbc5696dd0f9b3ddfce90dd905153f2b1a9e82f959f5a88302	-1
20251222020000	tag_cascade_delete	2026-10-31 00:21:12.134273+00	t	\\xabfb48c0da8522f5c8ea6d482eb5a5f4562ed41f6160a5915f0fd477c7dd0517aa84760ef99ab3a5db3e0f21b0c69b5f	-1
20251223232524	network_remove_default	2026-10-31 00:21:12.135765+00	t	\\x7099fe4e52405e46269d7ce364050da930b481e72484ad3c4772fd2911d2d505476d659fa9f400c63bc287512d033e18	-1
20251225100000	color_enum	2026-10-31 00:21:12.137367+00	t	\\x62cecd9d79a49835a3bea68a7959ab62aa0c1aaa7e2940dec6a7f8a714362df3649f0c1f9313672d9268295ed5a1cfa9	-1
20251227010000	topology_snapshot_migration	2026-10-31 00:21:12.142969+00	t	\\xc042591d254869c0e79c8b52a9ede680fd26f094e2c385f5f017e115f5e3f31ad155f4885d095344f2642ebb70755d54	-1
20251228010000	user_api_keys	2026-10-31 00:21:12.154903+00	t	\\xa41adb558a5b9d94a4e17af3f16839b83f7da072dbeac9251b12d8a84c7bec6df008009acf246468712a975bb36bb5f5	-1
20251230160000	daemon_version_and_maintainer	2026-10-31 00:21:12.158002+00	t	\\xafed3d9f00adb8c1b0896fb663af801926c218472a0a197f90ecdaa13305a78846a9e15af0043ec010328ba533fca68f	-1
20260103000000	service_position	2026-10-31 00:21:12.160262+00	t	\\x19d00e8c8b300d1c74d721931f4d771ec7bc4e06db0d6a78126e00785586fdc4bcff5b832eeae2fce0cb8d01e12a7fb5	-1
20260106000000	interface_mac_index	2026-10-31 00:21:12.162385+00	t	\\xa26248372a1e31af46a9c6fbdaef178982229e2ceeb90cc6a289d5764f87a38747294b3adf5f21276b5d171e42bdb6ac	-1
20260106204402	entity_tags_junction	2026-10-31 00:21:12.179012+00	t	\\xf73c604f9f0b8db065d990a861684b0dbd62c3ef9bead120c68431c933774de56491a53f021e79f09801680152f5a08a	-1
20260108033856	fix_entity_tags_json_format	2026-10-31 00:21:12.180862+00	t	\\x197eaa063d4f96dd0e897ad8fd96cc1ba9a54dda40a93a5c12eac14597e4dea4c806dd0a527736fb5807b7a8870d9916	-1
20260110000000	email_verification	2026-10-31 00:21:12.187829+00	t	\\xb8da8433f58ba4ce846b9fa0c2551795747a8473ad10266b19685504847458ea69d27a0ce430151cfb426f5f5fb6ac3a	-1
20260114145808	daemon_user_fk_set_null	2026-10-31 00:21:12.189399+00	t	\\x57b060be9fc314d7c5851c75661ca8269118feea6cf7ee9c61b147a0e117c4d39642cf0d1acdf7a723a9a76066c1b8ff	-1
20260116010000	snmp_credentials	2026-10-31 00:21:12.196756+00	t	\\x6f3971cf194d56883c61fa795406a8ab568307ed86544920d098b32a6a1ebb7effcb5ec38a70fdc9b617eff92d63d51e	-1
20260116020000	host_snmp_fields	2026-10-31 00:21:12.201195+00	t	\\xf2f088c13ab0dd34e1cb1e5327b0b4137440b0146e5ce1e78b8d2dfa05d9b5a12a328eeb807988453a8a43ad8a1c95ba	-1
20260116030000	if_entries	2026-10-31 00:21:12.217418+00	t	\\xa58391708f8b21901ab9250af528f638a6055462f70ffddfd7c451433aacdabd62825546fa8be108f23a3cae78b8ae28	-1
20260116100000	daemon_api_key_link	2026-10-31 00:21:12.22108+00	t	\\x41088aa314ab173344a6b416280721806b2f296a32a8d8cae58c7e5717f389fe599134ed03980ed97e4b7659e99c4f82	-1
20260131190000	add_hubspot_company_id	2026-10-31 00:21:12.22242+00	t	\\x4326f95f4954e176157c1c3e034074a3e5c44da4d60bbd7a9e4b6238c9ef52a30f8b38d3c887864b6e4c1163dc062beb	-1
20260201021238	fix_service_acronym_capitalization	2026-10-31 00:21:12.224122+00	t	\\x88b010ac8f0223d880ea6a730f11dc6d27fa5de9d8747de3431e46d59f1dbf2f72ae4a87c2e52c32152549f5c1f96bb2	-1
20260204004436	add_entity_tags_to_topology	2026-10-31 00:21:12.225665+00	t	\\x3eff1a1490e77065ec861ef1b9aad8c55de0170106a42720f7931b3929b179122b16e44390b2652771bf91bba32a7757	-1
20260205120000	billing_overhaul	2026-10-31 00:21:12.228084+00	t	\\xbf850cfa0c40a3c65f574efd15fd55a4b702296203d28077a09d1c22076fee8601f2b78345aef370ab9163657de767ab	-1
20260205183207	rename_hubspot_to_brevo	2026-10-31 00:21:12.229493+00	t	\\x4678a7d80215e5eafb5e80af0daa20e2868a3b4f2112e88cb1b2b9efc87d63de3fb96c133f359b224c658789ae4b0d13	-1
20260221120000	add_plan_limit_notifications	2026-10-31 00:21:12.231214+00	t	\\xef770dac07e1d80888832f33184dc46c1d3b8185b91c507cb404468d6ad8c29cacf455178801c67aa27b6a626d3ad82d	-1
20260222120000	add_pending_email	2026-10-31 00:21:12.232496+00	t	\\xddd220f7602c44548d56849c0a8d081ecd1da1383374a11e3e227c7d9becb73a49f5e5bb09ed65901c16df4c16e913e5	-1
20260301120000	add_if_name_to_if_entries	2026-10-31 00:21:12.233831+00	t	\\xc9fc0a2b77ecbf0e1d5ab292c4fe162a26113468c878dfd26a3c63d89c0ee1957ca328ecfe25c611867a0e73780f0cb6	-1
20260306002816	cleanup_standby	2026-10-31 00:21:12.268057+00	t	\\x01b0c236a8a4d0d97f0f633b18f8cbdb92b6d72063289989b90a1b7b6b303e65e0557eb09927b2580dcb7e8ee5966c75	-1
20260309120000	add_org_use_case	2026-10-31 00:21:12.26986+00	t	\\xdb8c8a2f0f9416ba3b687fc75453d7c12c50a6f386b4784d21bd6adfc4a4a7556c637c25cf116118402bbd12c0d5aafe	-1
20260313120000	snmp_extended_discovery	2026-10-31 00:21:12.273201+00	t	\\xc4e72539099de1b830d87a169bfbabba4b8fb378a3c4c4a1dfca698adf3e403d750040d784c26d9fa343be2908064c9d	-1
20260315120000	universal_credentials	2026-10-31 00:21:12.2969+00	t	\\x87dc6f39202e81d5555df78a9d056b143f11bd22e6d7f483065f605e242a360902c72c4d5a49717e7fcc24a366bb5ff5	-1
20260315120001	discovery_scan_settings	2026-10-31 00:21:12.301033+00	t	\\xe9da183fdd8e04e574f553f61f6f33efa046cdae38c846c8077b06c5260446fb4aa39da2449bda7f1d8cf3aa9f16e158	-1
20260315120002	backfill_org_created_milestone	2026-10-31 00:21:12.302871+00	t	\\x14f886a19773cd2263d86f88479be460d21f071d5212e3789c5c40b6415c293fc7d06c7b138351cc42108f89a14fe745	-1
20260316120000	fix_jsonb_null_if_entries	2026-10-31 00:21:12.305256+00	t	\\x65c358069710f7f86d6a3e257e658c2f241cc376433c3a0317b0ec9e1876a66f9738cb65c6ab1a5c197fe40d5aa2aa2b	-1
20260319120000	rename_snmp_to_snmpv2c	2026-10-31 00:21:12.306842+00	t	\\xdce5c9461f402e1672607078b2c571f0eb30b51d46f8e9414d8909efb40693f543e49e560cb7d703db274515043aa08e	-1
20260321120000	add_discovery_scan_count	2026-10-31 00:21:12.309809+00	t	\\x6c8201ab453a51632176d534c6604e0818e28a8a4a153e33e254f4dac0f9b67c9db394082cb663ff1b25941229cf96fc	-1
20260329120000	backfill_subnet_virtualization	2026-10-31 00:21:12.314316+00	t	\\xeac50ded27603dbb5e8773604a52143c9fa8654263e7dd12d3d128ce972c2feed84600e36b2e7a79525b58c44d2ad9d3	-1
20260402120000	rename_topology_node_types	2026-10-31 00:21:12.315779+00	t	\\xc4ba06868add823f83ff1948091bdfe17dbdde80bbec6fe2cf8da2b3689aeeebbe9e9de01b1292bff3c98a74d9e6279f	-1
20260403120000	topology_grouping_rules	2026-10-31 00:21:12.320456+00	t	\\x00799da1206d7c3b3c3db90b7d14437cc054ed2d7273020342e562c619a671e008ff4fdf0365170440b392956949e730	-1
20260405120000	rename_groups_to_dependencies	2026-10-31 00:21:12.334225+00	t	\\x9ce895b456366bf6e54316b22cabd2803aa542dd3733fffa680f0a3af5c4c55a612c5ee511371206921869b7f271c35b	-1
20260406120000	add_tag_is_application_group	2026-10-31 00:21:12.335913+00	t	\\xb7a71e5fdd96ca46c9c7577003309050a93bc53ad192ac5df78e7621f3ed64f07fb29b4658f17af55732cf6dfb7958c2	-1
20260406130000	add_vlans	2026-10-31 00:21:12.34992+00	t	\\x5b3e5d10578d90b5175e5718a28d7147a21b99af2fb3e0ed171d20ee8fd8838c290f648dafdd3b72ef60ff487f7f2494	-1
20260409000000	add_vlans_to_topologies	2026-10-31 00:21:12.35175+00	t	\\x5e0b9dc670580ceec3aa6eae005a39f98733fc27dc574b7f3922f4297813facd5d610af953dfec13e09d0b99eceb3865	-1
20260410000000	rename_interfaces_and_if_entries	2026-10-31 00:21:12.360371+00	t	\\x07f54a59869f458f41f45d75f250aee26b20a426f1ec29930606841770194d6aea0e9e6253a6375fbeebcf9b49121224	-1
20260414000000	add_share_enabled_views	2026-10-31 00:21:12.361876+00	t	\\xc56514355a5977c3242e728e7f5a2533e7b4a5cf8a7ce7757e412e51f1ad85e96d65c13ccd96d050be4a07799b9aef57	-1
20260415120000	rename_onboarding_first_group_created	2026-10-31 00:21:12.363396+00	t	\\x2c17035835d3ead105b76d98688c0b7bd328abdaf9f721d70d057c8afdf438819e93da56707deea5b469b81a7b84d5d7	-1
20260417000000	reindex_interfaces_identity	2026-10-31 00:21:12.529086+00	t	\\x10701e13bc3d838e2ec4a856555ebf338173792f220c405996d3c77e7987e9806798ca0328eb6259e4a62b7e05665b25	-1
20260418000000	add_standby_cleared_at_to_daemons	2026-10-31 00:21:12.530801+00	t	\\x547807de451d015a4ce1438796d5b95e2b98043c521015a21239f6778d10a8d3bf7d8b14e278e09aa0105f1935ad4181	-1
20260501000000	add_organization_billing_flags	2026-10-31 00:21:12.533699+00	t	\\x2de34c4af667d4cd8bc263c27f0526a4a2132022e2eb71ae94fd89edebbfb40cda840055a94e89a21925a317cdff285f	-1
20260502000000	scd2_add_columns	2026-10-31 00:21:12.549635+00	t	\\xe78a73574d86320c0de7fcd43682ee3cdd436dc64d1371f98cb67ef0cdf33097df6e8a92c9a26c5702795144103b085e	-1
20260502000001	scd2_add_discovery_fks_not_valid	2026-10-31 00:21:12.564411+00	t	\\xb49114eb6f8d77cb0c5062619da5c829e5bd65898710efb6207d2804be3677a95eebe88d41d21ae8cba21344fcd63f97	-1
20260502000002	scd2_validate_discovery_fks	2026-10-31 00:21:12.57701+00	t	\\x500201842b9f486d1397c0f0f3ee1a36bc101440d4d97fe0f0bfc971672f1f53f7945aed93c34e2e53d2d9cb913419f1	-1
20260502000003	scd2_backfill_with_metadata	2026-10-31 00:21:12.586729+00	t	\\xd299c1dbf7ec284995dc8a5fcf4b264e4f1ff970085a8631e6d00aa0bc6d6708a5e41d515bb9299c22ef128a958e68e9	-1
20260502000004	scd2_partial_unique_indexes	2026-10-31 00:21:12.740774+00	t	\\xbc30e554c37ec3ca72baa7fc634920257909944dc841231a966845fd4c5bc27ae20606c98867934c872e8883bd56efa1	-1
20260502000005	scd2_drop_old_unique_constraints	2026-10-31 00:21:12.745219+00	t	\\x660c0237299796c6e70a21780d529453a2ac8645f7b122a0b8aa4f362ff780de8849f53a66b71b3ea29ecc66877f47ad	-1
20260502000006	scd2_supporting_indexes	2026-10-31 00:21:12.82097+00	t	\\x4394aaf6951c4b7048f66334fb8c550c55d4cdc06beb29eb7f086a717ee9f26ddeb28e73a7b298633ef517f6c0f84ee0	-1
20260502000007	entitysource_metadata_strip	2026-10-31 00:21:12.824493+00	t	\\x1cfd1a00b100ca0b007be6e8b55f67dde412108f299f94b419a80af45b74b75fdead95637dccb13fdd57fed2f1132446	-1
20260502100000	add_user_email_settings	2026-10-31 00:21:12.82632+00	t	\\xe886b44ca9d3d6461d82b573f617391b8b5f22f18b8113a333aa8c275545ec6455060f68ecbfdd17af86fccc9a500d7a	-1
20260502120000	create_snapshots_table	2026-10-31 00:21:12.831728+00	t	\\x18f829b255b8e11e385903e51182f04fb835aa78ef0090865ce7743f747e4aaaa7b1a7b806a377e9b12af6205aed527a	-1
20260502120001	add_snapshot_id_fks	2026-10-31 00:21:13.040403+00	t	\\x0a080ceaa2922301464b30ea7f25e1f0de5493d15424d07a8dd1f2b4bf1880cf2d25f64d5bf1863f705a1557d867afa2	-1
20260502120002	add_snapshot_id_indexes	2026-10-31 00:21:13.068616+00	t	\\xb7514ff0cf637241d21e90b48214a3a3a5076d0142c99aca39479d2a82cf35e1abdbce8ad5663ca8272d5f74004ae5f9	-1
20260502120003	topology_snapshots_backfill	2026-10-31 00:21:13.075659+00	t	\\x5435da11acf8a9fc37e0520a5b2a959fce84b16080b2a5b67adcf249dea07ae533f858c56f87542152cfc271933c1428	-1
20260502120004	drop_legacy_topology_columns	2026-10-31 00:21:13.284513+00	t	\\xfe45e73be308e6267840664a662cd679fb43469a11f06a168b8b8cfc64b6e86eb10d948271b67a0541481dcc60447769	-1
20260627120000	add_discovery_integration_targets	2026-10-31 00:21:13.286562+00	t	\\x9adf9ea4c36868a3b86067214f8550b8e866aff5d3d6c6ab3d71d1a15ea5e4be0b4349db6d15308fefbceca8403f0cbb	-1
20260703120000	daemon_interfaced_subnets_junction	2026-10-31 00:21:13.291756+00	t	\\xf1399bf8104b1d9cff6a7f66c6040829f01dd64614294295f0ecc27412acc19fd6dd20eff66ce9eadf0cf0c0e8c4353e	-1
20260703120001	drop_discovery_pending_credential_ids	2026-10-31 00:21:13.293768+00	t	\\xc2cfe6f450d000ae36149b042ebcfed938c01487d78ca56a125d72bbeca0de0548824a2396761b3dc647dc80e2e43e80	-1
20260705120000	add_user_session_epoch	2026-10-31 00:21:13.295386+00	t	\\xb296babe8e50ecdd1f50770e12d4234b47ed41cdd826c545debf5f94226672cc2d6fd28eef0483a5fdc8837f5d280d88	-1
20260706120000	drop_credentials_target_ips_and_daemons_capabilities	2026-10-31 00:21:13.297442+00	t	\\xc544c88df68334ac0b3ec180da7ba2030837707f9e3cf2b75b4d56012646c24f8f60f094cc00f05c0f446a2a845e5e98	-1
20260716120000	api_keys_daemon_id	2026-10-31 00:21:13.301538+00	t	\\x484c3b2553bcf81c95aabce4be70b1e2048e097529e04a176a0b75c2edf28ecd0892c959bbd8e8166f621a9dc2dba0dc	-1
20260716120001	api_keys_daemon_id_validate	2026-10-31 00:21:13.303863+00	t	\\x723190451e98f9394b2134dccb89f8eeeb15f6a5644fb88cb24e9e0e7a4ebf5a0c455659930ce2d3096e20c004d7da89	-1
20260716120002	api_keys_daemon_id_unique	2026-10-31 00:21:13.306754+00	t	\\xf788be5a414917ec5c8b9040f85acd352ae3f580dab7e8833db5df19b7d2cf3b43b8a021c03a5082da27eca3bb7a07f0	-1
20260722120000	add_networks_stale_after_hours	2026-10-31 00:21:13.308093+00	t	\\x308d9e04ef943560d3e420c59a3c8017064d59bd28cb63f99c4b562124e2fa038630e71405171901d731420bd9de4e2f	-1
20260727120000	rename_plan_limit_notifications_to_notifications	2026-10-31 00:21:13.309982+00	t	\\x2447c735b8ac6885ae4341738294392f367abcc7c0f5e857bb08523dc5d5a8eccf77db5462b531370a0795135757098d	-1
20260728120000	rename_stale_if_entries_constraints	2026-10-31 00:21:13.313861+00	t	\\x577c54966f8c146a6e5101eb8a2f7f006ef7899ba95ec876be10970b3483b622b254d00dc9c9582efb1862404315764b	-1
20260803120000	drop_organizations_plan_limit_notifications	2026-10-31 00:21:13.316755+00	t	\\xc4c9b44a4cce6cf419cffcbbeade3d5eb119b51f321bdcd3c51a1046d8ec8df81d32f6e76c6472eefdcfae9899ef3107	-1
20260803120001	virtualization_service_id_column	2026-10-31 00:21:13.325518+00	t	\\xe07f6fee751430c94141be6d7ba17390f08f60bdcd01a65072708382b27e9543a9f27fb0cc76bb824940794d6ea67553	-1
20260819120000	hosts_name_source	2026-10-31 00:21:13.330232+00	t	\\x6526859d261bf724e6ac3bb2a51c8588764aab15ecdfad964e06e98bb59f05a753ea2950bdce40cbf7f945d157a230aa	-1
20260824120000	interfaces_neighbor_seen_at	2026-10-31 00:21:13.331729+00	t	\\x70a26934dcc110484dc95100cbc96879504a9bc0ae0fb12ca0effe032816e3282e255d90145c4d2c6bfe2eea351cb4c3	-1
20260826120000	subnets_cidr_source	2026-10-31 00:21:13.525671+00	t	\\x1d8c6a9258f1fd07a08a268cddf5f164bb3b8bb0af34d6d167e480d50b649cbdc5a7b89aa419a218266e9aa1b3d3b497	-1
20260827120000	hosts_firmware_revision	2026-10-31 00:21:13.528787+00	t	\\xa9b3a89986c699c70232d37f5909ab5fecd9c4d2c8708a11b2cad4775494a12080c6664964f942b0079a883f3ecd6ea2	-1
20260827130000	interfaces_unknown_port_facts	2026-10-31 00:21:13.530976+00	t	\\xfa35944166a6c60219a07a6792cd3abbaf909ba78e5494482d73d3d10ddacea618845822dc3dc875542772af742ac38d	-1
20260828120000	hosts_attribute_sources	2026-10-31 00:21:13.568749+00	t	\\x29220b8fb99e5ffb55ca71ea3c05f375214dd77d247d4a54dc7855c5654dbfbbbefceafeb5ddf10710c27085f0a866c4	-1
20260903120001	interface_neighbor_tables	2026-10-31 00:21:13.791968+00	t	\\xa8352a3ed9b845d96229adcb3e3eb8e796d22a8b6aa794bbc0d2ecafcbf76d68aa59a6e7713f134a57af0acceca3865a	-1
20260915120000	organization_license_columns	2026-10-31 00:21:13.799586+00	t	\\xd46a3e083f8179128aae52f68541627b57131d66c93d84c2ad0002a7bf6c025f757345490b47d810511d49cd9528525d	-1
20260917120000	organization_license_key_type	2026-10-31 00:21:13.976966+00	t	\\xac327476c465df672532ce71bfbe03a0d92ff5cc3053fae9a15fa24100308cda6b08e118267c3c1009118eeb90f46038	-1
20260930120000	clear_zero_mac_addresses	2026-10-31 00:21:13.98261+00	t	\\xef8416f55b61953f00f1fd249a04c82d63d0d8d510e9db6cae96f79db7a70a61dfc5a22029a6878e74da05e60b63b7dd	-1
20261002120000	daemon_os	2026-10-31 00:21:13.987864+00	t	\\x931ef66a80cd751db47512b5f9397a5d7df21aff77cbf28c5a190c69b27ce6040d2e2f2ed3c08e85faf13d28e5cba25f	-1
20261004120000	host_virtualization_interface	2026-10-31 00:21:13.993457+00	t	\\x79224f29c669331336f577a5c9f7e5f674dec637f1a879e9c4016e8705af2e921e63cf4ab178cb21af1026317d6e57be	-1
20261004120002	host_virtualization_interface_index	2026-10-31 00:21:13.999794+00	t	\\x64e39691a98e9ed761812e1bfdeaa7b8d9aa13eb0401a39fb40dd36f21222ac47dad7f1b4ff9465a3cd0da1af42039ee	-1
20261005120001	rename_networks_to_sites_backfill	2026-10-31 00:21:14.020822+00	t	\\x085c1f2e6ec6b548e7f47fb092519f5f8889629e6198b30bd4a91e4e1d6b5e502513efb213c0601abf444d1f39183f81	-1
20261009120000	user_email_unsubscribe_token	2026-10-31 00:21:14.23359+00	t	\\x41aa0b879e77d264d1d6fc17c65bebe7e2d7796b9785ad79a85b928e23cc93c3cb8a0532ecbc8d72e34fc6e202d25ce1	-1
20261011120000	backfill_interface_credential_milestone	2026-10-31 00:21:14.275732+00	t	\\xfcad57a2a59537e0c9033a6ff906abf7115789554bc06c00c0caa388c220a6a5d3dc880c7aec0df9c6f561b3b94d7477	-1
20261011130001	merge_and_seed_default_credentials	2026-10-31 00:21:14.29831+00	t	\\xc5fc006e4c216dd10154418fd2d52cf3e58b70039498b21a2fc74a1b3e8d9ce34af25da2b6bc197d0c7dff4f9ca910eb	-1
20261013120000	license_instances	2026-10-31 00:21:14.32618+00	t	\\x86796dc3b14fc5805b84f6c6fd92fa8aaa560a004ea4ca8f4affa76f55b830f8d2403393cddfcf215070e84d499bb5cb	-1
20261021120000	vlan_and_subnet_sources	2026-10-31 00:21:14.333862+00	t	\\x2535fce0c0903904a1d9bfa86132278e9238231438d3b15be9863c47a362c284cc26d4b4ecb7830ee128a004d3d99878	-1
20261023120000	snmp_community_credential	2026-10-31 00:21:14.34995+00	t	\\x71e050db43cceee2a66e206cbd87f478cc8a6932440def65d5ea5c79f55ae7339bff4a9e7727b4bd7a9696dc535816cf	-1
20261026120000	strip_discovery_run_type_warnings	2026-10-31 00:21:14.361788+00	t	\\xffd0edff777c52348517fd47b5b87eb9406eea878e1168f23c2660786bf1352031ebb04761ed43dc3e6f6886dc5d4f52	-1
20260828120001	mac_address_sources	2026-10-31 00:21:13.571416+00	t	\\x39ea28fed133ae2c317bd5fad54e4d16f402ee895b75a6290322e75b434f5502aeddc9f6359d7e95625bb0c0d4ca1d4e	-1
20260828140000	ip_addresses_network_mac_index	2026-10-31 00:21:13.762975+00	t	\\x838b853bbb63bcfe0d0976f1c7289960e3133563151c47904ddda0ce5a0078bdc4af9ce07de63d3c024d89b831995535	-1
20260908120000	interfaces_ip_configured	2026-10-31 00:21:13.79417+00	t	\\x2467c0b912e78e9cac3818ffdec4157356f6b4a3c96e87a0171d88496439943eacbafefc7a4b51a2b68cc08bbbd29656	-1
20260915120001	organizations_license_entitlement	2026-10-31 00:21:13.973861+00	t	\\x1da434d08d6afb9f0bbe4165f112cae53d3046c8578d4feb88d5f3ea0fa0649cd4bf2a69ac6c2c24978a561a6333fcb9	-1
20260917130000	organization_bills_by_invoice	2026-10-31 00:21:13.978681+00	t	\\x33bf32d3b3edce4fe6803c326785210f322b9cdfd065440098b53424df7042ba90da1ac6f99ade774f894da1b67b0979	-1
20261001120000	add_user_display_settings	2026-10-31 00:21:13.984215+00	t	\\xd7f9afb0762c40085ced713496cf39e9778c4257196e1ba02a0feec1af67eab309df78ce5b753ebde6d250f6fbf3ef32	-1
20261003120000	hosts_os	2026-10-31 00:21:13.989923+00	t	\\xb8a3b3a3e6ec9e21cc64468fa5b691dba7d39556aab7cd28fee01a21fcbde0782952b4b73cbdbd334a17c554769fbdce	-1
20261004120001	host_virtualization_interface_validate	2026-10-31 00:21:13.996618+00	t	\\xf03d5b6826e7aca5a5efe1ab57878591bfd12ac5c097b1d71e5539348cf9a5e794c2ef19c5fa3ee8aac039920b9f1f4e	-1
20261005120000	rename_networks_to_sites	2026-10-31 00:21:14.013559+00	t	\\x5e08911ddc549e9d522eece781caa57fe42ee4cb6f7d16fb25b63bf6c7f95e897d2cbd12db345040f5a4035d420ace43	-1
20261006120000	host_asset_tag	2026-10-31 00:21:14.229561+00	t	\\x1a9ef01df34f62321f6fdc26f4f75d952000cc366b54217a3107132643258e44957033c5f37b3618ff8d895d01eae918	-1
20261011130000	release_notes	2026-10-31 00:21:14.279788+00	t	\\x46e5e77f7da9f2a01cb042045c01461eeb40b09477a5d2a40f302582b17e2b02e40f6e9dbe40ed1b24e667227ba3a2b3	-1
20261012120000	collection_sources	2026-10-31 00:21:14.302428+00	t	\\xfa0a6238fae38ae0c45b9657d540271cf8201a4981d6a16d84c856e8ba192e312b22472db6e2f734e1cd0a56eae723de	-1
20261013140000	terms_notice_version	2026-10-31 00:21:14.327744+00	t	\\x624bbb290511d7491b8816b152b06194417047e2b9b19eeb8b1d954ec17e6fee77c73d9fd19fb74d62f8c44d66bb1a93	-1
20261022120000	credential_needs_newer_daemon	2026-10-31 00:21:14.341367+00	t	\\xfcc14b3c03c4c930f7ffa6a0086ff9fe12d35c77ccc331b4469ebaf19ddcff05a1a21801b4f2c36fd7d14df5c403a1a1	-1
20261024120000	topology_device_type_rule	2026-10-31 00:21:14.351506+00	t	\\xb5f60015b76437698dbd4381b285195bc4b866a3d20f1cfb588bd675b92aab51dedce82bcbc29749c5e04ed4e6050d87	-1
20261025120000	every_org_has_a_plan	2026-10-31 00:21:14.357376+00	t	\\x403a5bebde12227dbbfa913490de00196184c947a72da3d86f39809db010406715561f96eac6839546713c7c9aaf90c4	-1
20261027120000	hosts_matched_device_type	2026-10-31 00:21:14.363761+00	t	\\xe970e578c2fad70d71c4e1d4a882d5e477e6372d090d93dad467ab8f656738deff5354b05868b59d8c808a2bfb61d8bd	-1
20260828130000	hosts_software_revision	2026-10-31 00:21:13.759585+00	t	\\x83cdafd5691f1bc5fc2345a696ec9fd5c75700693f0011c65d352276d806a6f207f335189153ee8e0fa3fb7e60c18234	-1
20260903120000	interfaces_if_descr_optional	2026-10-31 00:21:13.764716+00	t	\\x54495748b0271deebaf7cb06e9e4da9241563e83a162e4bfffaa9e571516223565ab239fcc8bb51fac9e37cbb7625757	-1
20260914120000	hosts_hostname_source	2026-10-31 00:21:13.797648+00	t	\\xa57a5dc26c000ec5b0b4aaf66486018f637b1f1228ae994dafa06a653b5afae3d8dd2bb1264f5695c50053d656bccdc2	-1
20260916120000	organization_license_key_issued_at	2026-10-31 00:21:13.975526+00	t	\\x99d66ef126e64532e4e87c1a99b0ac630a2031b1b4c9ba41cbe257b8e3a83e47ec0fe1360a409b5cbb57401326d7a9d3	-1
20261001120001	credential_daemon_os	2026-10-31 00:21:13.986549+00	t	\\xe679b2c2caea51e608200d064b81b02f1f4cee20cfa60a929db29b87666ba8a5d01e5d0739a2b9953f401f4171c11c9d	-1
20261003130000	organization_license_server_version	2026-10-31 00:21:13.991286+00	t	\\xe2f1bf9692ed8bb1a08a1ae13b53f632fd5f2112d53225bbf13332af26639b9518edb28daa4b3ed98bf71ed4b1002d34	-1
20261006120100	tag_group_and_icon	2026-10-31 00:21:14.231519+00	t	\\x3bad6e7e42b3a23b29a94462ddefba434ba2e7e92d1d9464db16644d40e45b97d01a83716203f5cda27d7bb5623dd436	-1
20261009120001	user_email_unsubscribe_token_index	2026-10-31 00:21:14.236935+00	t	\\x3cdfe6e7f6fe847ff4f18c7ee1d2430ee9833e48c6aa4eb3585c873529bb007b8786c26e6bad1c193ee8cf0dc0b23f0c	-1
20261009120100	tag_groups	2026-10-31 00:21:14.273553+00	t	\\xbdae7fe6cd73793312edbf2c987d3425f1e2c88c9fa98717a9e21550503734aaf67b3a1b804e145269962cd361aaa16a	-1
20261012130000	discovery_warnings	2026-10-31 00:21:14.320193+00	t	\\xc760d410eaa7d4d14214bf602cd9ba7423c1a328b71768d79bcef0cee51b9b685c37438f74091fb9c3afd6483a98e46c	-1
20261019120000	dependency_members_keep_on_delete	2026-10-31 00:21:14.331364+00	t	\\x2391912be8b3a07910ba2240ffe21065efe5836453ee2ba9ee7ca5fda5dea58d701795486c7e03a70909c88e5cb5d595	-1
20261024120001	google_nest_capitalization	2026-10-31 00:21:14.353688+00	t	\\x204cc0286eea5cba45dded8756cd98207c511b505eef07ce0470fd35104d69092038adc091536ffb2dda65e288d73262	-1
20261029120000	unconfirmed_entities	2026-10-31 00:21:14.367305+00	t	\\xb2882cb53e9225fec0318c7ed531021d6aef20dda71386d972c97095771a885b6ff0edea3a785c0b60716fa88af31a28	-1
\.


--
-- Data for Name: api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.api_keys (id, key, site_id, name, created_at, updated_at, last_used, expires_at, is_enabled, plaintext, daemon_id) FROM stdin;
4f80fccc-9b54-4e32-b3fe-4e356c2123d7	bab6dd8f07a43fe2e54c0c7b9de814f92bf64d9d6acdff680c298b4f2bc80607	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	Compat Test API Key	2026-10-31 00:28:45.21241+00	2026-10-31 00:28:45.21241+00	2026-10-31 00:29:05.694834+00	\N	t	\N	\N
302c2b6d-6581-4523-b630-db15cf1024c4	a3d3c42b7e6e9679c192fc40cf3d35cf9ca8d0f9bbec13f7acacf1d78d465903	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	scanopy-daemon API Key	2026-10-31 00:21:16.601932+00	2026-10-31 00:21:16.601932+00	2026-10-31 00:29:26.443549+00	\N	t	\N	3683c875-6eb7-4fd4-8260-9b277607eb84
44ff1280-07ac-4329-8832-5fea1a53ce01	52305b0bc7c1ece798b84c43507d4b1314ded66bc51bf5b575a7a03a14fcabc4	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	scanopy-daemon-serverpoll API Key	2026-10-31 00:22:18.762279+00	2026-10-31 00:22:18.762279+00	2026-10-31 00:29:28.918578+00	\N	t	scp_d_g3VWQAyULL9SfWZ8uXYkx8r6OV1l9poS	1ede73cd-250b-4742-a8f4-9ca1ff4581b8
\.


--
-- Data for Name: bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.bindings (id, site_id, service_id, binding_type, ip_address_id, port_id, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
2a05b29f-5921-4c86-933e-671083e9e67f	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	9ff69b61-4ac2-4522-82fb-85cd0b229403	Port	fc3c5432-a2cb-4049-85e6-64b4d71473b3	c58a15f4-b1c5-4d91-83d6-72d9069b6eb7	2026-10-01 04:02:39.555404+00	2026-10-01 04:02:39.555404+00	2026-10-01 04:02:39.555404+00	\N	\N	2026-10-01 04:02:39.555404+00	\N	\N	\N
fdc20cfb-4964-413c-a11a-c45790ed9c64	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	9ff69b61-4ac2-4522-82fb-85cd0b229403	Port	ad5a60f7-f8f2-4b78-99f7-a43668ba7558	c58a15f4-b1c5-4d91-83d6-72d9069b6eb7	2026-10-01 04:02:39.555406+00	2026-10-01 04:02:39.555406+00	2026-10-01 04:02:39.555406+00	\N	\N	2026-10-01 04:02:39.555406+00	\N	\N	\N
\.


--
-- Data for Name: credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.credentials (id, organization_id, name, credential_type, created_at, updated_at, daemon_os, description) FROM stdin;
d661a266-6ee6-4bb6-8906-28801c8eeffa	57de48ad-f218-4835-9a53-267cd70e577f	SNMP (public)	{"type": "SnmpCommunity", "community": {"mode": "Inline", "value": "public"}}	2026-10-31 00:21:16.445169+00	2026-10-31 00:21:16.445169+00	\N	Assigned to every new site. Tries the SNMP community "public" on every host, over v2c and then v1.
3adc0d85-a4bd-47ac-8fa2-3467e46480d4	57de48ad-f218-4835-9a53-267cd70e577f	Docker socket	{"type": "DockerSocket"}	2026-10-31 00:21:16.448583+00	2026-10-31 00:21:16.448583+00	\N	Added to every new daemon. Reads the local Docker socket at its default path.
12da9018-ffb5-4235-8a1e-2d4c7ae5e16e	57de48ad-f218-4835-9a53-267cd70e577f	Podman socket	{"type": "PodmanSocket"}	2026-10-31 00:21:16.45202+00	2026-10-31 00:21:16.45202+00	\N	Added to every new daemon. Reads the local Podman socket at its default path.
\.


--
-- Data for Name: daemon_interfaced_subnets; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemon_interfaced_subnets (daemon_id, subnet_id) FROM stdin;
3683c875-6eb7-4fd4-8260-9b277607eb84	d0b8a284-36a1-4e60-a131-20992f033ee5
3683c875-6eb7-4fd4-8260-9b277607eb84	bf720fd4-edfb-4341-b9d4-97b0f30185d3
1ede73cd-250b-4742-a8f4-9ca1ff4581b8	d0b8a284-36a1-4e60-a131-20992f033ee5
1ede73cd-250b-4742-a8f4-9ca1ff4581b8	bf720fd4-edfb-4341-b9d4-97b0f30185d3
\.


--
-- Data for Name: daemons; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemons (id, site_id, host_id, created_at, last_seen, updated_at, mode, url, name, version, user_id, api_key_id, is_unreachable, standby, standby_cleared_at, os) FROM stdin;
3683c875-6eb7-4fd4-8260-9b277607eb84	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	a20cea9a-e475-4818-80d1-32a346c0085e	2026-10-31 00:21:16.615275+00	2026-10-31 00:29:26.461479+00	2026-10-31 00:21:16.615275+00	"daemon_poll"		scanopy-daemon	0.19.5	63bfba8a-f884-47bc-baa0-0510fd63acda	302c2b6d-6581-4523-b630-db15cf1024c4	f	f	\N	"linux"
1ede73cd-250b-4742-a8f4-9ca1ff4581b8	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	220c414e-bd23-4130-9909-fc0e105975aa	2026-10-31 00:22:18.769403+00	2026-10-31 00:29:28.923765+00	2026-10-31 00:22:18.769403+00	"server_poll"	http://daemon-serverpoll:60074	scanopy-daemon-serverpoll	0.19.5	63bfba8a-f884-47bc-baa0-0510fd63acda	44ff1280-07ac-4329-8832-5fea1a53ce01	f	f	2026-10-31 00:29:28.923766+00	"linux"
\.


--
-- Data for Name: dependencies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.dependencies (id, site_id, name, description, created_at, updated_at, source, color, edge_style, dependency_type, member_type, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
2c2a68ea-4690-4667-98d7-82539a9b8716	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98		\N	2026-10-31 00:28:44.479249+00	2026-10-31 00:28:44.479249+00	{"type": "Manual"}	Yellow	"Bezier"	RequestPath	Services	2026-10-31 00:28:44.479249+00	\N	\N	\N
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
1ef9c43d-3860-45d7-9882-24fa3d82859c	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "27ff5779-08b0-4970-aebd-04649d27c725", "started_at": "2026-04-18T01:11:11.891664364Z", "finished_at": "2026-10-31T00:28:46.511328596Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.619810347Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:11.891664+00	2026-10-31 00:28:46.513306+00	0	f	[]
5dec0067-efe2-452c-b79d-58274d8be83d	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Scheduled", "enabled": true, "last_run": "2026-10-31T00:21:22.054652643Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "a20cea9a-e475-4818-80d1-32a346c0085e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-31 00:21:16.618497+00	2026-10-31 00:22:17.981898+00	1	f	[]
0dbd4d27-71a7-4904-9ce3-4df1c4021c06	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 100, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "c1eaa2e7-7631-418b-8d5f-74597c08a4f5", "started_at": "2026-10-31T00:21:22.175714347Z", "finished_at": "2026-10-31T00:22:17.940294255Z", "discovery_id": "5dec0067-efe2-452c-b79d-58274d8be83d", "daemon_version": "0.19.5", "discovery_type": {"type": "Unified", "host_id": "a20cea9a-e475-4818-80d1-32a346c0085e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:22:17.975280952Z", "hosts_discovered": 6, "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "a20cea9a-e475-4818-80d1-32a346c0085e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-31 00:21:22.175714+00	2026-10-31 00:22:17.981249+00	0	f	[]
b07db85d-6c1e-4499-bb81-d7c2ea138105	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "5e741620-3de9-476c-86e7-7ce656d0a5a8", "started_at": "2026-04-18T01:11:17.892141982Z", "finished_at": "2026-10-31T00:28:46.516132067Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.693980318Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:17.892141+00	2026-10-31 00:28:46.518135+00	0	f	[]
3c2904af-2fb3-46f1-a69e-bcdcf5f8d932	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "71cbed11-9ec5-4ae6-a3ce-3767032c2ab8", "started_at": "2026-04-17T22:09:29.738977360Z", "finished_at": "2026-10-31T00:28:46.530861663Z", "discovery_id": "b5c43053-768e-422c-8daf-d36dfe355879", "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "e4ddba2b-4c4f-4f89-a625-8f3f022d7581", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.593766531Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e4ddba2b-4c4f-4f89-a625-8f3f022d7581", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-17 22:09:29.738977+00	2026-10-31 00:28:46.532532+00	0	f	[]
fcb21701-e770-40ec-9a29-8dcb00a1c166	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	1ede73cd-250b-4742-a8f4-9ca1ff4581b8	{"type": "Scheduled", "enabled": true, "last_run": "2026-10-31T00:22:44.415071659Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "220c414e-bd23-4130-9909-fc0e105975aa", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-31 00:22:18.772396+00	2026-10-31 00:25:44.436024+00	1	f	[]
21ddb9b1-8a18-4453-b792-99b11d9316a8	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	1ede73cd-250b-4742-a8f4-9ca1ff4581b8	{"type": "AdHoc", "last_run": "2026-10-31T00:22:45.266362086Z"}	{"type": "Unified", "host_id": "220c414e-bd23-4130-9909-fc0e105975aa", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	ServerPoll Integration Test Discovery	2026-10-31 00:22:45.256595+00	2026-10-31 00:28:44.438353+00	1	f	[]
e3225999-acd7-4c9b-b133-b7a201c6905e	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	1ede73cd-250b-4742-a8f4-9ca1ff4581b8	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 100, "warnings": [], "daemon_id": "1ede73cd-250b-4742-a8f4-9ca1ff4581b8", "session_id": "52bf46c3-09c9-450d-a5fb-ff9c2e9ea699", "started_at": "2026-10-31T00:22:44.430320874Z", "finished_at": "2026-10-31T00:25:15.256215523Z", "discovery_id": "fcb21701-e770-40ec-9a29-8dcb00a1c166", "daemon_version": "0.19.5", "discovery_type": {"type": "Unified", "host_id": "220c414e-bd23-4130-9909-fc0e105975aa", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:25:44.431139882Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "220c414e-bd23-4130-9909-fc0e105975aa", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-31 00:22:44.43032+00	2026-10-31 00:25:44.435346+00	0	f	[]
9f99ba01-595f-49b6-b08b-61b7f197255b	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 100, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "b64df0ca-f173-4ba8-a48b-edf7e372974a", "started_at": "2026-01-07T19:14:20.155782339Z", "finished_at": "2026-01-07T19:14:20.224698505Z", "discovery_id": null, "daemon_version": "0.16.0", "discovery_type": {"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}, "last_update_at": "2026-10-31T00:28:46.097256271Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}	Self Report — My Site	2026-01-07 19:14:20.155782+00	2026-10-31 00:28:46.099521+00	0	f	[]
83a54857-919b-41a7-bf7f-cfd2314ad640	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "34c804c2-c01c-4104-a3b3-e31ae6dd0b6f", "started_at": "2026-04-18T01:11:17.355650194Z", "finished_at": "2026-10-31T00:28:46.519046672Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.686100959Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:17.35565+00	2026-10-31 00:28:46.521011+00	0	f	[]
cb34f584-bf63-4401-8c9e-3758268575e4	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "abba33fb-bf1f-4e8b-985f-6a2d0b5d0380", "started_at": "2026-04-18T01:11:10.686796959Z", "finished_at": "2026-10-31T00:28:46.533509813Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.605041588Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:10.686796+00	2026-10-31 00:28:46.535551+00	0	f	[]
851e6981-db82-4332-98de-7445e4a7c719	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "4db08e28-b456-4951-bd43-585d26cf9421", "started_at": "2026-01-07T19:14:20.246263005Z", "finished_at": "2026-10-31T00:28:46.542945461Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:46.119091225Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-07 19:14:20.246263+00	2026-10-31 00:28:46.545326+00	0	f	[]
7968b644-b04f-4f81-8823-c99d9cce829f	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "6465c546-3856-4e33-9cd0-5b8a8d99d55b", "started_at": "2026-04-18T01:11:13.585680978Z", "finished_at": "2026-10-31T00:28:46.549424528Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "5ef6e4af-f8ab-48d8-9b0e-6f4b93a49ef0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.637705559Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "5ef6e4af-f8ab-48d8-9b0e-6f4b93a49ef0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:11:13.58568+00	2026-10-31 00:28:46.550506+00	0	f	[]
7f5202e7-e67c-459c-9514-44c39c46faff	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	1ede73cd-250b-4742-a8f4-9ca1ff4581b8	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 100, "warnings": [], "daemon_id": "1ede73cd-250b-4742-a8f4-9ca1ff4581b8", "session_id": "34c6a06b-c268-47a1-bcc4-08210c0526ec", "started_at": "2026-10-31T00:25:44.558144095Z", "finished_at": "2026-10-31T00:28:14.979286596Z", "discovery_id": "21ddb9b1-8a18-4453-b792-99b11d9316a8", "daemon_version": "0.19.5", "discovery_type": {"type": "Unified", "host_id": "220c414e-bd23-4130-9909-fc0e105975aa", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:44.433941490Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "220c414e-bd23-4130-9909-fc0e105975aa", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	ServerPoll Integration Test Discovery	2026-10-31 00:25:44.558144+00	2026-10-31 00:28:44.437695+00	0	f	[]
3390388d-d9ec-4951-8ec0-44a7fc1829ba	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "3b0868af-d558-45e3-b688-2cf55472b6ee", "started_at": "2026-04-18T01:11:12.704194079Z", "finished_at": "2026-10-31T00:28:46.462136752Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.630440043Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:12.704194+00	2026-10-31 00:28:46.464505+00	0	f	[]
3b28f86e-4d70-441c-b4bd-6651e66cb6b1	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "2abdbee0-ac03-472c-b137-4321f33f03ca", "started_at": "2026-04-18T01:11:17.727006742Z", "finished_at": "2026-10-31T00:28:46.466054742Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.690053220Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:11:17.727006+00	2026-10-31 00:28:46.46736+00	0	f	[]
b3ff6cc4-c882-4795-9dfd-ee497ac2acc4	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "d2c685dd-56ad-4d49-9d36-c64b0483a15a", "started_at": "2026-04-18T01:11:18.269780388Z", "finished_at": "2026-10-31T00:28:46.468420975Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "5a5b21eb-2566-4e13-83d8-00cb2675bde6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.697944747Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "5a5b21eb-2566-4e13-83d8-00cb2675bde6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:11:18.26978+00	2026-10-31 00:28:46.469587+00	0	f	[]
488ebc31-7923-417c-9ae8-12e56b2c60e3	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "df1323db-2f1d-4090-bd79-58d04eee33c1", "started_at": "2026-04-18T01:11:16.011978400Z", "finished_at": "2026-10-31T00:28:46.470690435Z", "discovery_id": "167db396-6808-46bf-934f-7062fc6928b0", "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "a6d5229d-0b50-4243-a00d-c3d95bb17a36", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.667147605Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a6d5229d-0b50-4243-a00d-c3d95bb17a36", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:11:16.011978+00	2026-10-31 00:28:46.47232+00	0	f	[]
996525a3-bdbd-424c-878b-292fb6018c95	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "34fe487d-22ef-418c-9099-42fa3376ded0", "started_at": "2026-04-18T01:08:24.495832378Z", "finished_at": "2026-10-31T00:28:46.473422594Z", "discovery_id": "6b552988-a1be-4f68-8187-f69624148974", "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "e9d19327-eb77-47c7-85d4-e590c7aa90a1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.589969580Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e9d19327-eb77-47c7-85d4-e590c7aa90a1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:08:24.495832+00	2026-10-31 00:28:46.474965+00	0	f	[]
7a9c74fe-1373-4216-a44b-d7f03168490a	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "9dc720f8-1115-45ea-8b6d-f2e58154594d", "started_at": "2026-04-18T01:07:01.115515322Z", "finished_at": "2026-10-31T00:28:46.488078586Z", "discovery_id": "10a07091-7357-4c9b-b234-cc779b57e8eb", "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "1ce3a67d-4aeb-485a-9976-c67c4801fd30", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.550003124Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "1ce3a67d-4aeb-485a-9976-c67c4801fd30", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:07:01.115515+00	2026-10-31 00:28:46.489673+00	0	f	[]
b2c354e8-0011-4724-8798-842ee0f0f0ba	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "0ffe5496-d64e-41a0-ab86-ff28eed28819", "started_at": "2026-04-18T01:11:14.286588432Z", "finished_at": "2026-10-31T00:28:46.521949879Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.648186054Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:14.286588+00	2026-10-31 00:28:46.524075+00	0	f	[]
37b44e2b-16e1-4b72-bd48-6f6d27dcefaa	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "22ea697f-065c-4d2e-a81f-0809764aad01", "started_at": "2026-04-18T01:11:15.640169759Z", "finished_at": "2026-10-31T00:28:46.536463200Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.662928549Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:15.640169+00	2026-10-31 00:28:46.538626+00	0	f	[]
27f03edf-9ba4-4524-81d8-a78d7a751908	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "7f54ecab-3771-4583-a7d0-f12569030e17", "started_at": "2026-04-18T01:11:18.430754762Z", "finished_at": "2026-10-31T00:28:46.546260118Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}, "last_update_at": "2026-10-31T00:28:45.701847235Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}	Self Report — My Site	2026-04-18 01:11:18.430754+00	2026-10-31 00:28:46.548494+00	0	f	[]
94a46f51-1a34-4ae6-be55-783aa7b29cd6	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "1a182cd9-9710-482e-8645-d95c40db018a", "started_at": "2026-01-26T14:03:24.338877430Z", "finished_at": "2026-10-31T00:28:48.373294311Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "SelfReport", "host_id": "7891ed81-377c-4eca-b05e-bc8a17129f90"}, "last_update_at": "2026-10-31T00:28:47.961092599Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "7891ed81-377c-4eca-b05e-bc8a17129f90"}	Self Report — My Site	2026-01-26 14:03:24.338877+00	2026-10-31 00:28:48.375628+00	0	f	[]
490a0e23-3e73-4edf-a338-6396cc462b22	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "bbe36108-56c4-4d82-9087-e577842ff202", "started_at": "2026-04-18T01:11:12.435139092Z", "finished_at": "2026-10-31T00:28:46.475946285Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.626952410Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:12.435139+00	2026-10-31 00:28:46.478002+00	0	f	[]
843a2496-4a60-46a9-adf1-5e8194183ba1	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "5bd5e93e-a606-4c6e-b159-b04879bdd801", "started_at": "2026-04-18T01:11:12.972982217Z", "finished_at": "2026-10-31T00:28:46.490592217Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.633967415Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:12.972982+00	2026-10-31 00:28:46.492726+00	0	f	[]
1bfd120f-840e-433f-8fbe-e4ea4ab57bd5	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "10fdd8f4-03b6-44ea-adb6-27e74136b365", "started_at": "2026-04-18T01:11:10.955161679Z", "finished_at": "2026-10-31T00:28:46.525004682Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.608739217Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:10.955161+00	2026-10-31 00:28:46.527058+00	0	f	[]
0e5c48b2-901e-4228-a453-d68b8234c875	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "030a56ec-f416-4d7f-864e-fde8841a379c", "started_at": "2026-09-03T17:57:32.046576969Z", "finished_at": "2026-10-31T00:28:46.986679127Z", "discovery_id": "240223f7-bb0b-40f9-9126-d837799e6ed4", "daemon_version": "0.15.4", "discovery_type": {"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:46.633434211Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-03 17:57:32.046576+00	2026-10-31 00:28:46.988399+00	0	f	[]
2aecf66d-e984-4c90-9017-f9423a898f42	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "7c9364ea-b95f-4e96-b958-a71e1e3826df", "started_at": "2026-08-26T01:54:31.081144679Z", "finished_at": "2026-10-31T00:28:46.992042868Z", "discovery_id": "8c082125-5cb3-4324-a0ce-6f699bb1177f", "daemon_version": "0.15.4", "discovery_type": {"type": "Unified", "host_id": "6a1dd7f3-9b36-49b1-b554-1ef783b14e5d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:46.640787683Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6a1dd7f3-9b36-49b1-b554-1ef783b14e5d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-26 01:54:31.081144+00	2026-10-31 00:28:46.993611+00	0	f	[]
6e3a254e-56c0-45df-a971-bce740577006	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "6845dcc8-6ea0-47af-9ce3-054b8cf1f667", "started_at": "2026-04-18T01:11:14.018637270Z", "finished_at": "2026-10-31T00:28:46.478849073Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}, "last_update_at": "2026-10-31T00:28:45.644803357Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}	Self Report — My Site	2026-04-18 01:11:14.018637+00	2026-10-31 00:28:46.481036+00	0	f	[]
6e8ae22a-0cdf-4436-b436-0085016eee09	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "59fba01e-fefd-4c66-9ef2-a85c0e76a811", "started_at": "2026-04-18T01:11:12.160794534Z", "finished_at": "2026-10-31T00:28:46.481921281Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.623351008Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:12.160794+00	2026-10-31 00:28:46.484056+00	0	f	[]
be119dfc-68fa-4767-baf8-c98586560bfa	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "ec9dc330-67d0-4ce5-94d8-506859c74940", "started_at": "2026-04-18T01:11:15.092803666Z", "finished_at": "2026-10-31T00:28:46.493587582Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.655096009Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:15.092803+00	2026-10-31 00:28:46.495694+00	0	f	[]
d6eca189-1eab-4d3f-96b0-344f7ef95575	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "857fbace-0358-4642-8ca0-b14bb999f353", "started_at": "2026-04-18T01:11:15.366415752Z", "finished_at": "2026-10-31T00:28:46.496617337Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.658723379Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:15.366415+00	2026-10-31 00:28:46.498684+00	0	f	[]
f1bc1491-72af-40b2-84fa-d87fe454982d	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "f2d5414b-3369-4280-929d-5422dd11a4b4", "started_at": "2026-04-18T01:11:14.552476135Z", "finished_at": "2026-10-31T00:28:46.527952335Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}, "last_update_at": "2026-10-31T00:28:45.651586128Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}	Self Report — My Site	2026-04-18 01:11:14.552476+00	2026-10-31 00:28:46.530059+00	0	f	[]
d5a6ccf7-53b5-4649-a394-21a31c1b93eb	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "21fc2969-cba5-46cf-b381-1978e0c2959e", "started_at": "2026-09-03T17:55:43.577416616Z", "finished_at": "2026-10-31T00:28:46.989403756Z", "discovery_id": "c5f263f6-0f1f-4b2d-8da3-ac360c1a8541", "daemon_version": "0.15.4", "discovery_type": {"type": "Unified", "host_id": "3fd2970b-7217-4821-8bd7-6985174b6a22", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:46.564426988Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "3fd2970b-7217-4821-8bd7-6985174b6a22", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-03 17:55:43.577416+00	2026-10-31 00:28:46.991021+00	0	f	[]
3d7cb127-2836-4336-bec7-e8bda44aabd2	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "c10d2eae-1267-4023-997e-5a51fdf97281", "started_at": "2026-04-18T01:11:13.755644675Z", "finished_at": "2026-10-31T00:28:46.485037164Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.641220653Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:13.755644+00	2026-10-31 00:28:46.487215+00	0	f	[]
c0af7604-d6df-4437-b7d6-fcd7965bfa5f	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "4273fe18-d031-4977-950e-ad5a49d2ea8e", "started_at": "2026-04-18T01:11:16.182328068Z", "finished_at": "2026-10-31T00:28:46.499588015Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.671178232Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:16.182328+00	2026-10-31 00:28:46.50181+00	0	f	[]
bb6bccd2-5fe3-46a1-9e5d-d26304869672	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "d2b26f9b-0329-4191-8a60-58cf3c796dba", "started_at": "2026-04-18T01:11:11.423409541Z", "finished_at": "2026-10-31T00:28:46.505793026Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.612526614Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:11:11.423409+00	2026-10-31 00:28:46.506949+00	0	f	[]
81742aca-223b-4a27-8e81-d7f3eec5468b	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "4e1e43c0-b63f-41df-ba3a-71683fef4763", "started_at": "2026-02-04T01:54:22.209258404Z", "finished_at": "2026-10-31T00:28:48.370168137Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:47.540648610Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-04 01:54:22.209258+00	2026-10-31 00:28:48.3724+00	0	f	[]
1279d8be-e62b-4dfa-bc4f-7b1e5f3720c6	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "ef1a4d64-afd3-4762-b6ae-2ff80e8dbc2f", "started_at": "2026-03-26T01:38:51.825984709Z", "finished_at": "2026-10-31T00:28:48.853286513Z", "discovery_id": null, "daemon_version": "0.13.6", "discovery_type": {"type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:48.469786208Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-26 01:38:51.825984+00	2026-10-31 00:28:48.85446+00	0	f	[]
d0731afc-91f5-45bb-a5ee-e343fb3f7755	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "5b19fece-cb82-45d8-a676-df53cc38a014", "started_at": "2026-04-18T01:11:16.444659954Z", "finished_at": "2026-10-31T00:28:46.502725444Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}, "last_update_at": "2026-10-31T00:28:45.675062728Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}	Self Report — My Site	2026-04-18 01:11:16.444659+00	2026-10-31 00:28:46.504816+00	0	f	[]
f5eca039-2923-4e65-bba3-ba21af57e938	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "969d115c-0112-4ba3-8757-bb1ece340112", "started_at": "2026-04-18T01:11:17.083410573Z", "finished_at": "2026-10-31T00:28:46.508391072Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.682327818Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:17.08341+00	2026-10-31 00:28:46.510368+00	0	f	[]
e6752a27-f2d7-4dfd-b34b-401ef2927694	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "032af69a-4335-4c54-ac9c-8941c44050a3", "started_at": "2026-04-18T01:11:11.692405958Z", "finished_at": "2026-10-31T00:28:46.514247747Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:45.616173523Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:11:11.692405+00	2026-10-31 00:28:46.515323+00	0	f	[]
25477a29-142b-4ad2-8b57-2b2bb8f83518	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "1164516e-9bd1-4a5c-8fe8-182c9c6923d8", "started_at": "2026-03-31T18:30:47.557400890Z", "finished_at": "2026-10-31T00:28:47.477047191Z", "discovery_id": null, "daemon_version": "0.14.3", "discovery_type": {"type": "Unified", "host_id": "52e0ccf9-3b92-408c-9a34-1bd2f053d047", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:47.010680008Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "52e0ccf9-3b92-408c-9a34-1bd2f053d047", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:30:47.5574+00	2026-10-31 00:28:47.478426+00	0	f	[]
986f8b38-a6c6-4db0-bd46-30fba526ce90	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "adfad9b0-e25d-4bd4-88bc-e0ecc0f3f4a4", "started_at": "2026-01-26T13:51:30.181086877Z", "finished_at": "2026-10-31T00:28:47.481584118Z", "discovery_id": null, "daemon_version": "0.14.3", "discovery_type": {"type": "SelfReport", "host_id": "01d97cc8-84d8-4806-877f-52bfd29791f8"}, "last_update_at": "2026-10-31T00:28:47.050239312Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "01d97cc8-84d8-4806-877f-52bfd29791f8"}	Self Report — My Site	2026-01-26 13:51:30.181086+00	2026-10-31 00:28:47.48392+00	0	f	[]
8ace75ff-7bb9-4f8b-ab2f-1ab24f46c68a	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "e46bbbd5-df29-425e-b933-bd8b0a332217", "started_at": "2026-03-15T19:34:41.999288315Z", "finished_at": "2026-10-31T00:28:48.364070861Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "SelfReport", "host_id": "67ddad71-cd34-47fe-ad5f-138fe7069c76"}, "last_update_at": "2026-10-31T00:28:47.904641996Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "67ddad71-cd34-47fe-ad5f-138fe7069c76"}	Self Report — My Site	2026-03-15 19:34:41.999288+00	2026-10-31 00:28:48.366389+00	0	f	[]
22bfccd6-6ba1-4412-98fa-fa08d106b355	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "c8c6530c-aaff-4d7c-a872-ba4457906d77", "started_at": "2026-04-18T01:11:19.267025681Z", "finished_at": "2026-10-31T00:28:46.539459120Z", "discovery_id": "00000000-0000-0000-0000-000000000000", "daemon_version": "0.17.14", "discovery_type": {"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}, "last_update_at": "2026-10-31T00:28:45.705540172Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}	Self Report — My Site	2026-04-18 01:11:19.267025+00	2026-10-31 00:28:46.541985+00	0	f	[]
0231419e-d663-4fd7-8942-10b19fc6812c	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "9f3bb823-ded6-4cb7-957b-27c4af0ebff2", "started_at": "2026-03-15T19:35:11.987886037Z", "finished_at": "2026-10-31T00:28:48.367246167Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:47.954054710Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-15 19:35:11.987886+00	2026-10-31 00:28:48.369273+00	0	f	[]
8e5da7d0-f7c2-41f6-893e-6ddb393aea2d	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "cd493a46-8909-4378-b924-ae1650a0f5af", "started_at": "2026-03-26T01:31:45.021343211Z", "finished_at": "2026-10-31T00:28:48.851173790Z", "discovery_id": null, "daemon_version": "0.13.6", "discovery_type": {"type": "Unified", "host_id": "711a5546-8640-4ee2-bf0b-8b268c4e2044", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:48.392406485Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "711a5546-8640-4ee2-bf0b-8b268c4e2044", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-26 01:31:45.021343+00	2026-10-31 00:28:48.852354+00	0	f	[]
77afcd22-a058-4554-8bc1-f6c2bb230078	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "afde3201-cb3d-4b85-bb8f-470b83cba2a3", "started_at": "2026-02-01T15:48:58.546419422Z", "finished_at": "2026-10-31T00:28:49.722227177Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:48.954094435Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-01 15:48:58.546419+00	2026-10-31 00:28:49.724295+00	0	f	[]
60074790-933d-4246-a35f-a6be993a2c23	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "8c70cc2e-9120-44c5-8651-e6a5bad04bb9", "started_at": "2026-08-14T17:08:52.205106028Z", "finished_at": "2026-10-31T00:28:51.331671909Z", "discovery_id": "d31f6c43-b7d0-46ad-a1e3-45ecb7b19da8", "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "b2d748df-ea85-45ad-8e23-79946b4b5772", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:50.942146895Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b2d748df-ea85-45ad-8e23-79946b4b5772", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-14 17:08:52.205106+00	2026-10-31 00:28:51.33327+00	0	f	[]
80ea224d-67bf-46b2-99a4-0a79efedee1a	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "ce6f84d7-a1a5-468b-887d-1a0c67e78012", "started_at": "2026-02-28T21:18:57.228697745Z", "finished_at": "2026-10-31T00:29:09.064670146Z", "discovery_id": null, "daemon_version": "0.14.16", "discovery_type": {"type": "SelfReport", "host_id": "b52f72ce-8332-4d4f-90a6-596cbbb0ba73"}, "last_update_at": "2026-10-31T00:29:07.831218194Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "b52f72ce-8332-4d4f-90a6-596cbbb0ba73"}	Self Report — My Site	2026-02-28 21:18:57.228697+00	2026-10-31 00:29:09.067294+00	0	f	[]
cbb42468-6f4b-4099-8e6c-c5aa721618d5	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "557486b8-0125-4f97-a586-fe9ae5f972d8", "started_at": "2026-09-03T18:01:02.050183807Z", "finished_at": "2026-10-31T00:28:46.994503681Z", "discovery_id": "0b968956-dec9-4ced-b685-ea6c9df360b8", "daemon_version": "0.15.4", "discovery_type": {"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:46.637124244Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-03 18:01:02.050183+00	2026-10-31 00:28:46.996199+00	0	f	[]
7b3a99e1-e833-41d4-88f8-6d79bc590e27	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "6b771cb0-0931-4c07-a677-efe6aa5bc5bd", "started_at": "2026-02-01T15:40:05.145636842Z", "finished_at": "2026-10-31T00:28:49.705454541Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:48.918124383Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-01 15:40:05.145636+00	2026-10-31 00:28:49.707656+00	0	f	[]
78d47e10-9239-413a-a8c2-6b6757439b26	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "937bc019-8b5b-4d82-b8f2-134c36a700a8", "started_at": "2026-06-28T16:07:16.820412532Z", "finished_at": "2026-10-31T00:28:52.706211590Z", "discovery_id": "bc916077-a7ff-4a11-baa9-3b38899e409a", "daemon_version": "0.17.2", "discovery_type": {"type": "Unified", "host_id": "bb6764eb-c535-449f-90e5-a950f9304f32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:52.365630274Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bb6764eb-c535-449f-90e5-a950f9304f32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:07:16.820412+00	2026-10-31 00:28:52.707889+00	0	f	[]
5592c08a-4816-4c34-9a68-278233150dc2	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "73503118-cf7f-4008-881e-7f39b457bf47", "started_at": "2026-06-28T16:07:12.405146675Z", "finished_at": "2026-10-31T00:28:52.711537765Z", "discovery_id": "8ff643e5-aa2a-4d4f-adde-0218c0c8f22f", "daemon_version": "0.17.2", "discovery_type": {"type": "Unified", "host_id": "e61c5740-38d0-46d2-a2d6-64ce68e6606f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:52.351529128Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e61c5740-38d0-46d2-a2d6-64ce68e6606f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:07:12.405146+00	2026-10-31 00:28:52.713163+00	0	f	[]
b243ea94-5d6c-4d7a-9669-695cf8c4fc98	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "34d1feb3-276c-4ea9-b889-7eab39b67a1c", "started_at": "2026-02-23T00:07:49.639240713Z", "finished_at": "2026-10-31T00:28:57.245200487Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:55.972239713Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-23 00:07:49.63924+00	2026-10-31 00:28:57.247397+00	0	f	[]
07a4ceef-398d-438f-85bb-0f5240dea7a2	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "18c3c94f-80cb-48e9-aedc-76dbc938706e", "started_at": "2026-03-31T18:35:21.063363405Z", "finished_at": "2026-10-31T00:28:47.479480408Z", "discovery_id": null, "daemon_version": "0.14.3", "discovery_type": {"type": "Unified", "host_id": "f5245920-25e1-4d8b-b567-0f935961690d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:47.082911520Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f5245920-25e1-4d8b-b567-0f935961690d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:35:21.063363+00	2026-10-31 00:28:47.48067+00	0	f	[]
9a7061b9-c378-4243-9ce7-96f6a181d937	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "2f915185-76ea-4a3a-8f81-7ca0298ae158", "started_at": "2026-03-31T18:35:19.970481609Z", "finished_at": "2026-10-31T00:28:47.484690497Z", "discovery_id": null, "daemon_version": "0.14.3", "discovery_type": {"type": "Unified", "host_id": "2a69c4df-31b5-4490-adc9-01e591b9f974", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:47.068766860Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2a69c4df-31b5-4490-adc9-01e591b9f974", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:35:19.970481+00	2026-10-31 00:28:47.485844+00	0	f	[]
f0d6b3b2-9095-477c-a137-0a04c31d32bb	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "f5a833cc-002f-41e1-b540-c0a623d6775c", "started_at": "2026-01-25T23:12:10.196614920Z", "finished_at": "2026-10-31T00:28:49.714369178Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644"}, "last_update_at": "2026-10-31T00:28:48.921906026Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644"}	Self Report — My Site	2026-01-25 23:12:10.196614+00	2026-10-31 00:28:49.716461+00	0	f	[]
fe338285-de0f-45cc-9f4b-7c90d9f964a2	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "be760657-cb8b-4175-8484-00ea5633e0a3", "started_at": "2026-10-15T03:33:48.001242310Z", "finished_at": "2026-10-31T00:28:49.719861259Z", "discovery_id": "a0e17c57-7a55-48b1-9b19-012a790e9e9b", "daemon_version": "0.17.7", "discovery_type": {"type": "Unified", "host_id": "6943745c-a092-47e4-9991-74ba816a3f6a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:49.295287316Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6943745c-a092-47e4-9991-74ba816a3f6a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:33:48.001242+00	2026-10-31 00:28:49.721395+00	0	f	[]
feb7724b-dfa2-4466-b2e2-84e92b59a6d8	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "b9089e15-81e6-435a-90fe-db5b24561924", "started_at": "2026-02-09T15:42:07.840272212Z", "finished_at": "2026-10-31T00:28:51.336933203Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:50.594453369Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-09 15:42:07.840272+00	2026-10-31 00:28:51.338889+00	0	f	[]
b336f3e9-d2d3-4f58-b2ed-8a31edd836ed	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "927175d4-7258-4b40-a656-8412141305c5", "started_at": "2026-02-04T01:53:52.212090885Z", "finished_at": "2026-10-31T00:28:48.376464494Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "SelfReport", "host_id": "e0313f98-eeac-4055-beb3-7012a7bd7510"}, "last_update_at": "2026-10-31T00:28:47.499435857Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "e0313f98-eeac-4055-beb3-7012a7bd7510"}	Self Report — My Site	2026-02-04 01:53:52.21209+00	2026-10-31 00:28:48.378678+00	0	f	[]
ac480c4f-acd7-4818-ac7b-5ed921e68570	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "13af9d3c-5935-4be7-90cf-7fb432a4db27", "started_at": "2026-08-06T01:38:34.928909970Z", "finished_at": "2026-10-31T00:28:51.345091211Z", "discovery_id": "bcbbedea-3468-43cd-98bf-26f71fb8ca79", "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "6be44535-29d4-4940-88c3-917ce0d69222", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:50.994402246Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6be44535-29d4-4940-88c3-917ce0d69222", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-06 01:38:34.928909+00	2026-10-31 00:28:51.346675+00	0	f	[]
af857929-a010-446e-b6de-37fac0418a97	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "52a4d1e0-c3b9-4b20-a4e8-0196198a1c15", "started_at": "2026-02-02T14:20:51.227652182Z", "finished_at": "2026-10-31T00:28:52.716543471Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "SelfReport", "host_id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182"}, "last_update_at": "2026-10-31T00:28:51.361047063Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182"}	Self Report — My Site	2026-02-02 14:20:51.227652+00	2026-10-31 00:28:52.718685+00	0	f	[]
3a29ac21-6dc4-4cb9-b949-5361312e8a91	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "13aa0c04-f207-47b3-9c03-0f3209efab76", "started_at": "2026-10-12T00:03:18.358288758Z", "finished_at": "2026-10-31T00:28:59.911383665Z", "discovery_id": "76612641-3505-4024-8e73-5b47cb07eb73", "daemon_version": "0.14.17", "discovery_type": {"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:59.576917409Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-12 00:03:18.358288+00	2026-10-31 00:28:59.913115+00	0	f	[]
098cf206-25dd-4717-96eb-d19bdbb8aaa7	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "9e24c5ae-ab53-4e0f-93db-281f3614fd56", "started_at": "2026-10-11T23:58:36.269774581Z", "finished_at": "2026-10-31T00:28:59.916782349Z", "discovery_id": "7a97836e-8ba2-4f05-b9be-6c9306272d17", "daemon_version": "0.14.17", "discovery_type": {"type": "Unified", "host_id": "8d1f0138-018b-4734-9af4-5f196dc2a028", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:59.507117131Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8d1f0138-018b-4734-9af4-5f196dc2a028", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-11 23:58:36.269774+00	2026-10-31 00:28:59.918345+00	0	f	[]
5948f649-dd4d-4c9f-9fc7-92092bd2cc30	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "466f3f06-3335-4195-94cd-7b1b85eceda4", "started_at": "2026-02-01T15:39:35.137282092Z", "finished_at": "2026-10-31T00:28:49.708645975Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "SelfReport", "host_id": "d6e89254-a0d6-418b-9678-e75d11712fd1"}, "last_update_at": "2026-10-31T00:28:48.867914089Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "d6e89254-a0d6-418b-9678-e75d11712fd1"}	Self Report — My Site	2026-02-01 15:39:35.137282+00	2026-10-31 00:28:49.710775+00	0	f	[]
72967a1e-f338-4327-b5bd-e84dd9ef75a2	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "e7c3d9e2-4cad-4942-98d6-ae6053485366", "started_at": "2026-03-11T06:19:24.616705132Z", "finished_at": "2026-10-31T00:28:50.123149680Z", "discovery_id": null, "daemon_version": "0.19.0", "discovery_type": {"type": "SelfReport", "host_id": "8c99dc12-cae3-41f2-a550-be436a112a35"}, "last_update_at": "2026-10-31T00:28:49.780902558Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8c99dc12-cae3-41f2-a550-be436a112a35"}	Self Report — My Site	2026-03-11 06:19:24.616705+00	2026-10-31 00:28:50.125332+00	0	f	[]
0cf87622-9d78-44b8-b31c-a5bcd1e828d1	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "30c1790e-6f3b-44f7-87be-9491e456edd0", "started_at": "2026-07-29T04:18:51.683205577Z", "finished_at": "2026-10-31T00:28:50.128787046Z", "discovery_id": "52128e55-e8b1-4c11-8ef6-6bad0489c39b", "daemon_version": "0.19.0", "discovery_type": {"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:49.773782838Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-29 04:18:51.683205+00	2026-10-31 00:28:50.130413+00	0	f	[]
719b4fdf-a5fe-47d2-b66f-d7b299f33cbd	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "6740c1be-987b-48e8-b547-871b2ea811c7", "started_at": "2026-10-23T00:17:23.541441434Z", "finished_at": "2026-10-31T00:28:50.538674101Z", "discovery_id": "81c70238-8e38-46d3-a337-5a211ad5f3c9", "daemon_version": "0.14.2", "discovery_type": {"type": "Unified", "host_id": "9134f9b8-a2ca-4af9-9f9d-f98529385af4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:50.146733435Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9134f9b8-a2ca-4af9-9f9d-f98529385af4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-23 00:17:23.541441+00	2026-10-31 00:28:50.540375+00	0	f	[]
40963f22-932d-45c3-8984-751c287c21d9	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "f36e9f5f-c1e1-4f52-b2b1-2de3128ff92c", "started_at": "2026-08-14T17:13:59.174760436Z", "finished_at": "2026-10-31T00:28:51.334239145Z", "discovery_id": "fbe80e37-1c44-4335-84d1-5a5661c06e4d", "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:50.990729554Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-14 17:13:59.17476+00	2026-10-31 00:28:51.336003+00	0	f	[]
eb294a71-9a6e-411c-be22-cdffd712730d	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "26c0ac92-5d7b-41b7-969c-9e07d17bd1be", "started_at": "2026-10-15T03:38:40.733230767Z", "finished_at": "2026-10-31T00:28:49.711748469Z", "discovery_id": "0584cb5a-fbfc-4aba-bf5b-fb2b0fd15e78", "daemon_version": "0.17.7", "discovery_type": {"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:49.363625217Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:38:40.73323+00	2026-10-31 00:28:49.713455+00	0	f	[]
01c1b54b-a1bb-456d-bdb0-cbec55d309d1	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "3326154e-ed93-4721-9226-b365a900ba24", "started_at": "2026-08-14T17:10:29.182979151Z", "finished_at": "2026-10-31T00:28:51.339772156Z", "discovery_id": "9db5951d-434d-4a19-b5e3-483e94927593", "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:50.987032401Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-14 17:10:29.182979+00	2026-10-31 00:28:51.341348+00	0	f	[]
d1b525b2-17dc-4978-87ae-d3a025e0e168	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "f1a2c4ba-d124-4b5d-9448-729e2a28c186", "started_at": "2026-06-23T01:16:12.751429360Z", "finished_at": "2026-10-31T00:28:53.762724331Z", "discovery_id": "5c6a8a47-1ddf-4448-99d3-2272573556b3", "daemon_version": "0.19.3", "discovery_type": {"type": "Unified", "host_id": "27a3457e-4127-4ee5-96e3-ba617595159b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:53.269956700Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "27a3457e-4127-4ee5-96e3-ba617595159b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-23 01:16:12.751429+00	2026-10-31 00:28:53.764231+00	0	f	[]
3fbd04fa-4d63-4468-9e2e-467e10ec493f	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "b18c8b1b-202b-4081-9911-d4f395fe9344", "started_at": "2026-10-27T05:03:46.928849924Z", "finished_at": "2026-10-31T00:28:54.173665386Z", "discovery_id": "089e70e9-7f44-4c9e-8f3f-66b5826ca302", "daemon_version": "0.19.5", "discovery_type": {"type": "Unified", "host_id": "8913babd-75d5-4960-a597-9eb267cea952", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:53.772542957Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8913babd-75d5-4960-a597-9eb267cea952", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-27 05:03:46.928849+00	2026-10-31 00:28:54.175335+00	0	f	[]
0a17646d-5d2e-4ff9-a0a7-9d134e35bf05	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "ae2ef896-c9ee-4c7d-92d0-2e940d717f51", "started_at": "2026-10-15T03:35:10.733740412Z", "finished_at": "2026-10-31T00:28:49.717271956Z", "discovery_id": "ce4403ee-148f-4efd-9d5f-bcbf0659cd17", "daemon_version": "0.17.7", "discovery_type": {"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:49.360003175Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:35:10.73374+00	2026-10-31 00:28:49.71884+00	0	f	[]
766c1015-875e-418d-a98a-42f38ab19b46	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "b5073709-1dc4-4f69-bab1-d17406b1b571", "started_at": "2026-07-29T04:22:21.659954694Z", "finished_at": "2026-10-31T00:28:50.126227216Z", "discovery_id": "4ee985f9-d43f-4d02-8038-a1eca5931c59", "daemon_version": "0.19.0", "discovery_type": {"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:49.777396858Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-29 04:22:21.659954+00	2026-10-31 00:28:50.127792+00	0	f	[]
8e21901b-44c0-4c95-9dd2-98b52e7038c3	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "890873b9-2054-463a-9fe8-11016f0d08f2", "started_at": "2026-07-29T04:17:17.562092815Z", "finished_at": "2026-10-31T00:28:50.136291296Z", "discovery_id": "5e8dc302-130c-4965-aa7e-b58c5bf1c560", "daemon_version": "0.19.0", "discovery_type": {"type": "Unified", "host_id": "f4930d3b-6bc0-42c1-b25c-bbf557bf56ce", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:49.737718456Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f4930d3b-6bc0-42c1-b25c-bbf557bf56ce", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-29 04:17:17.562092+00	2026-10-31 00:28:50.137876+00	0	f	[]
83bee8aa-4033-4e7a-a1ba-3075a7e4d7da	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "ee9f5e06-197c-4fbf-9d68-0ac7d7ec929e", "started_at": "2026-06-28T16:07:05.924226987Z", "finished_at": "2026-10-31T00:28:52.708823021Z", "discovery_id": "560a0797-0c19-4301-b70b-e391d63674f5", "daemon_version": "0.17.2", "discovery_type": {"type": "Unified", "host_id": "8d22037e-273c-4a29-9c5c-cdaaefe74227", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:52.271740365Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8d22037e-273c-4a29-9c5c-cdaaefe74227", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:07:05.924226+00	2026-10-31 00:28:52.710458+00	0	f	[]
20c6550e-a77e-402d-b2f9-aaea818edabf	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "f0555e1a-a4e0-4d26-b3e1-2a1c581c337d", "started_at": "2026-02-09T15:41:37.848773295Z", "finished_at": "2026-10-31T00:28:51.342272568Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "SelfReport", "host_id": "9e1eeb2e-e918-4dea-a437-6a61a65f4497"}, "last_update_at": "2026-10-31T00:28:50.553904864Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "9e1eeb2e-e918-4dea-a437-6a61a65f4497"}	Self Report — My Site	2026-02-09 15:41:37.848773+00	2026-10-31 00:28:51.344285+00	0	f	[]
f2ea00af-dd6e-4214-95a0-40e0fd511530	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "7612c47a-db41-4c37-8d16-fbf4ace0e41e", "started_at": "2026-01-23T03:55:10.336667754Z", "finished_at": "2026-10-31T00:28:52.725115386Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:51.850438596Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-23 03:55:10.336667+00	2026-10-31 00:28:52.727401+00	0	f	[]
8a07ed41-a1a8-45e8-96fc-fadd7efb6228	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "8dab7a99-5955-498a-b597-5e7bb0e5d62a", "started_at": "2026-04-28T19:55:22.652516977Z", "finished_at": "2026-10-31T00:28:57.256422224Z", "discovery_id": "774494b8-85ff-468c-afb6-0d7b65d4c042", "daemon_version": "0.18.1", "discovery_type": {"type": "Unified", "host_id": "c16a9026-e029-4161-8963-20186c87d56c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:56.844845336Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c16a9026-e029-4161-8963-20186c87d56c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-28 19:55:22.652516+00	2026-10-31 00:28:57.258068+00	0	f	[]
ef6bc362-7954-46fe-ae67-1e0e0c395af2	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "e8b2155f-5622-4046-aff2-4f8de2f868f0", "started_at": "2026-02-23T01:47:07.653242727Z", "finished_at": "2026-10-31T00:28:58.559262287Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "SelfReport", "host_id": "49b8f936-67c8-4835-bc8c-5d897e43cdb0"}, "last_update_at": "2026-10-31T00:28:57.699246177Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "49b8f936-67c8-4835-bc8c-5d897e43cdb0"}	Self Report — My Site	2026-02-23 01:47:07.653242+00	2026-10-31 00:28:58.561373+00	0	f	[]
9f476afa-b7e0-49e0-911f-fa76f3e868c3	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "69f77b5e-1350-4d55-ae3e-a0aa03bb47cd", "started_at": "2026-08-25T02:50:04.633793664Z", "finished_at": "2026-10-31T00:28:58.564650601Z", "discovery_id": "bf7af2e6-cd9c-4f7d-896f-1087c2bd83c0", "daemon_version": "0.15.1", "discovery_type": {"type": "Unified", "host_id": "802b4d69-588c-452a-83c3-1e47ca5d84d1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:58.196141381Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "802b4d69-588c-452a-83c3-1e47ca5d84d1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 02:50:04.633793+00	2026-10-31 00:28:58.566279+00	0	f	[]
140f532d-eeec-4187-b083-edfd88b6d96a	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "00342129-a6d7-490e-84ba-2bd11b17a768", "started_at": "2026-02-26T00:44:02.257915046Z", "finished_at": "2026-10-31T00:29:10.796780392Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:10.004074296Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-26 00:44:02.257915+00	2026-10-31 00:29:10.799005+00	0	f	[]
911d7429-6140-41c1-9c46-3c5992290cfb	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "06f71522-f4a4-4d87-9785-a442e797d994", "started_at": "2026-06-28T16:02:26.238749468Z", "finished_at": "2026-10-31T00:28:52.714035894Z", "discovery_id": "2d4192ca-ef16-45c9-8563-a20d2608ff57", "daemon_version": "0.17.2", "discovery_type": {"type": "Unified", "host_id": "793047bd-2b38-4e62-8dd9-36ab1aa73797", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:52.206724381Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "793047bd-2b38-4e62-8dd9-36ab1aa73797", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:02:26.238749+00	2026-10-31 00:28:52.715553+00	0	f	[]
f70482f9-7e36-47f6-b437-aca59fdd9e2b	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "e5274c5c-4478-40d5-86c6-aabbe025f093", "started_at": "2026-02-02T19:01:57.807520644Z", "finished_at": "2026-10-31T00:28:52.728332715Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "SelfReport", "host_id": "37e5888d-4a4e-4ca3-908c-a8939cc75bdd"}, "last_update_at": "2026-10-31T00:28:52.245646133Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "37e5888d-4a4e-4ca3-908c-a8939cc75bdd"}	Self Report — My Site	2026-02-02 19:01:57.80752+00	2026-10-31 00:28:52.730439+00	0	f	[]
2b7d75ef-f002-4178-be02-caf7429390e5	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "8473fe97-6e86-4b62-9b6d-2cd1349deeb8", "started_at": "2026-07-07T22:29:34.241374934Z", "finished_at": "2026-10-31T00:28:55.487712773Z", "discovery_id": "d0c14753-4347-4a57-825d-f81e0bda8af0", "daemon_version": "0.17.20", "discovery_type": {"type": "Unified", "host_id": "bb32f4ef-dc8e-4c79-89c6-04915971d7b0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:55.018091903Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bb32f4ef-dc8e-4c79-89c6-04915971d7b0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-07 22:29:34.241374+00	2026-10-31 00:28:55.489541+00	0	f	[]
55992c67-4895-4c5f-b56d-122097a1581b	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "20717e93-7bdb-4413-9112-9ba79c38289f", "started_at": "2026-10-03T03:59:01.064308022Z", "finished_at": "2026-10-31T00:28:55.914182264Z", "discovery_id": "e67c54a7-f162-4432-a21a-8c953a5435b5", "daemon_version": "0.14.6", "discovery_type": {"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:55.576357084Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-03 03:59:01.064308+00	2026-10-31 00:28:55.915856+00	0	f	[]
e85d30ac-b971-40de-b9ba-9f6a667c4879	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "d53d5b11-b914-4af1-bf04-5d6e47899de1", "started_at": "2026-01-23T03:40:39.007214962Z", "finished_at": "2026-10-31T00:28:57.241722477Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:56.391616121Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-23 03:40:39.007214+00	2026-10-31 00:28:57.244219+00	0	f	[]
25718855-fff4-4d95-871e-3e7c544da1fc	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "b049b806-80fc-4dc5-9400-f19f9a9857ff", "started_at": "2026-06-28T16:07:10.003081910Z", "finished_at": "2026-10-31T00:28:52.719597242Z", "discovery_id": "0f49ac2a-3bcc-4880-8623-073c6e609c41", "daemon_version": "0.17.2", "discovery_type": {"type": "Unified", "host_id": "37d3a1d8-b76b-4e2e-96e0-74362358f4f4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:52.320506511Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "37d3a1d8-b76b-4e2e-96e0-74362358f4f4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:07:10.003081+00	2026-10-31 00:28:52.721234+00	0	f	[]
baa1c96f-e743-4e93-8208-7e8667d2185e	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "7c20860a-b07c-4c4f-81bc-c7f44bee7a3d", "started_at": "2026-02-02T14:29:34.148055772Z", "finished_at": "2026-10-31T00:28:52.733683671Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:51.437351038Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-02 14:29:34.148055+00	2026-10-31 00:28:52.735739+00	0	f	[]
54cdebb7-efcf-42e7-89c5-10fd44180955	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "a8a4a82b-01b9-4d9b-9bf2-cc44f54ff1d3", "started_at": "2026-10-03T03:54:08.695159586Z", "finished_at": "2026-10-31T00:28:55.911662302Z", "discovery_id": "ab72799c-9b11-4aa4-8614-f53571b761d7", "daemon_version": "0.14.6", "discovery_type": {"type": "Unified", "host_id": "0e0131ba-ba9b-4828-813f-4a8c181f5836", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:55.506212582Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "0e0131ba-ba9b-4828-813f-4a8c181f5836", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-03 03:54:08.695159+00	2026-10-31 00:28:55.913213+00	0	f	[]
f47a27e0-fd0d-4380-804f-17cd859d97cd	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "485c7d46-bc1d-4c32-a425-5904751efe34", "started_at": "2026-10-03T03:55:31.060040280Z", "finished_at": "2026-10-31T00:28:55.916792043Z", "discovery_id": "158b85c7-d2e0-45d3-8b55-6b2d8831a9d1", "daemon_version": "0.14.6", "discovery_type": {"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:55.572053232Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-03 03:55:31.06004+00	2026-10-31 00:28:55.918423+00	0	f	[]
3463dbe7-97c0-4f59-b3ff-75fc5191e99d	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "c17fbe8e-8128-45b1-b2b5-afef7e2c082f", "started_at": "2026-02-23T01:47:37.649663614Z", "finished_at": "2026-10-31T00:28:58.553143301Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:57.741687692Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-23 01:47:37.649663+00	2026-10-31 00:28:58.555536+00	0	f	[]
4b35bccd-9080-496f-becd-2ff9df295913	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "bd93b206-4e22-4e0a-9dd0-0f38664a4247", "started_at": "2026-02-02T14:21:21.225991527Z", "finished_at": "2026-10-31T00:28:52.722138629Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:51.401624466Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-02 14:21:21.225991+00	2026-10-31 00:28:52.724264+00	0	f	[]
ada97b9f-cdf7-4a25-b6c5-d68dc6255687	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "eb250b58-76a9-4335-a813-c2d71a75e483", "started_at": "2026-02-18T17:07:18.061512748Z", "finished_at": "2026-10-31T00:28:57.258943818Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "SelfReport", "host_id": "25a3d274-a408-4ec8-b1fd-69eb88c7674d"}, "last_update_at": "2026-10-31T00:28:55.979297825Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "25a3d274-a408-4ec8-b1fd-69eb88c7674d"}	Self Report — My Site	2026-02-18 17:07:18.061512+00	2026-10-31 00:28:57.261063+00	0	f	[]
eb349198-93cc-434d-bb69-abd50290addc	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "69664faf-d5dd-440a-b554-0af052ce9005", "started_at": "2026-08-01T21:20:27.123050289Z", "finished_at": "2026-10-31T00:28:59.491939081Z", "discovery_id": "7d7c758c-41b2-4b75-9267-55061499cc08", "daemon_version": "0.18.0", "discovery_type": {"type": "Unified", "host_id": "b6b48830-db6a-4d8f-8c70-db5516f3e458", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:59.076503614Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b6b48830-db6a-4d8f-8c70-db5516f3e458", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-01 21:20:27.12305+00	2026-10-31 00:28:59.493485+00	0	f	[]
c1a502a7-d22e-4b7d-a186-38e541940c6d	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "b3287603-7c63-47ac-886a-4b6bb9cb2151", "started_at": "2026-08-01T21:25:32.482042816Z", "finished_at": "2026-10-31T00:28:59.497007782Z", "discovery_id": "b64bf272-1e98-46ab-be05-da2e56c0f97e", "daemon_version": "0.18.0", "discovery_type": {"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:59.124826304Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-01 21:25:32.482042+00	2026-10-31 00:28:59.498714+00	0	f	[]
4c0d8389-63cd-43c5-b9d0-608bd3d29d65	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "b3439cfa-af27-47ae-8b9d-8c79b9d149fa", "started_at": "2026-07-28T03:21:55.956404860Z", "finished_at": "2026-10-31T00:29:01.675424492Z", "discovery_id": "97f0ab3b-c0a5-4322-8b71-bd9638256344", "daemon_version": "0.14.13", "discovery_type": {"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:01.321421848Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-28 03:21:55.956404+00	2026-10-31 00:29:01.676997+00	0	f	[]
38f29558-4aca-41b4-abfb-bfd36691d850	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "fe678d3a-c2b0-4ee1-a912-7c07b0b6e1d7", "started_at": "2026-06-28T16:04:16.808363474Z", "finished_at": "2026-10-31T00:28:52.731270329Z", "discovery_id": "0d1e4e0c-1af6-4603-84af-69f5a81e19a3", "daemon_version": "0.17.2", "discovery_type": {"type": "Unified", "host_id": "28b6b18e-69b9-450c-a795-d14a671bb34f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:52.241934268Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "28b6b18e-69b9-450c-a795-d14a671bb34f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:04:16.808363+00	2026-10-31 00:28:52.732796+00	0	f	[]
54b18cbc-0fb2-49fd-b033-f99369ace3fa	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "bb0bdfdc-1516-4579-a727-079afeb571ea", "started_at": "2026-07-01T17:46:57.844855087Z", "finished_at": "2026-10-31T00:28:53.252081347Z", "discovery_id": "3b468078-99d7-49e0-ad1a-a1d48197652c", "daemon_version": "0.17.0", "discovery_type": {"type": "Unified", "host_id": "23bba0c3-a3f9-49d7-ac3c-8696037b8b40", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:52.796402479Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "23bba0c3-a3f9-49d7-ac3c-8696037b8b40", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:46:57.844855+00	2026-10-31 00:28:53.253888+00	0	f	[]
bf6cb06c-60bf-457c-a199-3429c4d709aa	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "580379b9-0101-428f-baef-843afbc5dfee", "started_at": "2026-01-26T14:03:54.326189222Z", "finished_at": "2026-10-31T00:28:55.490495985Z", "discovery_id": null, "daemon_version": "0.17.20", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:54.632944724Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-26 14:03:54.326189+00	2026-10-31 00:28:55.492603+00	0	f	[]
0b46a0e6-6434-4668-8b64-2d468b38b0c3	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "feba9c5a-f04c-4e5c-97cd-994c4dffc382", "started_at": "2026-07-07T22:32:35.373947919Z", "finished_at": "2026-10-31T00:28:55.495976460Z", "discovery_id": "5a60baca-2c89-4e99-be76-2f612ad32b4d", "daemon_version": "0.17.20", "discovery_type": {"type": "Unified", "host_id": "f1a03331-5149-4a21-91ed-42d9fd821651", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:55.115758988Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f1a03331-5149-4a21-91ed-42d9fd821651", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-07 22:32:35.373947+00	2026-10-31 00:28:55.497473+00	0	f	[]
2a4e1f61-2640-44d8-9b4e-f2740e7cef59	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "d10e30a1-1b5e-4a25-abe9-566d427e3186", "started_at": "2026-03-24T03:12:35.661709995Z", "finished_at": "2026-10-31T00:28:59.060873905Z", "discovery_id": null, "daemon_version": "0.17.8", "discovery_type": {"type": "Unified", "host_id": "80008868-cd5c-4f1e-92ea-0bd73728c9a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:58.581700588Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "80008868-cd5c-4f1e-92ea-0bd73728c9a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-24 03:12:35.661709+00	2026-10-31 00:28:59.062041+00	0	f	[]
c00ee7ce-07a8-4025-993b-1cf27ec6233f	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "5b2fda95-677f-4f93-a93f-daca41bf3af3", "started_at": "2026-07-01T17:45:06.329649066Z", "finished_at": "2026-10-31T00:28:53.254860656Z", "discovery_id": "0520da35-1935-4895-85a1-0948ac2871b8", "daemon_version": "0.17.0", "discovery_type": {"type": "Unified", "host_id": "e56072d7-8284-4090-9a48-d04892a27b80", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:52.750781744Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e56072d7-8284-4090-9a48-d04892a27b80", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:45:06.329649+00	2026-10-31 00:28:53.256598+00	0	f	[]
24e0fc00-c31a-44bf-aa7f-8f4b2cd86eb8	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "bfc425bd-2840-4447-8d67-666671033cd6", "started_at": "2026-06-23T01:20:59.927972592Z", "finished_at": "2026-10-31T00:28:53.759899394Z", "discovery_id": "04cada81-391f-4199-b289-b734a5122804", "daemon_version": "0.19.3", "discovery_type": {"type": "Unified", "host_id": "cf70c05a-1647-4958-806b-ff84119deb9a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:53.425533365Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "cf70c05a-1647-4958-806b-ff84119deb9a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-23 01:20:59.927972+00	2026-10-31 00:28:53.761731+00	0	f	[]
b237e827-4ca5-4d7d-9376-2631cddaf1da	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "c1eaa2e7-7631-418b-8d5f-74597c08a4f5", "started_at": "2026-10-31T00:21:22.175714347Z", "finished_at": "2026-10-31T00:28:54.578612037Z", "discovery_id": "5dec0067-efe2-452c-b79d-58274d8be83d", "daemon_version": "0.13.6", "discovery_type": {"type": "Unified", "host_id": "a20cea9a-e475-4818-80d1-32a346c0085e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:54.183715170Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a20cea9a-e475-4818-80d1-32a346c0085e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-31 00:21:22.175714+00	2026-10-31 00:28:54.580434+00	0	f	[]
8ef63136-4965-4396-8500-af634e4660bf	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "7ad7978f-e4a7-4518-bb77-8a4c8c1dd0c6", "started_at": "2026-07-07T22:27:42.015566135Z", "finished_at": "2026-10-31T00:28:55.493405813Z", "discovery_id": "2c815633-ee92-462f-975a-61b72bb55776", "daemon_version": "0.17.20", "discovery_type": {"type": "Unified", "host_id": "079502fe-1a72-44a6-ab38-040ead1355fb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:54.980421629Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "079502fe-1a72-44a6-ab38-040ead1355fb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-07 22:27:42.015566+00	2026-10-31 00:28:55.495098+00	0	f	[]
5f6cf611-7ae8-40b2-a565-b5435b776e5a	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "cd16f43c-4365-49c5-8fee-d081f3b53cd9", "started_at": "2026-02-23T00:07:19.655637880Z", "finished_at": "2026-10-31T00:28:57.253485385Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "SelfReport", "host_id": "abd72866-b3c5-4e9a-89f6-491d6af2797b"}, "last_update_at": "2026-10-31T00:28:55.931857386Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "abd72866-b3c5-4e9a-89f6-491d6af2797b"}	Self Report — My Site	2026-02-23 00:07:19.655637+00	2026-10-31 00:28:57.255617+00	0	f	[]
68f203a8-d190-47f7-b79f-18c1f8b51a2e	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "47b82122-5cf6-42a8-bf36-f15c9a6582f0", "started_at": "2026-04-28T19:50:40.314334020Z", "finished_at": "2026-10-31T00:28:57.248244975Z", "discovery_id": "fde2668b-80d7-4f3a-9ab5-dd997009e654", "daemon_version": "0.18.1", "discovery_type": {"type": "Unified", "host_id": "689f2fa8-dc17-4321-b680-7af464e5004f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:56.748516386Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "689f2fa8-dc17-4321-b680-7af464e5004f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-28 19:50:40.314334+00	2026-10-31 00:28:57.249912+00	0	f	[]
88ed6f14-a1d5-4b92-b47b-15bc42e5e322	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "d03e3468-38c8-46e6-bd30-43693e9db1d7", "started_at": "2026-03-09T05:16:11.595722227Z", "finished_at": "2026-10-31T00:29:00.790844318Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "SelfReport", "host_id": "3ef5ad9b-872d-4fc7-8f8f-e5c0186d1841"}, "last_update_at": "2026-10-31T00:28:59.932749337Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "3ef5ad9b-872d-4fc7-8f8f-e5c0186d1841"}	Self Report — My Site	2026-03-09 05:16:11.595722+00	2026-10-31 00:29:00.793054+00	0	f	[]
3d0f7081-c7d3-4bba-895f-ae8d460f7ca5	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "110d0739-6875-498e-9ed5-5c7c0dec84ca", "started_at": "2026-03-05T07:38:20.117782332Z", "finished_at": "2026-10-31T00:29:04.839746652Z", "discovery_id": null, "daemon_version": "0.15.3", "discovery_type": {"type": "SelfReport", "host_id": "9b3486eb-3e41-4b56-8062-c98ad29f0a3a"}, "last_update_at": "2026-10-31T00:29:03.997185886Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "9b3486eb-3e41-4b56-8062-c98ad29f0a3a"}	Self Report — My Site	2026-03-05 07:38:20.117782+00	2026-10-31 00:29:04.841709+00	0	f	[]
bcffae9a-8685-4b6c-a5c9-712610ab86cf	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "901cf5c8-1989-4bf1-bf5d-e587ee211cda", "started_at": "2026-03-26T17:23:47.082705682Z", "finished_at": "2026-10-31T00:29:05.703604509Z", "discovery_id": null, "daemon_version": "0.14.5", "discovery_type": {"type": "Unified", "host_id": "2bbce8eb-5555-41ef-b999-68a5af971278", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:04.859194345Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2bbce8eb-5555-41ef-b999-68a5af971278", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-26 17:23:47.082705+00	2026-10-31 00:29:05.70488+00	0	f	[]
35acdc9a-ba31-4199-89e8-6e3bb96745f1	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "56d2df9f-197b-49f8-b4ac-e9d4674dca7a", "started_at": "2026-10-07T13:08:30.855227146Z", "finished_at": "2026-10-31T00:29:06.972778050Z", "discovery_id": "4b0e74ec-cf90-4dc6-83e5-83a5ae245fac", "daemon_version": "0.14.4", "discovery_type": {"type": "Unified", "host_id": "1be4e31e-a229-440d-b0f4-20c31ee11254", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:06.551451564Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "1be4e31e-a229-440d-b0f4-20c31ee11254", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-07 13:08:30.855227+00	2026-10-31 00:29:06.974291+00	0	f	[]
133b8883-357f-4294-b5c0-01e45f20dd15	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "1a2a6d39-262b-45ff-80c8-1f8742f766af", "started_at": "2026-02-28T03:10:51.549945495Z", "finished_at": "2026-10-31T00:29:13.372015996Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "SelfReport", "host_id": "422f12c5-aed6-4cdc-aab7-a281212ef855"}, "last_update_at": "2026-10-31T00:29:12.554875272Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "422f12c5-aed6-4cdc-aab7-a281212ef855"}	Self Report — My Site	2026-02-28 03:10:51.549945+00	2026-10-31 00:29:13.374095+00	0	f	[]
f4137a77-d115-44ff-ad77-c604512dc615	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "c30ae4d6-aa01-4253-94db-acdabf3a54e4", "started_at": "2026-04-28T19:09:10.681879638Z", "finished_at": "2026-10-31T00:28:57.250942520Z", "discovery_id": "ff17792b-b48a-406e-b52d-9ca3815a3f85", "daemon_version": "0.18.1", "discovery_type": {"type": "Unified", "host_id": "dd6f90d2-d1df-49c8-bba4-cf36f7343201", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:56.787467633Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "dd6f90d2-d1df-49c8-bba4-cf36f7343201", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-28 19:09:10.681879+00	2026-10-31 00:28:57.252634+00	0	f	[]
30d60d1d-7d86-4e74-8d76-ce8d5a1b7308	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "5d98fc6a-1bba-4985-a654-bf83407df1f2", "started_at": "2026-10-14T05:48:09.745795958Z", "finished_at": "2026-10-31T00:28:57.677364891Z", "discovery_id": "907b065c-8d6e-4eef-9d32-7b70dbefb4e2", "daemon_version": "0.14.7", "discovery_type": {"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:57.340096566Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-14 05:48:09.745795+00	2026-10-31 00:28:57.679018+00	0	f	[]
bada1be0-9552-4dc0-8802-3c1d983a5d5a	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "833d88c3-2acf-4989-86de-466d44e47a7e", "started_at": "2026-10-14T05:44:39.739395480Z", "finished_at": "2026-10-31T00:28:57.682727431Z", "discovery_id": "f9b6f8ad-c5fa-491c-8c0b-192f08aefcd8", "daemon_version": "0.14.7", "discovery_type": {"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:57.336091746Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-14 05:44:39.739395+00	2026-10-31 00:28:57.684353+00	0	f	[]
9c84d484-abe9-4850-aab4-a135d527f464	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "faf2ae16-9ccd-4ba2-8be6-077b30ff3ae7", "started_at": "2026-08-26T01:59:44.730528896Z", "finished_at": "2026-10-31T00:28:58.562197008Z", "discovery_id": "b528d9a6-e5e5-4522-b18c-684cef39008c", "daemon_version": "0.15.1", "discovery_type": {"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:58.192265281Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-26 01:59:44.730528+00	2026-10-31 00:28:58.563774+00	0	f	[]
8268cf54-94c9-4828-804e-7269a517ed06	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "d7b0c883-f1dd-45b9-8008-cb06d0194514", "started_at": "2026-10-14T05:43:36.061177700Z", "finished_at": "2026-10-31T00:28:57.679976057Z", "discovery_id": "6cc84f8e-f0f6-45cf-ab85-0b74fb883182", "daemon_version": "0.14.7", "discovery_type": {"type": "Unified", "host_id": "d4dabb79-eb99-4045-8687-9ca8bd1e797a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:57.270173456Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d4dabb79-eb99-4045-8687-9ca8bd1e797a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-14 05:43:36.061177+00	2026-10-31 00:28:57.681763+00	0	f	[]
94feb378-a2ad-4329-9e16-08d21e44cb16	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "e0ea6622-5f78-4cc3-b1f9-257946754dd8", "started_at": "2026-03-09T05:29:17.642983004Z", "finished_at": "2026-03-09T05:29:17.643045281Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:00.040769379Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-09 05:29:17.642983+00	2026-10-31 00:29:00.042862+00	0	f	[]
490038ba-11fd-411c-bbc9-8900863fe18b	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "ed48cfed-4a98-4473-9c83-33781243c790", "started_at": "2026-03-09T05:16:41.592520993Z", "finished_at": "2026-10-31T00:29:00.796974055Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:59.975485549Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-09 05:16:41.59252+00	2026-10-31 00:29:00.79928+00	0	f	[]
1914ba12-e49c-4065-b885-04e47497fc53	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "f071713f-3367-4111-8f7b-7b7b50d33aa9", "started_at": "2026-03-23T23:57:20.088532334Z", "finished_at": "2026-10-31T00:29:03.532118835Z", "discovery_id": null, "daemon_version": "0.17.16", "discovery_type": {"type": "Unified", "host_id": "14fcf158-caea-47c9-82ad-38dcc2907791", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:03.052404911Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "14fcf158-caea-47c9-82ad-38dcc2907791", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-23 23:57:20.088532+00	2026-10-31 00:29:03.533334+00	0	f	[]
84be89b3-1aee-4e4a-a437-5ab63368e6df	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "ae3a3d90-03a9-4961-a1d4-840e661d29a8", "started_at": "2026-02-11T02:46:52.998653452Z", "finished_at": "2026-10-31T00:29:06.961641392Z", "discovery_id": null, "daemon_version": "0.14.4", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:05.761944653Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-11 02:46:52.998653+00	2026-10-31 00:29:06.963849+00	0	f	[]
7a6b1d7b-871e-49d5-b6c3-eb6a723a5314	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "3bba0463-9815-4685-8b2e-93eacc318f67", "started_at": "2026-08-26T01:56:14.716285279Z", "finished_at": "2026-10-31T00:28:58.556581322Z", "discovery_id": "70c11f19-a54c-40e5-8f80-44fda404d0d5", "daemon_version": "0.15.1", "discovery_type": {"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:58.188328277Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-26 01:56:14.716285+00	2026-10-31 00:28:58.558202+00	0	f	[]
3e08cec3-a769-474b-ab46-1dbe01f705db	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "a1b350a1-59b8-4caf-ae55-c9f3f8ca89fb", "started_at": "2026-03-05T09:32:41.117544068Z", "finished_at": "2026-10-31T00:29:02.609472977Z", "discovery_id": null, "daemon_version": "0.17.5", "discovery_type": {"type": "SelfReport", "host_id": "2a76c091-9847-4916-bc73-8668758f6f7d"}, "last_update_at": "2026-10-31T00:29:01.693319571Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "2a76c091-9847-4916-bc73-8668758f6f7d"}	Self Report — My Site	2026-03-05 09:32:41.117544+00	2026-10-31 00:29:02.611632+00	0	f	[]
b392c1b2-d963-4569-a552-5e3ac4b7b8da	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "457a544b-3c09-45bf-90d1-ab9e5e4a144d", "started_at": "2026-04-01T14:32:54.762125416Z", "finished_at": "2026-10-31T00:29:02.615684525Z", "discovery_id": null, "daemon_version": "0.17.5", "discovery_type": {"type": "Unified", "host_id": "a33f8415-bb39-43da-81c3-dfafa82595da", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:02.111448230Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a33f8415-bb39-43da-81c3-dfafa82595da", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-01 14:32:54.762125+00	2026-10-31 00:29:02.616856+00	0	f	[]
dbea56e4-f3ed-4ea4-a2fb-94ff77396c65	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "6808c839-ea5d-4f6e-9662-60f24a6dd0c7", "started_at": "2026-03-06T07:04:08.169432273Z", "finished_at": "2026-10-31T00:29:09.068221554Z", "discovery_id": null, "daemon_version": "0.14.16", "discovery_type": {"type": "SelfReport", "host_id": "07ca9676-4995-4fd4-b0a3-c60b0b356658"}, "last_update_at": "2026-10-31T00:29:08.239291118Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "07ca9676-4995-4fd4-b0a3-c60b0b356658"}	Self Report — My Site	2026-03-06 07:04:08.169432+00	2026-10-31 00:29:09.071006+00	0	f	[]
b6eac078-3732-4630-836a-fa10d5a62f6c	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "a49f4dc9-0185-4793-9f4f-0843c7021f81", "started_at": "2026-03-08T06:54:18.293920451Z", "finished_at": "2026-10-31T00:29:09.943010772Z", "discovery_id": null, "daemon_version": "0.14.8", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:09.135256828Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-08 06:54:18.29392+00	2026-10-31 00:29:09.945085+00	0	f	[]
b79052c8-e3a1-4f9c-b7a6-c39c99c49b5c	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "6bbb8f0d-ed69-4df8-81d6-8bc44cdf4380", "started_at": "2026-03-02T09:02:56.919081491Z", "finished_at": "2026-10-31T00:29:13.390457299Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:12.188414588Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-02 09:02:56.919081+00	2026-10-31 00:29:13.392544+00	0	f	[]
f5cd08ff-ccfb-4a8d-8f5a-41fc8a17b3f9	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "7e06b95e-3b67-41c4-9049-978411591e7c", "started_at": "2026-08-01T21:22:02.500665824Z", "finished_at": "2026-10-31T00:28:59.494443354Z", "discovery_id": "6b5576be-8ea3-48b5-8582-dfb31505d18f", "daemon_version": "0.18.0", "discovery_type": {"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:59.121028315Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-01 21:22:02.500665+00	2026-10-31 00:28:59.496043+00	0	f	[]
fac5a0fe-c5d1-4278-86e4-6c7d78cf82b8	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "37559eea-aa20-4b8c-ba39-ee899c2fc556", "started_at": "2026-01-05T04:59:56.694889798Z", "finished_at": "2026-10-31T00:29:00.793965700Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:00.449248255Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-05 04:59:56.694889+00	2026-10-31 00:29:00.796085+00	0	f	[]
ffcaffad-609d-40ae-b6fe-120f3eead845	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "80837ade-3bd8-4bfa-a94b-ec249e6517fa", "started_at": "2026-03-11T06:19:54.610393414Z", "finished_at": "2026-10-31T00:29:01.269823533Z", "discovery_id": null, "daemon_version": "0.17.6", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:00.852655716Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:19:54.610393+00	2026-10-31 00:29:01.271986+00	0	f	[]
d0ad3d86-8515-4be4-b762-88c98ebe8399	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "9c3d4777-a8a7-40f4-9070-c83a719cd6b6", "started_at": "2026-07-23T13:33:48.918734857Z", "finished_at": "2026-10-31T00:29:03.030506371Z", "discovery_id": "cc1c8c93-7562-48e4-a3e7-38135ee8375f", "daemon_version": "0.15.0", "discovery_type": {"type": "Unified", "host_id": "95fbda32-d081-4ef7-86c5-ac175f9995b6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:02.670648519Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "95fbda32-d081-4ef7-86c5-ac175f9995b6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-23 13:33:48.918734+00	2026-10-31 00:29:03.03227+00	0	f	[]
409bdfa4-3b96-49be-a183-4731e9238285	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "b43a3d5a-af6a-42f8-bf72-70c87206ab39", "started_at": "2026-07-22T23:04:06.688284640Z", "finished_at": "2026-10-31T00:29:03.035780269Z", "discovery_id": "8df236f9-b6c9-47ce-a2d9-2bef3b2327eb", "daemon_version": "0.15.0", "discovery_type": {"type": "Unified", "host_id": "ea19a9cd-5a65-4620-bff8-eb890602e187", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:02.674594036Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "ea19a9cd-5a65-4620-bff8-eb890602e187", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-22 23:04:06.688284+00	2026-10-31 00:29:03.037363+00	0	f	[]
52479d4b-7203-4f82-9858-6e3874f8aca9	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "2453f28d-e87c-4753-80ad-0416d47c71f8", "started_at": "2026-10-11T23:59:48.356682576Z", "finished_at": "2026-10-31T00:28:59.914200866Z", "discovery_id": "96e0b28e-db5b-4b0b-a43e-e6605bbbf850", "daemon_version": "0.14.17", "discovery_type": {"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:28:59.573134928Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-11 23:59:48.356682+00	2026-10-31 00:28:59.915937+00	0	f	[]
e345abed-806b-44da-9868-c75ae8c28d6c	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "f9ec8267-d3bb-4170-80d5-9c5a41add7eb", "started_at": "2026-07-28T03:20:23.724249500Z", "finished_at": "2026-10-31T00:29:01.672480708Z", "discovery_id": "366c3e64-fce7-48ec-baec-00dd8e73ab06", "daemon_version": "0.14.13", "discovery_type": {"type": "Unified", "host_id": "24f17e21-d958-4c17-ad9d-3f058f1bfd07", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:01.285801042Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "24f17e21-d958-4c17-ad9d-3f058f1bfd07", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-28 03:20:23.724249+00	2026-10-31 00:29:01.674328+00	0	f	[]
9a184022-04f1-4b6f-9cf5-2e0e5bfd0cf8	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "df28970e-d903-4f00-8b2a-1dfa823e410c", "started_at": "2026-07-28T03:25:25.938837651Z", "finished_at": "2026-10-31T00:29:01.677869713Z", "discovery_id": "9aee182d-fbe5-4846-a5aa-58516112347f", "daemon_version": "0.14.13", "discovery_type": {"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:01.325069920Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-28 03:25:25.938837+00	2026-10-31 00:29:01.679453+00	0	f	[]
f0fc1370-b8ac-4e19-9b0c-5947621d0946	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "72d38ac6-a392-41f5-96f5-17c22a19e8f5", "started_at": "2026-03-06T23:21:08.018383672Z", "finished_at": "2026-10-31T00:29:06.978102743Z", "discovery_id": null, "daemon_version": "0.14.4", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:06.175352086Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-06 23:21:08.018383+00	2026-10-31 00:29:06.980149+00	0	f	[]
f4e3e0fe-5d10-4a6c-bbb4-93acb0fad5ea	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "14a48b0e-4297-410e-b019-b1d98130f2c0", "started_at": "2026-02-10T04:10:50.083304178Z", "finished_at": "2026-10-31T00:29:07.807996309Z", "discovery_id": null, "daemon_version": "0.14.10", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:07.035611493Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-10 04:10:50.083304+00	2026-10-31 00:29:07.810159+00	0	f	[]
c8ce23d3-57b9-417a-8bb2-e86b47228ce2	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "44932e97-76ed-4c08-b694-4e6aa6bca1be", "started_at": "2026-03-05T09:33:11.114723268Z", "finished_at": "2026-10-31T00:29:02.612500941Z", "discovery_id": null, "daemon_version": "0.17.5", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:01.734869631Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-05 09:33:11.114723+00	2026-10-31 00:29:02.614734+00	0	f	[]
e5e64935-8c53-4c30-bd5f-c7ad84b942ec	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "68690167-4bf9-40db-9756-207731c78629", "started_at": "2026-09-16T19:13:38.792500966Z", "finished_at": "2026-10-31T00:29:03.969644312Z", "discovery_id": "7f626b34-e40a-452e-ab45-0c67c018097f", "daemon_version": "0.14.12", "discovery_type": {"type": "Unified", "host_id": "fe9f2b88-655a-44e9-b2b2-b89b797b34ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:03.622592324Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "fe9f2b88-655a-44e9-b2b2-b89b797b34ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 19:13:38.7925+00	2026-10-31 00:29:03.971523+00	0	f	[]
059b287b-36ca-47c2-9cda-a0e41f528b0e	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "24dd8f5a-4866-4462-a9ff-b65cb5a3061a", "started_at": "2026-09-16T20:16:31.390119148Z", "finished_at": "2026-10-31T00:29:03.975449964Z", "discovery_id": "50da0b65-d06d-497b-bcf7-5ee0d24c109e", "daemon_version": "0.14.12", "discovery_type": {"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:03.618931439Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 20:16:31.390119+00	2026-10-31 00:29:03.977106+00	0	f	[]
2a034b21-3414-447e-bc2f-ce079276ee82	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "93db1812-9fa5-4c4b-a36b-d687b363811c", "started_at": "2026-10-02T13:50:58.545317669Z", "finished_at": "2026-10-31T00:29:04.831311194Z", "discovery_id": "d0628a86-7aca-4ea5-a477-596bd69a6ebb", "daemon_version": "0.15.3", "discovery_type": {"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:04.485819591Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-02 13:50:58.545317+00	2026-10-31 00:29:04.833035+00	0	f	[]
9629948a-cef5-4597-a6af-a2394904b1a6	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "e39a2b83-3f31-4943-abbc-ba31d154ce16", "started_at": "2026-02-11T02:46:23.000125066Z", "finished_at": "2026-10-31T00:29:06.969838177Z", "discovery_id": null, "daemon_version": "0.14.4", "discovery_type": {"type": "SelfReport", "host_id": "d4234791-d4ee-4257-a9d3-a257b5e1e3b9"}, "last_update_at": "2026-10-31T00:29:05.719772056Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "d4234791-d4ee-4257-a9d3-a257b5e1e3b9"}	Self Report — My Site	2026-02-11 02:46:23.000125+00	2026-10-31 00:29:06.971991+00	0	f	[]
4b0fbb64-fb12-4995-9581-35384fb6a04b	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "d033c5f0-e137-4b0a-b42e-78daa430f113", "started_at": "2026-07-23T13:32:18.163549314Z", "finished_at": "2026-10-31T00:29:03.033262436Z", "discovery_id": "05d975db-052c-447d-85bc-3b17f3a680be", "daemon_version": "0.15.0", "discovery_type": {"type": "Unified", "host_id": "c37e16d2-78cb-4ad7-bffc-f8eaec8ac967", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:02.632399862Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c37e16d2-78cb-4ad7-bffc-f8eaec8ac967", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-23 13:32:18.163549+00	2026-10-31 00:29:03.034877+00	0	f	[]
07b5a090-cc0d-4560-8f1a-d563290dfe7b	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "947c4ec5-0754-4116-8282-283b6a09f803", "started_at": "2026-03-05T07:38:50.115878953Z", "finished_at": "2026-10-31T00:29:04.836628602Z", "discovery_id": null, "daemon_version": "0.15.3", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:04.047993252Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-05 07:38:50.115878+00	2026-10-31 00:29:04.838852+00	0	f	[]
f0f7b5c3-4033-436a-b45c-eb3d1efbc250	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "e903943a-3b77-43c1-a9ad-824cb543557b", "started_at": "2026-10-07T13:09:38.731246701Z", "finished_at": "2026-10-31T00:29:06.964811866Z", "discovery_id": "8d6d07df-d672-4717-95cd-c4507ba3c66d", "daemon_version": "0.14.4", "discovery_type": {"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:06.616504043Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-07 13:09:38.731246+00	2026-10-31 00:29:06.966392+00	0	f	[]
5cee57ff-ed16-4e01-88af-789bab66ad53	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "c063e961-df38-4fbb-b437-0e6e25593003", "started_at": "2026-09-24T21:00:16.922463583Z", "finished_at": "2026-10-31T00:29:07.811051583Z", "discovery_id": "a8e6d5fc-76c9-4ca2-b411-635c3d2f5d07", "daemon_version": "0.14.10", "discovery_type": {"type": "Unified", "host_id": "ad3c2bd6-9bf9-4c4d-8534-3689c0e013c3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:07.400250244Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "ad3c2bd6-9bf9-4c4d-8534-3689c0e013c3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-24 21:00:16.922463+00	2026-10-31 00:29:07.812735+00	0	f	[]
8bc39e2e-72fa-448b-9af1-0571645797e1	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "d36fca8d-dbbc-4cad-a19f-f495eacf6234", "started_at": "2026-02-28T21:19:27.224801293Z", "finished_at": "2026-10-31T00:29:09.060436348Z", "discovery_id": null, "daemon_version": "0.14.16", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:07.870823879Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-28 21:19:27.224801+00	2026-10-31 00:29:09.06304+00	0	f	[]
493c8d36-dc18-45ca-ad36-84178deda4a3	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "ca5b8138-b6c5-41bb-928c-59ab96c38227", "started_at": "2026-09-16T20:13:01.385050881Z", "finished_at": "2026-10-31T00:29:03.972643020Z", "discovery_id": "73e5a6ec-029e-4114-901d-219f3ea33f8d", "daemon_version": "0.14.12", "discovery_type": {"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:03.615065995Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 20:13:01.38505+00	2026-10-31 00:29:03.974483+00	0	f	[]
177fb0ca-f233-4e40-9e22-b8b8e7c9f67a	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "64824f2d-3081-44a6-a631-23dbe65e4721", "started_at": "2026-09-16T20:11:28.193921546Z", "finished_at": "2026-10-31T00:29:03.978007368Z", "discovery_id": "6781ee75-e068-4b84-81c4-c3d3a6c7c75a", "daemon_version": "0.14.12", "discovery_type": {"type": "Unified", "host_id": "505ad895-0816-4969-a72c-161ebc636002", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:03.547758634Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "505ad895-0816-4969-a72c-161ebc636002", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 20:11:28.193921+00	2026-10-31 00:29:03.97975+00	0	f	[]
b2517385-9abc-4333-a7bb-f33128f6509f	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "b368c96b-6dbf-4523-bcf9-fee0229c2dfb", "started_at": "2026-10-02T13:49:47.412762700Z", "finished_at": "2026-10-31T00:29:04.842644691Z", "discovery_id": "49d20653-ee1b-4a16-89e3-cd25c4716bbe", "daemon_version": "0.15.3", "discovery_type": {"type": "Unified", "host_id": "6aeae378-cd4b-493a-8bff-471792dff268", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:04.421935838Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6aeae378-cd4b-493a-8bff-471792dff268", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-02 13:49:47.412762+00	2026-10-31 00:29:04.844278+00	0	f	[]
c31d091b-7214-4c85-8bd6-383288a45e75	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "b39cff2d-d385-4ab8-8097-66c2190dae09", "started_at": "2026-03-06T23:20:38.018012792Z", "finished_at": "2026-10-31T00:29:06.975179861Z", "discovery_id": null, "daemon_version": "0.14.4", "discovery_type": {"type": "SelfReport", "host_id": "6fd7ccd6-6ea7-4880-b5e5-ad545df36544"}, "last_update_at": "2026-10-31T00:29:06.133694292Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "6fd7ccd6-6ea7-4880-b5e5-ad545df36544"}	Self Report — My Site	2026-03-06 23:20:38.018012+00	2026-10-31 00:29:06.977262+00	0	f	[]
e4b5282d-40b4-4e9c-81fc-67939252508b	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "289721de-ae03-40cd-95d5-6af379282969", "started_at": "2026-09-15T05:37:18.553708846Z", "finished_at": "2026-10-31T00:29:09.945990774Z", "discovery_id": "c3f82595-8e76-48b0-9870-b5afa08bdfa5", "daemon_version": "0.14.8", "discovery_type": {"type": "Unified", "host_id": "2a9b1a5e-51d9-4f13-b6e4-0b1e3e27e0dc", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:09.512989334Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2a9b1a5e-51d9-4f13-b6e4-0b1e3e27e0dc", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-15 05:37:18.553708+00	2026-10-31 00:29:09.94755+00	0	f	[]
db003c9f-e3a9-4154-a068-021dd02888ee	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "1e8cf04d-e26d-4e84-a90f-4a534c217faf", "started_at": "2026-10-02T13:54:28.548758981Z", "finished_at": "2026-10-31T00:29:04.834040257Z", "discovery_id": "21721b3e-c3a3-4e7f-95e4-67307304297e", "daemon_version": "0.15.3", "discovery_type": {"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:04.489663598Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-02 13:54:28.548758+00	2026-10-31 00:29:04.835702+00	0	f	[]
03f5b092-e42e-4294-ab7e-637283d476a8	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "61efb9ca-2ac2-4ab8-858e-07da2d3ef161", "started_at": "2026-02-10T04:10:20.090437467Z", "finished_at": "2026-10-31T00:29:07.804734295Z", "discovery_id": null, "daemon_version": "0.14.10", "discovery_type": {"type": "SelfReport", "host_id": "45cb5c93-df81-41f1-9047-97867efb47bb"}, "last_update_at": "2026-10-31T00:29:06.995017705Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "45cb5c93-df81-41f1-9047-97867efb47bb"}	Self Report — My Site	2026-02-10 04:10:20.090437+00	2026-10-31 00:29:07.806969+00	0	f	[]
92073f9e-cbf7-4412-973f-35ffdfdfc8cd	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "ff3dfff8-b772-4147-bc5c-c2806e8e84a4", "started_at": "2026-02-28T03:11:21.545902350Z", "finished_at": "2026-10-31T00:29:13.385157740Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:12.595851641Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-28 03:11:21.545902+00	2026-10-31 00:29:13.387222+00	0	f	[]
8b913b55-76b1-465f-ba23-8872200625aa	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "1811938e-d0f9-4568-bc69-ef3f2931e9ef", "started_at": "2026-08-25T01:24:07.586140654Z", "finished_at": "2026-10-31T00:29:14.728326774Z", "discovery_id": "45c9811a-5610-4ae9-8e52-823f0127eaa3", "daemon_version": "0.19.2", "discovery_type": {"type": "Unified", "host_id": "5f0170e1-a84c-4207-b391-c13297a54d48", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:13.983761810Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "5f0170e1-a84c-4207-b391-c13297a54d48", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 01:24:07.58614+00	2026-10-31 00:29:14.729924+00	0	f	[]
3d881015-0c2c-43bb-b32e-885390ab6ab2	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "0400691f-2af1-4b20-bade-e50044271bfe", "started_at": "2026-10-07T13:13:08.735400435Z", "finished_at": "2026-10-31T00:29:06.967274221Z", "discovery_id": "b34fc545-7791-4e07-84b9-0fa1347a985d", "daemon_version": "0.14.4", "discovery_type": {"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:06.620508251Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-07 13:13:08.7354+00	2026-10-31 00:29:06.968928+00	0	f	[]
b8b3f9aa-133b-423e-bb22-97a875542670	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "68d1cd6d-9bbc-4af3-9202-ecf0db0dee57", "started_at": "2026-08-06T01:43:36.139858287Z", "finished_at": "2026-10-31T00:29:09.072111142Z", "discovery_id": "aacd1f4a-d72f-4276-9818-952352760baf", "daemon_version": "0.14.16", "discovery_type": {"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:08.706184044Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-06 01:43:36.139858+00	2026-10-31 00:29:09.074201+00	0	f	[]
2c04aa7e-1af2-4bb5-a87d-309d4915662a	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "c8b7e98c-df16-4cc6-88f2-82866bbb959f", "started_at": "2026-09-15T05:38:49.198136490Z", "finished_at": "2026-10-31T00:29:09.940650788Z", "discovery_id": "ced7b27f-6c15-4efd-8037-035aafb5d8d9", "daemon_version": "0.14.8", "discovery_type": {"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:09.583796783Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-15 05:38:49.198136+00	2026-10-31 00:29:09.942158+00	0	f	[]
eb041af2-2898-49a2-ae81-7cc0e36fa522	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "c50795be-e20e-483e-857d-f04a9f103f60", "started_at": "2026-08-17T00:25:40.701352183Z", "finished_at": "2026-10-31T00:29:10.802791441Z", "discovery_id": "3d18340b-e32a-4964-af77-86a0c46ef83a", "daemon_version": "0.16.1", "discovery_type": {"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:10.402688957Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-17 00:25:40.701352+00	2026-10-31 00:29:10.804308+00	0	f	[]
85ba8501-c017-4616-abf3-8de317da7743	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "2b6e6d22-4644-4cad-a850-04fb5896df10", "started_at": "2026-03-02T09:02:26.922912054Z", "finished_at": "2026-10-31T00:29:13.393371019Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "SelfReport", "host_id": "b6dcbdab-8ba5-4b12-8dee-5177af2cf702"}, "last_update_at": "2026-10-31T00:29:12.148840125Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "b6dcbdab-8ba5-4b12-8dee-5177af2cf702"}	Self Report — My Site	2026-03-02 09:02:26.922912+00	2026-10-31 00:29:13.395504+00	0	f	[]
a3a2758c-1551-469d-bc80-40d4245aa7c0	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "3dec6b93-d9d7-46ab-94e2-ef221564d563", "started_at": "2026-09-24T21:01:31.815231601Z", "finished_at": "2026-10-31T00:29:07.813667781Z", "discovery_id": "2c866ea1-e9a5-4803-bd9f-2967d12aa176", "daemon_version": "0.14.10", "discovery_type": {"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:07.462936255Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-24 21:01:31.815231+00	2026-10-31 00:29:07.815308+00	0	f	[]
093d3ddf-51dc-4bd3-86a6-821edae3fb08	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "c9568c2f-1ae3-48c4-9136-6eb8fcd496a8", "started_at": "2026-08-06T01:40:06.139857038Z", "finished_at": "2026-10-31T00:29:09.057530205Z", "discovery_id": "f5836d95-dde9-412c-9748-eec76600aa3b", "daemon_version": "0.14.16", "discovery_type": {"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:08.702372596Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-06 01:40:06.139857+00	2026-10-31 00:29:09.059284+00	0	f	[]
d3da35b7-f7b3-417d-b620-b4676bbdfca0	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "b5591b69-0dfe-41fa-87c1-3d5e140cca15", "started_at": "2026-03-06T07:04:38.164982988Z", "finished_at": "2026-10-31T00:29:09.075122821Z", "discovery_id": null, "daemon_version": "0.14.16", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:08.281072094Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-06 07:04:38.164982+00	2026-10-31 00:29:09.077382+00	0	f	[]
c50f8fad-c34a-4854-a454-c24b2a0e8421	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "bbdc17b9-0273-4a91-8a0e-268a6fcf0277", "started_at": "2026-02-26T00:43:32.254923713Z", "finished_at": "2026-10-31T00:29:10.799866401Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "SelfReport", "host_id": "4c158030-00c1-49eb-b7e2-4e803057a3dd"}, "last_update_at": "2026-10-31T00:29:09.963897590Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "4c158030-00c1-49eb-b7e2-4e803057a3dd"}	Self Report — My Site	2026-02-26 00:43:32.254923+00	2026-10-31 00:29:10.80201+00	0	f	[]
047c83c2-5277-41ee-8010-c5ee76b36f20	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "44164544-aa86-4db3-ad9a-503d5933df88", "started_at": "2026-09-24T21:05:01.816315928Z", "finished_at": "2026-10-31T00:29:07.816171878Z", "discovery_id": "a4185036-689b-43db-a946-5803f5986255", "daemon_version": "0.14.10", "discovery_type": {"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:07.466706191Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-24 21:05:01.816315+00	2026-10-31 00:29:07.81779+00	0	f	[]
72dd87fa-2f4c-48f5-ac43-5faa86f4e29d	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "bfb7da47-98f1-4957-9f13-5b4f6f440e6a", "started_at": "2026-03-08T06:53:48.342722812Z", "finished_at": "2026-10-31T00:29:09.937411598Z", "discovery_id": null, "daemon_version": "0.14.8", "discovery_type": {"type": "SelfReport", "host_id": "caeb1c88-3a79-4e24-8a5a-fedd841b3a98"}, "last_update_at": "2026-10-31T00:29:09.092026436Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "caeb1c88-3a79-4e24-8a5a-fedd841b3a98"}	Self Report — My Site	2026-03-08 06:53:48.342722+00	2026-10-31 00:29:09.93972+00	0	f	[]
42dcc070-7525-4791-9fd6-40223d75683a	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "9478634a-d980-4211-b0fa-ac0e884de1ad", "started_at": "2026-10-05T14:23:50.058326236Z", "finished_at": "2026-10-31T00:29:13.380015464Z", "discovery_id": "1797c972-dbda-4fe8-879e-3e92131843a3", "daemon_version": "0.17.3", "discovery_type": {"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:13.032847924Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-05 14:23:50.058326+00	2026-10-31 00:29:13.381789+00	0	f	[]
dbf8b9f4-0145-4e4d-ac85-8c0bd5badb5d	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "01486396-5c85-4268-b3c8-b83da1042232", "started_at": "2026-09-15T05:42:19.268112358Z", "finished_at": "2026-10-31T00:29:09.948447541Z", "discovery_id": "0aae9bfe-445b-44ad-89cc-c50ed895fd8b", "daemon_version": "0.14.8", "discovery_type": {"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:09.587296815Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-15 05:42:19.268112+00	2026-10-31 00:29:09.950254+00	0	f	[]
2d53f60d-175b-45a1-9036-d127d6e8d3f2	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "2b8a5125-8bbc-4f50-9fd7-baad3f93a89f", "started_at": "2026-08-17T00:24:03.071614457Z", "finished_at": "2026-10-31T00:29:10.791412775Z", "discovery_id": "bd24e3cc-27a3-4b82-9dd9-0c1b8e0c1aac", "daemon_version": "0.16.1", "discovery_type": {"type": "Unified", "host_id": "51d0c941-a5a3-4d6c-b3a9-5afbcaed2d3d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:10.356378606Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "51d0c941-a5a3-4d6c-b3a9-5afbcaed2d3d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-17 00:24:03.071614+00	2026-10-31 00:29:10.793223+00	0	f	[]
44c214ed-96cd-4a8a-926a-10cdebe8e1b4	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "128947fe-516b-47a0-897f-975edb8d4122", "started_at": "2026-04-20T17:24:17.714515792Z", "finished_at": "2026-10-31T00:29:11.310248501Z", "discovery_id": "cd01bedc-e9cf-482d-a533-430b44c67c1d", "daemon_version": "0.19.1", "discovery_type": {"type": "Unified", "host_id": "41167b70-ef54-45fb-ac1c-df4f8be5f312", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:10.857162682Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "41167b70-ef54-45fb-ac1c-df4f8be5f312", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-20 17:24:17.714515+00	2026-10-31 00:29:11.311839+00	0	f	[]
af97afca-6caa-4e3e-8f9e-aa5e5d5e4bf3	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "6a03df7f-6023-4f74-893b-f9bb39cb8d98", "started_at": "2026-10-29T03:47:06.824079209Z", "finished_at": "2026-10-31T00:29:12.133221097Z", "discovery_id": "5a68f356-4bb5-415c-96b0-a23e816c169a", "daemon_version": "0.14.11", "discovery_type": {"type": "Unified", "host_id": "f591b8f3-e44c-4fab-bb81-9b67c29df370", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:11.739529347Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f591b8f3-e44c-4fab-bb81-9b67c29df370", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-29 03:47:06.824079+00	2026-10-31 00:29:12.134972+00	0	f	[]
1b3fdd7f-bd37-40ee-a6e9-b06f7924cacf	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "42d8b7a7-6a5c-409b-b8c2-d33c0af1e9c0", "started_at": "2026-08-17T00:29:10.692540259Z", "finished_at": "2026-10-31T00:29:10.794233561Z", "discovery_id": "7209f439-17e2-4568-b767-efcb4c655abc", "daemon_version": "0.16.1", "discovery_type": {"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:10.406433962Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-17 00:29:10.69254+00	2026-10-31 00:29:10.79588+00	0	f	[]
b698f6ad-c135-48c0-9344-50f9a2a5452b	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "dae6845a-5c1c-4537-8549-2cb0bb052dd7", "started_at": "2026-04-20T18:00:40.095617293Z", "finished_at": "2026-10-31T00:29:11.307549589Z", "discovery_id": "fd884f7d-eb11-4dac-aef5-0f7c57398e18", "daemon_version": "0.19.1", "discovery_type": {"type": "Unified", "host_id": "c1107482-106c-4d18-8ee2-07081aef2afb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:10.890706648Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c1107482-106c-4d18-8ee2-07081aef2afb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-20 18:00:40.095617+00	2026-10-31 00:29:11.309289+00	0	f	[]
e85c11fa-6c37-4f2b-837a-a9151fc229e4	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "04ade0d1-6874-420a-a1d3-6ece53cd5f6f", "started_at": "2026-04-20T17:56:03.901696836Z", "finished_at": "2026-10-31T00:29:11.312676287Z", "discovery_id": "f810a54a-f651-4361-aad4-125b9f0cdb3b", "daemon_version": "0.19.1", "discovery_type": {"type": "Unified", "host_id": "2551f8f8-f3cf-43d9-9d28-dd99c9468591", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:10.818639960Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2551f8f8-f3cf-43d9-9d28-dd99c9468591", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-20 17:56:03.901696+00	2026-10-31 00:29:11.314308+00	0	f	[]
c2436a6a-9d2e-4fc8-aa0d-9f6d696c842c	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "0073f594-456f-4861-8285-c8eb1345c48c", "started_at": "2026-10-25T01:45:04.782037127Z", "finished_at": "2026-10-31T00:29:11.729156894Z", "discovery_id": "eb7da626-8e6d-41ce-a7d2-2503c470e727", "daemon_version": "0.19.4", "discovery_type": {"type": "Unified", "host_id": "2d0259ed-6b96-4930-847a-fa3ebdbf38a3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:11.323051554Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2d0259ed-6b96-4930-847a-fa3ebdbf38a3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-25 01:45:04.782037+00	2026-10-31 00:29:11.730909+00	0	f	[]
045ad364-e618-4002-b3a5-e50b78dc4801	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "8625c9a3-252c-4570-9b6a-e4459dde2e43", "started_at": "2026-10-05T14:18:58.075801174Z", "finished_at": "2026-10-31T00:29:13.382727891Z", "discovery_id": "2265e81c-df9b-42b9-83df-d634d2d1d005", "daemon_version": "0.17.3", "discovery_type": {"type": "Unified", "host_id": "38b72a6c-0aea-4147-b599-5af096f07623", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:12.965509611Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "38b72a6c-0aea-4147-b599-5af096f07623", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-05 14:18:58.075801+00	2026-10-31 00:29:13.38429+00	0	f	[]
d56c6b2b-6424-482f-998e-e283936edc24	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "2fdd805f-0e91-427a-a614-e15a2a15a0f0", "started_at": "2026-07-05T00:02:02.946186416Z", "finished_at": "2026-10-31T00:29:13.916841081Z", "discovery_id": "d18dc568-9f52-4fe4-8e7a-491dd0b8bb9f", "daemon_version": "0.17.12", "discovery_type": {"type": "Unified", "host_id": "4ad32cd4-5017-4aa6-81fb-2a624a987ac2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:13.409053501Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "4ad32cd4-5017-4aa6-81fb-2a624a987ac2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-05 00:02:02.946186+00	2026-10-31 00:29:13.918492+00	0	f	[]
cb3f3a19-bad2-4b62-9b45-7c1b6ba84090	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "11299ee7-3101-4bae-be7a-097db8407dd7", "started_at": "2026-01-23T03:48:00.568581680Z", "finished_at": "2026-10-31T00:29:14.733263190Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:14.386960461Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-23 03:48:00.568581+00	2026-10-31 00:29:14.735683+00	0	f	[]
b7ea60e1-6598-4b79-9dd3-3577a9db400b	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "5d811057-9d57-4a37-997d-6d9ff0bde3f4", "started_at": "2026-10-05T14:20:20.057223827Z", "finished_at": "2026-10-31T00:29:13.388024205Z", "discovery_id": "4c93beb8-d8f2-4f65-b843-757ec520684e", "daemon_version": "0.17.3", "discovery_type": {"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:13.028342962Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-05 14:20:20.057223+00	2026-10-31 00:29:13.38962+00	0	f	[]
e7e32a76-2f87-40b1-97c7-fb7fb8ee6ebf	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "9d715165-15d9-4c53-8eeb-adadd14656ec", "started_at": "2026-10-26T16:01:31.798722266Z", "finished_at": "2026-10-31T00:29:15.148309488Z", "discovery_id": "30886c5d-88fc-433c-95c7-2bdd79bb2221", "daemon_version": "0.17.18", "discovery_type": {"type": "Unified", "host_id": "9b66035e-43fc-4de9-a9fa-9febda413452", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:14.747083920Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9b66035e-43fc-4de9-a9fa-9febda413452", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-26 16:01:31.798722+00	2026-10-31 00:29:15.149988+00	0	f	[]
9d149e47-969f-47bb-ab80-3d6ab8a89851	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "6fa2c508-5f31-44cd-9153-00c853afe9a7", "started_at": "2026-08-25T02:51:38.519652606Z", "finished_at": "2026-10-31T00:29:14.730804679Z", "discovery_id": "fe0818f1-099f-4b00-b4f8-e336fbd99a15", "daemon_version": "0.19.2", "discovery_type": {"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:13.975822917Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 02:51:38.519652+00	2026-10-31 00:29:14.732343+00	0	f	[]
6acff38e-0caf-4aef-84c9-81cfd39ec26d	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "675b82cb-527e-4541-8702-fef2a8eb5552", "started_at": "2026-08-25T02:55:08.502675512Z", "finished_at": "2026-10-31T00:29:14.736587679Z", "discovery_id": "cca9db70-f59f-45e3-95b4-599969ee4a47", "daemon_version": "0.19.2", "discovery_type": {"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:13.979920085Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 02:55:08.502675+00	2026-10-31 00:29:14.738265+00	0	f	[]
419eddc1-4beb-4a0b-83ef-ff0834745fd4	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "cbdca757-31a8-4a33-b45a-5ad52b4de60b", "started_at": "2026-10-01T04:02:39.535834801Z", "finished_at": "2026-10-31T00:29:26.473144437Z", "discovery_id": "0c1a623e-fbd9-43a8-b332-343baacd3e14", "daemon_version": "0.19.5", "discovery_type": {"type": "Unified", "host_id": "bd69aa7e-1ffb-4e1d-8ac6-adc468be6771", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:15.159049069Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bd69aa7e-1ffb-4e1d-8ac6-adc468be6771", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-01 04:02:39.535834+00	2026-10-31 00:29:26.476591+00	0	f	[]
d734de44-12ea-46d3-9a34-5371fe2b2eca	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "c540d032-e83e-4050-ab10-97635f0968a2", "started_at": "2026-10-01T04:07:23.928243114Z", "finished_at": "2026-10-31T00:29:26.478563239Z", "discovery_id": "91add506-0f87-4a8d-94b0-40e6bf348510", "daemon_version": "0.19.5", "discovery_type": {"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:15.227136116Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-01 04:07:23.928243+00	2026-10-31 00:29:26.481678+00	0	f	[]
eb9d3398-b25a-4b88-8e63-b550707a7bc1	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	3683c875-6eb7-4fd4-8260-9b277607eb84	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "928a95dd-f2e3-47eb-8f71-bc0fe4d97c98", "progress": 0, "warnings": [], "daemon_id": "3683c875-6eb7-4fd4-8260-9b277607eb84", "session_id": "075ee07b-1779-47db-8c94-50c17ce5f66d", "started_at": "2026-10-01T04:03:53.917292009Z", "finished_at": "2026-10-31T00:29:26.483303737Z", "discovery_id": "82cf15a8-82b5-4a8a-af1f-b8c61511dff9", "daemon_version": "0.19.5", "discovery_type": {"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-10-31T00:29:15.223231178Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-01 04:03:53.917292+00	2026-10-31 00:29:26.485426+00	0	f	[]
\.


--
-- Data for Name: discovery_warnings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.discovery_warnings (id, discovery_id, site_id, code, credential_id, host_id, remote_host_id, subnet_id, details, created_at) FROM stdin;
787113d8-90af-48c7-ae88-8b4631f487b9	9f99ba01-595f-49b6-b08b-61b7f197255b	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	OutdatedDaemonFormat	\N	\N	\N	\N	{"code": "OutdatedDaemonFormat", "daemon_version": "0.16.0"}	2026-10-31 00:28:46.100405+00
\.


--
-- Data for Name: entity_tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.entity_tags (id, entity_id, entity_type, tag_id, created_at, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
b05e681e-75f6-4f4e-84e2-5df5de5de82e	4d0c6051-56a4-476d-baf5-b133ae36a4bd	"Service"	92698861-c6e5-4ed0-82dc-2fcda6c803f6	2026-10-31 00:28:44.47322+00	2026-10-31 00:28:44.47322+00	\N	\N	\N
\.


--
-- Data for Name: host_credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.host_credentials (host_id, credential_id, ip_address_ids) FROM stdin;
\.


--
-- Data for Name: hosts; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.hosts (id, site_id, name, hostname, description, source, virtualization_metadata, created_at, updated_at, sys_descr, sys_object_id, sys_location, sys_contact, management_url, chassis_id, manufacturer, model, serial_number, sys_name, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id, virtualization_service_id, firmware_revision, sys_descr_source, sys_object_id_source, sys_location_source, sys_contact_source, management_url_source, chassis_id_source, sys_name_source, manufacturer_source, model_source, serial_number_source, firmware_revision_source, name_source, software_revision, software_revision_source, hostname_source, os, os_source, virtualization_interface_id, asset_tag, asset_tag_source, matched_device_type, matched_device_type_source, dismissed_at) FROM stdin;
bd69aa7e-1ffb-4e1d-8ac6-adc468be6771	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98		c01a23384611	Scanopy daemon	{"type": "Discovery"}	null	2026-10-31 00:29:15.171354+00	2026-10-31 00:29:15.171354+00	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-10-31 00:29:15.171354+00	\N	\N	2026-10-31 00:29:15.171354+00	\N	\N	\N	\N	\N	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	\N	"Unspecified"	"DaemonSelfReport"	\N	"Unspecified"	\N	\N	"Unspecified"	\N	"Unspecified"	\N
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
275bec1c-89c4-4411-8e5b-43c805a94d91	bd69aa7e-1ffb-4e1d-8ac6-adc468be6771	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	2026-10-31 00:29:15.171354+00	2026-10-31 00:29:15.171354+00	1	lo	\N	24	\N	1	1	\N	fc3c5432-a2cb-4049-85e6-64b4d71473b3	lo	\N	\N	\N	2026-10-31 00:29:15.171354+00	\N	\N	2026-10-31 00:29:15.171354+00	\N	\N	\N	"Unspecified"	f	"Unspecified"	"Unspecified"	"Unspecified"
9777cbce-1d35-422f-8abb-793f21ff7c13	bd69aa7e-1ffb-4e1d-8ac6-adc468be6771	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	2026-10-31 00:29:15.171354+00	2026-10-31 00:29:15.171354+00	2	eth0	\N	53	\N	1	1	d6:d6:66:fc:6c:c2	ad5a60f7-f8f2-4b78-99f7-a43668ba7558	eth0	\N	\N	\N	2026-10-31 00:29:15.171354+00	\N	\N	2026-10-31 00:29:15.171354+00	\N	\N	\N	"DaemonSelfReport"	f	"Unspecified"	"Unspecified"	"Unspecified"
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
fc3c5432-a2cb-4049-85e6-64b4d71473b3	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	bd69aa7e-1ffb-4e1d-8ac6-adc468be6771	bf720fd4-edfb-4341-b9d4-97b0f30185d3	127.0.0.1	\N	lo	0	2026-10-31 00:29:15.171354+00	2026-10-31 00:29:15.171354+00	2026-10-31 00:29:15.171354+00	\N	\N	2026-10-31 00:29:15.171354+00	\N	\N	\N	"Unspecified"	"Unspecified"
ad5a60f7-f8f2-4b78-99f7-a43668ba7558	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	bd69aa7e-1ffb-4e1d-8ac6-adc468be6771	d0b8a284-36a1-4e60-a131-20992f033ee5	172.25.0.4	d6:d6:66:fc:6c:c2	eth0	1	2026-10-31 00:29:15.171354+00	2026-10-31 00:29:15.171354+00	2026-10-31 00:29:15.171354+00	\N	\N	2026-10-31 00:29:15.171354+00	\N	\N	\N	"DaemonSelfReport"	"Unspecified"
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
57de48ad-f218-4835-9a53-267cd70e577f	My Organization	\N	{"rate": "Month", "type": "Community", "base_cents": 0, "host_cents": null, "seat_cents": null, "site_cents": null, "trial_days": 0, "included_orgs": null, "included_hosts": null, "included_seats": null, "included_sites": null}	active	2026-10-31 00:21:16.437493+00	2026-10-31 00:21:16.437493+00	["OrgCreated", "OnboardingModalCompleted", "FirstDaemonRegistered", "FirstHostDiscovered", "FirstDiscoveryCompleted", "FirstTagCreated", "FirstDependencyCreated", "FirstUserApiKeyCreated", "SecondSiteCreated"]	f	\N	other	\N	f	\N	null	\N	\N	\N	\N	{"hosts": "None", "seats": "None", "sites": "None", "license_over_instance_limit": false, "license_silence_reported_for": null, "airgap_expiry_notified_through": null, "sunset_notified_last_unsupported": null}	\N	\N	0	\N	\N	\N	\N	f	\N
\.


--
-- Data for Name: ports; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ports (id, site_id, host_id, port_number, protocol, port_type, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id, port_source) FROM stdin;
c58a15f4-b1c5-4d91-83d6-72d9069b6eb7	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	bd69aa7e-1ffb-4e1d-8ac6-adc468be6771	60073	Tcp	Custom	2026-10-31 00:29:15.171354+00	2026-10-31 00:29:15.171354+00	2026-10-31 00:29:15.171354+00	\N	\N	2026-10-31 00:29:15.171354+00	\N	\N	\N	"Unspecified"
\.


--
-- Data for Name: release_announcements; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.release_announcements (id, version, email_sent_at, created_at, updated_at) FROM stdin;
54c572fe-5592-46a0-bfb8-384d4152439d	0.19.5	\N	2026-10-31 00:21:14.471992+00	2026-10-31 00:21:14.471992+00
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
9ff69b61-4ac2-4522-82fb-85cd0b229403	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	2026-10-31 00:29:15.171354+00	2026-10-31 00:29:15.171354+00	Scanopy Daemon	bd69aa7e-1ffb-4e1d-8ac6-adc468be6771	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}}	0	2026-10-31 00:29:15.171354+00	\N	\N	2026-10-31 00:29:15.171354+00	\N	\N	\N	\N
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
928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	d661a266-6ee6-4bb6-8906-28801c8eeffa
\.


--
-- Data for Name: sites; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.sites (id, name, created_at, updated_at, organization_id, stale_after_hours) FROM stdin;
928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	My Site	2026-10-31 00:21:16.47003+00	2026-10-31 00:21:16.47003+00	57de48ad-f218-4835-9a53-267cd70e577f	\N
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

COPY public.subnets (id, site_id, created_at, updated_at, cidr, name, description, subnet_type, source, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id, virtualization_service_id, cidr_source, name_source, subnet_type_source) FROM stdin;
d0b8a284-36a1-4e60-a131-20992f033ee5	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	2026-10-01 04:02:39.48952+00	2026-10-01 04:02:39.48952+00	"172.25.0.0/28"	172.25.0.0/28	\N	Lan	{"type": "Discovery"}	2026-10-01 04:02:39.48952+00	\N	\N	2026-10-31 00:29:26.433668+00	\N	\N	\N	\N	"DaemonSelfReport"	"Unspecified"	"InterfaceNameMatch"
bf720fd4-edfb-4341-b9d4-97b0f30185d3	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	2026-10-01 04:02:39.48948+00	2026-10-01 04:02:39.48948+00	"127.0.0.0/8"	127.0.0.0/8	\N	Loopback	{"type": "Discovery"}	2026-10-01 04:02:39.48948+00	\N	\N	2026-10-31 00:29:26.433638+00	\N	\N	\N	\N	"DaemonSelfReport"	"Unspecified"	"InterfaceNameMatch"
9efd41a6-05a3-478d-ba32-aa71f323ca3e	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	2026-10-31 00:29:28.103653+00	2026-10-31 00:29:28.103653+00	"10.1.0.0/24"	Blocked Subnet	\N	Lan	{"type": "Manual"}	2026-10-31 00:29:28.103653+00	\N	\N	2026-10-31 00:29:28.103653+00	\N	\N	\N	\N	"Manual"	"Manual"	"Manual"
\.


--
-- Data for Name: tag_groups; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tag_groups (id, organization_id, name, description, group_type, entity_types, ordered, created_at, updated_at) FROM stdin;
ebaca5a3-c908-4a43-afc4-3cfb1bed1fda	57de48ad-f218-4835-9a53-267cd70e577f	Application	What each entity is part of. A service inherits its host's application tag, and the Application view groups by it.	Application	\N	f	2026-10-31 00:21:16.456684+00	2026-10-31 00:21:16.456684+00
a80322dd-6ff7-4e93-8477-1a306537e503	57de48ad-f218-4835-9a53-267cd70e577f	Status	Where an asset is in its lifecycle.	Status	\N	t	2026-10-31 00:21:16.458661+00	2026-10-31 00:21:16.458661+00
c8bc1e74-540c-4568-a970-d455fa205ddc	57de48ad-f218-4835-9a53-267cd70e577f	Visibility	Entities holding a tag of this group are left out of every topology.	Visibility	\N	f	2026-10-31 00:21:16.459752+00	2026-10-31 00:21:16.459752+00
bd953e59-b163-423f-b3e2-a0a90b4fbc42	57de48ad-f218-4835-9a53-267cd70e577f	Environment	Which stage of delivery an asset serves.	Custom	\N	t	2026-10-31 00:21:16.548881+00	2026-10-31 00:21:16.548881+00
d595fd38-6223-4109-aabd-f7b9796bb638	57de48ad-f218-4835-9a53-267cd70e577f	Criticality	How much an outage of the asset costs.	Custom	\N	t	2026-10-31 00:21:16.54976+00	2026-10-31 00:21:16.54976+00
2b046106-5f0a-4fbd-bfc3-834b01a5d3d7	57de48ad-f218-4835-9a53-267cd70e577f	Management	Who administers the asset.	Custom	\N	t	2026-10-31 00:21:16.551124+00	2026-10-31 00:21:16.551124+00
\.


--
-- Data for Name: tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tags (id, organization_id, name, description, created_at, updated_at, color, valid_from, valid_to, lineage_id, snapshot_id, icon, tag_group_id, "position") FROM stdin;
5df92272-834d-4856-aa8a-65e55aae6ec6	57de48ad-f218-4835-9a53-267cd70e577f	Hidden	Left out of topology.	2026-10-31 00:21:16.462709+00	2026-10-31 00:21:16.462709+00	Gray	2026-10-31 00:21:16.462709+00	\N	\N	\N	eye-off	c8bc1e74-540c-4568-a970-d455fa205ddc	\N
31f32b74-fde4-4c9a-8757-38e0d2d43ae5	57de48ad-f218-4835-9a53-267cd70e577f	Planned	Ordered or scheduled, not yet in service	2026-10-31 00:21:16.555072+00	2026-10-31 00:21:16.555072+00	Blue	2026-10-31 00:21:16.555072+00	\N	\N	\N	calendar-clock	a80322dd-6ff7-4e93-8477-1a306537e503	1
326ef47b-8ca7-4fda-9444-6edbbe6e22bd	57de48ad-f218-4835-9a53-267cd70e577f	Active	In service	2026-10-31 00:21:16.55508+00	2026-10-31 00:21:16.55508+00	Green	2026-10-31 00:21:16.55508+00	\N	\N	\N	circle-check	a80322dd-6ff7-4e93-8477-1a306537e503	2
91539610-297b-45b0-8d7f-9fb6e08edc5e	57de48ad-f218-4835-9a53-267cd70e577f	Decommissioning	Being retired; still on the network	2026-10-31 00:21:16.555081+00	2026-10-31 00:21:16.555081+00	Orange	2026-10-31 00:21:16.555081+00	\N	\N	\N	hourglass	a80322dd-6ff7-4e93-8477-1a306537e503	3
3173e1af-dbfb-443c-882d-621d665c9c83	57de48ad-f218-4835-9a53-267cd70e577f	Decommissioned	Retired from service; kept for the record	2026-10-31 00:21:16.555082+00	2026-10-31 00:21:16.555082+00	Gray	2026-10-31 00:21:16.555082+00	\N	\N	\N	archive	a80322dd-6ff7-4e93-8477-1a306537e503	4
38cf939b-cc88-4048-9957-dd0339ec7fb4	57de48ad-f218-4835-9a53-267cd70e577f	Production	Serves live users and business traffic	2026-10-31 00:21:16.555087+00	2026-10-31 00:21:16.555087+00	Red	2026-10-31 00:21:16.555087+00	\N	\N	\N	server	bd953e59-b163-423f-b3e2-a0a90b4fbc42	1
dac38ec0-a01e-4c20-813f-e51aaaf3373d	57de48ad-f218-4835-9a53-267cd70e577f	Staging	A pre-release copy of production for final testing	2026-10-31 00:21:16.555089+00	2026-10-31 00:21:16.555089+00	Amber	2026-10-31 00:21:16.555089+00	\N	\N	\N	flask-conical	bd953e59-b163-423f-b3e2-a0a90b4fbc42	2
1e75f526-9b9b-4515-9440-0294dca6ded2	57de48ad-f218-4835-9a53-267cd70e577f	Development	Used to build and test changes	2026-10-31 00:21:16.55509+00	2026-10-31 00:21:16.55509+00	Blue	2026-10-31 00:21:16.55509+00	\N	\N	\N	code	bd953e59-b163-423f-b3e2-a0a90b4fbc42	3
2c25c4f6-07eb-49a9-a573-c281105907ea	57de48ad-f218-4835-9a53-267cd70e577f	Lab	Test bench or experimental equipment	2026-10-31 00:21:16.555092+00	2026-10-31 00:21:16.555092+00	Purple	2026-10-31 00:21:16.555092+00	\N	\N	\N	test-tube	bd953e59-b163-423f-b3e2-a0a90b4fbc42	4
8d59312c-f241-4a5f-802b-a747540c301d	57de48ad-f218-4835-9a53-267cd70e577f	Critical	An outage stops the business	2026-10-31 00:21:16.555095+00	2026-10-31 00:21:16.555095+00	Red	2026-10-31 00:21:16.555095+00	\N	\N	\N	siren	d595fd38-6223-4109-aabd-f7b9796bb638	1
985ffdcb-2d87-4105-b29d-332f14279191	57de48ad-f218-4835-9a53-267cd70e577f	High	An outage disrupts many people	2026-10-31 00:21:16.555096+00	2026-10-31 00:21:16.555096+00	Orange	2026-10-31 00:21:16.555096+00	\N	\N	\N	arrow-up	d595fd38-6223-4109-aabd-f7b9796bb638	2
98062061-d1f1-4acd-bb14-7f3daf41ef9a	57de48ad-f218-4835-9a53-267cd70e577f	Medium	An outage disrupts some people	2026-10-31 00:21:16.555098+00	2026-10-31 00:21:16.555098+00	Yellow	2026-10-31 00:21:16.555098+00	\N	\N	\N	minus	d595fd38-6223-4109-aabd-f7b9796bb638	3
16610a0c-c87b-45e3-8215-330f9cce9425	57de48ad-f218-4835-9a53-267cd70e577f	Low	An outage has little impact	2026-10-31 00:21:16.555099+00	2026-10-31 00:21:16.555099+00	Gray	2026-10-31 00:21:16.555099+00	\N	\N	\N	arrow-down	d595fd38-6223-4109-aabd-f7b9796bb638	4
34151c9f-3041-4384-bc33-8e0f38144083	57de48ad-f218-4835-9a53-267cd70e577f	Managed	Administered by your IT team	2026-10-31 00:21:16.555103+00	2026-10-31 00:21:16.555103+00	Green	2026-10-31 00:21:16.555103+00	\N	\N	\N	shield-check	2b046106-5f0a-4fbd-bfc3-834b01a5d3d7	1
78bfd08c-d418-4e90-b18e-45bd36769072	57de48ad-f218-4835-9a53-267cd70e577f	Unmanaged	Administered by no one, such as a personal device	2026-10-31 00:21:16.555105+00	2026-10-31 00:21:16.555105+00	Orange	2026-10-31 00:21:16.555105+00	\N	\N	\N	shield-question-mark	2b046106-5f0a-4fbd-bfc3-834b01a5d3d7	2
7e5fa4c5-8583-4e26-b4aa-87bdd0404af0	57de48ad-f218-4835-9a53-267cd70e577f	Third-party	Administered by a vendor or another organization	2026-10-31 00:21:16.555106+00	2026-10-31 00:21:16.555106+00	Blue	2026-10-31 00:21:16.555106+00	\N	\N	\N	handshake	2b046106-5f0a-4fbd-bfc3-834b01a5d3d7	3
92698861-c6e5-4ed0-82dc-2fcda6c803f6	57de48ad-f218-4835-9a53-267cd70e577f	Integration Test Tag	\N	2026-10-31 00:28:44.450975+00	2026-10-31 00:28:44.450975+00	Yellow	2026-10-31 00:28:44.450975+00	\N	\N	\N	\N	\N	\N
5e9ba2c8-8963-4bee-844b-aa7d475c3d57	57de48ad-f218-4835-9a53-267cd70e577f	Test Tag	\N	2026-10-31 00:29:27.122974+00	2026-10-31 00:29:27.122974+00	Yellow	2026-10-31 00:29:27.122974+00	2026-10-31 00:29:27.141849+00	e7269fab-8b8b-4136-aefa-d05ab453c1dd	\N	\N	\N	\N
e7269fab-8b8b-4136-aefa-d05ab453c1dd	57de48ad-f218-4835-9a53-267cd70e577f	Updated Tag	\N	2026-10-31 00:29:27.122974+00	2026-10-31 00:29:27.122974+00	Yellow	2026-10-31 00:29:27.141849+00	2026-10-31 00:29:27.156254+00	\N	\N	\N	\N	\N
\.


--
-- Data for Name: topologies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.topologies (id, site_id, options, created_at, updated_at) FROM stdin;
d8d865d5-2cc1-4022-a5a8-62a064376c99	928a95dd-f2e3-47eb-8f71-bc0fe4d97c98	{"local": {"tag_filter": {}, "bundle_edges": true, "show_minimap": true, "no_fade_edges": false, "hide_edge_types": {"L3Logical": ["Hypervisor", "SameContainer"], "Workloads": ["RequestPath", "HubAndSpoke", "PhysicalLink", "NeighborLink"], "L2Physical": ["SameHost"], "Application": ["ContainerRuntime"]}}, "request": {"element_sort": {}, "element_rules": [{"id": "df452bfa-7e8d-415e-ad74-cd17be6ac17a", "rule": "ByTrunkPort"}, {"id": "ad0af77e-8022-4b79-836f-adc575a244c6", "rule": "ByVLAN"}, {"id": "b0aad044-2f36-4786-b2da-5df7d9647140", "rule": "ByPortOpStatus"}, {"id": "4473f0e8-a35b-44a3-9787-bc692481e8d8", "rule": {"ByServiceCategory": {"title": "Infrastructure", "categories": ["NetworkCore", "NetworkAccess", "NetworkAppliance", "RemoteAccess", "Workstation", "Mobile", "IoT", "Printer", "OpenPorts", "NetworkIdentities"], "is_infra_rule": true}}}, {"id": "da24ac3c-cea8-422a-93c3-6d491b18c253", "rule": {"ByTag": {"title": null, "tag_ids": []}}}, {"id": "973f176b-5e6b-424e-ad10-846771d4dd3c", "rule": "ByDeviceType"}, {"id": "5f052f58-2446-4c8d-9a52-4d57a3e3453d", "rule": "ByHypervisor"}, {"id": "e974b5e3-d87c-4f47-8f79-2650b518c41c", "rule": "ByContainerRuntime"}, {"id": "523e321c-13ac-4fad-acae-12dc1537cd2f", "rule": "ByStack"}], "hide_entities": {}, "container_rules": {"L3Logical": [{"id": "f1a0248e-1a1b-4c91-9237-41526b402f44", "rule": "BySubnet"}, {"id": "921409fc-7a2f-4483-806c-47aee8d6e7ae", "rule": "MergeContainerBridges"}], "Workloads": [{"id": "302cbaf5-ac1f-46a4-8b84-5f0ff389171a", "rule": "ByHost"}], "L2Physical": [{"id": "302cbaf5-ac1f-46a4-8b84-5f0ff389171a", "rule": "ByHost"}], "Application": [{"id": "601abf9d-6ce4-4e2c-b362-4e35b193240f", "rule": {"ByApplication": {"tag_ids": []}}}]}, "hide_metadata_values": {"L3Logical": {"Service": {"Category": ["OpenPorts"]}}, "Workloads": {"Service": {"Category": ["OpenPorts"]}}, "L2Physical": {"Service": {"Category": ["OpenPorts"]}, "Interface": {"LinkState": ["Unlinked"]}}, "Application": {"Service": {"Category": ["OpenPorts"]}}}}}	2026-10-31 00:21:16.489153+00	2026-10-31 00:21:16.489153+00
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
63bfba8a-f884-47bc-baa0-0510fd63acda	2026-10-31 00:21:16.439628+00	2026-10-31 00:21:16.439628+00	$argon2id$v=19$m=19456,t=2,p=1$QNC+t/lDDg+8oQ5OtKUhZg$SBqmCqUXboUYjTcbLDUfyk5h4u40dqmg1G8xjVe/aSU	\N	\N	\N	user@gmail.com	57de48ad-f218-4835-9a53-267cd70e577f	Owner	{}	\N	t	\N	\N	\N	\N	\N	{"daemon_alerts": true, "product_updates": true, "trial_and_usage": true, "discovery_digest": true, "product_onboarding": true}	0	{"clock": "browser_default", "time_zone": null, "date_order": "browser_default", "timestamps": "relative", "week_start": "monday", "table_density": "compact"}	\N	0.19.5	\N
b54be648-f20a-4261-8431-f8595071b4ce	2026-10-31 00:29:27.787699+00	2026-10-31 00:29:27.787699+00	\N	\N	\N	\N	user@example.com	57de48ad-f218-4835-9a53-267cd70e577f	Owner	{}	\N	f	\N	\N	\N	\N	\N	{"daemon_alerts": true, "product_updates": true, "trial_and_usage": true, "discovery_digest": true, "product_onboarding": true}	0	{"clock": "browser_default", "time_zone": null, "date_order": "browser_default", "timestamps": "relative", "week_start": "monday", "table_density": "compact"}	\N	\N	\N
\.


--
-- Data for Name: vlans; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.vlans (id, vlan_number, name, description, site_id, organization_id, source, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id, name_source) FROM stdin;
\.


--
-- Data for Name: session; Type: TABLE DATA; Schema: tower_sessions; Owner: postgres
--

COPY tower_sessions.session (id, data, expiry_date) FROM stdin;
42H2cqevMejJVaLwMM11mg	\\x93c4109a75cd30f0a255c9e831afa772f661e382a7757365725f6964d92436336266626138612d663838342d343762632d626161302d303531306664363361636461ad73657373696f6e5f65706f63680099cd07eacd0137001510ce1a856d7f000000	2026-11-07 00:21:16.444951+00
KszpRgJeAfJKaCZaW7zw_g	\\x93c410fef0bc5b5a26684af2015e0246e9cc2a83a7757365725f6964d92436336266626138612d663838342d343762632d626161302d303531306664363361636461ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92438366630646435362d376339342d343161312d386638632d363038336366646462393765a87573655f63617365a56f74686572ad73657373696f6e5f65706f63680099cd07eacd0137001c2dce0c84415d000000	2026-11-07 00:28:45.209994+00
T8B3C7jT3qLj1X5WlExsmQ	\\x93c410996c4c94567ed5e3a2ded3b80b77c04f83a7757365725f6964d92436336266626138612d663838342d343762632d626161302d303531306664363361636461ad73657373696f6e5f65706f636800ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92465376562303363612d373130312d343664632d393665352d666162643038383563313335a87573655f63617365a56f7468657299cd07eacd0137001d0fce243edbf3000000	2026-11-07 00:29:15.608099+00
CxjZ7ZRSErvEG_mRc39-xQ	\\x93c410c57e7f7391f91bc4bb125294edd9180b83ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92432613366363836392d653934302d346331332d386165632d353535326132393963353333a87573655f63617365a56f74686572a7757365725f6964d92436336266626138612d663838342d343762632d626161302d303531306664363361636461ad73657373696f6e5f65706f63680099cd07eacd0137001d1ace32190f89000000	2026-11-07 00:29:26.840503+00
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

\unrestrict uPffam4jfSPMVEX8U71uOgdufbhRqxaJFDwm2XVpQTf8Ss7S4LTP7BzNamU1MIq

