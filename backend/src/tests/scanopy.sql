--
-- PostgreSQL database dump
--

\restrict Z6oehPEXm3bmRWcuDQDHQEX5uXEAn53kDfXCleMgj3y0nrIu2ydPcqdugCWafLe

-- Dumped from database version 17.12
-- Dumped by pg_dump version 17.12

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
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_organization_id_fkey;
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
ALTER TABLE IF EXISTS ONLY public.site_assignments DROP CONSTRAINT IF EXISTS site_assignments_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.site_assignments DROP CONSTRAINT IF EXISTS site_assignments_site_id_fkey;
ALTER TABLE IF EXISTS ONLY public.site_assignments DROP CONSTRAINT IF EXISTS site_assignments_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.site_assignments DROP CONSTRAINT IF EXISTS site_assignments_invite_id_fkey;
ALTER TABLE IF EXISTS ONLY public.site_assignments DROP CONSTRAINT IF EXISTS site_assignments_api_key_id_fkey;
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
ALTER TABLE IF EXISTS ONLY public.credentials DROP CONSTRAINT IF EXISTS credentials_login_credential_id_fkey;
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
DROP INDEX IF EXISTS public.site_assignments_user_scope;
DROP INDEX IF EXISTS public.site_assignments_site;
DROP INDEX IF EXISTS public.site_assignments_organization;
DROP INDEX IF EXISTS public.site_assignments_invite_scope;
DROP INDEX IF EXISTS public.site_assignments_api_key_scope;
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
DROP INDEX IF EXISTS public.idx_user_api_keys_user;
DROP INDEX IF EXISTS public.idx_user_api_keys_org;
DROP INDEX IF EXISTS public.idx_user_api_keys_key;
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
DROP INDEX IF EXISTS public.idx_credentials_login_credential_id;
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
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_pkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_key_key;
ALTER TABLE IF EXISTS ONLY public.topologies DROP CONSTRAINT IF EXISTS topologies_pkey;
ALTER TABLE IF EXISTS ONLY public.tags DROP CONSTRAINT IF EXISTS tags_pkey;
ALTER TABLE IF EXISTS ONLY public.tag_groups DROP CONSTRAINT IF EXISTS tag_groups_pkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_pkey;
ALTER TABLE IF EXISTS ONLY public.subnet_vlans DROP CONSTRAINT IF EXISTS subnet_vlans_pkey;
ALTER TABLE IF EXISTS ONLY public.snapshots DROP CONSTRAINT IF EXISTS snapshots_pkey;
ALTER TABLE IF EXISTS ONLY public.sites DROP CONSTRAINT IF EXISTS sites_pkey;
ALTER TABLE IF EXISTS ONLY public.site_credentials DROP CONSTRAINT IF EXISTS site_credentials_pkey;
ALTER TABLE IF EXISTS ONLY public.site_assignments DROP CONSTRAINT IF EXISTS site_assignments_pkey;
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
DROP TABLE IF EXISTS public.user_api_keys;
DROP TABLE IF EXISTS public.topologies;
DROP TABLE IF EXISTS public.tags;
DROP TABLE IF EXISTS public.tag_groups;
DROP TABLE IF EXISTS public.subnets;
DROP TABLE IF EXISTS public.subnet_vlans;
DROP TABLE IF EXISTS public.snapshots;
DROP TABLE IF EXISTS public.sites;
DROP TABLE IF EXISTS public.site_credentials;
DROP TABLE IF EXISTS public.site_assignments;
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
    description text,
    login_credential_id uuid,
    source jsonb DEFAULT '{"type": "Manual"}'::jsonb NOT NULL
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
    license_server_version text,
    setup jsonb DEFAULT '[]'::jsonb NOT NULL
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
-- Name: site_assignments; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.site_assignments (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    organization_id uuid NOT NULL,
    user_id uuid,
    api_key_id uuid,
    invite_id uuid,
    role text NOT NULL,
    site_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT site_assignments_one_principal CHECK ((num_nonnulls(user_id, api_key_id, invite_id) = 1)),
    CONSTRAINT site_assignments_role_check CHECK ((role = ANY (ARRAY['Admin'::text, 'Member'::text, 'Viewer'::text])))
);


ALTER TABLE public.site_assignments OWNER TO postgres;

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
    "position" integer,
    source jsonb DEFAULT '{"type": "Manual"}'::jsonb NOT NULL
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
20251006215000	users	2026-11-13 00:32:13.782576+00	t	\\x4f13ce14ff67ef0b7145987c7b22b588745bf9fbb7b673450c26a0f2f9a36ef8ca980e456c8d77cfb1b2d7a4577a64d7	-1
20251006215100	networks	2026-11-13 00:32:13.789392+00	t	\\xeaa5a07a262709f64f0c59f31e25519580c79e2d1a523ce72736848946a34b17dd9adc7498eaf90551af6b7ec6d4e0e3	-1
20251006215151	create_hosts	2026-11-13 00:32:13.794406+00	t	\\x6ec7487074c0724932d21df4cf1ed66645313cf62c159a7179e39cbc261bcb81a24f7933a0e3cf58504f2a90fc5c1962	-1
20251006215155	create_subnets	2026-11-13 00:32:13.799188+00	t	\\xefb5b25742bd5f4489b67351d9f2494a95f307428c911fd8c5f475bfb03926347bdc269bbd048d2ddb06336945b27926	-1
20251006215201	create_groups	2026-11-13 00:32:13.804177+00	t	\\x0a7032bf4d33a0baf020e905da865cde240e2a09dda2f62aa535b2c5d4b26b20be30a3286f1b5192bd94cd4a5dbb5bcd	-1
20251006215204	create_daemons	2026-11-13 00:32:13.809408+00	t	\\xcfea93403b1f9cf9aac374711d4ac72d8a223e3c38a1d2a06d9edb5f94e8a557debac3668271f8176368eadc5105349f	-1
20251006215212	create_services	2026-11-13 00:32:13.815487+00	t	\\xd5b07f82fc7c9da2782a364d46078d7d16b5c08df70cfbf02edcfe9b1b24ab6024ad159292aeea455f15cfd1f4740c1d	-1
20251029193448	user-auth	2026-11-13 00:32:13.824119+00	t	\\xfde8161a8db89d51eeade7517d90a41d560f19645620f2298f78f116219a09728b18e91251ae31e46a47f6942d5a9032	-1
20251030044828	daemon_api	2026-11-13 00:32:13.826405+00	t	\\x181eb3541f51ef5b038b2064660370775d1b364547a214a20dde9c9d4bb95a1c273cd4525ef29e61fa65a3eb4fee0400	-1
20251030170438	host-hide	2026-11-13 00:32:13.828371+00	t	\\x87c6fda7f8456bf610a78e8e98803158caa0e12857c5bab466a5bb0004d41b449004a68e728ca13f17e051f662a15454	-1
20251102224919	create_discovery	2026-11-13 00:32:13.843636+00	t	\\xb32a04abb891aba48f92a059fae7341442355ca8e4af5d109e28e2a4f79ee8e11b2a8f40453b7f6725c2dd6487f26573	-1
20251106235621	normalize-daemon-cols	2026-11-13 00:32:13.846471+00	t	\\x5b137118d506e2708097c432358bf909265b3cf3bacd662b02e2c81ba589a9e0100631c7801cffd9c57bb10a6674fb3b	-1
20251107034459	api_keys	2026-11-13 00:32:13.858223+00	t	\\x3133ec043c0c6e25b6e55f7da84cae52b2a72488116938a2c669c8512c2efe72a74029912bcba1f2a2a0a8b59ef01dde	-1
20251107222650	oidc-auth	2026-11-13 00:32:13.88849+00	t	\\xd349750e0298718cbcd98eaff6e152b3fb45c3d9d62d06eedeb26c75452e9ce1af65c3e52c9f2de4bd532939c2f31096	-1
20251110181948	orgs-billing	2026-11-13 00:32:13.902019+00	t	\\x5bbea7a2dfc9d00213bd66b473289ddd66694eff8a4f3eaab937c985b64c5f8c3ad2d64e960afbb03f335ac6766687aa	-1
20251113223656	group-enhancements	2026-11-13 00:32:13.903831+00	t	\\xbe0699486d85df2bd3edc1f0bf4f1f096d5b6c5070361702c4d203ec2bb640811be88bb1979cfe51b40805ad84d1de65	-1
20251117032720	daemon-mode	2026-11-13 00:32:13.905599+00	t	\\xdd0d899c24b73d70e9970e54b2c748d6b6b55c856ca0f8590fe990da49cc46c700b1ce13f57ff65abd6711f4bd8a6481	-1
20251118143058	set-default-plan	2026-11-13 00:32:13.907489+00	t	\\xd19142607aef84aac7cfb97d60d29bda764d26f513f2c72306734c03cec2651d23eee3ce6cacfd36ca52dbddc462f917	-1
20251118225043	save-topology	2026-11-13 00:32:13.921592+00	t	\\x011a594740c69d8d0f8b0149d49d1b53cfbf948b7866ebd84403394139cb66a44277803462846b06e762577adc3e61a3	-1
20251123232748	network-permissions	2026-11-13 00:32:13.925497+00	t	\\x161be7ae5721c06523d6488606f1a7b1f096193efa1183ecdd1c2c9a4a9f4cad4884e939018917314aaf261d9a3f97ae	-1
20251125001342	billing-updates	2026-11-13 00:32:13.927059+00	t	\\xa235d153d95aeb676e3310a52ccb69dfbd7ca36bba975d5bbca165ceeec7196da12119f23597ea5276c364f90f23db1e	-1
20251128035448	org-onboarding-status	2026-11-13 00:32:13.929429+00	t	\\x1d7a7e9bf23b5078250f31934d1bc47bbaf463ace887e7746af30946e843de41badfc2b213ed64912a18e07b297663d8	-1
20251129180942	nfs-consolidate	2026-11-13 00:32:13.931231+00	t	\\xb38f41d30699a475c2b967f8e43156f3b49bb10341bddbde01d9fb5ba805f6724685e27e53f7e49b6c8b59e29c74f98e	-1
20251206052641	discovery-progress	2026-11-13 00:32:13.933509+00	t	\\x9d433b7b8c58d0d5437a104497e5e214febb2d1441a3ad7c28512e7497ed14fb9458e0d4ff786962a59954cb30da1447	-1
20251206202200	plan-fix	2026-11-13 00:32:13.934913+00	t	\\x242f6699dbf485cf59a8d1b8cd9d7c43aeef635a9316be815a47e15238c5e4af88efaa0daf885be03572948dc0c9edac	-1
20251207061341	daemon-url	2026-11-13 00:32:13.938319+00	t	\\x01172455c4f2d0d57371d18ef66d2ab3b7a8525067ef8a86945c616982e6ce06f5ea1e1560a8f20dadcd5be2223e6df1	-1
20251210045929	tags	2026-11-13 00:32:13.949818+00	t	\\xe3dde83d39f8552b5afcdc1493cddfeffe077751bf55472032bc8b35fc8fc2a2caa3b55b4c2354ace7de03c3977982db	-1
20251210175035	terms	2026-11-13 00:32:13.951523+00	t	\\xe47f0cf7aba1bffa10798bede953da69fd4bfaebf9c75c76226507c558a3595c6bfc6ac8920d11398dbdf3b762769992	-1
20251213025048	hash-keys	2026-11-13 00:32:13.962395+00	t	\\xfc7cbb8ce61f0c225322297f7459dcbe362242b9001c06cb874b7f739cea7ae888d8f0cfaed6623bcbcb9ec54c8cd18b	-1
20251214050638	scanopy	2026-11-13 00:32:13.964529+00	t	\\x0108bb39832305f024126211710689adc48d973ff66e5e59ff49468389b75c1ff95d1fbbb7bdb50e33ec1333a1f29ea6	-1
20251215215724	topo-scanopy-fix	2026-11-13 00:32:13.96574+00	t	\\xed88a4b71b3c9b61d46322b5053362e5a25a9293cd3c420c9df9fcaeb3441254122b8a18f58c297f535c842b8a8b0a38	-1
20251217153736	category_rename	2026-11-13 00:32:13.968388+00	t	\\x03af7ec905e11a77e25038a3c272645da96014da7c50c585a25cea3f9a7579faba3ff45114a5e589d144c9550ba42421	-1
20251218053111	invite-persistence	2026-11-13 00:32:13.975384+00	t	\\x21d12f48b964acfd600f88e70ceb14abd9cf2a8a10db2eae2a6d8f44cf7d20749f93293631e6123e92b7c3c1793877c2	-1
20251219211216	create_shares	2026-11-13 00:32:13.984319+00	t	\\x036485debd3536f9e58ead728f461b925585911acf565970bf3b2ab295b12a2865606d6a56d334c5641dcd42adeb3d68	-1
20251220170928	permissions-cleanup	2026-11-13 00:32:13.986376+00	t	\\x632f7b6702b494301e0d36fd3b900686b1a7f9936aef8c084b5880f1152b8256a125566e2b5ac40216eaadd3c4c64a03	-1
20251220180000	commercial-to-community	2026-11-13 00:32:13.987679+00	t	\\x26fc298486c225f2f01271d611418377c403183ae51daf32fef104ec07c027f2017d138910c4fbfb5f49819a5f4194d6	-1
20251221010000	cleanup_subnet_type	2026-11-13 00:32:13.989017+00	t	\\xb521121f3fd3a10c0de816977ac2a2ffb6118f34f8474ffb9058722abc0dc4cf5cbec83bc6ee49e79a68e6b715087f40	-1
20251221020000	remove_host_target	2026-11-13 00:32:13.990592+00	t	\\x77b5f8872705676ca81a5704bd1eaee90b9a52b404bdaa27a23da2ffd4858d3e131680926a5a00ad2a0d7a24ba229046	-1
20251221030000	user_network_access	2026-11-13 00:32:13.999504+00	t	\\x5c23f5bb6b0b8ca699a17eee6730c4197a006ca21fecc79136a5e5697b9211a81b4cd08ceda70dace6a26408d021ff3a	-1
20251221040000	interfaces_table	2026-11-13 00:32:14.012106+00	t	\\xf7977b6f1e7e5108c614397d03a38c9bd9243fdc422575ec29610366a0c88f443de2132185878d8e291f06a50a8c3244	-1
20251221050000	ports_table	2026-11-13 00:32:14.024184+00	t	\\xdf72f9306b405be7be62c39003ef38408115e740b120f24e8c78b8e136574fff7965c52023b3bc476899613fa5f4fe35	-1
20251221060000	bindings_table	2026-11-13 00:32:14.037991+00	t	\\x933648a724bd179c7f47305e4080db85342d48712cde39374f0f88cde9d7eba8fe5fafba360937331e2a8178dec420c4	-1
20251221070000	group_bindings	2026-11-13 00:32:14.046055+00	t	\\x697475802f6c42e38deee6596f4ba786b09f7b7cd91742fbc5696dd0f9b3ddfce90dd905153f2b1a9e82f959f5a88302	-1
20251222020000	tag_cascade_delete	2026-11-13 00:32:14.04814+00	t	\\xabfb48c0da8522f5c8ea6d482eb5a5f4562ed41f6160a5915f0fd477c7dd0517aa84760ef99ab3a5db3e0f21b0c69b5f	-1
20251223232524	network_remove_default	2026-11-13 00:32:14.049776+00	t	\\x7099fe4e52405e46269d7ce364050da930b481e72484ad3c4772fd2911d2d505476d659fa9f400c63bc287512d033e18	-1
20251225100000	color_enum	2026-11-13 00:32:14.051659+00	t	\\x62cecd9d79a49835a3bea68a7959ab62aa0c1aaa7e2940dec6a7f8a714362df3649f0c1f9313672d9268295ed5a1cfa9	-1
20251227010000	topology_snapshot_migration	2026-11-13 00:32:14.058119+00	t	\\xc042591d254869c0e79c8b52a9ede680fd26f094e2c385f5f017e115f5e3f31ad155f4885d095344f2642ebb70755d54	-1
20251228010000	user_api_keys	2026-11-13 00:32:14.072761+00	t	\\xa41adb558a5b9d94a4e17af3f16839b83f7da072dbeac9251b12d8a84c7bec6df008009acf246468712a975bb36bb5f5	-1
20251230160000	daemon_version_and_maintainer	2026-11-13 00:32:14.076622+00	t	\\xafed3d9f00adb8c1b0896fb663af801926c218472a0a197f90ecdaa13305a78846a9e15af0043ec010328ba533fca68f	-1
20260103000000	service_position	2026-11-13 00:32:14.079349+00	t	\\x19d00e8c8b300d1c74d721931f4d771ec7bc4e06db0d6a78126e00785586fdc4bcff5b832eeae2fce0cb8d01e12a7fb5	-1
20260106000000	interface_mac_index	2026-11-13 00:32:14.081932+00	t	\\xa26248372a1e31af46a9c6fbdaef178982229e2ceeb90cc6a289d5764f87a38747294b3adf5f21276b5d171e42bdb6ac	-1
20260106204402	entity_tags_junction	2026-11-13 00:32:14.097465+00	t	\\xf73c604f9f0b8db065d990a861684b0dbd62c3ef9bead120c68431c933774de56491a53f021e79f09801680152f5a08a	-1
20260108033856	fix_entity_tags_json_format	2026-11-13 00:32:14.099683+00	t	\\x197eaa063d4f96dd0e897ad8fd96cc1ba9a54dda40a93a5c12eac14597e4dea4c806dd0a527736fb5807b7a8870d9916	-1
20260110000000	email_verification	2026-11-13 00:32:14.104188+00	t	\\xb8da8433f58ba4ce846b9fa0c2551795747a8473ad10266b19685504847458ea69d27a0ce430151cfb426f5f5fb6ac3a	-1
20260114145808	daemon_user_fk_set_null	2026-11-13 00:32:14.106005+00	t	\\x57b060be9fc314d7c5851c75661ca8269118feea6cf7ee9c61b147a0e117c4d39642cf0d1acdf7a723a9a76066c1b8ff	-1
20260116010000	snmp_credentials	2026-11-13 00:32:14.11439+00	t	\\x6f3971cf194d56883c61fa795406a8ab568307ed86544920d098b32a6a1ebb7effcb5ec38a70fdc9b617eff92d63d51e	-1
20260116020000	host_snmp_fields	2026-11-13 00:32:14.119455+00	t	\\xf2f088c13ab0dd34e1cb1e5327b0b4137440b0146e5ce1e78b8d2dfa05d9b5a12a328eeb807988453a8a43ad8a1c95ba	-1
20260116030000	if_entries	2026-11-13 00:32:14.13636+00	t	\\xa58391708f8b21901ab9250af528f638a6055462f70ffddfd7c451433aacdabd62825546fa8be108f23a3cae78b8ae28	-1
20260116100000	daemon_api_key_link	2026-11-13 00:32:14.140567+00	t	\\x41088aa314ab173344a6b416280721806b2f296a32a8d8cae58c7e5717f389fe599134ed03980ed97e4b7659e99c4f82	-1
20260131190000	add_hubspot_company_id	2026-11-13 00:32:14.142114+00	t	\\x4326f95f4954e176157c1c3e034074a3e5c44da4d60bbd7a9e4b6238c9ef52a30f8b38d3c887864b6e4c1163dc062beb	-1
20260201021238	fix_service_acronym_capitalization	2026-11-13 00:32:14.144144+00	t	\\x88b010ac8f0223d880ea6a730f11dc6d27fa5de9d8747de3431e46d59f1dbf2f72ae4a87c2e52c32152549f5c1f96bb2	-1
20260204004436	add_entity_tags_to_topology	2026-11-13 00:32:14.145857+00	t	\\x3eff1a1490e77065ec861ef1b9aad8c55de0170106a42720f7931b3929b179122b16e44390b2652771bf91bba32a7757	-1
20260205120000	billing_overhaul	2026-11-13 00:32:14.148667+00	t	\\xbf850cfa0c40a3c65f574efd15fd55a4b702296203d28077a09d1c22076fee8601f2b78345aef370ab9163657de767ab	-1
20260205183207	rename_hubspot_to_brevo	2026-11-13 00:32:14.150314+00	t	\\x4678a7d80215e5eafb5e80af0daa20e2868a3b4f2112e88cb1b2b9efc87d63de3fb96c133f359b224c658789ae4b0d13	-1
20260221120000	add_plan_limit_notifications	2026-11-13 00:32:14.152206+00	t	\\xef770dac07e1d80888832f33184dc46c1d3b8185b91c507cb404468d6ad8c29cacf455178801c67aa27b6a626d3ad82d	-1
20260222120000	add_pending_email	2026-11-13 00:32:14.153595+00	t	\\xddd220f7602c44548d56849c0a8d081ecd1da1383374a11e3e227c7d9becb73a49f5e5bb09ed65901c16df4c16e913e5	-1
20260301120000	add_if_name_to_if_entries	2026-11-13 00:32:14.155102+00	t	\\xc9fc0a2b77ecbf0e1d5ab292c4fe162a26113468c878dfd26a3c63d89c0ee1957ca328ecfe25c611867a0e73780f0cb6	-1
20260306002816	cleanup_standby	2026-11-13 00:32:14.156505+00	t	\\x01b0c236a8a4d0d97f0f633b18f8cbdb92b6d72063289989b90a1b7b6b303e65e0557eb09927b2580dcb7e8ee5966c75	-1
20260309120000	add_org_use_case	2026-11-13 00:32:14.15789+00	t	\\xdb8c8a2f0f9416ba3b687fc75453d7c12c50a6f386b4784d21bd6adfc4a4a7556c637c25cf116118402bbd12c0d5aafe	-1
20260313120000	snmp_extended_discovery	2026-11-13 00:32:14.160146+00	t	\\xc4e72539099de1b830d87a169bfbabba4b8fb378a3c4c4a1dfca698adf3e403d750040d784c26d9fa343be2908064c9d	-1
20260315120000	universal_credentials	2026-11-13 00:32:14.182615+00	t	\\x87dc6f39202e81d5555df78a9d056b143f11bd22e6d7f483065f605e242a360902c72c4d5a49717e7fcc24a366bb5ff5	-1
20260315120001	discovery_scan_settings	2026-11-13 00:32:14.184353+00	t	\\xe9da183fdd8e04e574f553f61f6f33efa046cdae38c846c8077b06c5260446fb4aa39da2449bda7f1d8cf3aa9f16e158	-1
20260315120002	backfill_org_created_milestone	2026-11-13 00:32:14.185744+00	t	\\x14f886a19773cd2263d86f88479be460d21f071d5212e3789c5c40b6415c293fc7d06c7b138351cc42108f89a14fe745	-1
20260316120000	fix_jsonb_null_if_entries	2026-11-13 00:32:14.187952+00	t	\\x65c358069710f7f86d6a3e257e658c2f241cc376433c3a0317b0ec9e1876a66f9738cb65c6ab1a5c197fe40d5aa2aa2b	-1
20260319120000	rename_snmp_to_snmpv2c	2026-11-13 00:32:14.1895+00	t	\\xdce5c9461f402e1672607078b2c571f0eb30b51d46f8e9414d8909efb40693f543e49e560cb7d703db274515043aa08e	-1
20260321120000	add_discovery_scan_count	2026-11-13 00:32:14.192317+00	t	\\x6c8201ab453a51632176d534c6604e0818e28a8a4a153e33e254f4dac0f9b67c9db394082cb663ff1b25941229cf96fc	-1
20260329120000	backfill_subnet_virtualization	2026-11-13 00:32:14.195307+00	t	\\xeac50ded27603dbb5e8773604a52143c9fa8654263e7dd12d3d128ce972c2feed84600e36b2e7a79525b58c44d2ad9d3	-1
20260402120000	rename_topology_node_types	2026-11-13 00:32:14.196683+00	t	\\xc4ba06868add823f83ff1948091bdfe17dbdde80bbec6fe2cf8da2b3689aeeebbe9e9de01b1292bff3c98a74d9e6279f	-1
20260403120000	topology_grouping_rules	2026-11-13 00:32:14.198912+00	t	\\x00799da1206d7c3b3c3db90b7d14437cc054ed2d7273020342e562c619a671e008ff4fdf0365170440b392956949e730	-1
20260405120000	rename_groups_to_dependencies	2026-11-13 00:32:14.209467+00	t	\\x9ce895b456366bf6e54316b22cabd2803aa542dd3733fffa680f0a3af5c4c55a612c5ee511371206921869b7f271c35b	-1
20260406120000	add_tag_is_application_group	2026-11-13 00:32:14.211299+00	t	\\xb7a71e5fdd96ca46c9c7577003309050a93bc53ad192ac5df78e7621f3ed64f07fb29b4658f17af55732cf6dfb7958c2	-1
20260406130000	add_vlans	2026-11-13 00:32:14.224879+00	t	\\x5b3e5d10578d90b5175e5718a28d7147a21b99af2fb3e0ed171d20ee8fd8838c290f648dafdd3b72ef60ff487f7f2494	-1
20260409000000	add_vlans_to_topologies	2026-11-13 00:32:14.226896+00	t	\\x5e0b9dc670580ceec3aa6eae005a39f98733fc27dc574b7f3922f4297813facd5d610af953dfec13e09d0b99eceb3865	-1
20260410000000	rename_interfaces_and_if_entries	2026-11-13 00:32:14.235575+00	t	\\x07f54a59869f458f41f45d75f250aee26b20a426f1ec29930606841770194d6aea0e9e6253a6375fbeebcf9b49121224	-1
20260414000000	add_share_enabled_views	2026-11-13 00:32:14.237302+00	t	\\xc56514355a5977c3242e728e7f5a2533e7b4a5cf8a7ce7757e412e51f1ad85e96d65c13ccd96d050be4a07799b9aef57	-1
20260415120000	rename_onboarding_first_group_created	2026-11-13 00:32:14.238597+00	t	\\x2c17035835d3ead105b76d98688c0b7bd328abdaf9f721d70d057c8afdf438819e93da56707deea5b469b81a7b84d5d7	-1
20260417000000	reindex_interfaces_identity	2026-11-13 00:32:14.243502+00	t	\\x10701e13bc3d838e2ec4a856555ebf338173792f220c405996d3c77e7987e9806798ca0328eb6259e4a62b7e05665b25	-1
20260418000000	add_standby_cleared_at_to_daemons	2026-11-13 00:32:14.244989+00	t	\\x547807de451d015a4ce1438796d5b95e2b98043c521015a21239f6778d10a8d3bf7d8b14e278e09aa0105f1935ad4181	-1
20260501000000	add_organization_billing_flags	2026-11-13 00:32:14.248062+00	t	\\x2de34c4af667d4cd8bc263c27f0526a4a2132022e2eb71ae94fd89edebbfb40cda840055a94e89a21925a317cdff285f	-1
20260502000000	scd2_add_columns	2026-11-13 00:32:14.267717+00	t	\\xe78a73574d86320c0de7fcd43682ee3cdd436dc64d1371f98cb67ef0cdf33097df6e8a92c9a26c5702795144103b085e	-1
20260502000001	scd2_add_discovery_fks_not_valid	2026-11-13 00:32:14.2825+00	t	\\xb49114eb6f8d77cb0c5062619da5c829e5bd65898710efb6207d2804be3677a95eebe88d41d21ae8cba21344fcd63f97	-1
20260502000002	scd2_validate_discovery_fks	2026-11-13 00:32:14.294729+00	t	\\x500201842b9f486d1397c0f0f3ee1a36bc101440d4d97fe0f0bfc971672f1f53f7945aed93c34e2e53d2d9cb913419f1	-1
20260502000003	scd2_backfill_with_metadata	2026-11-13 00:32:14.300443+00	t	\\xd299c1dbf7ec284995dc8a5fcf4b264e4f1ff970085a8631e6d00aa0bc6d6708a5e41d515bb9299c22ef128a958e68e9	-1
20260502000004	scd2_partial_unique_indexes	2026-11-13 00:32:14.329613+00	t	\\xbc30e554c37ec3ca72baa7fc634920257909944dc841231a966845fd4c5bc27ae20606c98867934c872e8883bd56efa1	-1
20260502000005	scd2_drop_old_unique_constraints	2026-11-13 00:32:14.334124+00	t	\\x660c0237299796c6e70a21780d529453a2ac8645f7b122a0b8aa4f362ff780de8849f53a66b71b3ea29ecc66877f47ad	-1
20260502000006	scd2_supporting_indexes	2026-11-13 00:32:14.435345+00	t	\\x4394aaf6951c4b7048f66334fb8c550c55d4cdc06beb29eb7f086a717ee9f26ddeb28e73a7b298633ef517f6c0f84ee0	-1
20260502000007	entitysource_metadata_strip	2026-11-13 00:32:14.439602+00	t	\\x1cfd1a00b100ca0b007be6e8b55f67dde412108f299f94b419a80af45b74b75fdead95637dccb13fdd57fed2f1132446	-1
20260502100000	add_user_email_settings	2026-11-13 00:32:14.441641+00	t	\\xe886b44ca9d3d6461d82b573f617391b8b5f22f18b8113a333aa8c275545ec6455060f68ecbfdd17af86fccc9a500d7a	-1
20260502120000	create_snapshots_table	2026-11-13 00:32:14.446949+00	t	\\x18f829b255b8e11e385903e51182f04fb835aa78ef0090865ce7743f747e4aaaa7b1a7b806a377e9b12af6205aed527a	-1
20260502120001	add_snapshot_id_fks	2026-11-13 00:32:14.460362+00	t	\\x0a080ceaa2922301464b30ea7f25e1f0de5493d15424d07a8dd1f2b4bf1880cf2d25f64d5bf1863f705a1557d867afa2	-1
20260502120002	add_snapshot_id_indexes	2026-11-13 00:32:14.49122+00	t	\\xb7514ff0cf637241d21e90b48214a3a3a5076d0142c99aca39479d2a82cf35e1abdbce8ad5663ca8272d5f74004ae5f9	-1
20260502120003	topology_snapshots_backfill	2026-11-13 00:32:14.499205+00	t	\\x5435da11acf8a9fc37e0520a5b2a959fce84b16080b2a5b67adcf249dea07ae533f858c56f87542152cfc271933c1428	-1
20260502120004	drop_legacy_topology_columns	2026-11-13 00:32:14.512273+00	t	\\xfe45e73be308e6267840664a662cd679fb43469a11f06a168b8b8cfc64b6e86eb10d948271b67a0541481dcc60447769	-1
20260627120000	add_discovery_integration_targets	2026-11-13 00:32:14.514313+00	t	\\x9adf9ea4c36868a3b86067214f8550b8e866aff5d3d6c6ab3d71d1a15ea5e4be0b4349db6d15308fefbceca8403f0cbb	-1
20260703120000	daemon_interfaced_subnets_junction	2026-11-13 00:32:14.519804+00	t	\\xf1399bf8104b1d9cff6a7f66c6040829f01dd64614294295f0ecc27412acc19fd6dd20eff66ce9eadf0cf0c0e8c4353e	-1
20260703120001	drop_discovery_pending_credential_ids	2026-11-13 00:32:14.521899+00	t	\\xc2cfe6f450d000ae36149b042ebcfed938c01487d78ca56a125d72bbeca0de0548824a2396761b3dc647dc80e2e43e80	-1
20260705120000	add_user_session_epoch	2026-11-13 00:32:14.523786+00	t	\\xb296babe8e50ecdd1f50770e12d4234b47ed41cdd826c545debf5f94226672cc2d6fd28eef0483a5fdc8837f5d280d88	-1
20260706120000	drop_credentials_target_ips_and_daemons_capabilities	2026-11-13 00:32:14.526104+00	t	\\xc544c88df68334ac0b3ec180da7ba2030837707f9e3cf2b75b4d56012646c24f8f60f094cc00f05c0f446a2a845e5e98	-1
20260716120000	api_keys_daemon_id	2026-11-13 00:32:14.528913+00	t	\\x484c3b2553bcf81c95aabce4be70b1e2048e097529e04a176a0b75c2edf28ecd0892c959bbd8e8166f621a9dc2dba0dc	-1
20260716120001	api_keys_daemon_id_validate	2026-11-13 00:32:14.530867+00	t	\\x723190451e98f9394b2134dccb89f8eeeb15f6a5644fb88cb24e9e0e7a4ebf5a0c455659930ce2d3096e20c004d7da89	-1
20260716120002	api_keys_daemon_id_unique	2026-11-13 00:32:14.533998+00	t	\\xf788be5a414917ec5c8b9040f85acd352ae3f580dab7e8833db5df19b7d2cf3b43b8a021c03a5082da27eca3bb7a07f0	-1
20260722120000	add_networks_stale_after_hours	2026-11-13 00:32:14.535464+00	t	\\x308d9e04ef943560d3e420c59a3c8017064d59bd28cb63f99c4b562124e2fa038630e71405171901d731420bd9de4e2f	-1
20260727120000	rename_plan_limit_notifications_to_notifications	2026-11-13 00:32:14.537665+00	t	\\x2447c735b8ac6885ae4341738294392f367abcc7c0f5e857bb08523dc5d5a8eccf77db5462b531370a0795135757098d	-1
20260728120000	rename_stale_if_entries_constraints	2026-11-13 00:32:14.541702+00	t	\\x577c54966f8c146a6e5101eb8a2f7f006ef7899ba95ec876be10970b3483b622b254d00dc9c9582efb1862404315764b	-1
20260803120000	drop_organizations_plan_limit_notifications	2026-11-13 00:32:14.543862+00	t	\\xc4c9b44a4cce6cf419cffcbbeade3d5eb119b51f321bdcd3c51a1046d8ec8df81d32f6e76c6472eefdcfae9899ef3107	-1
20260803120001	virtualization_service_id_column	2026-11-13 00:32:14.554368+00	t	\\xe07f6fee751430c94141be6d7ba17390f08f60bdcd01a65072708382b27e9543a9f27fb0cc76bb824940794d6ea67553	-1
20260819120000	hosts_name_source	2026-11-13 00:32:14.560836+00	t	\\x6526859d261bf724e6ac3bb2a51c8588764aab15ecdfad964e06e98bb59f05a753ea2950bdce40cbf7f945d157a230aa	-1
20260824120000	interfaces_neighbor_seen_at	2026-11-13 00:32:14.563029+00	t	\\x70a26934dcc110484dc95100cbc96879504a9bc0ae0fb12ca0effe032816e3282e255d90145c4d2c6bfe2eea351cb4c3	-1
20260826120000	subnets_cidr_source	2026-11-13 00:32:14.566851+00	t	\\x1d8c6a9258f1fd07a08a268cddf5f164bb3b8bb0af34d6d167e480d50b649cbdc5a7b89aa419a218266e9aa1b3d3b497	-1
20260827120000	hosts_firmware_revision	2026-11-13 00:32:14.570198+00	t	\\xa9b3a89986c699c70232d37f5909ab5fecd9c4d2c8708a11b2cad4775494a12080c6664964f942b0079a883f3ecd6ea2	-1
20260827130000	interfaces_unknown_port_facts	2026-11-13 00:32:14.572668+00	t	\\xfa35944166a6c60219a07a6792cd3abbaf909ba78e5494482d73d3d10ddacea618845822dc3dc875542772af742ac38d	-1
20260828120000	hosts_attribute_sources	2026-11-13 00:32:14.600773+00	t	\\x29220b8fb99e5ffb55ca71ea3c05f375214dd77d247d4a54dc7855c5654dbfbbbefceafeb5ddf10710c27085f0a866c4	-1
20260828120001	mac_address_sources	2026-11-13 00:32:14.603709+00	t	\\x39ea28fed133ae2c317bd5fad54e4d16f402ee895b75a6290322e75b434f5502aeddc9f6359d7e95625bb0c0d4ca1d4e	-1
20260828130000	hosts_software_revision	2026-11-13 00:32:14.60616+00	t	\\x83cdafd5691f1bc5fc2345a696ec9fd5c75700693f0011c65d352276d806a6f207f335189153ee8e0fa3fb7e60c18234	-1
20260828140000	ip_addresses_network_mac_index	2026-11-13 00:32:14.609793+00	t	\\x838b853bbb63bcfe0d0976f1c7289960e3133563151c47904ddda0ce5a0078bdc4af9ce07de63d3c024d89b831995535	-1
20260903120000	interfaces_if_descr_optional	2026-11-13 00:32:14.611574+00	t	\\x54495748b0271deebaf7cb06e9e4da9241563e83a162e4bfffaa9e571516223565ab239fcc8bb51fac9e37cbb7625757	-1
20260903120001	interface_neighbor_tables	2026-11-13 00:32:14.643795+00	t	\\xa8352a3ed9b845d96229adcb3e3eb8e796d22a8b6aa794bbc0d2ecafcbf76d68aa59a6e7713f134a57af0acceca3865a	-1
20260908120000	interfaces_ip_configured	2026-11-13 00:32:14.646233+00	t	\\x2467c0b912e78e9cac3818ffdec4157356f6b4a3c96e87a0171d88496439943eacbafefc7a4b51a2b68cc08bbbd29656	-1
20261005120001	rename_networks_to_sites_backfill	2026-11-13 00:32:14.710513+00	t	\\x085c1f2e6ec6b548e7f47fb092519f5f8889629e6198b30bd4a91e4e1d6b5e502513efb213c0601abf444d1f39183f81	-1
20261006120000	host_asset_tag	2026-11-13 00:32:14.713001+00	t	\\x1a9ef01df34f62321f6fdc26f4f75d952000cc366b54217a3107132643258e44957033c5f37b3618ff8d895d01eae918	-1
20261006120100	tag_group_and_icon	2026-11-13 00:32:14.714715+00	t	\\x3bad6e7e42b3a23b29a94462ddefba434ba2e7e92d1d9464db16644d40e45b97d01a83716203f5cda27d7bb5623dd436	-1
20261009120000	user_email_unsubscribe_token	2026-11-13 00:32:14.716717+00	t	\\x41aa0b879e77d264d1d6fc17c65bebe7e2d7796b9785ad79a85b928e23cc93c3cb8a0532ecbc8d72e34fc6e202d25ce1	-1
20261009120001	user_email_unsubscribe_token_index	2026-11-13 00:32:14.720106+00	t	\\x3cdfe6e7f6fe847ff4f18c7ee1d2430ee9833e48c6aa4eb3585c873529bb007b8786c26e6bad1c193ee8cf0dc0b23f0c	-1
20261009120100	tag_groups	2026-11-13 00:32:14.757033+00	t	\\xbdae7fe6cd73793312edbf2c987d3425f1e2c88c9fa98717a9e21550503734aaf67b3a1b804e145269962cd361aaa16a	-1
20261011120000	backfill_interface_credential_milestone	2026-11-13 00:32:14.759041+00	t	\\xfcad57a2a59537e0c9033a6ff906abf7115789554bc06c00c0caa388c220a6a5d3dc880c7aec0df9c6f561b3b94d7477	-1
20261011130000	release_notes	2026-11-13 00:32:14.763994+00	t	\\x46e5e77f7da9f2a01cb042045c01461eeb40b09477a5d2a40f302582b17e2b02e40f6e9dbe40ed1b24e667227ba3a2b3	-1
20261011130001	merge_and_seed_default_credentials	2026-11-13 00:32:14.786544+00	t	\\xc5fc006e4c216dd10154418fd2d52cf3e58b70039498b21a2fc74a1b3e8d9ce34af25da2b6bc197d0c7dff4f9ca910eb	-1
20261012120000	collection_sources	2026-11-13 00:32:14.792118+00	t	\\xfa0a6238fae38ae0c45b9657d540271cf8201a4981d6a16d84c856e8ba192e312b22472db6e2f734e1cd0a56eae723de	-1
20261029120000	unconfirmed_entities	2026-11-13 00:32:14.874093+00	t	\\xb2882cb53e9225fec0318c7ed531021d6aef20dda71386d972c97095771a885b6ff0edea3a785c0b60716fa88af31a28	-1
20261030120000	site_assignments	2026-11-13 00:32:14.90216+00	t	\\xdda847f8830d4d1b6e54d872e8ab95392fe2dcb0628e5ff26d8a6fd90d13c93af9682c8afe8011b1b34d304db68afdae	-1
20261031120000	hypervisor_guest_warnings_and_ssh_logins	2026-11-13 00:32:14.912211+00	t	\\xc2f3e4ab34a0fe5834edc0e0a6df3e67d5e59437be44e35e10b3df45894993ac8c43422b16b059df0561032e41b102eb	-1
20260914120000	hosts_hostname_source	2026-11-13 00:32:14.650561+00	t	\\xa57a5dc26c000ec5b0b4aaf66486018f637b1f1228ae994dafa06a653b5afae3d8dd2bb1264f5695c50053d656bccdc2	-1
20260915120000	organization_license_columns	2026-11-13 00:32:14.65319+00	t	\\xd46a3e083f8179128aae52f68541627b57131d66c93d84c2ad0002a7bf6c025f757345490b47d810511d49cd9528525d	-1
20260915120001	organizations_license_entitlement	2026-11-13 00:32:14.655116+00	t	\\x1da434d08d6afb9f0bbe4165f112cae53d3046c8578d4feb88d5f3ea0fa0649cd4bf2a69ac6c2c24978a561a6333fcb9	-1
20260916120000	organization_license_key_issued_at	2026-11-13 00:32:14.656661+00	t	\\x99d66ef126e64532e4e87c1a99b0ac630a2031b1b4c9ba41cbe257b8e3a83e47ec0fe1360a409b5cbb57401326d7a9d3	-1
20260917120000	organization_license_key_type	2026-11-13 00:32:14.658163+00	t	\\xac327476c465df672532ce71bfbe03a0d92ff5cc3053fae9a15fa24100308cda6b08e118267c3c1009118eeb90f46038	-1
20260917130000	organization_bills_by_invoice	2026-11-13 00:32:14.660115+00	t	\\x33bf32d3b3edce4fe6803c326785210f322b9cdfd065440098b53424df7042ba90da1ac6f99ade774f894da1b67b0979	-1
20260930120000	clear_zero_mac_addresses	2026-11-13 00:32:14.665284+00	t	\\xef8416f55b61953f00f1fd249a04c82d63d0d8d510e9db6cae96f79db7a70a61dfc5a22029a6878e74da05e60b63b7dd	-1
20261001120000	add_user_display_settings	2026-11-13 00:32:14.66732+00	t	\\xd7f9afb0762c40085ced713496cf39e9778c4257196e1ba02a0feec1af67eab309df78ce5b753ebde6d250f6fbf3ef32	-1
20261001120001	credential_daemon_os	2026-11-13 00:32:14.669772+00	t	\\xe679b2c2caea51e608200d064b81b02f1f4cee20cfa60a929db29b87666ba8a5d01e5d0739a2b9953f401f4171c11c9d	-1
20261002120000	daemon_os	2026-11-13 00:32:14.671327+00	t	\\x931ef66a80cd751db47512b5f9397a5d7df21aff77cbf28c5a190c69b27ce6040d2e2f2ed3c08e85faf13d28e5cba25f	-1
20261003120000	hosts_os	2026-11-13 00:32:14.67355+00	t	\\xb8a3b3a3e6ec9e21cc64468fa5b691dba7d39556aab7cd28fee01a21fcbde0782952b4b73cbdbd334a17c554769fbdce	-1
20261003130000	organization_license_server_version	2026-11-13 00:32:14.675079+00	t	\\xe2f1bf9692ed8bb1a08a1ae13b53f632fd5f2112d53225bbf13332af26639b9518edb28daa4b3ed98bf71ed4b1002d34	-1
20261004120000	host_virtualization_interface	2026-11-13 00:32:14.678379+00	t	\\x79224f29c669331336f577a5c9f7e5f674dec637f1a879e9c4016e8705af2e921e63cf4ab178cb21af1026317d6e57be	-1
20261004120001	host_virtualization_interface_validate	2026-11-13 00:32:14.680896+00	t	\\xf03d5b6826e7aca5a5efe1ab57878591bfd12ac5c097b1d71e5539348cf9a5e794c2ef19c5fa3ee8aac039920b9f1f4e	-1
20261004120002	host_virtualization_interface_index	2026-11-13 00:32:14.684377+00	t	\\x64e39691a98e9ed761812e1bfdeaa7b8d9aa13eb0401a39fb40dd36f21222ac47dad7f1b4ff9465a3cd0da1af42039ee	-1
20261005120000	rename_networks_to_sites	2026-11-13 00:32:14.702148+00	t	\\x5e08911ddc549e9d522eece781caa57fe42ee4cb6f7d16fb25b63bf6c7f95e897d2cbd12db345040f5a4035d420ace43	-1
20261012130000	discovery_warnings	2026-11-13 00:32:14.813479+00	t	\\xc760d410eaa7d4d14214bf602cd9ba7423c1a328b71768d79bcef0cee51b9b685c37438f74091fb9c3afd6483a98e46c	-1
20261013120000	license_instances	2026-11-13 00:32:14.820972+00	t	\\x86796dc3b14fc5805b84f6c6fd92fa8aaa560a004ea4ca8f4affa76f55b830f8d2403393cddfcf215070e84d499bb5cb	-1
20261013140000	terms_notice_version	2026-11-13 00:32:14.822693+00	t	\\x624bbb290511d7491b8816b152b06194417047e2b9b19eeb8b1d954ec17e6fee77c73d9fd19fb74d62f8c44d66bb1a93	-1
20261019120000	dependency_members_keep_on_delete	2026-11-13 00:32:14.826831+00	t	\\x2391912be8b3a07910ba2240ffe21065efe5836453ee2ba9ee7ca5fda5dea58d701795486c7e03a70909c88e5cb5d595	-1
20261021120000	vlan_and_subnet_sources	2026-11-13 00:32:14.830117+00	t	\\x2535fce0c0903904a1d9bfa86132278e9238231438d3b15be9863c47a362c284cc26d4b4ecb7830ee128a004d3d99878	-1
20261022120000	credential_needs_newer_daemon	2026-11-13 00:32:14.839442+00	t	\\xfcc14b3c03c4c930f7ffa6a0086ff9fe12d35c77ccc331b4469ebaf19ddcff05a1a21801b4f2c36fd7d14df5c403a1a1	-1
20261023120000	snmp_community_credential	2026-11-13 00:32:14.852462+00	t	\\x71e050db43cceee2a66e206cbd87f478cc8a6932440def65d5ea5c79f55ae7339bff4a9e7727b4bd7a9696dc535816cf	-1
20261024120000	topology_device_type_rule	2026-11-13 00:32:14.854601+00	t	\\xb5f60015b76437698dbd4381b285195bc4b866a3d20f1cfb588bd675b92aab51dedce82bcbc29749c5e04ed4e6050d87	-1
20261024120001	google_nest_capitalization	2026-11-13 00:32:14.857186+00	t	\\x204cc0286eea5cba45dded8756cd98207c511b505eef07ce0470fd35104d69092038adc091536ffb2dda65e288d73262	-1
20261025120000	every_org_has_a_plan	2026-11-13 00:32:14.862191+00	t	\\x403a5bebde12227dbbfa913490de00196184c947a72da3d86f39809db010406715561f96eac6839546713c7c9aaf90c4	-1
20261026120000	strip_discovery_run_type_warnings	2026-11-13 00:32:14.867033+00	t	\\xffd0edff777c52348517fd47b5b87eb9406eea878e1168f23c2660786bf1352031ebb04761ed43dc3e6f6886dc5d4f52	-1
20261027120000	hosts_matched_device_type	2026-11-13 00:32:14.870069+00	t	\\xe970e578c2fad70d71c4e1d4a882d5e477e6372d090d93dad467ab8f656738deff5354b05868b59d8c808a2bfb61d8bd	-1
20261108120000	organization_setup_and_entity_source	2026-11-13 00:32:14.921327+00	t	\\xb6a079ad40e031c99f12bb9f5e5911ea84716a1cefcc9429a3f04174acaea84c0851e87665eee8ec171c9f7c64dee5b8	-1
\.


--
-- Data for Name: api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.api_keys (id, key, site_id, name, created_at, updated_at, last_used, expires_at, is_enabled, plaintext, daemon_id) FROM stdin;
a272e050-a90e-4a44-ada6-5c2ec861c6d5	f9db849daef8d841908a605e11ed73186aa5569a3752716c4403fbb5cc138749	1acbe937-22e8-4c40-ae6a-76b4615931da	Compat Test API Key	2026-11-13 00:41:15.908475+00	2026-11-13 00:41:15.908475+00	2026-11-13 00:41:47.216383+00	\N	t	\N	\N
af241e68-2b76-4cf0-ae26-38e6de78d947	c96d078a349fe56f3f1f0a7c767b9c987492f55118e1bd9c0936d247aedc923c	1acbe937-22e8-4c40-ae6a-76b4615931da	scanopy-daemon API Key	2026-11-13 00:32:17.118801+00	2026-11-13 00:32:17.118801+00	2026-11-13 00:42:06.262591+00	\N	t	\N	67da0afb-e420-4b41-8ec3-e805ac2e9681
048dc7fe-9cb7-4ae8-a470-0d0a584e4dbe	95b665d1cfa039d456b2fb878f3b8a685658e69518e7c0e2c185be1db34c6a5a	1acbe937-22e8-4c40-ae6a-76b4615931da	scanopy-daemon-serverpoll API Key	2026-11-13 00:33:55.967447+00	2026-11-13 00:33:55.967447+00	2026-11-13 00:42:09.295819+00	\N	t	scp_d_fLJmCN1kFzSA6gQkrw01EF9qZhT12C5X	68fcb628-c5a1-4c16-9290-ac9cc9f0acc5
\.


--
-- Data for Name: bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.bindings (id, site_id, service_id, binding_type, ip_address_id, port_id, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
116ba20d-e20d-4619-9b95-5466171405f3	1acbe937-22e8-4c40-ae6a-76b4615931da	a6bceb85-8a66-414b-93ad-e7a855b019b8	Port	81412bd9-bca5-4e10-9ab3-ddba0f10d91b	59eca2e1-5650-44fc-bcc0-a436c484f2da	2026-03-15 19:34:42.017404+00	2026-03-15 19:34:42.017404+00	2026-11-13 00:41:54.321468+00	\N	\N	2026-11-13 00:41:54.321471+00	\N	\N	\N
\.


--
-- Data for Name: credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.credentials (id, organization_id, name, credential_type, created_at, updated_at, daemon_os, description, login_credential_id, source) FROM stdin;
b4a4ea58-1337-4585-b46b-a52224e4b49a	60fc216c-ed5f-471d-9b98-5905ff49bff0	SNMP (public)	{"type": "SnmpCommunity", "community": {"mode": "Inline", "value": "public"}}	2026-11-13 00:32:16.947474+00	2026-11-13 00:32:16.947474+00	\N	Assigned to every new site. Tries the SNMP community "public" on every host, over v2c and then v1.	\N	{"type": "System"}
b8e335ac-7e05-4f7d-9f7c-8028d0b454c7	60fc216c-ed5f-471d-9b98-5905ff49bff0	Docker socket	{"type": "DockerSocket"}	2026-11-13 00:32:16.951288+00	2026-11-13 00:32:16.951288+00	\N	Added to every new daemon. Reads the local Docker socket at its default path.	\N	{"type": "System"}
fe1eb50b-e326-4ebe-8ab7-1060c4b31bf8	60fc216c-ed5f-471d-9b98-5905ff49bff0	Podman socket	{"type": "PodmanSocket"}	2026-11-13 00:32:16.954488+00	2026-11-13 00:32:16.954488+00	\N	Added to every new daemon. Reads the local Podman socket at its default path.	\N	{"type": "System"}
\.


--
-- Data for Name: daemon_interfaced_subnets; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemon_interfaced_subnets (daemon_id, subnet_id) FROM stdin;
67da0afb-e420-4b41-8ec3-e805ac2e9681	4ad11d12-d29a-4f8d-87e9-dc2c288a785d
\.


--
-- Data for Name: daemons; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemons (id, site_id, host_id, created_at, last_seen, updated_at, mode, url, name, version, user_id, api_key_id, is_unreachable, standby, standby_cleared_at, os) FROM stdin;
67da0afb-e420-4b41-8ec3-e805ac2e9681	1acbe937-22e8-4c40-ae6a-76b4615931da	7e411a60-a1e9-45b3-a3cb-235dddeede18	2026-11-13 00:32:17.13766+00	2026-11-13 00:42:06.271447+00	2026-11-13 00:32:17.13766+00	"daemon_poll"		scanopy-daemon	0.20.0	f605e587-918a-411d-9fd8-9afed452c38c	af241e68-2b76-4cf0-ae26-38e6de78d947	f	f	\N	"linux"
68fcb628-c5a1-4c16-9290-ac9cc9f0acc5	1acbe937-22e8-4c40-ae6a-76b4615931da	18b6c6b1-34cf-4a4a-bee5-dbc2cdf46def	2026-11-13 00:33:55.976707+00	2026-11-13 00:42:09.30201+00	2026-11-13 00:33:55.976707+00	"server_poll"	http://daemon-serverpoll:60074	scanopy-daemon-serverpoll	0.20.0	f605e587-918a-411d-9fd8-9afed452c38c	048dc7fe-9cb7-4ae8-a470-0d0a584e4dbe	f	f	2026-11-13 00:42:09.302013+00	"linux"
\.


--
-- Data for Name: dependencies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.dependencies (id, site_id, name, description, created_at, updated_at, source, color, edge_style, dependency_type, member_type, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
55a05e65-2751-444c-9e42-80eb4fa62d04	1acbe937-22e8-4c40-ae6a-76b4615931da		\N	2026-11-13 00:41:15.053371+00	2026-11-13 00:41:15.053371+00	{"type": "Manual"}	Yellow	"Bezier"	RequestPath	Services	2026-11-13 00:41:15.053371+00	\N	\N	\N
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
4da13570-9884-487a-b8d4-0c1c1609139d	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "1e8cf04d-e26d-4e84-a90f-4a534c217faf", "started_at": "2026-10-02T13:54:28.548758981Z", "finished_at": "2026-11-13T00:41:16.807205Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:16.402757Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-02 13:54:28.548758+00	2026-11-13 00:41:16.808593+00	0	f	[]
c9b5b5e7-c6a2-473d-a7ee-50997bf5e091	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Scheduled", "enabled": true, "last_run": "2026-11-13T00:32:29.773364Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "7e411a60-a1e9-45b3-a3cb-235dddeede18", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-13 00:32:17.141311+00	2026-11-13 00:33:55.544938+00	1	f	[]
b16ef416-a8d0-44a0-8504-2ba0ebadf21d	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 100, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "3f148a32-bc73-4ae2-aa1c-99962a96f62e", "started_at": "2026-11-13T00:32:59.803265Z", "finished_at": "2026-11-13T00:33:55.483291Z", "discovery_id": "c9b5b5e7-c6a2-473d-a7ee-50997bf5e091", "daemon_version": "0.20.0", "discovery_type": {"type": "Unified", "host_id": "7e411a60-a1e9-45b3-a3cb-235dddeede18", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:33:55.535884Z", "hosts_discovered": 6, "last_activity_at": "2026-11-13T00:33:55.473Z", "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "7e411a60-a1e9-45b3-a3cb-235dddeede18", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-13 00:32:59.803265+00	2026-11-13 00:33:55.543479+00	0	f	[]
acf9ae73-6840-435a-a67e-145014c04891	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "890873b9-2054-463a-9fe8-11016f0d08f2", "started_at": "2026-07-29T04:17:17.562092815Z", "finished_at": "2026-11-13T00:41:17.312965Z", "discovery_id": null, "daemon_version": "0.17.17", "discovery_type": {"type": "Unified", "host_id": "f4930d3b-6bc0-42c1-b25c-bbf557bf56ce", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:16.828975Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f4930d3b-6bc0-42c1-b25c-bbf557bf56ce", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-29 04:17:17.562092+00	2026-11-13 00:41:17.314442+00	0	f	[]
3916357b-3b2e-4ec8-89d5-4879f3e1928b	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "f2d5414b-3369-4280-929d-5422dd11a4b4", "started_at": "2026-02-04T02:06:44.396505189Z", "finished_at": "2026-11-13T00:41:19.952998Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}, "last_update_at": "2026-11-13T00:41:19.018794Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}	Self Report — My Site	2026-02-04 02:06:44.396505+00	2026-11-13 00:41:19.95572+00	0	f	[]
2762c275-162d-473b-8354-1fb976597549	1acbe937-22e8-4c40-ae6a-76b4615931da	68fcb628-c5a1-4c16-9290-ac9cc9f0acc5	{"type": "Scheduled", "enabled": true, "last_run": "2026-11-13T00:34:14.975117Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "18b6c6b1-34cf-4a4a-bee5-dbc2cdf46def", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-13 00:33:55.97887+00	2026-11-13 00:38:14.999669+00	1	f	[]
d9f9aaa9-d118-411c-9545-a44fe2e7e79a	1acbe937-22e8-4c40-ae6a-76b4615931da	68fcb628-c5a1-4c16-9290-ac9cc9f0acc5	{"type": "AdHoc", "last_run": "2026-11-13T00:34:16.510893Z"}	{"type": "Unified", "host_id": "18b6c6b1-34cf-4a4a-bee5-dbc2cdf46def", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	ServerPoll Integration Test Discovery	2026-11-13 00:34:16.498724+00	2026-11-13 00:41:15.009873+00	1	f	[]
8e7cfc0f-3ee7-4b0e-8ced-d93fab6cd228	1acbe937-22e8-4c40-ae6a-76b4615931da	68fcb628-c5a1-4c16-9290-ac9cc9f0acc5	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 100, "warnings": [], "daemon_id": "68fcb628-c5a1-4c16-9290-ac9cc9f0acc5", "session_id": "0bef0edf-a2c6-415f-bb9a-730878ba57e2", "started_at": "2026-11-13T00:35:14.985009Z", "finished_at": "2026-11-13T00:37:45.815738Z", "discovery_id": "2762c275-162d-473b-8354-1fb976597549", "daemon_version": "0.20.0", "discovery_type": {"type": "Unified", "host_id": "18b6c6b1-34cf-4a4a-bee5-dbc2cdf46def", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:38:14.994533Z", "hosts_discovered": null, "last_activity_at": "2026-11-13T00:37:39.814Z", "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "18b6c6b1-34cf-4a4a-bee5-dbc2cdf46def", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-13 00:35:14.985009+00	2026-11-13 00:38:14.99884+00	0	f	[]
aaa30b39-86b2-4f45-aff2-90f5754ceea2	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "24dd8f5a-4866-4462-a9ff-b65cb5a3061a", "started_at": "2026-09-16T20:16:31.390119148Z", "finished_at": "2026-11-13T00:41:18.385648Z", "discovery_id": null, "daemon_version": "0.13.6", "discovery_type": {"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:17.963038Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 20:16:31.390119+00	2026-11-13 00:41:18.387089+00	0	f	[]
831b0bfb-656e-45f1-8b65-957adefbd28c	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "f5a833cc-002f-41e1-b540-c0a623d6775c", "started_at": "2026-01-25T23:12:10.196614920Z", "finished_at": "2026-11-13T00:41:19.960327Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644"}, "last_update_at": "2026-11-13T00:41:18.474425Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644"}	Self Report — My Site	2026-01-25 23:12:10.196614+00	2026-11-13 00:41:19.962817+00	0	f	[]
b8ab0553-9d42-4e88-a991-6229f355013a	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "afde3201-cb3d-4b85-bb8f-470b83cba2a3", "started_at": "2026-02-01T15:48:58.546419422Z", "finished_at": "2026-11-13T00:41:20.001349Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:18.525735Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-01 15:48:58.546419+00	2026-11-13 00:41:20.003792+00	0	f	[]
5264f734-7568-43ea-940d-b8aec457550a	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "6845dcc8-6ea0-47af-9ce3-054b8cf1f667", "started_at": "2026-02-04T02:06:44.655234805Z", "finished_at": "2026-11-13T00:41:20.007384Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}, "last_update_at": "2026-11-13T00:41:19.024804Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}	Self Report — My Site	2026-02-04 02:06:44.655234+00	2026-11-13 00:41:20.009806+00	0	f	[]
bd36b7d7-27b3-484f-911a-e5b0caea91a4	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "4273fe18-d031-4977-950e-ad5a49d2ea8e", "started_at": "2026-03-24T00:04:21.150690914Z", "finished_at": "2026-11-13T00:41:22.200024Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.701381Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:21.15069+00	2026-11-13 00:41:22.202854+00	0	f	[]
26992c74-a3f4-41ab-9ef5-5dd72ce21de2	1acbe937-22e8-4c40-ae6a-76b4615931da	68fcb628-c5a1-4c16-9290-ac9cc9f0acc5	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 100, "warnings": [], "daemon_id": "68fcb628-c5a1-4c16-9290-ac9cc9f0acc5", "session_id": "bd047269-5327-475d-8055-116e34581c77", "started_at": "2026-11-13T00:38:15.067848Z", "finished_at": "2026-11-13T00:40:45.608840Z", "discovery_id": "d9f9aaa9-d118-411c-9545-a44fe2e7e79a", "daemon_version": "0.20.0", "discovery_type": {"type": "Unified", "host_id": "18b6c6b1-34cf-4a4a-bee5-dbc2cdf46def", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:15.004029Z", "hosts_discovered": null, "last_activity_at": "2026-11-13T00:40:39.607Z", "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "18b6c6b1-34cf-4a4a-bee5-dbc2cdf46def", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	ServerPoll Integration Test Discovery	2026-11-13 00:38:15.067848+00	2026-11-13 00:41:15.009002+00	0	f	[]
b7a342c2-96a5-48b7-b14b-14eba64f36e4	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "93db1812-9fa5-4c4b-a36b-d687b363811c", "started_at": "2026-10-02T13:50:58.545317669Z", "finished_at": "2026-11-13T00:41:16.801935Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:16.396526Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-02 13:50:58.545317+00	2026-11-13 00:41:16.803364+00	0	f	[]
84fa36d3-8f0e-4c90-a9f9-625c6f2a9c0f	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "b368c96b-6dbf-4523-bcf9-fee0229c2dfb", "started_at": "2026-10-02T13:49:47.412762700Z", "finished_at": "2026-11-13T00:41:16.804684Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "Unified", "host_id": "6aeae378-cd4b-493a-8bff-471792dff268", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:16.307289Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6aeae378-cd4b-493a-8bff-471792dff268", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-02 13:49:47.412762+00	2026-11-13 00:41:16.806116+00	0	f	[]
be598f6f-c9b5-493d-af49-14d1355855b2	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "30c1790e-6f3b-44f7-87be-9491e456edd0", "started_at": "2026-07-29T04:18:51.683205577Z", "finished_at": "2026-11-13T00:41:17.316516Z", "discovery_id": null, "daemon_version": "0.17.17", "discovery_type": {"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:16.885049Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-29 04:18:51.683205+00	2026-11-13 00:41:17.3179+00	0	f	[]
c861efd0-6e2e-408a-aee5-3f5a14a9414c	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "e7c3d9e2-4cad-4942-98d6-ae6053485366", "started_at": "2026-03-11T06:19:24.616705132Z", "finished_at": "2026-11-13T00:41:17.321779Z", "discovery_id": null, "daemon_version": "0.17.17", "discovery_type": {"type": "SelfReport", "host_id": "8c99dc12-cae3-41f2-a550-be436a112a35"}, "last_update_at": "2026-11-13T00:41:16.897050Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8c99dc12-cae3-41f2-a550-be436a112a35"}	Self Report — My Site	2026-03-11 06:19:24.616705+00	2026-11-13 00:41:17.324654+00	0	f	[]
c01565a9-3987-4b03-b7b3-dc3ae95e7a5f	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "b5073709-1dc4-4f69-bab1-d17406b1b571", "started_at": "2026-07-29T04:22:21.659954694Z", "finished_at": "2026-11-13T00:41:17.319210Z", "discovery_id": null, "daemon_version": "0.17.17", "discovery_type": {"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:16.891160Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-29 04:22:21.659954+00	2026-11-13 00:41:17.320637+00	0	f	[]
afc8abb2-6cb0-410d-a0a1-ba52962b0a3d	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "44164544-aa86-4db3-ad9a-503d5933df88", "started_at": "2026-09-24T21:05:01.816315928Z", "finished_at": "2026-11-13T00:41:17.837165Z", "discovery_id": null, "daemon_version": "0.17.16", "discovery_type": {"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:17.430416Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-24 21:05:01.816315+00	2026-11-13 00:41:17.838893+00	0	f	[]
2e8e193f-d51d-47cd-80e6-23c1826ec91d	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "b64df0ca-f173-4ba8-a48b-edf7e372974a", "started_at": "2026-02-04T02:06:45.712590434Z", "finished_at": "2026-11-13T00:41:19.967640Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}, "last_update_at": "2026-11-13T00:41:19.037078Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}	Self Report — My Site	2026-02-04 02:06:45.71259+00	2026-11-13 00:41:19.970177+00	0	f	[]
f9eda19d-59ee-49c6-96fd-a18db75f1d0a	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "075ee07b-1779-47db-8c94-50c17ce5f66d", "started_at": "2026-10-01T04:03:53.917292009Z", "finished_at": "2026-11-13T00:41:19.974797Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:19.540333Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-01 04:03:53.917292+00	2026-11-13 00:41:19.976188+00	0	f	[]
e2ffc699-416d-4310-af90-565474642ebb	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "0ffe5496-d64e-41a0-ab86-ff28eed28819", "started_at": "2026-02-04T02:00:38.542462976Z", "finished_at": "2026-11-13T00:41:19.979711Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:19.006238Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-04 02:00:38.542462+00	2026-11-13 00:41:19.982317+00	0	f	[]
5c4d2f00-815b-4bd0-804b-90485bb2effd	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "c063e961-df38-4fbb-b437-0e6e25593003", "started_at": "2026-09-24T21:00:16.922463583Z", "finished_at": "2026-11-13T00:41:17.840560Z", "discovery_id": null, "daemon_version": "0.17.16", "discovery_type": {"type": "Unified", "host_id": "ad3c2bd6-9bf9-4c4d-8534-3689c0e013c3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:17.336495Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "ad3c2bd6-9bf9-4c4d-8534-3689c0e013c3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-24 21:00:16.922463+00	2026-11-13 00:41:17.842097+00	0	f	[]
2a7db917-79d2-446f-b72f-f1ec83a327b4	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "3dec6b93-d9d7-46ab-94e2-ef221564d563", "started_at": "2026-09-24T21:01:31.815231601Z", "finished_at": "2026-11-13T00:41:17.843515Z", "discovery_id": null, "daemon_version": "0.17.16", "discovery_type": {"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:17.424008Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-24 21:01:31.815231+00	2026-11-13 00:41:17.845459+00	0	f	[]
5cc21665-fd59-4f08-aabc-2801183156c4	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "68690167-4bf9-40db-9756-207731c78629", "started_at": "2026-09-16T19:13:38.792500966Z", "finished_at": "2026-11-13T00:41:18.388323Z", "discovery_id": null, "daemon_version": "0.13.6", "discovery_type": {"type": "Unified", "host_id": "fe9f2b88-655a-44e9-b2b2-b89b797b34ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:17.969192Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "fe9f2b88-655a-44e9-b2b2-b89b797b34ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 19:13:38.7925+00	2026-11-13 00:41:18.389752+00	0	f	[]
2bfe87cd-c9ed-41b9-989b-6c249f6df055	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "466f3f06-3335-4195-94cd-7b1b85eceda4", "started_at": "2026-02-01T15:39:35.137282092Z", "finished_at": "2026-11-13T00:41:19.971309Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "d6e89254-a0d6-418b-9678-e75d11712fd1"}, "last_update_at": "2026-11-13T00:41:18.409564Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "d6e89254-a0d6-418b-9678-e75d11712fd1"}	Self Report — My Site	2026-02-01 15:39:35.137282+00	2026-11-13 00:41:19.973826+00	0	f	[]
91baf408-071e-4fb9-8d3f-5dca424820e4	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "1a182cd9-9710-482e-8645-d95c40db018a", "started_at": "2026-01-26T14:03:24.338877430Z", "finished_at": "2026-11-13T00:41:19.983307Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "7891ed81-377c-4eca-b05e-bc8a17129f90"}, "last_update_at": "2026-11-13T00:41:19.012374Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "7891ed81-377c-4eca-b05e-bc8a17129f90"}	Self Report — My Site	2026-01-26 14:03:24.338877+00	2026-11-13 00:41:19.98586+00	0	f	[]
acd54ac5-5e24-4863-bf12-3f5e16e9f5ae	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "7f54ecab-3771-4583-a7d0-f12569030e17", "started_at": "2026-02-04T02:06:45.451848584Z", "finished_at": "2026-11-13T00:41:19.990821Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}, "last_update_at": "2026-11-13T00:41:19.031081Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}	Self Report — My Site	2026-02-04 02:06:45.451848+00	2026-11-13 00:41:19.993356+00	0	f	[]
88f0ec08-b93a-476f-905d-368246cf93e2	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "64824f2d-3081-44a6-a631-23dbe65e4721", "started_at": "2026-09-16T20:11:28.193921546Z", "finished_at": "2026-11-13T00:41:18.380337Z", "discovery_id": null, "daemon_version": "0.13.6", "discovery_type": {"type": "Unified", "host_id": "505ad895-0816-4969-a72c-161ebc636002", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:17.867578Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "505ad895-0816-4969-a72c-161ebc636002", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 20:11:28.193921+00	2026-11-13 00:41:18.381745+00	0	f	[]
63920597-8f5a-4324-bf5e-c5560a16d011	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "4e1e43c0-b63f-41df-ba3a-71683fef4763", "started_at": "2026-02-04T01:54:22.209258404Z", "finished_at": "2026-11-13T00:41:19.963830Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:18.999973Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-04 01:54:22.209258+00	2026-11-13 00:41:19.966503+00	0	f	[]
8c75edd4-4a97-4800-a28e-a879d4010fb2	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "cbdca757-31a8-4a33-b45a-5ad52b4de60b", "started_at": "2026-10-01T04:02:39.535834801Z", "finished_at": "2026-11-13T00:41:19.977285Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Unified", "host_id": "bd69aa7e-1ffb-4e1d-8ac6-adc468be6771", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:19.451337Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bd69aa7e-1ffb-4e1d-8ac6-adc468be6771", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-01 04:02:39.535834+00	2026-11-13 00:41:19.978659+00	0	f	[]
1238e81b-6cfb-4fcf-b2b2-d87484da447f	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "969d115c-0112-4ba3-8757-bb1ece340112", "started_at": "2026-03-24T00:04:20.606954763Z", "finished_at": "2026-11-13T00:41:22.204200Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.690084Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:20.606954+00	2026-11-13 00:41:22.206953+00	0	f	[]
5c30f4ea-de28-4a40-8f8a-af9e1f9e3214	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "3b0868af-d558-45e3-b688-2cf55472b6ee", "started_at": "2026-03-24T00:04:24.139743985Z", "finished_at": "2026-11-13T00:41:22.245535Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.768776Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:24.139743+00	2026-11-13 00:41:22.248047+00	0	f	[]
2c838b25-ae0b-4212-b6aa-4e93cb229a24	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "e0ea6622-5f78-4cc3-b1f9-257946754dd8", "started_at": "2026-03-09T05:29:17.642983004Z", "finished_at": "2026-03-09T05:29:17.643045281Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:22.445460Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-09 05:29:17.642983+00	2026-11-13 00:41:22.448157+00	0	f	[]
6c2fd6a9-6437-4533-9c09-2b2a6f4035e0	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "ca5b8138-b6c5-41bb-928c-59ab96c38227", "started_at": "2026-09-16T20:13:01.385050881Z", "finished_at": "2026-11-13T00:41:18.383046Z", "discovery_id": null, "daemon_version": "0.13.6", "discovery_type": {"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:17.956965Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 20:13:01.38505+00	2026-11-13 00:41:18.384424+00	0	f	[]
6f48d6bd-a0be-4980-b737-7538318b61a9	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "6b771cb0-0931-4c07-a677-efe6aa5bc5bd", "started_at": "2026-02-01T15:40:05.145636842Z", "finished_at": "2026-11-13T00:41:19.956783Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:18.468121Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-01 15:40:05.145636+00	2026-11-13 00:41:19.959286+00	0	f	[]
d94265f7-7b40-4f79-b56d-616c562b365b	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "5b19fece-cb82-45d8-a676-df53cc38a014", "started_at": "2026-02-01T15:46:28.576568381Z", "finished_at": "2026-11-13T00:41:19.997888Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}, "last_update_at": "2026-11-13T00:41:18.510304Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}	Self Report — My Site	2026-02-01 15:46:28.576568+00	2026-11-13 00:41:20.000349+00	0	f	[]
8ab8a849-990c-48ce-aed6-fb5bd18cf4e8	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "c540d032-e83e-4050-ab10-97635f0968a2", "started_at": "2026-10-01T04:07:23.928243114Z", "finished_at": "2026-11-13T00:41:20.004815Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:19.546562Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-01 04:07:23.928243+00	2026-11-13 00:41:20.006305+00	0	f	[]
9f873d2e-4fce-4d7b-9f36-c7e5d89f35b3	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "5e741620-3de9-476c-86e7-7ce656d0a5a8", "started_at": "2026-03-24T00:04:23.604406719Z", "finished_at": "2026-11-13T00:41:22.215355Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.756503Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:23.604406+00	2026-11-13 00:41:22.217968+00	0	f	[]
740642eb-fda4-4846-b5dd-ab87b0011718	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "bbe36108-56c4-4d82-9087-e577842ff202", "started_at": "2026-03-24T00:04:22.515854945Z", "finished_at": "2026-11-13T00:41:22.256276Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.732128Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:22.515854+00	2026-11-13 00:41:22.258833+00	0	f	[]
d6ff385f-a34b-4959-a13c-11b76356f2e3	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "927175d4-7258-4b40-a656-8412141305c5", "started_at": "2026-02-04T01:53:52.212090885Z", "finished_at": "2026-11-13T00:41:19.987Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "e0313f98-eeac-4055-beb3-7012a7bd7510"}, "last_update_at": "2026-11-13T00:41:18.942410Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "e0313f98-eeac-4055-beb3-7012a7bd7510"}	Self Report — My Site	2026-02-04 01:53:52.21209+00	2026-11-13 00:41:19.989614+00	0	f	[]
1660c366-b0d3-4ac5-8f6e-a8933d7a53fa	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "56d2df9f-197b-49f8-b4ac-e9d4674dca7a", "started_at": "2026-10-07T13:08:30.855227146Z", "finished_at": "2026-11-13T00:41:21.027627Z", "discovery_id": null, "daemon_version": "0.18.2", "discovery_type": {"type": "Unified", "host_id": "1be4e31e-a229-440d-b0f4-20c31ee11254", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:20.523563Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "1be4e31e-a229-440d-b0f4-20c31ee11254", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-07 13:08:30.855227+00	2026-11-13 00:41:21.028968+00	0	f	[]
60a3d593-ce95-45ee-bba3-0a6550b346fb	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "be760657-cb8b-4175-8484-00ea5633e0a3", "started_at": "2026-10-15T03:33:48.001242310Z", "finished_at": "2026-11-13T00:41:21.553967Z", "discovery_id": null, "daemon_version": "0.15.0", "discovery_type": {"type": "Unified", "host_id": "6943745c-a092-47e4-9991-74ba816a3f6a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:21.043517Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6943745c-a092-47e4-9991-74ba816a3f6a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:33:48.001242+00	2026-11-13 00:41:21.555476+00	0	f	[]
bd4ce261-b2cb-472c-bcc1-549374c63e0e	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "857fbace-0358-4642-8ca0-b14bb999f353", "started_at": "2026-03-24T00:04:21.968361234Z", "finished_at": "2026-11-13T00:41:22.211726Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.719282Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:21.968361+00	2026-11-13 00:41:22.214352+00	0	f	[]
d43f7f21-1a9c-4b11-9b71-9d8c7290fe1e	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "5bd5e93e-a606-4c6e-b159-b04879bdd801", "started_at": "2026-03-24T00:04:23.332055633Z", "finished_at": "2026-11-13T00:41:22.252523Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.750303Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:23.332055+00	2026-11-13 00:41:22.255171+00	0	f	[]
ff3a3072-9e56-4cb2-a322-e68a0d46c978	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "d03e3468-38c8-46e6-bd30-43693e9db1d7", "started_at": "2026-03-09T05:16:11.595722227Z", "finished_at": "2026-11-13T00:41:23.302351Z", "discovery_id": null, "daemon_version": "0.20.0", "discovery_type": {"type": "SelfReport", "host_id": "3ef5ad9b-872d-4fc7-8f8f-e5c0186d1841"}, "last_update_at": "2026-11-13T00:41:22.278678Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "3ef5ad9b-872d-4fc7-8f8f-e5c0186d1841"}	Self Report — My Site	2026-03-09 05:16:11.595722+00	2026-11-13 00:41:23.305639+00	0	f	[]
f1e01ef1-8496-4e2c-8621-5000f95294bb	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "c8c6530c-aaff-4d7c-a872-ba4457906d77", "started_at": "2026-02-04T02:06:45.972533343Z", "finished_at": "2026-11-13T00:41:19.994357Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}, "last_update_at": "2026-11-13T00:41:19.043085Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}	Self Report — My Site	2026-02-04 02:06:45.972533+00	2026-11-13 00:41:19.996881+00	0	f	[]
2ec77abe-b77a-4413-aaba-76b93f72b568	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "0400691f-2af1-4b20-bade-e50044271bfe", "started_at": "2026-10-07T13:13:08.735400435Z", "finished_at": "2026-11-13T00:41:21.030018Z", "discovery_id": null, "daemon_version": "0.18.2", "discovery_type": {"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:20.611101Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-07 13:13:08.7354+00	2026-11-13 00:41:21.031392+00	0	f	[]
b44079d1-8388-450d-8bdf-05782db7c82d	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "34c804c2-c01c-4104-a3b3-e31ae6dd0b6f", "started_at": "2026-03-24T00:04:19.801043433Z", "finished_at": "2026-11-13T00:41:22.218926Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.678685Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:19.801043+00	2026-11-13 00:41:22.221422+00	0	f	[]
561563be-8a39-4a59-813f-cc2467c9a9a7	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "18c3c94f-80cb-48e9-aedc-76dbc938706e", "started_at": "2026-03-31T18:35:21.063363405Z", "finished_at": "2026-11-13T00:41:24.986068Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Unified", "host_id": "f5245920-25e1-4d8b-b567-0f935961690d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:24.453711Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f5245920-25e1-4d8b-b567-0f935961690d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:35:21.063363+00	2026-11-13 00:41:24.987416+00	0	f	[]
fbde9ec4-c3d9-4353-ae5f-22871f0e2e1c	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "d2c685dd-56ad-4d49-9d36-c64b0483a15a", "started_at": "2026-07-01T17:49:49.938683105Z", "finished_at": "2026-11-13T00:41:27.365580Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "5a5b21eb-2566-4e13-83d8-00cb2675bde6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:26.784742Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "5a5b21eb-2566-4e13-83d8-00cb2675bde6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:49:49.938683+00	2026-11-13 00:41:27.36716+00	0	f	[]
3ebc168b-816b-4079-92a8-44f6f3975e39	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "9d715165-15d9-4c53-8eeb-adadd14656ec", "started_at": "2026-10-26T16:01:31.798722266Z", "finished_at": "2026-11-13T00:41:20.509624Z", "discovery_id": null, "daemon_version": "0.17.22", "discovery_type": {"type": "Unified", "host_id": "9b66035e-43fc-4de9-a9fa-9febda413452", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:20.022304Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9b66035e-43fc-4de9-a9fa-9febda413452", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-26 16:01:31.798722+00	2026-11-13 00:41:20.511133+00	0	f	[]
3aaa4cc1-ed27-4b81-9815-055a0f083a0b	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "ec9dc330-67d0-4ce5-94d8-506859c74940", "started_at": "2026-03-24T00:04:23.061291004Z", "finished_at": "2026-11-13T00:41:22.208092Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.744365Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:23.061291+00	2026-11-13 00:41:22.210744+00	0	f	[]
035a545c-48cf-47c3-891c-f3834c428871	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "c10d2eae-1267-4023-997e-5a51fdf97281", "started_at": "2026-03-24T00:04:18.984158426Z", "finished_at": "2026-11-13T00:41:22.248998Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.660290Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:18.984158+00	2026-11-13 00:41:22.251542+00	0	f	[]
b5bcf12d-7778-499c-923b-9e314bdbcaa8	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "ef1a4d64-afd3-4762-b6ae-2ff80e8dbc2f", "started_at": "2026-03-26T01:38:51.825984709Z", "finished_at": "2026-11-13T00:41:25.637880Z", "discovery_id": null, "daemon_version": "0.14.7", "discovery_type": {"type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:25.135718Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-26 01:38:51.825984+00	2026-11-13 00:41:25.639507+00	0	f	[]
54294e47-c2d5-4494-920d-3e6d149029f8	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "e8b2155f-5622-4046-aff2-4f8de2f868f0", "started_at": "2026-02-23T01:47:07.653242727Z", "finished_at": "2026-11-13T00:41:26.665961Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "SelfReport", "host_id": "49b8f936-67c8-4835-bc8c-5d897e43cdb0"}, "last_update_at": "2026-11-13T00:41:25.663178Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "49b8f936-67c8-4835-bc8c-5d897e43cdb0"}	Self Report — My Site	2026-02-23 01:47:07.653242+00	2026-11-13 00:41:26.668726+00	0	f	[]
ec226a33-fc2a-490b-bcb5-cf9eff51c9e9	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "0073f594-456f-4861-8285-c8eb1345c48c", "started_at": "2026-10-25T01:45:04.782037127Z", "finished_at": "2026-11-13T00:41:26.673446Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Unified", "host_id": "2d0259ed-6b96-4930-847a-fa3ebdbf38a3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:26.179106Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2d0259ed-6b96-4930-847a-fa3ebdbf38a3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-25 01:45:04.782037+00	2026-11-13 00:41:26.674914+00	0	f	[]
e25d1f64-73d9-4d99-9784-e5c2c928ee8b	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "e903943a-3b77-43c1-a9ad-824cb543557b", "started_at": "2026-10-07T13:09:38.731246701Z", "finished_at": "2026-11-13T00:41:21.024955Z", "discovery_id": null, "daemon_version": "0.18.2", "discovery_type": {"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:20.605025Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-07 13:09:38.731246+00	2026-11-13 00:41:21.026432+00	0	f	[]
0032c953-6f95-4f7f-b2ca-beebd8f2b2ed	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "ae2ef896-c9ee-4c7d-92d0-2e940d717f51", "started_at": "2026-10-15T03:35:10.733740412Z", "finished_at": "2026-11-13T00:41:21.556535Z", "discovery_id": null, "daemon_version": "0.15.0", "discovery_type": {"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:21.131497Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:35:10.73374+00	2026-11-13 00:41:21.557936+00	0	f	[]
34acb6dc-3cc6-40b2-9b90-7eee98bdafc5	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "59fba01e-fefd-4c66-9ef2-a85c0e76a811", "started_at": "2026-03-24T00:04:22.788677537Z", "finished_at": "2026-11-13T00:41:22.226373Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.738183Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:22.788677+00	2026-11-13 00:41:22.229109+00	0	f	[]
304f5fae-bab0-4d28-8a33-cbd79e725769	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "10fdd8f4-03b6-44ea-adb6-27e74136b365", "started_at": "2026-03-24T00:04:20.878919695Z", "finished_at": "2026-11-13T00:41:22.232797Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.695782Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:20.878919+00	2026-11-13 00:41:22.235321+00	0	f	[]
cccf0dd7-c6c5-4395-92db-b438070cd04e	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "27ff5779-08b0-4970-aebd-04649d27c725", "started_at": "2026-03-24T00:04:21.695856405Z", "finished_at": "2026-11-13T00:41:22.238600Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.713239Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:21.695856+00	2026-11-13 00:41:22.241026+00	0	f	[]
ea5dfdf5-0e28-47c7-a16b-fbf0a6c34721	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "3f148a32-bc73-4ae2-aa1c-99962a96f62e", "started_at": "2026-11-13T00:32:59.803265Z", "finished_at": "2026-11-13T00:41:24.321648Z", "discovery_id": null, "daemon_version": "0.15.4", "discovery_type": {"type": "Unified", "host_id": "7e411a60-a1e9-45b3-a3cb-235dddeede18", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:23.344913Z", "hosts_discovered": null, "last_activity_at": "2026-11-13T00:32:59.803Z", "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "7e411a60-a1e9-45b3-a3cb-235dddeede18", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-13 00:32:59.803265+00	2026-11-13 00:41:24.32316+00	0	f	[]
21fc12d3-c132-4868-a909-47571edbf5e6	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "26c0ac92-5d7b-41b7-969c-9e07d17bd1be", "started_at": "2026-10-15T03:38:40.733230767Z", "finished_at": "2026-11-13T00:41:21.551184Z", "discovery_id": null, "daemon_version": "0.15.0", "discovery_type": {"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:21.138154Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:38:40.73323+00	2026-11-13 00:41:21.552673+00	0	f	[]
343f8e56-ce89-445c-911e-b5ea38a3e34a	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "22ea697f-065c-4d2e-a81f-0809764aad01", "started_at": "2026-03-24T00:04:22.243245356Z", "finished_at": "2026-11-13T00:41:22.222537Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.725974Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:22.243245+00	2026-11-13 00:41:22.22517+00	0	f	[]
88783315-d407-4d20-abf7-fba5d9e80eb3	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "d2b26f9b-0329-4191-8a60-58cf3c796dba", "started_at": "2026-03-24T00:00:06.445031185Z", "finished_at": "2026-11-13T00:41:22.230129Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:21.630656Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-24 00:00:06.445031+00	2026-11-13 00:41:22.231595+00	0	f	[]
f2caf589-6a7c-4dee-a3d2-6b0460454747	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "f071713f-3367-4111-8f7b-7b7b50d33aa9", "started_at": "2026-03-23T23:57:20.088532334Z", "finished_at": "2026-11-13T00:41:22.236204Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "Unified", "host_id": "14fcf158-caea-47c9-82ad-38dcc2907791", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:21.578764Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "14fcf158-caea-47c9-82ad-38dcc2907791", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-23 23:57:20.088532+00	2026-11-13 00:41:22.237613+00	0	f	[]
5bb68b9a-56fa-479a-ad9a-7bbf86e33a0d	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "2abdbee0-ac03-472c-b137-4321f33f03ca", "started_at": "2026-03-31T18:35:25.967010190Z", "finished_at": "2026-11-13T00:41:24.977430Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:24.556520Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:35:25.96701+00	2026-11-13 00:41:24.978909+00	0	f	[]
f5687a89-cb68-4f22-9ce8-af80aa0f9512	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "abba33fb-bf1f-4e8b-985f-6a2d0b5d0380", "started_at": "2026-03-24T00:04:21.423333702Z", "finished_at": "2026-11-13T00:41:22.241968Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-13T00:41:21.707180Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:21.423333+00	2026-11-13 00:41:22.244434+00	0	f	[]
e924ffed-43be-45c6-b2be-2a6008e73544	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "d53d5b11-b914-4af1-bf04-5d6e47899de1", "started_at": "2026-01-23T03:40:39.007214962Z", "finished_at": "2026-11-13T00:41:24.317763Z", "discovery_id": null, "daemon_version": "0.15.4", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:23.897012Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-23 03:40:39.007214+00	2026-11-13 00:41:24.320378+00	0	f	[]
0d720d63-867a-4d70-ba29-d30a0ec594f9	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "1164516e-9bd1-4a5c-8fe8-182c9c6923d8", "started_at": "2026-03-31T18:30:47.557400890Z", "finished_at": "2026-11-13T00:41:24.991431Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Unified", "host_id": "52e0ccf9-3b92-408c-9a34-1bd2f053d047", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:24.344484Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "52e0ccf9-3b92-408c-9a34-1bd2f053d047", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:30:47.5574+00	2026-11-13 00:41:24.99288+00	0	f	[]
90804bac-235e-41f0-8532-020b184fa29c	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "5b2fda95-677f-4f93-a93f-daca41bf3af3", "started_at": "2026-07-01T17:45:06.329649066Z", "finished_at": "2026-11-13T00:41:27.362980Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "e56072d7-8284-4090-9a48-d04892a27b80", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:26.695439Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e56072d7-8284-4090-9a48-d04892a27b80", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:45:06.329649+00	2026-11-13 00:41:27.364476+00	0	f	[]
96b7b729-6fde-4eb2-ac35-e98c2c5af824	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "ed48cfed-4a98-4473-9c83-33781243c790", "started_at": "2026-03-09T05:16:41.592520993Z", "finished_at": "2026-11-13T00:41:23.307159Z", "discovery_id": null, "daemon_version": "0.20.0", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:22.336295Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-09 05:16:41.59252+00	2026-11-13 00:41:23.310619+00	0	f	[]
c3c9e334-2567-4139-8433-41eced351ef9	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "b049b806-80fc-4dc5-9400-f19f9a9857ff", "started_at": "2026-07-01T17:49:54.577783598Z", "finished_at": "2026-11-13T00:41:27.370986Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "37d3a1d8-b76b-4e2e-96e0-74362358f4f4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:26.884092Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "37d3a1d8-b76b-4e2e-96e0-74362358f4f4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:49:54.577783+00	2026-11-13 00:41:27.372613+00	0	f	[]
782a4ee6-c7f9-477d-bd3f-c06ab4dc8c4f	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "fe678d3a-c2b0-4ee1-a912-7c07b0b6e1d7", "started_at": "2026-07-01T17:49:50.486612447Z", "finished_at": "2026-11-13T00:41:27.377632Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "28b6b18e-69b9-450c-a795-d14a671bb34f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:26.798394Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "28b6b18e-69b9-450c-a795-d14a671bb34f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:49:50.486612+00	2026-11-13 00:41:27.379128+00	0	f	[]
0e2215be-fab6-4452-9978-3e75b09ea654	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "032af69a-4335-4c54-ac9c-8941c44050a3", "started_at": "2026-03-31T18:35:21.335253166Z", "finished_at": "2026-11-13T00:41:24.974748Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:24.460295Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:35:21.335253+00	2026-11-13 00:41:24.976158+00	0	f	[]
cd332424-1243-4285-b5b2-59f53d98b644	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "adfad9b0-e25d-4bd4-88bc-e0ecc0f3f4a4", "started_at": "2026-01-26T13:51:30.181086877Z", "finished_at": "2026-11-13T00:41:24.979964Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "SelfReport", "host_id": "01d97cc8-84d8-4806-877f-52bfd29791f8"}, "last_update_at": "2026-11-13T00:41:24.401067Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "01d97cc8-84d8-4806-877f-52bfd29791f8"}	Self Report — My Site	2026-01-26 13:51:30.181086+00	2026-11-13 00:41:24.982677+00	0	f	[]
201d73e1-8e50-4d61-b966-bf4ee706c408	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "cd493a46-8909-4378-b924-ae1650a0f5af", "started_at": "2026-03-26T01:31:45.021343211Z", "finished_at": "2026-11-13T00:41:25.640693Z", "discovery_id": null, "daemon_version": "0.14.7", "discovery_type": {"type": "Unified", "host_id": "711a5546-8640-4ee2-bf0b-8b268c4e2044", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:25.013560Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "711a5546-8640-4ee2-bf0b-8b268c4e2044", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-26 01:31:45.021343+00	2026-11-13 00:41:25.642109+00	0	f	[]
fe401701-e10a-40be-bbc6-20a33d658b71	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "2f915185-76ea-4a3a-8f81-7ca0298ae158", "started_at": "2026-03-31T18:35:19.970481609Z", "finished_at": "2026-11-13T00:41:24.983607Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Unified", "host_id": "2a69c4df-31b5-4490-adc9-01e591b9f974", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:24.429216Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2a69c4df-31b5-4490-adc9-01e591b9f974", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:35:19.970481+00	2026-11-13 00:41:24.985025+00	0	f	[]
ecc5b542-23bf-4b31-a2a7-348a477a0fee	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "c17fbe8e-8128-45b1-b2b5-afef7e2c082f", "started_at": "2026-02-23T01:47:37.649663614Z", "finished_at": "2026-11-13T00:41:26.669813Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:25.721005Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-23 01:47:37.649663+00	2026-11-13 00:41:26.672327+00	0	f	[]
dd4851c2-bd81-4568-8587-56afd18c6f76	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "73503118-cf7f-4008-881e-7f39b457bf47", "started_at": "2026-07-01T17:49:57.001482027Z", "finished_at": "2026-11-13T00:41:27.368307Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "e61c5740-38d0-46d2-a2d6-64ce68e6606f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:26.937044Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e61c5740-38d0-46d2-a2d6-64ce68e6606f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:49:57.001482+00	2026-11-13 00:41:27.369889+00	0	f	[]
9f697b43-749d-4d24-a2a4-ceeeae917199	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "e5274c5c-4478-40d5-86c6-aabbe025f093", "started_at": "2026-02-02T19:01:57.807520644Z", "finished_at": "2026-11-13T00:41:27.373725Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "SelfReport", "host_id": "37e5888d-4a4e-4ca3-908c-a8939cc75bdd"}, "last_update_at": "2026-11-13T00:41:26.754237Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "37e5888d-4a4e-4ca3-908c-a8939cc75bdd"}	Self Report — My Site	2026-02-02 19:01:57.80752+00	2026-11-13 00:41:27.376614+00	0	f	[]
e65ad772-17ba-4532-988a-67ac8de35227	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "6465c546-3856-4e33-9cd0-5b8a8d99d55b", "started_at": "2026-03-31T18:32:33.461505006Z", "finished_at": "2026-11-13T00:41:24.988620Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Unified", "host_id": "5ef6e4af-f8ab-48d8-9b0e-6f4b93a49ef0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:24.394469Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "5ef6e4af-f8ab-48d8-9b0e-6f4b93a49ef0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:32:33.461505+00	2026-11-13 00:41:24.990103+00	0	f	[]
82e8851e-e2aa-4890-adda-0a018d245f0c	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "bb0bdfdc-1516-4579-a727-079afeb571ea", "started_at": "2026-07-01T17:46:57.844855087Z", "finished_at": "2026-11-13T00:41:27.360239Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "23bba0c3-a3f9-49d7-ac3c-8696037b8b40", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:26.747606Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "23bba0c3-a3f9-49d7-ac3c-8696037b8b40", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:46:57.844855+00	2026-11-13 00:41:27.36178+00	0	f	[]
13f4bfd0-73da-49f3-b30f-10b2002a2849	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "34fe487d-22ef-418c-9099-42fa3376ded0", "started_at": "2026-07-01T17:49:52.939865021Z", "finished_at": "2026-11-13T00:41:27.356220Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "e9d19327-eb77-47c7-85d4-e590c7aa90a1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:26.847989Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e9d19327-eb77-47c7-85d4-e590c7aa90a1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:49:52.939865+00	2026-11-13 00:41:27.357851+00	0	f	[]
e1716322-a21f-4b7b-93ca-8b45e255a4d6	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "485c7d46-bc1d-4c32-a425-5904751efe34", "started_at": "2026-10-03T03:55:31.060040280Z", "finished_at": "2026-11-13T00:41:28.471556Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:28.052887Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-03 03:55:31.06004+00	2026-11-13 00:41:28.47298+00	0	f	[]
69a5b654-cf1b-4a40-a203-3822065a4493	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "a8a4a82b-01b9-4d9b-9bf2-cc44f54ff1d3", "started_at": "2026-10-03T03:54:08.695159586Z", "finished_at": "2026-11-13T00:41:28.474538Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Unified", "host_id": "0e0131ba-ba9b-4828-813f-4a8c181f5836", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:27.943276Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "0e0131ba-ba9b-4828-813f-4a8c181f5836", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-03 03:54:08.695159+00	2026-11-13 00:41:28.476691+00	0	f	[]
c52efa54-a6c3-4ad8-b60b-64f21cf1aece	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "20717e93-7bdb-4413-9112-9ba79c38289f", "started_at": "2026-10-03T03:59:01.064308022Z", "finished_at": "2026-11-13T00:41:28.478131Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:28.060114Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-03 03:59:01.064308+00	2026-11-13 00:41:28.479496+00	0	f	[]
66337ac5-c155-4a60-826e-6c46b84cac68	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "52a4d1e0-c3b9-4b20-a4e8-0196198a1c15", "started_at": "2026-02-02T14:20:51.227652182Z", "finished_at": "2026-11-13T00:41:28.480777Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "SelfReport", "host_id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182"}, "last_update_at": "2026-11-13T00:41:27.399808Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182"}	Self Report — My Site	2026-02-02 14:20:51.227652+00	2026-11-13 00:41:28.483342+00	0	f	[]
c3f91281-6662-42be-941d-9946b8aac727	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "bd93b206-4e22-4e0a-9dd0-0f38664a4247", "started_at": "2026-02-02T14:21:21.225991527Z", "finished_at": "2026-11-13T00:41:28.484306Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:27.459101Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-02 14:21:21.225991+00	2026-11-13 00:41:28.486907+00	0	f	[]
b7d9cdf0-1831-4727-8edd-ab3beb294d19	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "7c20860a-b07c-4c4f-81bc-c7f44bee7a3d", "started_at": "2026-02-02T14:29:34.148055772Z", "finished_at": "2026-11-13T00:41:28.488037Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:27.516081Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-02 14:29:34.148055+00	2026-11-13 00:41:28.490438+00	0	f	[]
abd299d9-4683-483a-b9a6-6a8337550dfb	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "feba9c5a-f04c-4e5c-97cd-994c4dffc382", "started_at": "2026-07-05T00:03:52.021458299Z", "finished_at": "2026-11-13T00:41:29.183519Z", "discovery_id": null, "daemon_version": "0.14.14", "discovery_type": {"type": "Unified", "host_id": "f1a03331-5149-4a21-91ed-42d9fd821651", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:28.572019Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f1a03331-5149-4a21-91ed-42d9fd821651", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-05 00:03:52.021458+00	2026-11-13 00:41:29.184933+00	0	f	[]
4600b553-ded7-4e36-8641-9ad8c183422a	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "2fdd805f-0e91-427a-a614-e15a2a15a0f0", "started_at": "2026-07-05T00:02:02.946186416Z", "finished_at": "2026-11-13T00:41:29.186549Z", "discovery_id": null, "daemon_version": "0.14.14", "discovery_type": {"type": "Unified", "host_id": "4ad32cd4-5017-4aa6-81fb-2a624a987ac2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:28.512069Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "4ad32cd4-5017-4aa6-81fb-2a624a987ac2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-05 00:02:02.946186+00	2026-11-13 00:41:29.188028+00	0	f	[]
5e51e2bd-eb86-457e-8bb6-92b9baf07d0f	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "937bc019-8b5b-4d82-b8f2-134c36a700a8", "started_at": "2026-07-05T00:06:52.008699875Z", "finished_at": "2026-11-13T00:41:29.189222Z", "discovery_id": null, "daemon_version": "0.14.14", "discovery_type": {"type": "Unified", "host_id": "bb6764eb-c535-449f-90e5-a950f9304f32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:28.790464Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bb6764eb-c535-449f-90e5-a950f9304f32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-05 00:06:52.008699+00	2026-11-13 00:41:29.190651+00	0	f	[]
e813fcd7-07ef-4f58-84ce-d8fd38548c09	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "4db08e28-b456-4951-bd43-585d26cf9421", "started_at": "2026-01-07T19:14:20.246263005Z", "finished_at": "2026-11-13T00:41:30.711951Z", "discovery_id": null, "daemon_version": "0.14.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:30.300509Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-07 19:14:20.246263+00	2026-11-13 00:41:30.71461+00	0	f	[]
2bea2c74-7856-4304-b818-93ad14d5a545	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "b43a3d5a-af6a-42f8-bf72-70c87206ab39", "started_at": "2026-07-22T23:04:06.688284640Z", "finished_at": "2026-11-13T00:41:30.716068Z", "discovery_id": null, "daemon_version": "0.14.2", "discovery_type": {"type": "Unified", "host_id": "ea19a9cd-5a65-4620-bff8-eb890602e187", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:29.795214Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "ea19a9cd-5a65-4620-bff8-eb890602e187", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-22 23:04:06.688284+00	2026-11-13 00:41:30.717484+00	0	f	[]
b5f3f74c-4adf-49b6-8aa3-edfa562fce44	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "b5591b69-0dfe-41fa-87c1-3d5e140cca15", "started_at": "2026-03-06T07:04:38.164982988Z", "finished_at": "2026-11-13T00:41:30.727846Z", "discovery_id": null, "daemon_version": "0.14.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:29.269169Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-06 07:04:38.164982+00	2026-11-13 00:41:30.730584+00	0	f	[]
fbe5e09e-1790-4df9-959a-8ca83af42dec	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "ee9f5e06-197c-4fbf-9d68-0ac7d7ec929e", "started_at": "2026-06-28T16:07:05.924226987Z", "finished_at": "2026-11-13T00:41:31.896528Z", "discovery_id": null, "daemon_version": "0.17.10", "discovery_type": {"type": "Unified", "host_id": "8d22037e-273c-4a29-9c5c-cdaaefe74227", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:31.326217Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8d22037e-273c-4a29-9c5c-cdaaefe74227", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:07:05.924226+00	2026-11-13 00:41:31.898224+00	0	f	[]
4232523c-3591-46e3-be36-fa6df3e28e57	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "06f71522-f4a4-4d87-9785-a442e797d994", "started_at": "2026-06-28T16:02:26.238749468Z", "finished_at": "2026-11-13T00:41:31.899664Z", "discovery_id": null, "daemon_version": "0.17.10", "discovery_type": {"type": "Unified", "host_id": "793047bd-2b38-4e62-8dd9-36ab1aa73797", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:31.229351Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "793047bd-2b38-4e62-8dd9-36ab1aa73797", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:02:26.238749+00	2026-11-13 00:41:31.901666+00	0	f	[]
0684784b-6f3d-4627-8229-915d56966dfc	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "d033c5f0-e137-4b0a-b42e-78daa430f113", "started_at": "2026-07-23T13:32:18.163549314Z", "finished_at": "2026-11-13T00:41:30.718786Z", "discovery_id": null, "daemon_version": "0.14.2", "discovery_type": {"type": "Unified", "host_id": "c37e16d2-78cb-4ad7-bffc-f8eaec8ac967", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:29.733198Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c37e16d2-78cb-4ad7-bffc-f8eaec8ac967", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-23 13:32:18.163549+00	2026-11-13 00:41:30.720333+00	0	f	[]
60e3021d-afdc-49aa-9031-f58381c9bb28	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "9c3d4777-a8a7-40f4-9070-c83a719cd6b6", "started_at": "2026-07-23T13:33:48.918734857Z", "finished_at": "2026-11-13T00:41:30.721501Z", "discovery_id": null, "daemon_version": "0.14.2", "discovery_type": {"type": "Unified", "host_id": "95fbda32-d081-4ef7-86c5-ac175f9995b6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:29.788990Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "95fbda32-d081-4ef7-86c5-ac175f9995b6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-23 13:33:48.918734+00	2026-11-13 00:41:30.722964+00	0	f	[]
bbf991d7-ac59-4a0f-b175-7608ebee47a8	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "6808c839-ea5d-4f6e-9662-60f24a6dd0c7", "started_at": "2026-03-06T07:04:08.169432273Z", "finished_at": "2026-11-13T00:41:30.724274Z", "discovery_id": null, "daemon_version": "0.14.2", "discovery_type": {"type": "SelfReport", "host_id": "07ca9676-4995-4fd4-b0a3-c60b0b356658"}, "last_update_at": "2026-11-13T00:41:29.210821Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "07ca9676-4995-4fd4-b0a3-c60b0b356658"}	Self Report — My Site	2026-03-06 07:04:08.169432+00	2026-11-13 00:41:30.726839+00	0	f	[]
a1319c2d-9ee0-42f7-84a7-575cda35ff22	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "ae3a3d90-03a9-4961-a1d4-840e661d29a8", "started_at": "2026-02-11T02:46:52.998653452Z", "finished_at": "2026-11-13T00:41:34.001823Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:32.503311Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-11 02:46:52.998653+00	2026-11-13 00:41:34.004496+00	0	f	[]
33ab47f4-d877-4525-8597-98ede2a2cced	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "eb250b58-76a9-4335-a813-c2d71a75e483", "started_at": "2026-02-18T17:07:18.061512748Z", "finished_at": "2026-11-13T00:41:34.008194Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "SelfReport", "host_id": "25a3d274-a408-4ec8-b1fd-69eb88c7674d"}, "last_update_at": "2026-11-13T00:41:33.026390Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "25a3d274-a408-4ec8-b1fd-69eb88c7674d"}	Self Report — My Site	2026-02-18 17:07:18.061512+00	2026-11-13 00:41:34.011075+00	0	f	[]
2385a8a3-0def-464d-83b4-074a7a65afa2	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "a1b350a1-59b8-4caf-ae55-c9f3f8ca89fb", "started_at": "2026-03-05T09:32:41.117544068Z", "finished_at": "2026-11-13T00:41:42.321189Z", "discovery_id": null, "daemon_version": "0.17.21", "discovery_type": {"type": "SelfReport", "host_id": "2a76c091-9847-4916-bc73-8668758f6f7d"}, "last_update_at": "2026-11-13T00:41:41.185703Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "2a76c091-9847-4916-bc73-8668758f6f7d"}	Self Report — My Site	2026-03-05 09:32:41.117544+00	2026-11-13 00:41:42.323899+00	0	f	[]
741a6e5c-2ec5-49ab-b572-dcb6af4a0bfb	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "b9089e15-81e6-435a-90fe-db5b24561924", "started_at": "2026-02-09T15:42:07.840272212Z", "finished_at": "2026-11-13T00:41:31.891359Z", "discovery_id": null, "daemon_version": "0.17.10", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:30.811590Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-09 15:42:07.840272+00	2026-11-13 00:41:31.895081+00	0	f	[]
f5bdec7a-7411-447e-bded-610d3e818637	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "f36e9f5f-c1e1-4f52-b2b1-2de3128ff92c", "started_at": "2026-08-14T17:13:59.174760436Z", "finished_at": "2026-11-13T00:41:32.425814Z", "discovery_id": null, "daemon_version": "0.14.5", "discovery_type": {"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:32.000490Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-14 17:13:59.17476+00	2026-11-13 00:41:32.427219+00	0	f	[]
4b700fc6-788e-4763-8ac3-8055e3ebe6f8	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "34d1feb3-276c-4ea9-b889-7eab39b67a1c", "started_at": "2026-02-23T00:07:49.639240713Z", "finished_at": "2026-11-13T00:41:34.015800Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:33.014247Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-23 00:07:49.63924+00	2026-11-13 00:41:34.018597+00	0	f	[]
c91706b5-d126-4d05-8f24-03b6278f8895	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "47b82122-5cf6-42a8-bf36-f15c9a6582f0", "started_at": "2026-04-28T19:50:40.314334020Z", "finished_at": "2026-11-13T00:41:34.715850Z", "discovery_id": null, "daemon_version": "0.18.0", "discovery_type": {"type": "Unified", "host_id": "689f2fa8-dc17-4321-b680-7af464e5004f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:34.046953Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "689f2fa8-dc17-4321-b680-7af464e5004f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-28 19:50:40.314334+00	2026-11-13 00:41:34.717462+00	0	f	[]
d7d1f690-c51d-470f-b1f2-331dfa328ac5	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "2453f28d-e87c-4753-80ad-0416d47c71f8", "started_at": "2026-10-11T23:59:48.356682576Z", "finished_at": "2026-11-13T00:41:35.253372Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:34.822783Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-11 23:59:48.356682+00	2026-11-13 00:41:35.254664+00	0	f	[]
b879dd49-e199-4e96-b92a-90808728ca39	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "f0555e1a-a4e0-4d26-b3e1-2a1c581c337d", "started_at": "2026-02-09T15:41:37.848773295Z", "finished_at": "2026-11-13T00:41:31.903177Z", "discovery_id": null, "daemon_version": "0.17.10", "discovery_type": {"type": "SelfReport", "host_id": "9e1eeb2e-e918-4dea-a437-6a61a65f4497"}, "last_update_at": "2026-11-13T00:41:30.752280Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "9e1eeb2e-e918-4dea-a437-6a61a65f4497"}	Self Report — My Site	2026-02-09 15:41:37.848773+00	2026-11-13 00:41:31.906837+00	0	f	[]
c0ff04e8-b131-4761-9c9c-0977913d63f6	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "13af9d3c-5935-4be7-90cf-7fb432a4db27", "started_at": "2026-08-06T01:38:34.928909970Z", "finished_at": "2026-11-13T00:41:32.420411Z", "discovery_id": null, "daemon_version": "0.14.5", "discovery_type": {"type": "Unified", "host_id": "6be44535-29d4-4940-88c3-917ce0d69222", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:32.006407Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6be44535-29d4-4940-88c3-917ce0d69222", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-06 01:38:34.928909+00	2026-11-13 00:41:32.421927+00	0	f	[]
f05e6190-d746-4365-a377-3a3075a06c22	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "69664faf-d5dd-440a-b554-0af052ce9005", "started_at": "2026-08-01T21:20:27.123050289Z", "finished_at": "2026-11-13T00:41:39.667717Z", "discovery_id": null, "daemon_version": "0.17.12", "discovery_type": {"type": "Unified", "host_id": "b6b48830-db6a-4d8f-8c70-db5516f3e458", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:38.685023Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b6b48830-db6a-4d8f-8c70-db5516f3e458", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-01 21:20:27.12305+00	2026-11-13 00:41:39.669237+00	0	f	[]
5b242a86-b718-4120-9cbe-90c59edea57d	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "b3287603-7c63-47ac-886a-4b6bb9cb2151", "started_at": "2026-08-01T21:25:32.482042816Z", "finished_at": "2026-11-13T00:41:39.673989Z", "discovery_id": null, "daemon_version": "0.17.12", "discovery_type": {"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:38.752336Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-01 21:25:32.482042+00	2026-11-13 00:41:39.675319+00	0	f	[]
56205efa-0441-4513-9ccb-5cc36800bd3e	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "6740c1be-987b-48e8-b547-871b2ea811c7", "started_at": "2026-10-23T00:17:23.541441434Z", "finished_at": "2026-11-13T00:41:41.161388Z", "discovery_id": null, "daemon_version": "0.14.13", "discovery_type": {"type": "Unified", "host_id": "9134f9b8-a2ca-4af9-9f9d-f98529385af4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:40.687501Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9134f9b8-a2ca-4af9-9f9d-f98529385af4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-23 00:17:23.541441+00	2026-11-13 00:41:41.162711+00	0	f	[]
d2c5d367-dba4-41e2-a733-f37cc11081c8	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "8c70cc2e-9120-44c5-8651-e6a5bad04bb9", "started_at": "2026-08-14T17:08:52.205106028Z", "finished_at": "2026-11-13T00:41:32.417656Z", "discovery_id": null, "daemon_version": "0.14.5", "discovery_type": {"type": "Unified", "host_id": "b2d748df-ea85-45ad-8e23-79946b4b5772", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:31.930269Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b2d748df-ea85-45ad-8e23-79946b4b5772", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-14 17:08:52.205106+00	2026-11-13 00:41:32.419126+00	0	f	[]
f5dcf11a-d58c-4e49-b3ae-18462fef9f68	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "8dab7a99-5955-498a-b597-5e7bb0e5d62a", "started_at": "2026-04-28T19:55:22.652516977Z", "finished_at": "2026-11-13T00:41:34.718676Z", "discovery_id": null, "daemon_version": "0.18.0", "discovery_type": {"type": "Unified", "host_id": "c16a9026-e029-4161-8963-20186c87d56c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:34.215491Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c16a9026-e029-4161-8963-20186c87d56c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-28 19:55:22.652516+00	2026-11-13 00:41:34.72009+00	0	f	[]
76d0dc02-6173-4a4f-913e-648eb6e42f5f	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "f9ec8267-d3bb-4170-80d5-9c5a41add7eb", "started_at": "2026-07-28T03:20:23.724249500Z", "finished_at": "2026-11-13T00:41:38.659810Z", "discovery_id": null, "daemon_version": "0.17.8", "discovery_type": {"type": "Unified", "host_id": "24f17e21-d958-4c17-ad9d-3f058f1bfd07", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:38.168624Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "24f17e21-d958-4c17-ad9d-3f058f1bfd07", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-28 03:20:23.724249+00	2026-11-13 00:41:38.661291+00	0	f	[]
bcff8da0-7914-4dc1-bd02-a49b75cfd85f	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "9478634a-d980-4211-b0fa-ac0e884de1ad", "started_at": "2026-10-05T14:23:50.058326236Z", "finished_at": "2026-11-13T00:41:42.841145Z", "discovery_id": null, "daemon_version": "0.19.6", "discovery_type": {"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:42.430234Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-05 14:23:50.058326+00	2026-11-13 00:41:42.842657+00	0	f	[]
db163e64-d7fc-4d56-b5df-4998efaf1353	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "37559eea-aa20-4b8c-ba39-ee899c2fc556", "started_at": "2026-01-05T04:59:56.694889798Z", "finished_at": "2026-11-13T00:41:53.112932Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:52.177537Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-05 04:59:56.694889+00	2026-11-13 00:41:53.115448+00	0	f	[]
0283f057-9e40-4696-9747-8827dc01385a	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "3326154e-ed93-4721-9226-b365a900ba24", "started_at": "2026-08-14T17:10:29.182979151Z", "finished_at": "2026-11-13T00:41:32.423068Z", "discovery_id": null, "daemon_version": "0.14.5", "discovery_type": {"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:31.994157Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-14 17:10:29.182979+00	2026-11-13 00:41:32.424556+00	0	f	[]
21eed59f-7ca4-4c7e-859d-b6b7de61f5be	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "cd16f43c-4365-49c5-8fee-d081f3b53cd9", "started_at": "2026-02-23T00:07:19.655637880Z", "finished_at": "2026-11-13T00:41:34.012158Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "SelfReport", "host_id": "abd72866-b3c5-4e9a-89f6-491d6af2797b"}, "last_update_at": "2026-11-13T00:41:32.955907Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "abd72866-b3c5-4e9a-89f6-491d6af2797b"}	Self Report — My Site	2026-02-23 00:07:19.655637+00	2026-11-13 00:41:34.014822+00	0	f	[]
1a705816-50db-4249-91bd-13672402276b	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "3bba0463-9815-4685-8b2e-93eacc318f67", "started_at": "2026-08-26T01:56:14.716285279Z", "finished_at": "2026-11-13T00:41:34.019715Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:33.566313Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-26 01:56:14.716285+00	2026-11-13 00:41:34.021115+00	0	f	[]
6e6eb691-8630-4672-9e1e-5a01629a81ce	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "5d98fc6a-1bba-4985-a654-bf83407df1f2", "started_at": "2026-10-14T05:48:09.745795958Z", "finished_at": "2026-11-13T00:41:35.772135Z", "discovery_id": null, "daemon_version": "0.17.0", "discovery_type": {"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:35.365441Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-14 05:48:09.745795+00	2026-11-13 00:41:35.773645+00	0	f	[]
feb300ff-d40c-4ce4-ae65-a3079c42d94f	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "6a03df7f-6023-4f74-893b-f9bb39cb8d98", "started_at": "2026-10-29T03:47:06.824079209Z", "finished_at": "2026-11-13T00:41:36.946837Z", "discovery_id": null, "daemon_version": "0.19.3", "discovery_type": {"type": "Unified", "host_id": "f591b8f3-e44c-4fab-bb81-9b67c29df370", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:36.468704Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f591b8f3-e44c-4fab-bb81-9b67c29df370", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-29 03:47:06.824079+00	2026-11-13 00:41:36.948376+00	0	f	[]
61865da9-d0e1-46b7-9d27-8400a8427fc3	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "e39a2b83-3f31-4943-abbc-ba31d154ce16", "started_at": "2026-02-11T02:46:23.000125066Z", "finished_at": "2026-11-13T00:41:33.997764Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "SelfReport", "host_id": "d4234791-d4ee-4257-a9d3-a257b5e1e3b9"}, "last_update_at": "2026-11-13T00:41:32.448353Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "d4234791-d4ee-4257-a9d3-a257b5e1e3b9"}	Self Report — My Site	2026-02-11 02:46:23.000125+00	2026-11-13 00:41:34.000657+00	0	f	[]
4190f941-e749-4a72-a274-067a2c98bb2a	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "faf2ae16-9ccd-4ba2-8be6-077b30ff3ae7", "started_at": "2026-08-26T01:59:44.730528896Z", "finished_at": "2026-11-13T00:41:34.005639Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:33.572853Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-26 01:59:44.730528+00	2026-11-13 00:41:34.007082+00	0	f	[]
53e4ed8e-284e-43d7-895b-ae7eec0a7bd0	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "7c9364ea-b95f-4e96-b958-a71e1e3826df", "started_at": "2026-08-26T01:54:31.081144679Z", "finished_at": "2026-11-13T00:41:34.024739Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "Unified", "host_id": "6a1dd7f3-9b36-49b1-b554-1ef783b14e5d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:33.481450Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6a1dd7f3-9b36-49b1-b554-1ef783b14e5d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-26 01:54:31.081144+00	2026-11-13 00:41:34.026187+00	0	f	[]
c2e509f4-dc29-4bed-aa45-e666a19847fd	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "f1a2c4ba-d124-4b5d-9448-729e2a28c186", "started_at": "2026-06-23T01:16:12.751429360Z", "finished_at": "2026-11-13T00:41:36.454811Z", "discovery_id": null, "daemon_version": "0.19.4", "discovery_type": {"type": "Unified", "host_id": "27a3457e-4127-4ee5-96e3-ba617595159b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:35.800605Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "27a3457e-4127-4ee5-96e3-ba617595159b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-23 01:16:12.751429+00	2026-11-13 00:41:36.456169+00	0	f	[]
451445fe-c867-42f3-ad49-f50509d28adc	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "11299ee7-3101-4bae-be7a-097db8407dd7", "started_at": "2026-01-23T03:48:00.568581680Z", "finished_at": "2026-11-13T00:41:39.670433Z", "discovery_id": null, "daemon_version": "0.17.12", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:39.254090Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-23 03:48:00.568581+00	2026-11-13 00:41:39.672976+00	0	f	[]
8aa9409d-5392-495a-bb64-5a56492beccd	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "69f77b5e-1350-4d55-ae3e-a0aa03bb47cd", "started_at": "2026-08-25T02:50:04.633793664Z", "finished_at": "2026-11-13T00:41:34.022184Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "Unified", "host_id": "802b4d69-588c-452a-83c3-1e47ca5d84d1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:33.579381Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "802b4d69-588c-452a-83c3-1e47ca5d84d1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 02:50:04.633793+00	2026-11-13 00:41:34.023621+00	0	f	[]
659dad4a-db0f-4438-8cd9-fb62576767f5	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "b18c8b1b-202b-4081-9911-d4f395fe9344", "started_at": "2026-10-27T05:03:46.928849924Z", "finished_at": "2026-11-13T00:41:37.452303Z", "discovery_id": null, "daemon_version": "0.17.4", "discovery_type": {"type": "Unified", "host_id": "8913babd-75d5-4960-a597-9eb267cea952", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:36.961930Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8913babd-75d5-4960-a597-9eb267cea952", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-27 05:03:46.928849+00	2026-11-13 00:41:37.453677+00	0	f	[]
5cf0d2f5-2e9b-4f46-a696-16cd3252209b	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "675b82cb-527e-4541-8702-fef2a8eb5552", "started_at": "2026-08-25T02:55:08.502675512Z", "finished_at": "2026-11-13T00:41:40.184791Z", "discovery_id": null, "daemon_version": "0.14.8", "discovery_type": {"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:39.764653Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 02:55:08.502675+00	2026-11-13 00:41:40.186153+00	0	f	[]
d622ba82-ad58-43ac-9d6a-0c47ed5c7e3a	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "8625c9a3-252c-4570-9b6a-e4459dde2e43", "started_at": "2026-10-05T14:18:58.075801174Z", "finished_at": "2026-11-13T00:41:42.838559Z", "discovery_id": null, "daemon_version": "0.19.6", "discovery_type": {"type": "Unified", "host_id": "38b72a6c-0aea-4147-b599-5af096f07623", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:42.339877Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "38b72a6c-0aea-4147-b599-5af096f07623", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-05 14:18:58.075801+00	2026-11-13 00:41:42.840001+00	0	f	[]
c7830a51-4062-475c-b3bd-06baa3bac414	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "72d38ac6-a392-41f5-96f5-17c22a19e8f5", "started_at": "2026-03-06T23:21:08.018383672Z", "finished_at": "2026-11-13T00:41:53.142552Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:51.144566Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-06 23:21:08.018383+00	2026-11-13 00:41:53.145071+00	0	f	[]
aa73e6e3-aedb-4fbb-9dd7-627eed876387	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "c30ae4d6-aa01-4253-94db-acdabf3a54e4", "started_at": "2026-04-28T19:09:10.681879638Z", "finished_at": "2026-11-13T00:41:34.721527Z", "discovery_id": null, "daemon_version": "0.18.0", "discovery_type": {"type": "Unified", "host_id": "dd6f90d2-d1df-49c8-bba4-cf36f7343201", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:34.120374Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "dd6f90d2-d1df-49c8-bba4-cf36f7343201", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-28 19:09:10.681879+00	2026-11-13 00:41:34.722941+00	0	f	[]
00f32fbc-87b1-4f1a-bcb6-dd640edbf943	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "13aa0c04-f207-47b3-9c03-0f3209efab76", "started_at": "2026-10-12T00:03:18.358288758Z", "finished_at": "2026-11-13T00:41:35.250857Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:34.829846Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-12 00:03:18.358288+00	2026-11-13 00:41:35.252278+00	0	f	[]
513e0a78-98a5-43e7-9364-b4eaaaf49733	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "df28970e-d903-4f00-8b2a-1dfa823e410c", "started_at": "2026-07-28T03:25:25.938837651Z", "finished_at": "2026-11-13T00:41:38.657030Z", "discovery_id": null, "daemon_version": "0.17.8", "discovery_type": {"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:38.228971Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-28 03:25:25.938837+00	2026-11-13 00:41:38.658556+00	0	f	[]
3d3abe74-3c6c-44c2-b43c-08686c1377f3	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "00342129-a6d7-490e-84ba-2bd11b17a768", "started_at": "2026-02-26T00:44:02.257915046Z", "finished_at": "2026-11-13T00:41:41.157613Z", "discovery_id": null, "daemon_version": "0.14.13", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:40.266438Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-26 00:44:02.257915+00	2026-11-13 00:41:41.160142+00	0	f	[]
4631ed87-dcf0-4542-980c-a146ae72e234	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "bbdc17b9-0273-4a91-8a0e-268a6fcf0277", "started_at": "2026-02-26T00:43:32.254923713Z", "finished_at": "2026-11-13T00:41:41.163697Z", "discovery_id": null, "daemon_version": "0.14.13", "discovery_type": {"type": "SelfReport", "host_id": "4c158030-00c1-49eb-b7e2-4e803057a3dd"}, "last_update_at": "2026-11-13T00:41:40.208904Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "4c158030-00c1-49eb-b7e2-4e803057a3dd"}	Self Report — My Site	2026-02-26 00:43:32.254923+00	2026-11-13 00:41:41.166105+00	0	f	[]
1d2f7fd9-fc57-4f70-b859-eb39a5d76bcf	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "9e24c5ae-ab53-4e0f-93db-281f3614fd56", "started_at": "2026-10-11T23:58:36.269774581Z", "finished_at": "2026-11-13T00:41:35.248132Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "Unified", "host_id": "8d1f0138-018b-4734-9af4-5f196dc2a028", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:34.736212Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8d1f0138-018b-4734-9af4-5f196dc2a028", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-11 23:58:36.269774+00	2026-11-13 00:41:35.249552+00	0	f	[]
1e8d06c9-a59a-4563-8500-8b1380aca920	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "d7b0c883-f1dd-45b9-8008-cb06d0194514", "started_at": "2026-10-14T05:43:36.061177700Z", "finished_at": "2026-11-13T00:41:35.777239Z", "discovery_id": null, "daemon_version": "0.17.0", "discovery_type": {"type": "Unified", "host_id": "d4dabb79-eb99-4045-8687-9ca8bd1e797a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:35.267923Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d4dabb79-eb99-4045-8687-9ca8bd1e797a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-14 05:43:36.061177+00	2026-11-13 00:41:35.778652+00	0	f	[]
b1708b31-27a6-4d02-8ef5-dd0bca1ec225	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "7ad7978f-e4a7-4518-bb77-8a4c8c1dd0c6", "started_at": "2026-07-07T22:27:42.015566135Z", "finished_at": "2026-11-13T00:41:38.142285Z", "discovery_id": null, "daemon_version": "0.17.6", "discovery_type": {"type": "Unified", "host_id": "079502fe-1a72-44a6-ab38-040ead1355fb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:37.473676Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "079502fe-1a72-44a6-ab38-040ead1355fb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-07 22:27:42.015566+00	2026-11-13 00:41:38.143767+00	0	f	[]
d5991615-a6fd-4254-b52c-79b9d9acadb1	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "b3439cfa-af27-47ae-8b9d-8c79b9d149fa", "started_at": "2026-07-28T03:21:55.956404860Z", "finished_at": "2026-11-13T00:41:38.662317Z", "discovery_id": null, "daemon_version": "0.17.8", "discovery_type": {"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:38.222468Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-28 03:21:55.956404+00	2026-11-13 00:41:38.663657+00	0	f	[]
a01a8e6f-d0e4-42b3-831c-a45a1374bf35	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "e46bbbd5-df29-425e-b933-bd8b0a332217", "started_at": "2026-03-15T19:34:41.999288315Z", "finished_at": "2026-11-13T00:42:06.279737Z", "discovery_id": null, "daemon_version": "0.20.0", "discovery_type": {"type": "SelfReport", "host_id": "67ddad71-cd34-47fe-ad5f-138fe7069c76"}, "last_update_at": "2026-11-13T00:41:54.305962Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "67ddad71-cd34-47fe-ad5f-138fe7069c76"}	Self Report — My Site	2026-03-15 19:34:41.999288+00	2026-11-13 00:42:06.282634+00	0	f	[]
ce8521c3-e62c-4cd6-830b-de51f1f2d4e1	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "833d88c3-2acf-4989-86de-466d44e47a7e", "started_at": "2026-10-14T05:44:39.739395480Z", "finished_at": "2026-11-13T00:41:35.774808Z", "discovery_id": null, "daemon_version": "0.17.0", "discovery_type": {"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:35.358920Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-14 05:44:39.739395+00	2026-11-13 00:41:35.776181+00	0	f	[]
b5829415-2f92-45f0-9c53-8978f436da76	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "bfc425bd-2840-4447-8d67-666671033cd6", "started_at": "2026-06-23T01:20:59.927972592Z", "finished_at": "2026-11-13T00:41:36.452100Z", "discovery_id": null, "daemon_version": "0.19.4", "discovery_type": {"type": "Unified", "host_id": "cf70c05a-1647-4958-806b-ff84119deb9a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:36.049866Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "cf70c05a-1647-4958-806b-ff84119deb9a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-23 01:20:59.927972+00	2026-11-13 00:41:36.453654+00	0	f	[]
9e8bbdf6-53c8-48e9-98a9-9c2b4abc145b	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "1811938e-d0f9-4568-bc69-ef3f2931e9ef", "started_at": "2026-08-25T01:24:07.586140654Z", "finished_at": "2026-11-13T00:41:40.181899Z", "discovery_id": null, "daemon_version": "0.14.8", "discovery_type": {"type": "Unified", "host_id": "5f0170e1-a84c-4207-b391-c13297a54d48", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:39.770960Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "5f0170e1-a84c-4207-b391-c13297a54d48", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 01:24:07.58614+00	2026-11-13 00:41:40.183439+00	0	f	[]
6e0ec7f0-1949-4cbc-8812-607ff229899d	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "44932e97-76ed-4c08-b694-4e6aa6bca1be", "started_at": "2026-03-05T09:33:11.114723268Z", "finished_at": "2026-11-13T00:41:42.324896Z", "discovery_id": null, "daemon_version": "0.17.21", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:41.243691Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-05 09:33:11.114723+00	2026-11-13 00:41:42.32754+00	0	f	[]
869df807-65fc-49c8-8a93-253761b89179	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "c1eaa2e7-7631-418b-8d5f-74597c08a4f5", "started_at": "2026-10-31T00:21:22.175714347Z", "finished_at": "2026-11-13T00:41:49.007293Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "a20cea9a-e475-4818-80d1-32a346c0085e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:48.506050Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a20cea9a-e475-4818-80d1-32a346c0085e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-31 00:21:22.175714+00	2026-11-13 00:41:49.008764+00	0	f	[]
bddd0ab4-eaba-4ad5-afd4-6b5c32fecaf3	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "8473fe97-6e86-4b62-9b6d-2cd1349deeb8", "started_at": "2026-07-07T22:29:34.241374934Z", "finished_at": "2026-11-13T00:41:38.144965Z", "discovery_id": null, "daemon_version": "0.17.6", "discovery_type": {"type": "Unified", "host_id": "bb32f4ef-dc8e-4c79-89c6-04915971d7b0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:37.523166Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bb32f4ef-dc8e-4c79-89c6-04915971d7b0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-07 22:29:34.241374+00	2026-11-13 00:41:38.146341+00	0	f	[]
16111275-b4d2-45e4-ab8b-42f6e0a79337	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "7e06b95e-3b67-41c4-9049-978411591e7c", "started_at": "2026-08-01T21:22:02.500665824Z", "finished_at": "2026-11-13T00:41:39.676397Z", "discovery_id": null, "daemon_version": "0.17.12", "discovery_type": {"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:38.745840Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-01 21:22:02.500665+00	2026-11-13 00:41:39.677741+00	0	f	[]
779bf1ee-d77a-4bfb-ae0b-7679841466df	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "6fa2c508-5f31-44cd-9153-00c853afe9a7", "started_at": "2026-08-25T02:51:38.519652606Z", "finished_at": "2026-11-13T00:41:40.187272Z", "discovery_id": null, "daemon_version": "0.14.8", "discovery_type": {"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:39.758059Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 02:51:38.519652+00	2026-11-13 00:41:40.188715+00	0	f	[]
ffb95430-9249-41b1-8b3f-e23b4d954bf0	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "5d811057-9d57-4a37-997d-6d9ff0bde3f4", "started_at": "2026-10-05T14:20:20.057223827Z", "finished_at": "2026-11-13T00:41:42.836005Z", "discovery_id": null, "daemon_version": "0.19.6", "discovery_type": {"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:42.423755Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-05 14:20:20.057223+00	2026-11-13 00:41:42.837452+00	0	f	[]
2d623d61-c95b-4970-b403-cb534ff88306	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "901cf5c8-1989-4bf1-bf5d-e587ee211cda", "started_at": "2026-03-26T17:23:47.082705682Z", "finished_at": "2026-11-13T00:41:42.318432Z", "discovery_id": null, "daemon_version": "0.17.21", "discovery_type": {"type": "Unified", "host_id": "2bbce8eb-5555-41ef-b999-68a5af971278", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:41.706778Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2bbce8eb-5555-41ef-b999-68a5af971278", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-26 17:23:47.082705+00	2026-11-13 00:41:42.319933+00	0	f	[]
e6460869-a1c9-4849-9370-c24296700c58	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "271037a5-7bfa-49d0-bff8-3879ffaa6d30", "started_at": "2026-11-04T16:49:09.224254Z", "finished_at": "2026-11-13T00:41:43.335776Z", "discovery_id": null, "daemon_version": "0.15.5", "discovery_type": {"type": "Unified", "host_id": "f35cae23-aec3-4e90-8c72-278e1d887c57", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:42.855069Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f35cae23-aec3-4e90-8c72-278e1d887c57", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-04 16:49:09.224254+00	2026-11-13 00:41:43.33727+00	0	f	[]
91e57dbe-458d-40e0-ae81-21a6b92f2b8f	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "457a544b-3c09-45bf-90d1-ab9e5e4a144d", "started_at": "2026-04-01T14:32:54.762125416Z", "finished_at": "2026-11-13T00:41:43.997860Z", "discovery_id": null, "daemon_version": "0.17.15", "discovery_type": {"type": "Unified", "host_id": "a33f8415-bb39-43da-81c3-dfafa82595da", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:43.357939Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a33f8415-bb39-43da-81c3-dfafa82595da", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-01 14:32:54.762125+00	2026-11-13 00:41:43.999368+00	0	f	[]
d423f1f4-5994-415d-a4fa-70b08989a015	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "21fc2969-cba5-46cf-b381-1978e0c2959e", "started_at": "2026-09-03T17:55:43.577416616Z", "finished_at": "2026-11-13T00:41:44.547850Z", "discovery_id": null, "daemon_version": "0.17.11", "discovery_type": {"type": "Unified", "host_id": "3fd2970b-7217-4821-8bd7-6985174b6a22", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:44.117368Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "3fd2970b-7217-4821-8bd7-6985174b6a22", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-03 17:55:43.577416+00	2026-11-13 00:41:44.549371+00	0	f	[]
93fcef1f-3ba0-4c33-a8e0-50b2a4ce7ef4	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "289721de-ae03-40cd-95d5-6af379282969", "started_at": "2026-09-15T05:37:18.553708846Z", "finished_at": "2026-11-13T00:41:44.540303Z", "discovery_id": null, "daemon_version": "0.17.11", "discovery_type": {"type": "Unified", "host_id": "2a9b1a5e-51d9-4f13-b6e4-0b1e3e27e0dc", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:44.020062Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2a9b1a5e-51d9-4f13-b6e4-0b1e3e27e0dc", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-15 05:37:18.553708+00	2026-11-13 00:41:44.541777+00	0	f	[]
613b4ce7-9bfd-4c89-97d0-355f3c091c7e	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "2b6e6d22-4644-4cad-a850-04fb5896df10", "started_at": "2026-03-02T09:02:26.922912054Z", "finished_at": "2026-11-13T00:41:47.235201Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "SelfReport", "host_id": "b6dcbdab-8ba5-4b12-8dee-5177af2cf702"}, "last_update_at": "2026-11-13T00:41:45.598620Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "b6dcbdab-8ba5-4b12-8dee-5177af2cf702"}	Self Report — My Site	2026-03-02 09:02:26.922912+00	2026-11-13 00:41:47.237667+00	0	f	[]
5605e892-c1c6-4da7-8671-fbb808673ad6	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "1a2a6d39-262b-45ff-80c8-1f8742f766af", "started_at": "2026-02-28T03:10:51.549945495Z", "finished_at": "2026-11-13T00:41:47.247012Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "SelfReport", "host_id": "422f12c5-aed6-4cdc-aab7-a281212ef855"}, "last_update_at": "2026-11-13T00:41:45.083075Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "422f12c5-aed6-4cdc-aab7-a281212ef855"}	Self Report — My Site	2026-02-28 03:10:51.549945+00	2026-11-13 00:41:47.24956+00	0	f	[]
c1efac93-9cf2-42c9-9057-0e4ffb419b22	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "71cbed11-9ec5-4ae6-a3ce-3767032c2ab8", "started_at": "2026-04-17T22:09:29.738977360Z", "finished_at": "2026-11-13T00:41:47.253993Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Unified", "host_id": "e4ddba2b-4c4f-4f89-a625-8f3f022d7581", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:46.667357Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e4ddba2b-4c4f-4f89-a625-8f3f022d7581", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-17 22:09:29.738977+00	2026-11-13 00:41:47.255394+00	0	f	[]
0a738562-12e0-404d-bcf4-308e4910548b	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "d10e30a1-1b5e-4a25-abe9-566d427e3186", "started_at": "2026-03-24T03:12:35.661709995Z", "finished_at": "2026-11-13T00:41:47.868644Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Unified", "host_id": "80008868-cd5c-4f1e-92ea-0bd73728c9a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:47.275023Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "80008868-cd5c-4f1e-92ea-0bd73728c9a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-24 03:12:35.661709+00	2026-11-13 00:41:47.870074+00	0	f	[]
9a4bb0bf-5453-4a7f-b7f4-4e62123bb9ec	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "c8b7e98c-df16-4cc6-88f2-82866bbb959f", "started_at": "2026-09-15T05:38:49.198136490Z", "finished_at": "2026-11-13T00:41:44.543083Z", "discovery_id": null, "daemon_version": "0.17.11", "discovery_type": {"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:44.104443Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-15 05:38:49.198136+00	2026-11-13 00:41:44.544411+00	0	f	[]
3fa41f8e-83b1-4352-b3ed-199bca36b616	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "6bbb8f0d-ed69-4df8-81d6-8bc44cdf4380", "started_at": "2026-03-02T09:02:56.919081491Z", "finished_at": "2026-11-13T00:41:47.250480Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:45.658742Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-02 09:02:56.919081+00	2026-11-13 00:41:47.252914+00	0	f	[]
aa0b69ed-26ce-436f-86f0-4666f860df44	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "947c4ec5-0754-4116-8282-283b6a09f803", "started_at": "2026-03-05T07:38:50.115878953Z", "finished_at": "2026-11-13T00:41:53.105742Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:50.119205Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-05 07:38:50.115878+00	2026-11-13 00:41:53.10838+00	0	f	[]
540009ae-5a4e-47c2-afc2-feaa931d1d12	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "d36fca8d-dbbc-4cad-a19f-f495eacf6234", "started_at": "2026-02-28T21:19:27.224801293Z", "finished_at": "2026-11-13T00:41:53.146043Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:49.607928Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-28 21:19:27.224801+00	2026-11-13 00:41:53.148578+00	0	f	[]
72a1bb4c-a9db-46d6-8c87-40c41ee39787	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "9f3bb823-ded6-4cb7-957b-27c4af0ebff2", "started_at": "2026-03-15T19:35:11.987886037Z", "finished_at": "2026-11-13T00:42:06.274174Z", "discovery_id": null, "daemon_version": "0.20.0", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:54.360805Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-15 19:35:11.987886+00	2026-11-13 00:42:06.278508+00	0	f	[]
0f74a175-f425-429f-99f9-228009c9470e	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "01486396-5c85-4268-b3c8-b83da1042232", "started_at": "2026-09-15T05:42:19.268112358Z", "finished_at": "2026-11-13T00:41:44.545454Z", "discovery_id": null, "daemon_version": "0.17.11", "discovery_type": {"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:44.110973Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-15 05:42:19.268112+00	2026-11-13 00:41:44.546804+00	0	f	[]
6f7352ad-539e-4a78-9e96-a7f4de541ee5	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "c50795be-e20e-483e-857d-f04a9f103f60", "started_at": "2026-08-17T00:25:40.701352183Z", "finished_at": "2026-11-13T00:41:45.060773Z", "discovery_id": null, "daemon_version": "0.14.9", "discovery_type": {"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:44.634036Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-17 00:25:40.701352+00	2026-11-13 00:41:45.062096+00	0	f	[]
1ef373cc-f9d5-4e99-819d-1320c69a823a	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "030a56ec-f416-4d7f-864e-fde8841a379c", "started_at": "2026-09-03T17:57:32.046576969Z", "finished_at": "2026-11-13T00:41:49.530924Z", "discovery_id": null, "daemon_version": "0.14.10", "discovery_type": {"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:49.109967Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-03 17:57:32.046576+00	2026-11-13 00:41:49.532461+00	0	f	[]
0a711048-59b8-4028-9cad-2bd0a926b647	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "b39cff2d-d385-4ab8-8097-66c2190dae09", "started_at": "2026-03-06T23:20:38.018012792Z", "finished_at": "2026-11-13T00:41:53.109510Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "SelfReport", "host_id": "6fd7ccd6-6ea7-4880-b5e5-ad545df36544"}, "last_update_at": "2026-11-13T00:41:51.088659Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "6fd7ccd6-6ea7-4880-b5e5-ad545df36544"}	Self Report — My Site	2026-03-06 23:20:38.018012+00	2026-11-13 00:41:53.112019+00	0	f	[]
d5ab3fae-91f5-4add-84c1-f6f1c6815209	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "68d1cd6d-9bbc-4af3-9202-ecf0db0dee57", "started_at": "2026-08-06T01:43:36.139858287Z", "finished_at": "2026-11-13T00:41:53.122209Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:52.665615Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-06 01:43:36.139858+00	2026-11-13 00:41:53.1235+00	0	f	[]
5baa3524-f65a-45f5-8db9-709266f8871a	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "2b8a5125-8bbc-4f50-9fd7-baad3f93a89f", "started_at": "2026-08-17T00:24:03.071614457Z", "finished_at": "2026-11-13T00:41:45.055465Z", "discovery_id": null, "daemon_version": "0.14.9", "discovery_type": {"type": "Unified", "host_id": "51d0c941-a5a3-4d6c-b3a9-5afbcaed2d3d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:44.569945Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "51d0c941-a5a3-4d6c-b3a9-5afbcaed2d3d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-17 00:24:03.071614+00	2026-11-13 00:41:45.057016+00	0	f	[]
9e69f48b-1d93-4342-aeac-179550b15df3	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "80837ade-3bd8-4bfa-a94b-ec249e6517fa", "started_at": "2026-03-11T06:19:54.610393414Z", "finished_at": "2026-11-13T00:41:48.489820Z", "discovery_id": null, "daemon_version": "0.19.5", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:47.949970Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:19:54.610393+00	2026-11-13 00:41:48.492598+00	0	f	[]
3343dcab-79f9-4317-bba0-d22f80c3d61e	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "ce6f84d7-a1a5-468b-887d-1a0c67e78012", "started_at": "2026-02-28T21:18:57.228697745Z", "finished_at": "2026-11-13T00:41:53.131520Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "SelfReport", "host_id": "b52f72ce-8332-4d4f-90a6-596cbbb0ba73"}, "last_update_at": "2026-11-13T00:41:49.552283Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "b52f72ce-8332-4d4f-90a6-596cbbb0ba73"}	Self Report — My Site	2026-02-28 21:18:57.228697+00	2026-11-13 00:41:53.134032+00	0	f	[]
df9f6199-464e-4c79-a3a8-1d115d32a85a	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "42d8b7a7-6a5c-409b-b8c2-d33c0af1e9c0", "started_at": "2026-08-17T00:29:10.692540259Z", "finished_at": "2026-11-13T00:41:45.058243Z", "discovery_id": null, "daemon_version": "0.14.9", "discovery_type": {"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:44.641136Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-17 00:29:10.69254+00	2026-11-13 00:41:45.059668+00	0	f	[]
26de45ea-2a08-488d-a321-16824a57bb96	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "580379b9-0101-428f-baef-843afbc5dfee", "started_at": "2026-01-26T14:03:54.326189222Z", "finished_at": "2026-11-13T00:41:47.231198Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:46.186618Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-26 14:03:54.326189+00	2026-11-13 00:41:47.234052+00	0	f	[]
02a84437-c280-4e8a-955d-312181961309	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "df1323db-2f1d-4090-bd79-58d04eee33c1", "started_at": "2026-04-18T01:11:16.011978400Z", "finished_at": "2026-11-13T00:41:47.238637Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Unified", "host_id": "a6d5229d-0b50-4243-a00d-c3d95bb17a36", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:46.778909Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a6d5229d-0b50-4243-a00d-c3d95bb17a36", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:11:16.011978+00	2026-11-13 00:41:47.239978+00	0	f	[]
6a083423-ed71-4125-a052-5c28806d095c	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "ff3dfff8-b772-4147-bc5c-c2806e8e84a4", "started_at": "2026-02-28T03:11:21.545902350Z", "finished_at": "2026-11-13T00:41:47.243510Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:45.141529Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-28 03:11:21.545902+00	2026-11-13 00:41:47.246077+00	0	f	[]
95436e11-2aed-4ad0-b951-29354065cb54	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "bfb7da47-98f1-4957-9f13-5b4f6f440e6a", "started_at": "2026-03-08T06:53:48.342722812Z", "finished_at": "2026-11-13T00:41:53.135102Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "SelfReport", "host_id": "caeb1c88-3a79-4e24-8a5a-fedd841b3a98"}, "last_update_at": "2026-11-13T00:41:51.596917Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "caeb1c88-3a79-4e24-8a5a-fedd841b3a98"}	Self Report — My Site	2026-03-08 06:53:48.342722+00	2026-11-13 00:41:53.137624+00	0	f	[]
00d8152b-3cb0-4865-87d4-e9d4a40dbb94	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "9dc720f8-1115-45ea-8b6d-f2e58154594d", "started_at": "2026-04-18T01:07:01.115515322Z", "finished_at": "2026-11-13T00:41:47.241058Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Unified", "host_id": "1ce3a67d-4aeb-485a-9976-c67c4801fd30", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:46.614461Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "1ce3a67d-4aeb-485a-9976-c67c4801fd30", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:07:01.115515+00	2026-11-13 00:41:47.242416+00	0	f	[]
94f65173-b2fd-4e8f-a128-7e04f17d04bd	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "61efb9ca-2ac2-4ab8-858e-07da2d3ef161", "started_at": "2026-02-10T04:10:20.090437467Z", "finished_at": "2026-11-13T00:41:53.116457Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "SelfReport", "host_id": "45cb5c93-df81-41f1-9047-97867efb47bb"}, "last_update_at": "2026-11-13T00:41:50.572480Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "45cb5c93-df81-41f1-9047-97867efb47bb"}	Self Report — My Site	2026-02-10 04:10:20.090437+00	2026-11-13 00:41:53.118879+00	0	f	[]
87602ce6-ce39-4694-8c68-f2caf23dae1d	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "110d0739-6875-498e-9ed5-5c7c0dec84ca", "started_at": "2026-03-05T07:38:20.117782332Z", "finished_at": "2026-11-13T00:41:53.128111Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "SelfReport", "host_id": "9b3486eb-3e41-4b56-8062-c98ad29f0a3a"}, "last_update_at": "2026-11-13T00:41:50.063573Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "9b3486eb-3e41-4b56-8062-c98ad29f0a3a"}	Self Report — My Site	2026-03-05 07:38:20.117782+00	2026-11-13 00:41:53.130561+00	0	f	[]
0f37d8a2-c519-4a8e-af03-12b186b72a53	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "128947fe-516b-47a0-897f-975edb8d4122", "started_at": "2026-04-20T17:24:17.714515792Z", "finished_at": "2026-11-13T00:41:54.285842Z", "discovery_id": null, "daemon_version": "0.14.19", "discovery_type": {"type": "Unified", "host_id": "41167b70-ef54-45fb-ac1c-df4f8be5f312", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:53.221459Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "41167b70-ef54-45fb-ac1c-df4f8be5f312", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-20 17:24:17.714515+00	2026-11-13 00:41:54.287098+00	0	f	[]
bf9d180d-5a03-4da9-bfd9-6cdaa45d7aca	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "557486b8-0125-4f97-a586-fe9ae5f972d8", "started_at": "2026-09-03T18:01:02.050183807Z", "finished_at": "2026-11-13T00:41:49.528088Z", "discovery_id": null, "daemon_version": "0.14.10", "discovery_type": {"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:49.116248Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-03 18:01:02.050183+00	2026-11-13 00:41:49.529686+00	0	f	[]
a6cff766-8d1d-48aa-9fd9-8dd67e717a13	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "14a48b0e-4297-410e-b019-b1d98130f2c0", "started_at": "2026-02-10T04:10:50.083304178Z", "finished_at": "2026-11-13T00:41:53.138633Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:50.628556Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-10 04:10:50.083304+00	2026-11-13 00:41:53.141458+00	0	f	[]
e3d23b80-f98f-4cf4-96ad-430416d3e7e1	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "7612c47a-db41-4c37-8d16-fbf4ace0e41e", "started_at": "2026-01-23T03:55:10.336667754Z", "finished_at": "2026-11-13T00:41:54.279592Z", "discovery_id": null, "daemon_version": "0.14.19", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:53.881970Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-23 03:55:10.336667+00	2026-11-13 00:41:54.282381+00	0	f	[]
c0ee9ca7-e4cc-4706-92ac-6014878ce720	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "c9568c2f-1ae3-48c4-9136-6eb8fcd496a8", "started_at": "2026-08-06T01:40:06.139857038Z", "finished_at": "2026-11-13T00:41:53.119868Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:52.659935Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-06 01:40:06.139857+00	2026-11-13 00:41:53.121117+00	0	f	[]
946380cb-4345-4e24-96a0-4e5929776aab	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "a49f4dc9-0185-4793-9f4f-0843c7021f81", "started_at": "2026-03-08T06:54:18.293920451Z", "finished_at": "2026-11-13T00:41:53.124471Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:51.653482Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-08 06:54:18.29392+00	2026-11-13 00:41:53.127167+00	0	f	[]
7f25cab9-c194-4913-96f4-6829e90d6f09	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "dae6845a-5c1c-4537-8549-2cb0bb052dd7", "started_at": "2026-04-20T18:00:40.095617293Z", "finished_at": "2026-11-13T00:41:54.276926Z", "discovery_id": null, "daemon_version": "0.14.19", "discovery_type": {"type": "Unified", "host_id": "c1107482-106c-4d18-8ee2-07081aef2afb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:53.269822Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c1107482-106c-4d18-8ee2-07081aef2afb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-20 18:00:40.095617+00	2026-11-13 00:41:54.27844+00	0	f	[]
00158b66-1db4-44cc-b373-702b6b782c3e	1acbe937-22e8-4c40-ae6a-76b4615931da	67da0afb-e420-4b41-8ec3-e805ac2e9681	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "1acbe937-22e8-4c40-ae6a-76b4615931da", "progress": 0, "warnings": [], "daemon_id": "67da0afb-e420-4b41-8ec3-e805ac2e9681", "session_id": "04ade0d1-6874-420a-a1d3-6ece53cd5f6f", "started_at": "2026-04-20T17:56:03.901696836Z", "finished_at": "2026-11-13T00:41:54.283452Z", "discovery_id": null, "daemon_version": "0.14.19", "discovery_type": {"type": "Unified", "host_id": "2551f8f8-f3cf-43d9-9d28-dd99c9468591", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-13T00:41:53.168170Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2551f8f8-f3cf-43d9-9d28-dd99c9468591", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-20 17:56:03.901696+00	2026-11-13 00:41:54.284854+00	0	f	[]
\.


--
-- Data for Name: discovery_warnings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.discovery_warnings (id, discovery_id, site_id, code, credential_id, host_id, remote_host_id, subnet_id, details, created_at) FROM stdin;
\.


--
-- Data for Name: entity_tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.entity_tags (id, entity_id, entity_type, tag_id, created_at, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
a22554f8-51c1-41a4-a28e-08e10eab04a6	9736f98c-139e-4bf1-afeb-50e47f6a737f	"Service"	aba9085a-fe4a-4c5e-8cc8-4bf7b9e96313	2026-11-13 00:41:15.047379+00	2026-11-13 00:41:15.047379+00	\N	\N	\N
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
18b6c6b1-34cf-4a4a-bee5-dbc2cdf46def	1acbe937-22e8-4c40-ae6a-76b4615931da	scanopy-daemon-serverpoll	7d7b02ec4c9c	\N	{"type": "Discovery"}	null	2026-11-13 00:33:55.968569+00	2026-11-13 00:33:55.968569+00	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-11-13 00:33:55.968569+00	\N	\N	2026-11-13 00:39:53.714639+00	26992c74-a3f4-41ab-9ef5-5dd72ce21de2	8e7cfc0f-3ee7-4b0e-8ced-d93fab6cd228	\N	\N	\N	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	\N	"Unspecified"	"DaemonSelfReport"	{"name": "Debian", "family": "Linux", "version": "12.0.0", "codename": "bookworm", "kernel_version": "7.0.0-1019-azure"}	"DaemonSelfReport"	\N	\N	"Unspecified"	\N	"Unspecified"	\N
7e411a60-a1e9-45b3-a3cb-235dddeede18	1acbe937-22e8-4c40-ae6a-76b4615931da	Scanopy Daemon	4d03e693439b	\N	{"type": "Discovery"}	null	2026-11-13 00:32:17.120809+00	2026-11-13 00:32:17.120809+00	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-11-13 00:32:17.120809+00	\N	\N	2026-11-13 00:41:23.356106+00	b16ef416-a8d0-44a0-8504-2ba0ebadf21d	b16ef416-a8d0-44a0-8504-2ba0ebadf21d	\N	\N	\N	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"ServiceMatch"	\N	"Unspecified"	"DaemonSelfReport"	{"name": "Debian", "family": "Linux", "version": "12.0.0", "codename": "bookworm", "kernel_version": "7.0.0-1019-azure"}	"DaemonSelfReport"	\N	\N	"Unspecified"	\N	"Unspecified"	\N
67ddad71-cd34-47fe-ad5f-138fe7069c76	1acbe937-22e8-4c40-ae6a-76b4615931da	aea74b9b3d22	aea74b9b3d22	Scanopy daemon	{"type": "Discovery"}	null	2026-11-13 00:41:54.324129+00	2026-11-13 00:41:54.324129+00	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-11-13 00:41:54.324129+00	\N	\N	2026-11-13 00:41:54.324129+00	\N	\N	\N	\N	\N	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	\N	"Unspecified"	"Unspecified"	\N	"Unspecified"	\N	\N	"Unspecified"	\N	"Unspecified"	\N
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

COPY public.invites (id, organization_id, permissions, url, created_by, created_at, updated_at, expires_at, send_to) FROM stdin;
\.


--
-- Data for Name: ip_addresses; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ip_addresses (id, site_id, host_id, subnet_id, ip_address, mac_address, name, "position", created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id, mac_address_source, ip_address_source) FROM stdin;
81412bd9-bca5-4e10-9ab3-ddba0f10d91b	1acbe937-22e8-4c40-ae6a-76b4615931da	67ddad71-cd34-47fe-ad5f-138fe7069c76	4ad11d12-d29a-4f8d-87e9-dc2c288a785d	172.25.0.4	36:4d:0a:e4:6f:0d	eth0	0	2026-11-13 00:41:54.324129+00	2026-11-13 00:41:54.324129+00	2026-11-13 00:41:54.324129+00	\N	\N	2026-11-13 00:41:54.324129+00	\N	\N	\N	"Unspecified"	"Unspecified"
\.


--
-- Data for Name: license_instances; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.license_instances (id, organization_id, instance_id, server_version, last_seen_ip, non_production, first_seen_at, last_seen_at, active_since, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: organizations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.organizations (id, name, stripe_customer_id, plan, plan_status, created_at, updated_at, onboarding, has_payment_method, trial_end_date, use_case, last_paused_at, trial_extended_used, last_downgrade_at, last_downgrade_from_plan, last_discount_at, discount_save_offer_percent_off, discount_save_offer_active_until, next_renewal_at, notifications, license_paid_through, license_checkin_at, license_key_version, license_entitlement, license_entitlement_at, license_key_issued_at, license_key_type, bills_by_invoice, license_server_version, setup) FROM stdin;
60fc216c-ed5f-471d-9b98-5905ff49bff0	My Organization	\N	{"rate": "Month", "type": "Community", "base_cents": 0, "trial_days": 0}	active	2026-11-13 00:32:16.929536+00	2026-11-13 00:32:16.929536+00	[]	f	\N	other	\N	f	\N	null	\N	\N	\N	\N	{"hosts": "None", "seats": "None", "sites": "None", "setup_items_open": [], "setup_items_reported": [], "license_over_instance_limit": false, "license_silence_reported_for": null, "airgap_expiry_notified_through": null, "sunset_notified_last_unsupported": null}	\N	\N	0	\N	\N	\N	\N	f	\N	["InstallDaemon", "RunFirstScan"]
\.


--
-- Data for Name: ports; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ports (id, site_id, host_id, port_number, protocol, port_type, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id, port_source) FROM stdin;
59eca2e1-5650-44fc-bcc0-a436c484f2da	1acbe937-22e8-4c40-ae6a-76b4615931da	67ddad71-cd34-47fe-ad5f-138fe7069c76	60073	Tcp	Custom	2026-11-13 00:41:54.324129+00	2026-11-13 00:41:54.324129+00	2026-11-13 00:41:54.324129+00	\N	\N	2026-11-13 00:41:54.324129+00	\N	\N	\N	"Unspecified"
\.


--
-- Data for Name: release_announcements; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.release_announcements (id, version, email_sent_at, created_at, updated_at) FROM stdin;
b91dd98b-7898-4e75-b6fc-1aff61c0ba99	0.20.0	\N	2026-11-13 00:32:15.062568+00	2026-11-13 00:32:15.062568+00
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
a6bceb85-8a66-414b-93ad-e7a855b019b8	1acbe937-22e8-4c40-ae6a-76b4615931da	2026-11-13 00:41:54.324129+00	2026-11-13 00:41:54.324129+00	Scanopy Daemon	67ddad71-cd34-47fe-ad5f-138fe7069c76	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}}	0	2026-11-13 00:41:54.324129+00	\N	\N	2026-11-13 00:41:54.324129+00	\N	\N	\N	\N
\.


--
-- Data for Name: shares; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.shares (id, topology_id, site_id, created_by, name, is_enabled, expires_at, password_hash, allowed_domains, options, created_at, updated_at, enabled_views) FROM stdin;
\.


--
-- Data for Name: site_assignments; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.site_assignments (id, organization_id, user_id, api_key_id, invite_id, role, site_id, created_at) FROM stdin;
ef800abd-f89e-4962-8e39-973cb646afbc	60fc216c-ed5f-471d-9b98-5905ff49bff0	f605e587-918a-411d-9fd8-9afed452c38c	\N	\N	Admin	\N	2026-11-13 00:42:09.090598+00
8bd0f2bf-ac55-4cad-a614-0964944c578b	60fc216c-ed5f-471d-9b98-5905ff49bff0	77c20b2e-5556-4673-b55f-7ba75c50b05f	\N	\N	Admin	\N	2026-11-13 00:42:09.090598+00
\.


--
-- Data for Name: site_credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.site_credentials (site_id, credential_id) FROM stdin;
1acbe937-22e8-4c40-ae6a-76b4615931da	b4a4ea58-1337-4585-b46b-a52224e4b49a
\.


--
-- Data for Name: sites; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.sites (id, name, created_at, updated_at, organization_id, stale_after_hours) FROM stdin;
1acbe937-22e8-4c40-ae6a-76b4615931da	My Site	2026-11-13 00:32:16.973247+00	2026-11-13 00:32:16.973247+00	60fc216c-ed5f-471d-9b98-5905ff49bff0	\N
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
4ad11d12-d29a-4f8d-87e9-dc2c288a785d	1acbe937-22e8-4c40-ae6a-76b4615931da	1970-01-01 00:00:00+00	2026-03-15 19:34:41.989523+00	"172.25.0.0/28"	172.25.0.0/28	\N	Lan	{"type": "Discovery"}	1970-01-01 00:00:00+00	\N	\N	1970-01-01 00:00:00+00	\N	\N	\N	\N	"Unspecified"	"Unspecified"	"Unspecified"
99b84c46-b493-45a4-b906-22a9bf789033	1acbe937-22e8-4c40-ae6a-76b4615931da	2026-11-13 00:42:08.298298+00	2026-11-13 00:42:08.298298+00	"10.1.0.0/24"	Blocked Subnet	\N	Lan	{"type": "Manual"}	2026-11-13 00:42:08.298298+00	\N	\N	2026-11-13 00:42:08.298298+00	\N	\N	\N	\N	"Manual"	"Manual"	"Manual"
\.


--
-- Data for Name: tag_groups; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tag_groups (id, organization_id, name, description, group_type, entity_types, ordered, created_at, updated_at) FROM stdin;
e39ce273-dbdc-4b43-aa57-31b1cb255cfb	60fc216c-ed5f-471d-9b98-5905ff49bff0	Application	What each entity is part of. A service inherits its host's application tag, and the Application view groups by it.	Application	\N	f	2026-11-13 00:32:16.960834+00	2026-11-13 00:32:16.960834+00
8f579392-5439-490d-baa8-13341f1ebe6b	60fc216c-ed5f-471d-9b98-5905ff49bff0	Status	Where an asset is in its lifecycle.	Status	\N	t	2026-11-13 00:32:16.962986+00	2026-11-13 00:32:16.962986+00
ad461685-8cb8-4298-9893-43953d795496	60fc216c-ed5f-471d-9b98-5905ff49bff0	Visibility	Entities holding a tag of this group are left out of every topology.	Visibility	\N	f	2026-11-13 00:32:16.964609+00	2026-11-13 00:32:16.964609+00
3c28172e-d7f8-46a7-920f-efb935b3f07b	60fc216c-ed5f-471d-9b98-5905ff49bff0	Environment	Which stage of delivery an asset serves.	Custom	\N	t	2026-11-13 00:32:17.058628+00	2026-11-13 00:32:17.058628+00
1bc74494-a513-448b-b2c5-5ef08f6583f8	60fc216c-ed5f-471d-9b98-5905ff49bff0	Criticality	How much an outage of the asset costs.	Custom	\N	t	2026-11-13 00:32:17.059736+00	2026-11-13 00:32:17.059736+00
72cdd1f4-ce25-4d6a-9168-27f502d26048	60fc216c-ed5f-471d-9b98-5905ff49bff0	Management	Who administers the asset.	Custom	\N	t	2026-11-13 00:32:17.060734+00	2026-11-13 00:32:17.060734+00
\.


--
-- Data for Name: tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tags (id, organization_id, name, description, created_at, updated_at, color, valid_from, valid_to, lineage_id, snapshot_id, icon, tag_group_id, "position", source) FROM stdin;
a56bb817-1dfa-4c84-9e78-a9bdb2f3f874	60fc216c-ed5f-471d-9b98-5905ff49bff0	Hidden	Left out of topology.	2026-11-13 00:32:16.96586+00	2026-11-13 00:32:16.96586+00	Gray	2026-11-13 00:32:16.96586+00	\N	\N	\N	eye-off	ad461685-8cb8-4298-9893-43953d795496	\N	{"type": "System"}
d2cc3b7c-1f8d-42de-b346-3d0ebcf1998e	60fc216c-ed5f-471d-9b98-5905ff49bff0	Planned	Ordered or scheduled, not yet in service	2026-11-13 00:32:17.065118+00	2026-11-13 00:32:17.065118+00	Blue	2026-11-13 00:32:17.065118+00	\N	\N	\N	calendar-clock	8f579392-5439-490d-baa8-13341f1ebe6b	1	{"type": "System"}
b3c312a0-694c-41a3-a608-bfb53cce107f	60fc216c-ed5f-471d-9b98-5905ff49bff0	Active	In service	2026-11-13 00:32:17.065128+00	2026-11-13 00:32:17.065128+00	Green	2026-11-13 00:32:17.065128+00	\N	\N	\N	circle-check	8f579392-5439-490d-baa8-13341f1ebe6b	2	{"type": "System"}
30bcb681-1d9c-4c6e-a991-18c72d1ffbb6	60fc216c-ed5f-471d-9b98-5905ff49bff0	Decommissioning	Being retired; still on the network	2026-11-13 00:32:17.06513+00	2026-11-13 00:32:17.06513+00	Orange	2026-11-13 00:32:17.06513+00	\N	\N	\N	hourglass	8f579392-5439-490d-baa8-13341f1ebe6b	3	{"type": "System"}
e878f4e1-d792-4baa-b979-c2968973b906	60fc216c-ed5f-471d-9b98-5905ff49bff0	Decommissioned	Retired from service; kept for the record	2026-11-13 00:32:17.065132+00	2026-11-13 00:32:17.065132+00	Gray	2026-11-13 00:32:17.065132+00	\N	\N	\N	archive	8f579392-5439-490d-baa8-13341f1ebe6b	4	{"type": "System"}
814cac2a-4a24-4025-8c46-3c988a920b5e	60fc216c-ed5f-471d-9b98-5905ff49bff0	Production	Serves live users and business traffic	2026-11-13 00:32:17.065137+00	2026-11-13 00:32:17.065137+00	Red	2026-11-13 00:32:17.065137+00	\N	\N	\N	server	3c28172e-d7f8-46a7-920f-efb935b3f07b	1	{"type": "System"}
fce7d8c8-e3bf-4825-a824-5fb388a64a37	60fc216c-ed5f-471d-9b98-5905ff49bff0	Staging	A pre-release copy of production for final testing	2026-11-13 00:32:17.065139+00	2026-11-13 00:32:17.065139+00	Amber	2026-11-13 00:32:17.065139+00	\N	\N	\N	flask-conical	3c28172e-d7f8-46a7-920f-efb935b3f07b	2	{"type": "System"}
e9455cc9-165b-4d4d-aa87-694ac0332741	60fc216c-ed5f-471d-9b98-5905ff49bff0	Development	Used to build and test changes	2026-11-13 00:32:17.065141+00	2026-11-13 00:32:17.065141+00	Blue	2026-11-13 00:32:17.065141+00	\N	\N	\N	code	3c28172e-d7f8-46a7-920f-efb935b3f07b	3	{"type": "System"}
c8160375-d3ed-4b54-83da-3df89a442c1c	60fc216c-ed5f-471d-9b98-5905ff49bff0	Lab	Test bench or experimental equipment	2026-11-13 00:32:17.065143+00	2026-11-13 00:32:17.065143+00	Purple	2026-11-13 00:32:17.065143+00	\N	\N	\N	test-tube	3c28172e-d7f8-46a7-920f-efb935b3f07b	4	{"type": "System"}
fb725a84-02dd-4cdb-ba6f-9bd46f09ac62	60fc216c-ed5f-471d-9b98-5905ff49bff0	Critical	An outage stops the business	2026-11-13 00:32:17.065147+00	2026-11-13 00:32:17.065147+00	Red	2026-11-13 00:32:17.065147+00	\N	\N	\N	siren	1bc74494-a513-448b-b2c5-5ef08f6583f8	1	{"type": "System"}
4b63ba1a-5353-43db-85e4-654b16671b45	60fc216c-ed5f-471d-9b98-5905ff49bff0	High	An outage disrupts many people	2026-11-13 00:32:17.065149+00	2026-11-13 00:32:17.065149+00	Orange	2026-11-13 00:32:17.065149+00	\N	\N	\N	arrow-up	1bc74494-a513-448b-b2c5-5ef08f6583f8	2	{"type": "System"}
6587f9cf-a0f9-41e0-a539-658de3170d75	60fc216c-ed5f-471d-9b98-5905ff49bff0	Medium	An outage disrupts some people	2026-11-13 00:32:17.065151+00	2026-11-13 00:32:17.065151+00	Yellow	2026-11-13 00:32:17.065151+00	\N	\N	\N	minus	1bc74494-a513-448b-b2c5-5ef08f6583f8	3	{"type": "System"}
ddfde3e3-7ab7-489d-840c-45cc3a3457d4	60fc216c-ed5f-471d-9b98-5905ff49bff0	Low	An outage has little impact	2026-11-13 00:32:17.065152+00	2026-11-13 00:32:17.065152+00	Gray	2026-11-13 00:32:17.065152+00	\N	\N	\N	arrow-down	1bc74494-a513-448b-b2c5-5ef08f6583f8	4	{"type": "System"}
50534b4f-67ea-4d3e-96bc-298a0b96a385	60fc216c-ed5f-471d-9b98-5905ff49bff0	Managed	Administered by your IT team	2026-11-13 00:32:17.065158+00	2026-11-13 00:32:17.065158+00	Green	2026-11-13 00:32:17.065158+00	\N	\N	\N	shield-check	72cdd1f4-ce25-4d6a-9168-27f502d26048	1	{"type": "System"}
eda2a3c2-82ee-40eb-9f84-5e962be22bef	60fc216c-ed5f-471d-9b98-5905ff49bff0	Unmanaged	Administered by no one, such as a personal device	2026-11-13 00:32:17.065159+00	2026-11-13 00:32:17.065159+00	Orange	2026-11-13 00:32:17.065159+00	\N	\N	\N	shield-question-mark	72cdd1f4-ce25-4d6a-9168-27f502d26048	2	{"type": "System"}
77763abd-ce8d-40a9-b089-ece9ff11d706	60fc216c-ed5f-471d-9b98-5905ff49bff0	Third-party	Administered by a vendor or another organization	2026-11-13 00:32:17.065161+00	2026-11-13 00:32:17.065161+00	Blue	2026-11-13 00:32:17.065161+00	\N	\N	\N	handshake	72cdd1f4-ce25-4d6a-9168-27f502d26048	3	{"type": "System"}
aba9085a-fe4a-4c5e-8cc8-4bf7b9e96313	60fc216c-ed5f-471d-9b98-5905ff49bff0	Integration Test Tag	\N	2026-11-13 00:41:15.02719+00	2026-11-13 00:41:15.02719+00	Yellow	2026-11-13 00:41:15.02719+00	\N	\N	\N	\N	\N	\N	{"type": "Manual"}
03cabf5f-caad-4d5c-9a13-ca4e7be8add2	60fc216c-ed5f-471d-9b98-5905ff49bff0	Test Tag	\N	2026-11-13 00:42:07.041378+00	2026-11-13 00:42:07.041378+00	Yellow	2026-11-13 00:42:07.041378+00	2026-11-13 00:42:07.064542+00	4230e58f-6a62-49a1-8f0f-a128e8b1c20d	\N	\N	\N	\N	{"type": "Manual"}
4230e58f-6a62-49a1-8f0f-a128e8b1c20d	60fc216c-ed5f-471d-9b98-5905ff49bff0	Updated Tag	\N	2026-11-13 00:42:07.041378+00	2026-11-13 00:42:07.041378+00	Yellow	2026-11-13 00:42:07.064542+00	2026-11-13 00:42:07.08333+00	\N	\N	\N	\N	\N	{"type": "Manual"}
\.


--
-- Data for Name: topologies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.topologies (id, site_id, options, created_at, updated_at) FROM stdin;
cb74a793-e489-4f36-aadc-388dfb610174	1acbe937-22e8-4c40-ae6a-76b4615931da	{"local": {"tag_filter": {}, "bundle_edges": true, "show_minimap": true, "no_fade_edges": false, "hide_edge_types": {"L3Logical": ["Hypervisor", "SameContainer"], "Workloads": ["RequestPath", "HubAndSpoke", "PhysicalLink", "NeighborLink"], "L2Physical": ["SameHost"], "Application": ["ContainerRuntime"]}}, "request": {"element_sort": {}, "element_rules": [{"id": "be3aae35-b4bb-44c9-9c74-2c794d103ff2", "rule": "ByTrunkPort"}, {"id": "0dcbf0cf-574b-4322-b0ab-f6cc25b47106", "rule": "ByVLAN"}, {"id": "0e107edf-293c-4771-954d-e80a005a2b5a", "rule": "ByPortOpStatus"}, {"id": "2b75292f-0306-4e32-808d-0ce2e87c6807", "rule": {"ByServiceCategory": {"title": "Infrastructure", "categories": ["NetworkCore", "NetworkAccess", "NetworkAppliance", "RemoteAccess", "Workstation", "Mobile", "IoT", "Printer", "OpenPorts", "NetworkIdentities"], "is_infra_rule": true}}}, {"id": "ed3fe1ec-ece5-419d-9afc-80eb21cae43c", "rule": {"ByTag": {"title": null, "tag_ids": []}}}, {"id": "89581e70-c14a-4f03-8df4-87f9310a4be3", "rule": "ByDeviceType"}, {"id": "c5c56c0f-fbb0-45f2-a663-9888d0ba36d6", "rule": "ByHypervisor"}, {"id": "d1b08c29-0298-4fcd-bd87-7cf3501ad92a", "rule": "ByContainerRuntime"}, {"id": "862a20fe-e188-42fc-ad5d-d386a4454d4b", "rule": "ByStack"}], "hide_entities": {}, "container_rules": {"L3Logical": [{"id": "97b13148-f182-4e5e-9438-98872b7c22c3", "rule": "BySubnet"}, {"id": "21ba521c-530f-4056-a795-72364ba5e4b5", "rule": "MergeContainerBridges"}], "Workloads": [{"id": "8e4b5b25-ec41-4bdd-9d41-2d1755024366", "rule": "ByHost"}], "L2Physical": [{"id": "8e4b5b25-ec41-4bdd-9d41-2d1755024366", "rule": "ByHost"}], "Application": [{"id": "3ad72bee-1673-4358-a0de-632d715a72f1", "rule": {"ByApplication": {"tag_ids": []}}}]}, "hide_metadata_values": {"L3Logical": {"Service": {"Category": ["OpenPorts"]}}, "Workloads": {"Service": {"Category": ["OpenPorts"]}}, "L2Physical": {"Service": {"Category": ["OpenPorts"]}, "Interface": {"LinkState": ["Unlinked"]}}, "Application": {"Service": {"Category": ["OpenPorts"]}}}}}	2026-11-13 00:32:16.997946+00	2026-11-13 00:32:16.997946+00
\.


--
-- Data for Name: user_api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_api_keys (id, key, user_id, organization_id, permissions, name, created_at, updated_at, last_used, expires_at, is_enabled) FROM stdin;
\.


--
-- Data for Name: users; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.users (id, created_at, updated_at, password_hash, oidc_provider, oidc_subject, oidc_linked_at, email, organization_id, permissions, tags, terms_accepted_at, email_verified, email_verification_token, email_verification_expires, password_reset_token, password_reset_expires, pending_email, email_settings, session_epoch, display_settings, email_unsubscribe_token, release_notes_seen_version, terms_notice_version) FROM stdin;
f605e587-918a-411d-9fd8-9afed452c38c	2026-11-13 00:32:16.931901+00	2026-11-13 00:32:16.931901+00	$argon2id$v=19$m=19456,t=2,p=1$YEldmjB+JG1SNC0U9exQpw$UXTfbXEwPKdpFAnZLh4ezvD27VcOP9Hq0tnAIEFOq6c	\N	\N	\N	user@gmail.com	60fc216c-ed5f-471d-9b98-5905ff49bff0	Owner	{}	\N	t	\N	\N	\N	\N	\N	{"daemon_alerts": true, "product_updates": true, "trial_and_usage": true, "discovery_digest": true, "product_onboarding": true}	0	{"clock": "browser_default", "time_zone": null, "date_order": "browser_default", "timestamps": "relative", "week_start": "monday", "table_density": "compact"}	\N	0.20.0	\N
77c20b2e-5556-4673-b55f-7ba75c50b05f	2026-11-13 00:42:07.910199+00	2026-11-13 00:42:07.910199+00	\N	\N	\N	\N	user@example.com	60fc216c-ed5f-471d-9b98-5905ff49bff0	Owner	{}	\N	f	\N	\N	\N	\N	\N	{"daemon_alerts": true, "product_updates": true, "trial_and_usage": true, "discovery_digest": true, "product_onboarding": true}	0	{"clock": "browser_default", "time_zone": null, "date_order": "browser_default", "timestamps": "relative", "week_start": "monday", "table_density": "compact"}	\N	\N	\N
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
ffT9WlLcXysu23IEduf3Qw	\\x93c41043f7e7760472db2e2b5fdc525afdf47d82a7757365725f6964d92466363035653538372d393138612d343131642d396664382d396166656434353263333863ad73657373696f6e5f65706f63680099cd07eacd0144002010ce38891483000000	2026-11-20 00:32:16.948507+00
I1IFGxYlLioZTuQ9ZR9oLA	\\x93c4102c681f653de44e192a2e25161b05522383a7757365725f6964d92466363035653538372d393138612d343131642d396664382d396166656434353263333863ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92433373561326663612d323866662d343262612d626530332d323864393463313430646533a87573655f63617365a56f74686572ad73657373696f6e5f65706f63680099cd07eacd014400290fce35fd1ceb000000	2026-11-20 00:41:15.90578+00
UKqWTDRNuFH-PZb9RvB_PA	\\x93c4103c7ff046fd963dfe51b84d344c96aa5083ad73657373696f6e5f65706f636800ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92463653466633564332d343531622d346662342d623363382d353661633366383634663135a87573655f63617365a56f74686572a7757365725f6964d92466363035653538372d393138612d343131642d396664382d39616665643435326333386399cd07eacd0144002936ce36a9efda000000	2026-11-20 00:41:54.917106+00
wPZ2b0GdPIVYPxkLS4CVYA	\\x93c4106095804b0b193f58853c9d416f76f6c083ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92466616634636239642d653362392d343032642d393564312d653838623232386237616164a87573655f63617365a56f74686572ad73657373696f6e5f65706f636800a7757365725f6964d92466363035653538372d393138612d343131642d396664382d39616665643435326333386399cd07eacd0144002a06ce2ac84bdd000000	2026-11-20 00:42:06.717769+00
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
-- Name: site_assignments site_assignments_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.site_assignments
    ADD CONSTRAINT site_assignments_pkey PRIMARY KEY (id);


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
-- Name: idx_credentials_login_credential_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_credentials_login_credential_id ON public.credentials USING btree (login_credential_id);


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
-- Name: site_assignments_api_key_scope; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX site_assignments_api_key_scope ON public.site_assignments USING btree (api_key_id, COALESCE(site_id, '00000000-0000-0000-0000-000000000000'::uuid)) WHERE (api_key_id IS NOT NULL);


--
-- Name: site_assignments_invite_scope; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX site_assignments_invite_scope ON public.site_assignments USING btree (invite_id, COALESCE(site_id, '00000000-0000-0000-0000-000000000000'::uuid)) WHERE (invite_id IS NOT NULL);


--
-- Name: site_assignments_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX site_assignments_organization ON public.site_assignments USING btree (organization_id);


--
-- Name: site_assignments_site; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX site_assignments_site ON public.site_assignments USING btree (site_id);


--
-- Name: site_assignments_user_scope; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX site_assignments_user_scope ON public.site_assignments USING btree (user_id, COALESCE(site_id, '00000000-0000-0000-0000-000000000000'::uuid)) WHERE (user_id IS NOT NULL);


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
-- Name: credentials credentials_login_credential_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.credentials
    ADD CONSTRAINT credentials_login_credential_id_fkey FOREIGN KEY (login_credential_id) REFERENCES public.credentials(id);


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
-- Name: site_assignments site_assignments_api_key_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.site_assignments
    ADD CONSTRAINT site_assignments_api_key_id_fkey FOREIGN KEY (api_key_id) REFERENCES public.user_api_keys(id) ON DELETE CASCADE;


--
-- Name: site_assignments site_assignments_invite_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.site_assignments
    ADD CONSTRAINT site_assignments_invite_id_fkey FOREIGN KEY (invite_id) REFERENCES public.invites(id) ON DELETE CASCADE;


--
-- Name: site_assignments site_assignments_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.site_assignments
    ADD CONSTRAINT site_assignments_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: site_assignments site_assignments_site_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.site_assignments
    ADD CONSTRAINT site_assignments_site_id_fkey FOREIGN KEY (site_id) REFERENCES public.sites(id) ON DELETE CASCADE;


--
-- Name: site_assignments site_assignments_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.site_assignments
    ADD CONSTRAINT site_assignments_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


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

\unrestrict Z6oehPEXm3bmRWcuDQDHQEX5uXEAn53kDfXCleMgj3y0nrIu2ydPcqdugCWafLe

