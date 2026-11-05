--
-- PostgreSQL database dump
--

\restrict G3wKpFQsW5syLKLkic0HnJhgEKkRF2DPNxAQQG5AbvSz7JF7tfLMv205qMcW7Vb

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
    login_credential_id uuid
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
20251006215000	users	2026-11-04 16:48:44.386637+00	t	\\x4f13ce14ff67ef0b7145987c7b22b588745bf9fbb7b673450c26a0f2f9a36ef8ca980e456c8d77cfb1b2d7a4577a64d7	-1
20251006215100	networks	2026-11-04 16:48:44.392225+00	t	\\xeaa5a07a262709f64f0c59f31e25519580c79e2d1a523ce72736848946a34b17dd9adc7498eaf90551af6b7ec6d4e0e3	-1
20251006215151	create_hosts	2026-11-04 16:48:44.39643+00	t	\\x6ec7487074c0724932d21df4cf1ed66645313cf62c159a7179e39cbc261bcb81a24f7933a0e3cf58504f2a90fc5c1962	-1
20251006215155	create_subnets	2026-11-04 16:48:44.400597+00	t	\\xefb5b25742bd5f4489b67351d9f2494a95f307428c911fd8c5f475bfb03926347bdc269bbd048d2ddb06336945b27926	-1
20251006215201	create_groups	2026-11-04 16:48:44.404878+00	t	\\x0a7032bf4d33a0baf020e905da865cde240e2a09dda2f62aa535b2c5d4b26b20be30a3286f1b5192bd94cd4a5dbb5bcd	-1
20251006215204	create_daemons	2026-11-04 16:48:44.409412+00	t	\\xcfea93403b1f9cf9aac374711d4ac72d8a223e3c38a1d2a06d9edb5f94e8a557debac3668271f8176368eadc5105349f	-1
20251006215212	create_services	2026-11-04 16:48:44.414593+00	t	\\xd5b07f82fc7c9da2782a364d46078d7d16b5c08df70cfbf02edcfe9b1b24ab6024ad159292aeea455f15cfd1f4740c1d	-1
20251029193448	user-auth	2026-11-04 16:48:44.421691+00	t	\\xfde8161a8db89d51eeade7517d90a41d560f19645620f2298f78f116219a09728b18e91251ae31e46a47f6942d5a9032	-1
20251030044828	daemon_api	2026-11-04 16:48:44.423734+00	t	\\x181eb3541f51ef5b038b2064660370775d1b364547a214a20dde9c9d4bb95a1c273cd4525ef29e61fa65a3eb4fee0400	-1
20251030170438	host-hide	2026-11-04 16:48:44.425275+00	t	\\x87c6fda7f8456bf610a78e8e98803158caa0e12857c5bab466a5bb0004d41b449004a68e728ca13f17e051f662a15454	-1
20251102224919	create_discovery	2026-11-04 16:48:44.435681+00	t	\\xb32a04abb891aba48f92a059fae7341442355ca8e4af5d109e28e2a4f79ee8e11b2a8f40453b7f6725c2dd6487f26573	-1
20251106235621	normalize-daemon-cols	2026-11-04 16:48:44.43799+00	t	\\x5b137118d506e2708097c432358bf909265b3cf3bacd662b02e2c81ba589a9e0100631c7801cffd9c57bb10a6674fb3b	-1
20251107034459	api_keys	2026-11-04 16:48:44.446427+00	t	\\x3133ec043c0c6e25b6e55f7da84cae52b2a72488116938a2c669c8512c2efe72a74029912bcba1f2a2a0a8b59ef01dde	-1
20251107222650	oidc-auth	2026-11-04 16:48:44.472694+00	t	\\xd349750e0298718cbcd98eaff6e152b3fb45c3d9d62d06eedeb26c75452e9ce1af65c3e52c9f2de4bd532939c2f31096	-1
20251110181948	orgs-billing	2026-11-04 16:48:44.483526+00	t	\\x5bbea7a2dfc9d00213bd66b473289ddd66694eff8a4f3eaab937c985b64c5f8c3ad2d64e960afbb03f335ac6766687aa	-1
20251113223656	group-enhancements	2026-11-04 16:48:44.48516+00	t	\\xbe0699486d85df2bd3edc1f0bf4f1f096d5b6c5070361702c4d203ec2bb640811be88bb1979cfe51b40805ad84d1de65	-1
20251117032720	daemon-mode	2026-11-04 16:48:44.486752+00	t	\\xdd0d899c24b73d70e9970e54b2c748d6b6b55c856ca0f8590fe990da49cc46c700b1ce13f57ff65abd6711f4bd8a6481	-1
20251118143058	set-default-plan	2026-11-04 16:48:44.488394+00	t	\\xd19142607aef84aac7cfb97d60d29bda764d26f513f2c72306734c03cec2651d23eee3ce6cacfd36ca52dbddc462f917	-1
20251118225043	save-topology	2026-11-04 16:48:44.496939+00	t	\\x011a594740c69d8d0f8b0149d49d1b53cfbf948b7866ebd84403394139cb66a44277803462846b06e762577adc3e61a3	-1
20251123232748	network-permissions	2026-11-04 16:48:44.500242+00	t	\\x161be7ae5721c06523d6488606f1a7b1f096193efa1183ecdd1c2c9a4a9f4cad4884e939018917314aaf261d9a3f97ae	-1
20251125001342	billing-updates	2026-11-04 16:48:44.50167+00	t	\\xa235d153d95aeb676e3310a52ccb69dfbd7ca36bba975d5bbca165ceeec7196da12119f23597ea5276c364f90f23db1e	-1
20251128035448	org-onboarding-status	2026-11-04 16:48:44.503605+00	t	\\x1d7a7e9bf23b5078250f31934d1bc47bbaf463ace887e7746af30946e843de41badfc2b213ed64912a18e07b297663d8	-1
20251129180942	nfs-consolidate	2026-11-04 16:48:44.505304+00	t	\\xb38f41d30699a475c2b967f8e43156f3b49bb10341bddbde01d9fb5ba805f6724685e27e53f7e49b6c8b59e29c74f98e	-1
20251206052641	discovery-progress	2026-11-04 16:48:44.507341+00	t	\\x9d433b7b8c58d0d5437a104497e5e214febb2d1441a3ad7c28512e7497ed14fb9458e0d4ff786962a59954cb30da1447	-1
20251206202200	plan-fix	2026-11-04 16:48:44.508695+00	t	\\x242f6699dbf485cf59a8d1b8cd9d7c43aeef635a9316be815a47e15238c5e4af88efaa0daf885be03572948dc0c9edac	-1
20251207061341	daemon-url	2026-11-04 16:48:44.511465+00	t	\\x01172455c4f2d0d57371d18ef66d2ab3b7a8525067ef8a86945c616982e6ce06f5ea1e1560a8f20dadcd5be2223e6df1	-1
20251210045929	tags	2026-11-04 16:48:44.520826+00	t	\\xe3dde83d39f8552b5afcdc1493cddfeffe077751bf55472032bc8b35fc8fc2a2caa3b55b4c2354ace7de03c3977982db	-1
20251210175035	terms	2026-11-04 16:48:44.522313+00	t	\\xe47f0cf7aba1bffa10798bede953da69fd4bfaebf9c75c76226507c558a3595c6bfc6ac8920d11398dbdf3b762769992	-1
20251213025048	hash-keys	2026-11-04 16:48:44.531539+00	t	\\xfc7cbb8ce61f0c225322297f7459dcbe362242b9001c06cb874b7f739cea7ae888d8f0cfaed6623bcbcb9ec54c8cd18b	-1
20251214050638	scanopy	2026-11-04 16:48:44.533738+00	t	\\x0108bb39832305f024126211710689adc48d973ff66e5e59ff49468389b75c1ff95d1fbbb7bdb50e33ec1333a1f29ea6	-1
20251215215724	topo-scanopy-fix	2026-11-04 16:48:44.534899+00	t	\\xed88a4b71b3c9b61d46322b5053362e5a25a9293cd3c420c9df9fcaeb3441254122b8a18f58c297f535c842b8a8b0a38	-1
20251217153736	category_rename	2026-11-04 16:48:44.537176+00	t	\\x03af7ec905e11a77e25038a3c272645da96014da7c50c585a25cea3f9a7579faba3ff45114a5e589d144c9550ba42421	-1
20251218053111	invite-persistence	2026-11-04 16:48:44.542852+00	t	\\x21d12f48b964acfd600f88e70ceb14abd9cf2a8a10db2eae2a6d8f44cf7d20749f93293631e6123e92b7c3c1793877c2	-1
20251219211216	create_shares	2026-11-04 16:48:44.550102+00	t	\\x036485debd3536f9e58ead728f461b925585911acf565970bf3b2ab295b12a2865606d6a56d334c5641dcd42adeb3d68	-1
20251220170928	permissions-cleanup	2026-11-04 16:48:44.551992+00	t	\\x632f7b6702b494301e0d36fd3b900686b1a7f9936aef8c084b5880f1152b8256a125566e2b5ac40216eaadd3c4c64a03	-1
20251220180000	commercial-to-community	2026-11-04 16:48:44.553227+00	t	\\x26fc298486c225f2f01271d611418377c403183ae51daf32fef104ec07c027f2017d138910c4fbfb5f49819a5f4194d6	-1
20251221010000	cleanup_subnet_type	2026-11-04 16:48:44.554474+00	t	\\xb521121f3fd3a10c0de816977ac2a2ffb6118f34f8474ffb9058722abc0dc4cf5cbec83bc6ee49e79a68e6b715087f40	-1
20251221020000	remove_host_target	2026-11-04 16:48:44.555948+00	t	\\x77b5f8872705676ca81a5704bd1eaee90b9a52b404bdaa27a23da2ffd4858d3e131680926a5a00ad2a0d7a24ba229046	-1
20251221030000	user_network_access	2026-11-04 16:48:44.563416+00	t	\\x5c23f5bb6b0b8ca699a17eee6730c4197a006ca21fecc79136a5e5697b9211a81b4cd08ceda70dace6a26408d021ff3a	-1
20251221040000	interfaces_table	2026-11-04 16:48:44.573672+00	t	\\xf7977b6f1e7e5108c614397d03a38c9bd9243fdc422575ec29610366a0c88f443de2132185878d8e291f06a50a8c3244	-1
20251221050000	ports_table	2026-11-04 16:48:44.582854+00	t	\\xdf72f9306b405be7be62c39003ef38408115e740b120f24e8c78b8e136574fff7965c52023b3bc476899613fa5f4fe35	-1
20251221060000	bindings_table	2026-11-04 16:48:44.593928+00	t	\\x933648a724bd179c7f47305e4080db85342d48712cde39374f0f88cde9d7eba8fe5fafba360937331e2a8178dec420c4	-1
20251221070000	group_bindings	2026-11-04 16:48:44.600823+00	t	\\x697475802f6c42e38deee6596f4ba786b09f7b7cd91742fbc5696dd0f9b3ddfce90dd905153f2b1a9e82f959f5a88302	-1
20251222020000	tag_cascade_delete	2026-11-04 16:48:44.602695+00	t	\\xabfb48c0da8522f5c8ea6d482eb5a5f4562ed41f6160a5915f0fd477c7dd0517aa84760ef99ab3a5db3e0f21b0c69b5f	-1
20251223232524	network_remove_default	2026-11-04 16:48:44.604189+00	t	\\x7099fe4e52405e46269d7ce364050da930b481e72484ad3c4772fd2911d2d505476d659fa9f400c63bc287512d033e18	-1
20251225100000	color_enum	2026-11-04 16:48:44.605882+00	t	\\x62cecd9d79a49835a3bea68a7959ab62aa0c1aaa7e2940dec6a7f8a714362df3649f0c1f9313672d9268295ed5a1cfa9	-1
20251227010000	topology_snapshot_migration	2026-11-04 16:48:44.611006+00	t	\\xc042591d254869c0e79c8b52a9ede680fd26f094e2c385f5f017e115f5e3f31ad155f4885d095344f2642ebb70755d54	-1
20251228010000	user_api_keys	2026-11-04 16:48:44.62244+00	t	\\xa41adb558a5b9d94a4e17af3f16839b83f7da072dbeac9251b12d8a84c7bec6df008009acf246468712a975bb36bb5f5	-1
20251230160000	daemon_version_and_maintainer	2026-11-04 16:48:44.625693+00	t	\\xafed3d9f00adb8c1b0896fb663af801926c218472a0a197f90ecdaa13305a78846a9e15af0043ec010328ba533fca68f	-1
20260103000000	service_position	2026-11-04 16:48:44.628054+00	t	\\x19d00e8c8b300d1c74d721931f4d771ec7bc4e06db0d6a78126e00785586fdc4bcff5b832eeae2fce0cb8d01e12a7fb5	-1
20260106000000	interface_mac_index	2026-11-04 16:48:44.630245+00	t	\\xa26248372a1e31af46a9c6fbdaef178982229e2ceeb90cc6a289d5764f87a38747294b3adf5f21276b5d171e42bdb6ac	-1
20260106204402	entity_tags_junction	2026-11-04 16:48:44.64308+00	t	\\xf73c604f9f0b8db065d990a861684b0dbd62c3ef9bead120c68431c933774de56491a53f021e79f09801680152f5a08a	-1
20260108033856	fix_entity_tags_json_format	2026-11-04 16:48:44.645326+00	t	\\x197eaa063d4f96dd0e897ad8fd96cc1ba9a54dda40a93a5c12eac14597e4dea4c806dd0a527736fb5807b7a8870d9916	-1
20260110000000	email_verification	2026-11-04 16:48:44.649103+00	t	\\xb8da8433f58ba4ce846b9fa0c2551795747a8473ad10266b19685504847458ea69d27a0ce430151cfb426f5f5fb6ac3a	-1
20260114145808	daemon_user_fk_set_null	2026-11-04 16:48:44.650748+00	t	\\x57b060be9fc314d7c5851c75661ca8269118feea6cf7ee9c61b147a0e117c4d39642cf0d1acdf7a723a9a76066c1b8ff	-1
20260116010000	snmp_credentials	2026-11-04 16:48:44.657673+00	t	\\x6f3971cf194d56883c61fa795406a8ab568307ed86544920d098b32a6a1ebb7effcb5ec38a70fdc9b617eff92d63d51e	-1
20260116020000	host_snmp_fields	2026-11-04 16:48:44.66207+00	t	\\xf2f088c13ab0dd34e1cb1e5327b0b4137440b0146e5ce1e78b8d2dfa05d9b5a12a328eeb807988453a8a43ad8a1c95ba	-1
20260116030000	if_entries	2026-11-04 16:48:44.676229+00	t	\\xa58391708f8b21901ab9250af528f638a6055462f70ffddfd7c451433aacdabd62825546fa8be108f23a3cae78b8ae28	-1
20260116100000	daemon_api_key_link	2026-11-04 16:48:44.68007+00	t	\\x41088aa314ab173344a6b416280721806b2f296a32a8d8cae58c7e5717f389fe599134ed03980ed97e4b7659e99c4f82	-1
20260131190000	add_hubspot_company_id	2026-11-04 16:48:44.6815+00	t	\\x4326f95f4954e176157c1c3e034074a3e5c44da4d60bbd7a9e4b6238c9ef52a30f8b38d3c887864b6e4c1163dc062beb	-1
20260201021238	fix_service_acronym_capitalization	2026-11-04 16:48:44.683351+00	t	\\x88b010ac8f0223d880ea6a730f11dc6d27fa5de9d8747de3431e46d59f1dbf2f72ae4a87c2e52c32152549f5c1f96bb2	-1
20260204004436	add_entity_tags_to_topology	2026-11-04 16:48:44.684947+00	t	\\x3eff1a1490e77065ec861ef1b9aad8c55de0170106a42720f7931b3929b179122b16e44390b2652771bf91bba32a7757	-1
20260205120000	billing_overhaul	2026-11-04 16:48:44.687423+00	t	\\xbf850cfa0c40a3c65f574efd15fd55a4b702296203d28077a09d1c22076fee8601f2b78345aef370ab9163657de767ab	-1
20260205183207	rename_hubspot_to_brevo	2026-11-04 16:48:44.688907+00	t	\\x4678a7d80215e5eafb5e80af0daa20e2868a3b4f2112e88cb1b2b9efc87d63de3fb96c133f359b224c658789ae4b0d13	-1
20260221120000	add_plan_limit_notifications	2026-11-04 16:48:44.690585+00	t	\\xef770dac07e1d80888832f33184dc46c1d3b8185b91c507cb404468d6ad8c29cacf455178801c67aa27b6a626d3ad82d	-1
20260222120000	add_pending_email	2026-11-04 16:48:44.69203+00	t	\\xddd220f7602c44548d56849c0a8d081ecd1da1383374a11e3e227c7d9becb73a49f5e5bb09ed65901c16df4c16e913e5	-1
20260301120000	add_if_name_to_if_entries	2026-11-04 16:48:44.69344+00	t	\\xc9fc0a2b77ecbf0e1d5ab292c4fe162a26113468c878dfd26a3c63d89c0ee1957ca328ecfe25c611867a0e73780f0cb6	-1
20260306002816	cleanup_standby	2026-11-04 16:48:44.694805+00	t	\\x01b0c236a8a4d0d97f0f633b18f8cbdb92b6d72063289989b90a1b7b6b303e65e0557eb09927b2580dcb7e8ee5966c75	-1
20260309120000	add_org_use_case	2026-11-04 16:48:44.696211+00	t	\\xdb8c8a2f0f9416ba3b687fc75453d7c12c50a6f386b4784d21bd6adfc4a4a7556c637c25cf116118402bbd12c0d5aafe	-1
20260313120000	snmp_extended_discovery	2026-11-04 16:48:44.69839+00	t	\\xc4e72539099de1b830d87a169bfbabba4b8fb378a3c4c4a1dfca698adf3e403d750040d784c26d9fa343be2908064c9d	-1
20260315120000	universal_credentials	2026-11-04 16:48:44.716405+00	t	\\x87dc6f39202e81d5555df78a9d056b143f11bd22e6d7f483065f605e242a360902c72c4d5a49717e7fcc24a366bb5ff5	-1
20260315120001	discovery_scan_settings	2026-11-04 16:48:44.717934+00	t	\\xe9da183fdd8e04e574f553f61f6f33efa046cdae38c846c8077b06c5260446fb4aa39da2449bda7f1d8cf3aa9f16e158	-1
20260315120002	backfill_org_created_milestone	2026-11-04 16:48:44.71914+00	t	\\x14f886a19773cd2263d86f88479be460d21f071d5212e3789c5c40b6415c293fc7d06c7b138351cc42108f89a14fe745	-1
20260316120000	fix_jsonb_null_if_entries	2026-11-04 16:48:44.721208+00	t	\\x65c358069710f7f86d6a3e257e658c2f241cc376433c3a0317b0ec9e1876a66f9738cb65c6ab1a5c197fe40d5aa2aa2b	-1
20260319120000	rename_snmp_to_snmpv2c	2026-11-04 16:48:44.722702+00	t	\\xdce5c9461f402e1672607078b2c571f0eb30b51d46f8e9414d8909efb40693f543e49e560cb7d703db274515043aa08e	-1
20260321120000	add_discovery_scan_count	2026-11-04 16:48:44.725227+00	t	\\x6c8201ab453a51632176d534c6604e0818e28a8a4a153e33e254f4dac0f9b67c9db394082cb663ff1b25941229cf96fc	-1
20260329120000	backfill_subnet_virtualization	2026-11-04 16:48:44.727789+00	t	\\xeac50ded27603dbb5e8773604a52143c9fa8654263e7dd12d3d128ce972c2feed84600e36b2e7a79525b58c44d2ad9d3	-1
20260402120000	rename_topology_node_types	2026-11-04 16:48:44.729111+00	t	\\xc4ba06868add823f83ff1948091bdfe17dbdde80bbec6fe2cf8da2b3689aeeebbe9e9de01b1292bff3c98a74d9e6279f	-1
20260403120000	topology_grouping_rules	2026-11-04 16:48:44.730989+00	t	\\x00799da1206d7c3b3c3db90b7d14437cc054ed2d7273020342e562c619a671e008ff4fdf0365170440b392956949e730	-1
20260405120000	rename_groups_to_dependencies	2026-11-04 16:48:44.739765+00	t	\\x9ce895b456366bf6e54316b22cabd2803aa542dd3733fffa680f0a3af5c4c55a612c5ee511371206921869b7f271c35b	-1
20260406120000	add_tag_is_application_group	2026-11-04 16:48:44.741575+00	t	\\xb7a71e5fdd96ca46c9c7577003309050a93bc53ad192ac5df78e7621f3ed64f07fb29b4658f17af55732cf6dfb7958c2	-1
20260406130000	add_vlans	2026-11-04 16:48:44.752548+00	t	\\x5b3e5d10578d90b5175e5718a28d7147a21b99af2fb3e0ed171d20ee8fd8838c290f648dafdd3b72ef60ff487f7f2494	-1
20260409000000	add_vlans_to_topologies	2026-11-04 16:48:44.754562+00	t	\\x5e0b9dc670580ceec3aa6eae005a39f98733fc27dc574b7f3922f4297813facd5d610af953dfec13e09d0b99eceb3865	-1
20260410000000	rename_interfaces_and_if_entries	2026-11-04 16:48:44.761959+00	t	\\x07f54a59869f458f41f45d75f250aee26b20a426f1ec29930606841770194d6aea0e9e6253a6375fbeebcf9b49121224	-1
20260414000000	add_share_enabled_views	2026-11-04 16:48:44.763506+00	t	\\xc56514355a5977c3242e728e7f5a2533e7b4a5cf8a7ce7757e412e51f1ad85e96d65c13ccd96d050be4a07799b9aef57	-1
20260415120000	rename_onboarding_first_group_created	2026-11-04 16:48:44.764782+00	t	\\x2c17035835d3ead105b76d98688c0b7bd328abdaf9f721d70d057c8afdf438819e93da56707deea5b469b81a7b84d5d7	-1
20260417000000	reindex_interfaces_identity	2026-11-04 16:48:44.769203+00	t	\\x10701e13bc3d838e2ec4a856555ebf338173792f220c405996d3c77e7987e9806798ca0328eb6259e4a62b7e05665b25	-1
20260418000000	add_standby_cleared_at_to_daemons	2026-11-04 16:48:44.770567+00	t	\\x547807de451d015a4ce1438796d5b95e2b98043c521015a21239f6778d10a8d3bf7d8b14e278e09aa0105f1935ad4181	-1
20260501000000	add_organization_billing_flags	2026-11-04 16:48:44.773216+00	t	\\x2de34c4af667d4cd8bc263c27f0526a4a2132022e2eb71ae94fd89edebbfb40cda840055a94e89a21925a317cdff285f	-1
20260502000000	scd2_add_columns	2026-11-04 16:48:44.788188+00	t	\\xe78a73574d86320c0de7fcd43682ee3cdd436dc64d1371f98cb67ef0cdf33097df6e8a92c9a26c5702795144103b085e	-1
20260502000001	scd2_add_discovery_fks_not_valid	2026-11-04 16:48:44.800519+00	t	\\xb49114eb6f8d77cb0c5062619da5c829e5bd65898710efb6207d2804be3677a95eebe88d41d21ae8cba21344fcd63f97	-1
20260502000002	scd2_validate_discovery_fks	2026-11-04 16:48:44.810591+00	t	\\x500201842b9f486d1397c0f0f3ee1a36bc101440d4d97fe0f0bfc971672f1f53f7945aed93c34e2e53d2d9cb913419f1	-1
20260502000003	scd2_backfill_with_metadata	2026-11-04 16:48:44.815182+00	t	\\xd299c1dbf7ec284995dc8a5fcf4b264e4f1ff970085a8631e6d00aa0bc6d6708a5e41d515bb9299c22ef128a958e68e9	-1
20260502000004	scd2_partial_unique_indexes	2026-11-04 16:48:44.838577+00	t	\\xbc30e554c37ec3ca72baa7fc634920257909944dc841231a966845fd4c5bc27ae20606c98867934c872e8883bd56efa1	-1
20260502000005	scd2_drop_old_unique_constraints	2026-11-04 16:48:44.842309+00	t	\\x660c0237299796c6e70a21780d529453a2ac8645f7b122a0b8aa4f362ff780de8849f53a66b71b3ea29ecc66877f47ad	-1
20260502000006	scd2_supporting_indexes	2026-11-04 16:48:44.912647+00	t	\\x4394aaf6951c4b7048f66334fb8c550c55d4cdc06beb29eb7f086a717ee9f26ddeb28e73a7b298633ef517f6c0f84ee0	-1
20260502000007	entitysource_metadata_strip	2026-11-04 16:48:44.916091+00	t	\\x1cfd1a00b100ca0b007be6e8b55f67dde412108f299f94b419a80af45b74b75fdead95637dccb13fdd57fed2f1132446	-1
20260502100000	add_user_email_settings	2026-11-04 16:48:44.917799+00	t	\\xe886b44ca9d3d6461d82b573f617391b8b5f22f18b8113a333aa8c275545ec6455060f68ecbfdd17af86fccc9a500d7a	-1
20260502120000	create_snapshots_table	2026-11-04 16:48:44.922345+00	t	\\x18f829b255b8e11e385903e51182f04fb835aa78ef0090865ce7743f747e4aaaa7b1a7b806a377e9b12af6205aed527a	-1
20260502120001	add_snapshot_id_fks	2026-11-04 16:48:44.933392+00	t	\\x0a080ceaa2922301464b30ea7f25e1f0de5493d15424d07a8dd1f2b4bf1880cf2d25f64d5bf1863f705a1557d867afa2	-1
20260502120002	add_snapshot_id_indexes	2026-11-04 16:48:44.957351+00	t	\\xb7514ff0cf637241d21e90b48214a3a3a5076d0142c99aca39479d2a82cf35e1abdbce8ad5663ca8272d5f74004ae5f9	-1
20260502120003	topology_snapshots_backfill	2026-11-04 16:48:44.963393+00	t	\\x5435da11acf8a9fc37e0520a5b2a959fce84b16080b2a5b67adcf249dea07ae533f858c56f87542152cfc271933c1428	-1
20260502120004	drop_legacy_topology_columns	2026-11-04 16:48:44.974576+00	t	\\xfe45e73be308e6267840664a662cd679fb43469a11f06a168b8b8cfc64b6e86eb10d948271b67a0541481dcc60447769	-1
20260627120000	add_discovery_integration_targets	2026-11-04 16:48:44.976582+00	t	\\x9adf9ea4c36868a3b86067214f8550b8e866aff5d3d6c6ab3d71d1a15ea5e4be0b4349db6d15308fefbceca8403f0cbb	-1
20260703120000	daemon_interfaced_subnets_junction	2026-11-04 16:48:44.981584+00	t	\\xf1399bf8104b1d9cff6a7f66c6040829f01dd64614294295f0ecc27412acc19fd6dd20eff66ce9eadf0cf0c0e8c4353e	-1
20260703120001	drop_discovery_pending_credential_ids	2026-11-04 16:48:44.983475+00	t	\\xc2cfe6f450d000ae36149b042ebcfed938c01487d78ca56a125d72bbeca0de0548824a2396761b3dc647dc80e2e43e80	-1
20260705120000	add_user_session_epoch	2026-11-04 16:48:44.985137+00	t	\\xb296babe8e50ecdd1f50770e12d4234b47ed41cdd826c545debf5f94226672cc2d6fd28eef0483a5fdc8837f5d280d88	-1
20260706120000	drop_credentials_target_ips_and_daemons_capabilities	2026-11-04 16:48:44.987246+00	t	\\xc544c88df68334ac0b3ec180da7ba2030837707f9e3cf2b75b4d56012646c24f8f60f094cc00f05c0f446a2a845e5e98	-1
20260716120000	api_keys_daemon_id	2026-11-04 16:48:44.989902+00	t	\\x484c3b2553bcf81c95aabce4be70b1e2048e097529e04a176a0b75c2edf28ecd0892c959bbd8e8166f621a9dc2dba0dc	-1
20260716120001	api_keys_daemon_id_validate	2026-11-04 16:48:44.991638+00	t	\\x723190451e98f9394b2134dccb89f8eeeb15f6a5644fb88cb24e9e0e7a4ebf5a0c455659930ce2d3096e20c004d7da89	-1
20260716120002	api_keys_daemon_id_unique	2026-11-04 16:48:44.994856+00	t	\\xf788be5a414917ec5c8b9040f85acd352ae3f580dab7e8833db5df19b7d2cf3b43b8a021c03a5082da27eca3bb7a07f0	-1
20260722120000	add_networks_stale_after_hours	2026-11-04 16:48:44.996238+00	t	\\x308d9e04ef943560d3e420c59a3c8017064d59bd28cb63f99c4b562124e2fa038630e71405171901d731420bd9de4e2f	-1
20260727120000	rename_plan_limit_notifications_to_notifications	2026-11-04 16:48:44.998209+00	t	\\x2447c735b8ac6885ae4341738294392f367abcc7c0f5e857bb08523dc5d5a8eccf77db5462b531370a0795135757098d	-1
20260728120000	rename_stale_if_entries_constraints	2026-11-04 16:48:45.001453+00	t	\\x577c54966f8c146a6e5101eb8a2f7f006ef7899ba95ec876be10970b3483b622b254d00dc9c9582efb1862404315764b	-1
20260803120000	drop_organizations_plan_limit_notifications	2026-11-04 16:48:45.003349+00	t	\\xc4c9b44a4cce6cf419cffcbbeade3d5eb119b51f321bdcd3c51a1046d8ec8df81d32f6e76c6472eefdcfae9899ef3107	-1
20260803120001	virtualization_service_id_column	2026-11-04 16:48:45.01149+00	t	\\xe07f6fee751430c94141be6d7ba17390f08f60bdcd01a65072708382b27e9543a9f27fb0cc76bb824940794d6ea67553	-1
20260819120000	hosts_name_source	2026-11-04 16:48:45.016667+00	t	\\x6526859d261bf724e6ac3bb2a51c8588764aab15ecdfad964e06e98bb59f05a753ea2950bdce40cbf7f945d157a230aa	-1
20260824120000	interfaces_neighbor_seen_at	2026-11-04 16:48:45.018676+00	t	\\x70a26934dcc110484dc95100cbc96879504a9bc0ae0fb12ca0effe032816e3282e255d90145c4d2c6bfe2eea351cb4c3	-1
20260826120000	subnets_cidr_source	2026-11-04 16:48:45.02171+00	t	\\x1d8c6a9258f1fd07a08a268cddf5f164bb3b8bb0af34d6d167e480d50b649cbdc5a7b89aa419a218266e9aa1b3d3b497	-1
20260827120000	hosts_firmware_revision	2026-11-04 16:48:45.026226+00	t	\\xa9b3a89986c699c70232d37f5909ab5fecd9c4d2c8708a11b2cad4775494a12080c6664964f942b0079a883f3ecd6ea2	-1
20260827130000	interfaces_unknown_port_facts	2026-11-04 16:48:45.028981+00	t	\\xfa35944166a6c60219a07a6792cd3abbaf909ba78e5494482d73d3d10ddacea618845822dc3dc875542772af742ac38d	-1
20260828120000	hosts_attribute_sources	2026-11-04 16:48:45.057576+00	t	\\x29220b8fb99e5ffb55ca71ea3c05f375214dd77d247d4a54dc7855c5654dbfbbbefceafeb5ddf10710c27085f0a866c4	-1
20260828120001	mac_address_sources	2026-11-04 16:48:45.060093+00	t	\\x39ea28fed133ae2c317bd5fad54e4d16f402ee895b75a6290322e75b434f5502aeddc9f6359d7e95625bb0c0d4ca1d4e	-1
20260828130000	hosts_software_revision	2026-11-04 16:48:45.06209+00	t	\\x83cdafd5691f1bc5fc2345a696ec9fd5c75700693f0011c65d352276d806a6f207f335189153ee8e0fa3fb7e60c18234	-1
20260828140000	ip_addresses_network_mac_index	2026-11-04 16:48:45.065331+00	t	\\x838b853bbb63bcfe0d0976f1c7289960e3133563151c47904ddda0ce5a0078bdc4af9ce07de63d3c024d89b831995535	-1
20260903120000	interfaces_if_descr_optional	2026-11-04 16:48:45.066993+00	t	\\x54495748b0271deebaf7cb06e9e4da9241563e83a162e4bfffaa9e571516223565ab239fcc8bb51fac9e37cbb7625757	-1
20260903120001	interface_neighbor_tables	2026-11-04 16:48:45.092488+00	t	\\xa8352a3ed9b845d96229adcb3e3eb8e796d22a8b6aa794bbc0d2ecafcbf76d68aa59a6e7713f134a57af0acceca3865a	-1
20260908120000	interfaces_ip_configured	2026-11-04 16:48:45.094621+00	t	\\x2467c0b912e78e9cac3818ffdec4157356f6b4a3c96e87a0171d88496439943eacbafefc7a4b51a2b68cc08bbbd29656	-1
20261005120001	rename_networks_to_sites_backfill	2026-11-04 16:48:45.150568+00	t	\\x085c1f2e6ec6b548e7f47fb092519f5f8889629e6198b30bd4a91e4e1d6b5e502513efb213c0601abf444d1f39183f81	-1
20261006120000	host_asset_tag	2026-11-04 16:48:45.152746+00	t	\\x1a9ef01df34f62321f6fdc26f4f75d952000cc366b54217a3107132643258e44957033c5f37b3618ff8d895d01eae918	-1
20261006120100	tag_group_and_icon	2026-11-04 16:48:45.154356+00	t	\\x3bad6e7e42b3a23b29a94462ddefba434ba2e7e92d1d9464db16644d40e45b97d01a83716203f5cda27d7bb5623dd436	-1
20261009120000	user_email_unsubscribe_token	2026-11-04 16:48:45.156161+00	t	\\x41aa0b879e77d264d1d6fc17c65bebe7e2d7796b9785ad79a85b928e23cc93c3cb8a0532ecbc8d72e34fc6e202d25ce1	-1
20261009120001	user_email_unsubscribe_token_index	2026-11-04 16:48:45.160232+00	t	\\x3cdfe6e7f6fe847ff4f18c7ee1d2430ee9833e48c6aa4eb3585c873529bb007b8786c26e6bad1c193ee8cf0dc0b23f0c	-1
20261009120100	tag_groups	2026-11-04 16:48:45.196125+00	t	\\xbdae7fe6cd73793312edbf2c987d3425f1e2c88c9fa98717a9e21550503734aaf67b3a1b804e145269962cd361aaa16a	-1
20261011120000	backfill_interface_credential_milestone	2026-11-04 16:48:45.198053+00	t	\\xfcad57a2a59537e0c9033a6ff906abf7115789554bc06c00c0caa388c220a6a5d3dc880c7aec0df9c6f561b3b94d7477	-1
20261011130000	release_notes	2026-11-04 16:48:45.202663+00	t	\\x46e5e77f7da9f2a01cb042045c01461eeb40b09477a5d2a40f302582b17e2b02e40f6e9dbe40ed1b24e667227ba3a2b3	-1
20261011130001	merge_and_seed_default_credentials	2026-11-04 16:48:45.221216+00	t	\\xc5fc006e4c216dd10154418fd2d52cf3e58b70039498b21a2fc74a1b3e8d9ce34af25da2b6bc197d0c7dff4f9ca910eb	-1
20261012120000	collection_sources	2026-11-04 16:48:45.225598+00	t	\\xfa0a6238fae38ae0c45b9657d540271cf8201a4981d6a16d84c856e8ba192e312b22472db6e2f734e1cd0a56eae723de	-1
20261029120000	unconfirmed_entities	2026-11-04 16:48:45.293503+00	t	\\xb2882cb53e9225fec0318c7ed531021d6aef20dda71386d972c97095771a885b6ff0edea3a785c0b60716fa88af31a28	-1
20261030120000	site_assignments	2026-11-04 16:48:45.31562+00	t	\\xdda847f8830d4d1b6e54d872e8ab95392fe2dcb0628e5ff26d8a6fd90d13c93af9682c8afe8011b1b34d304db68afdae	-1
20261031120000	hypervisor_guest_warnings_and_ssh_logins	2026-11-04 16:48:45.32399+00	t	\\xc2f3e4ab34a0fe5834edc0e0a6df3e67d5e59437be44e35e10b3df45894993ac8c43422b16b059df0561032e41b102eb	-1
20260914120000	hosts_hostname_source	2026-11-04 16:48:45.09875+00	t	\\xa57a5dc26c000ec5b0b4aaf66486018f637b1f1228ae994dafa06a653b5afae3d8dd2bb1264f5695c50053d656bccdc2	-1
20260915120000	organization_license_columns	2026-11-04 16:48:45.100946+00	t	\\xd46a3e083f8179128aae52f68541627b57131d66c93d84c2ad0002a7bf6c025f757345490b47d810511d49cd9528525d	-1
20260915120001	organizations_license_entitlement	2026-11-04 16:48:45.102483+00	t	\\x1da434d08d6afb9f0bbe4165f112cae53d3046c8578d4feb88d5f3ea0fa0649cd4bf2a69ac6c2c24978a561a6333fcb9	-1
20260916120000	organization_license_key_issued_at	2026-11-04 16:48:45.103966+00	t	\\x99d66ef126e64532e4e87c1a99b0ac630a2031b1b4c9ba41cbe257b8e3a83e47ec0fe1360a409b5cbb57401326d7a9d3	-1
20260917120000	organization_license_key_type	2026-11-04 16:48:45.105466+00	t	\\xac327476c465df672532ce71bfbe03a0d92ff5cc3053fae9a15fa24100308cda6b08e118267c3c1009118eeb90f46038	-1
20260917130000	organization_bills_by_invoice	2026-11-04 16:48:45.107226+00	t	\\x33bf32d3b3edce4fe6803c326785210f322b9cdfd065440098b53424df7042ba90da1ac6f99ade774f894da1b67b0979	-1
20260930120000	clear_zero_mac_addresses	2026-11-04 16:48:45.111515+00	t	\\xef8416f55b61953f00f1fd249a04c82d63d0d8d510e9db6cae96f79db7a70a61dfc5a22029a6878e74da05e60b63b7dd	-1
20261001120000	add_user_display_settings	2026-11-04 16:48:45.113289+00	t	\\xd7f9afb0762c40085ced713496cf39e9778c4257196e1ba02a0feec1af67eab309df78ce5b753ebde6d250f6fbf3ef32	-1
20261001120001	credential_daemon_os	2026-11-04 16:48:45.115525+00	t	\\xe679b2c2caea51e608200d064b81b02f1f4cee20cfa60a929db29b87666ba8a5d01e5d0739a2b9953f401f4171c11c9d	-1
20261002120000	daemon_os	2026-11-04 16:48:45.117072+00	t	\\x931ef66a80cd751db47512b5f9397a5d7df21aff77cbf28c5a190c69b27ce6040d2e2f2ed3c08e85faf13d28e5cba25f	-1
20261003120000	hosts_os	2026-11-04 16:48:45.119066+00	t	\\xb8a3b3a3e6ec9e21cc64468fa5b691dba7d39556aab7cd28fee01a21fcbde0782952b4b73cbdbd334a17c554769fbdce	-1
20261003130000	organization_license_server_version	2026-11-04 16:48:45.120552+00	t	\\xe2f1bf9692ed8bb1a08a1ae13b53f632fd5f2112d53225bbf13332af26639b9518edb28daa4b3ed98bf71ed4b1002d34	-1
20261004120000	host_virtualization_interface	2026-11-04 16:48:45.123377+00	t	\\x79224f29c669331336f577a5c9f7e5f674dec637f1a879e9c4016e8705af2e921e63cf4ab178cb21af1026317d6e57be	-1
20261004120001	host_virtualization_interface_validate	2026-11-04 16:48:45.125592+00	t	\\xf03d5b6826e7aca5a5efe1ab57878591bfd12ac5c097b1d71e5539348cf9a5e794c2ef19c5fa3ee8aac039920b9f1f4e	-1
20261004120002	host_virtualization_interface_index	2026-11-04 16:48:45.128835+00	t	\\x64e39691a98e9ed761812e1bfdeaa7b8d9aa13eb0401a39fb40dd36f21222ac47dad7f1b4ff9465a3cd0da1af42039ee	-1
20261005120000	rename_networks_to_sites	2026-11-04 16:48:45.143032+00	t	\\x5e08911ddc549e9d522eece781caa57fe42ee4cb6f7d16fb25b63bf6c7f95e897d2cbd12db345040f5a4035d420ace43	-1
20261012130000	discovery_warnings	2026-11-04 16:48:45.243679+00	t	\\xc760d410eaa7d4d14214bf602cd9ba7423c1a328b71768d79bcef0cee51b9b685c37438f74091fb9c3afd6483a98e46c	-1
20261013120000	license_instances	2026-11-04 16:48:45.250657+00	t	\\x86796dc3b14fc5805b84f6c6fd92fa8aaa560a004ea4ca8f4affa76f55b830f8d2403393cddfcf215070e84d499bb5cb	-1
20261013140000	terms_notice_version	2026-11-04 16:48:45.252296+00	t	\\x624bbb290511d7491b8816b152b06194417047e2b9b19eeb8b1d954ec17e6fee77c73d9fd19fb74d62f8c44d66bb1a93	-1
20261019120000	dependency_members_keep_on_delete	2026-11-04 16:48:45.255779+00	t	\\x2391912be8b3a07910ba2240ffe21065efe5836453ee2ba9ee7ca5fda5dea58d701795486c7e03a70909c88e5cb5d595	-1
20261021120000	vlan_and_subnet_sources	2026-11-04 16:48:45.258522+00	t	\\x2535fce0c0903904a1d9bfa86132278e9238231438d3b15be9863c47a362c284cc26d4b4ecb7830ee128a004d3d99878	-1
20261022120000	credential_needs_newer_daemon	2026-11-04 16:48:45.266677+00	t	\\xfcc14b3c03c4c930f7ffa6a0086ff9fe12d35c77ccc331b4469ebaf19ddcff05a1a21801b4f2c36fd7d14df5c403a1a1	-1
20261023120000	snmp_community_credential	2026-11-04 16:48:45.276921+00	t	\\x71e050db43cceee2a66e206cbd87f478cc8a6932440def65d5ea5c79f55ae7339bff4a9e7727b4bd7a9696dc535816cf	-1
20261024120000	topology_device_type_rule	2026-11-04 16:48:45.278754+00	t	\\xb5f60015b76437698dbd4381b285195bc4b866a3d20f1cfb588bd675b92aab51dedce82bcbc29749c5e04ed4e6050d87	-1
20261024120001	google_nest_capitalization	2026-11-04 16:48:45.28081+00	t	\\x204cc0286eea5cba45dded8756cd98207c511b505eef07ce0470fd35104d69092038adc091536ffb2dda65e288d73262	-1
20261025120000	every_org_has_a_plan	2026-11-04 16:48:45.284507+00	t	\\x403a5bebde12227dbbfa913490de00196184c947a72da3d86f39809db010406715561f96eac6839546713c7c9aaf90c4	-1
20261026120000	strip_discovery_run_type_warnings	2026-11-04 16:48:45.287951+00	t	\\xffd0edff777c52348517fd47b5b87eb9406eea878e1168f23c2660786bf1352031ebb04761ed43dc3e6f6886dc5d4f52	-1
20261027120000	hosts_matched_device_type	2026-11-04 16:48:45.290235+00	t	\\xe970e578c2fad70d71c4e1d4a882d5e477e6372d090d93dad467ab8f656738deff5354b05868b59d8c808a2bfb61d8bd	-1
\.


--
-- Data for Name: api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.api_keys (id, key, site_id, name, created_at, updated_at, last_used, expires_at, is_enabled, plaintext, daemon_id) FROM stdin;
fa47f367-e05f-4c5a-9c52-425586002714	9ed0fb4398ebcbff1022329ff5d32569fcb166f633d1465c1bbd468a7db4204f	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	Compat Test API Key	2026-11-04 16:56:16.229997+00	2026-11-04 16:56:16.229997+00	2026-11-04 16:56:46.587183+00	\N	t	\N	\N
9f7eae96-4300-4455-8843-dba4d88c0040	b955a0f1474afd1fbb6dc29ecac04aaae314a18867eafc73a663534fb61167f9	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	scanopy-daemon API Key	2026-11-04 16:48:48.637803+00	2026-11-04 16:48:48.637803+00	2026-11-04 16:57:02.231338+00	\N	t	\N	51c5b65c-3dc1-4c23-a049-42eac5bf85de
b71ab7eb-31be-438c-95c3-e683c48dd389	ac190b420746ff9b1ca0ef11ddb3d2dea8086ac5d028d4ec723900f81c03fe37	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	scanopy-daemon-serverpoll API Key	2026-11-04 16:50:05.373302+00	2026-11-04 16:50:05.373302+00	2026-11-04 16:57:05.113817+00	\N	t	scp_d_chuR3n1BlY4OAchbvYsULa9FFOxrCgwb	1429b925-2df0-45db-8c88-73bf1c2829a0
\.


--
-- Data for Name: bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.bindings (id, site_id, service_id, binding_type, ip_address_id, port_id, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
2a05b29f-5921-4c86-933e-671083e9e67f	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	9ff69b61-4ac2-4522-82fb-85cd0b229403	Port	fc3c5432-a2cb-4049-85e6-64b4d71473b3	c58a15f4-b1c5-4d91-83d6-72d9069b6eb7	2026-10-01 04:02:39.555404+00	2026-10-01 04:02:39.555404+00	2026-10-01 04:02:39.555404+00	\N	\N	2026-10-01 04:02:39.555404+00	\N	\N	\N
fdc20cfb-4964-413c-a11a-c45790ed9c64	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	9ff69b61-4ac2-4522-82fb-85cd0b229403	Port	ad5a60f7-f8f2-4b78-99f7-a43668ba7558	c58a15f4-b1c5-4d91-83d6-72d9069b6eb7	2026-10-01 04:02:39.555406+00	2026-10-01 04:02:39.555406+00	2026-10-01 04:02:39.555406+00	\N	\N	2026-10-01 04:02:39.555406+00	\N	\N	\N
\.


--
-- Data for Name: credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.credentials (id, organization_id, name, credential_type, created_at, updated_at, daemon_os, description, login_credential_id) FROM stdin;
5619c59f-06d2-44b0-8195-474b377f1da5	b708babc-d623-484f-8d9f-67f46c1b72d1	SNMP (public)	{"type": "SnmpCommunity", "community": {"mode": "Inline", "value": "public"}}	2026-11-04 16:48:48.473015+00	2026-11-04 16:48:48.473015+00	\N	Assigned to every new site. Tries the SNMP community "public" on every host, over v2c and then v1.	\N
d5ab7472-9a37-45b9-a691-f455da085a3f	b708babc-d623-484f-8d9f-67f46c1b72d1	Docker socket	{"type": "DockerSocket"}	2026-11-04 16:48:48.475471+00	2026-11-04 16:48:48.475471+00	\N	Added to every new daemon. Reads the local Docker socket at its default path.	\N
d7d66ae3-da70-48d5-b19a-28bfe3cad59a	b708babc-d623-484f-8d9f-67f46c1b72d1	Podman socket	{"type": "PodmanSocket"}	2026-11-04 16:48:48.479411+00	2026-11-04 16:48:48.479411+00	\N	Added to every new daemon. Reads the local Podman socket at its default path.	\N
\.


--
-- Data for Name: daemon_interfaced_subnets; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemon_interfaced_subnets (daemon_id, subnet_id) FROM stdin;
1429b925-2df0-45db-8c88-73bf1c2829a0	d0b8a284-36a1-4e60-a131-20992f033ee5
1429b925-2df0-45db-8c88-73bf1c2829a0	bf720fd4-edfb-4341-b9d4-97b0f30185d3
51c5b65c-3dc1-4c23-a049-42eac5bf85de	d0b8a284-36a1-4e60-a131-20992f033ee5
51c5b65c-3dc1-4c23-a049-42eac5bf85de	bf720fd4-edfb-4341-b9d4-97b0f30185d3
\.


--
-- Data for Name: daemons; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemons (id, site_id, host_id, created_at, last_seen, updated_at, mode, url, name, version, user_id, api_key_id, is_unreachable, standby, standby_cleared_at, os) FROM stdin;
51c5b65c-3dc1-4c23-a049-42eac5bf85de	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	f35cae23-aec3-4e90-8c72-278e1d887c57	2026-11-04 16:48:48.655151+00	2026-11-04 16:57:02.2387+00	2026-11-04 16:48:48.655151+00	"daemon_poll"		scanopy-daemon	0.19.6	d06bee5b-4fed-42b4-b27a-1abd1baca9ff	9f7eae96-4300-4455-8843-dba4d88c0040	f	f	\N	"linux"
1429b925-2df0-45db-8c88-73bf1c2829a0	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	cd5dc797-4d92-45ee-a10e-7f17db4c6422	2026-11-04 16:50:05.383393+00	2026-11-04 16:57:05.120099+00	2026-11-04 16:50:05.383393+00	"server_poll"	http://daemon-serverpoll:60074	scanopy-daemon-serverpoll	0.19.6	d06bee5b-4fed-42b4-b27a-1abd1baca9ff	b71ab7eb-31be-438c-95c3-e683c48dd389	f	f	2026-11-04 16:57:05.120103+00	"linux"
\.


--
-- Data for Name: dependencies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.dependencies (id, site_id, name, description, created_at, updated_at, source, color, edge_style, dependency_type, member_type, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
0716481e-fe9a-464e-8365-d78cf1b554ca	c45ecd9e-e3a2-43fb-931d-25fd1afbc034		\N	2026-11-04 16:56:15.465678+00	2026-11-04 16:56:15.465678+00	{"type": "Manual"}	Yellow	"Bezier"	RequestPath	Services	2026-11-04 16:56:15.465678+00	\N	\N	\N
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
242dde5c-d8f6-4d73-9a3d-762943697337	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 100, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "b64df0ca-f173-4ba8-a48b-edf7e372974a", "started_at": "2026-01-07T19:14:20.155782339Z", "finished_at": "2026-01-07T19:14:20.224698505Z", "discovery_id": null, "daemon_version": "0.16.0", "discovery_type": {"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}, "last_update_at": "2026-11-04T16:56:17.253503Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}	Self Report — My Site	2026-01-07 19:14:20.155782+00	2026-11-04 16:56:17.256205+00	0	f	[]
3c9e7730-6d12-44a4-8b69-53c37dfd49bd	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Scheduled", "enabled": true, "last_run": "2026-11-04T16:49:09.197830Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "f35cae23-aec3-4e90-8c72-278e1d887c57", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-04 16:48:48.661568+00	2026-11-04 16:50:04.976866+00	1	f	[]
dba0d6aa-70d4-400b-a22b-1872f4020249	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 100, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "271037a5-7bfa-49d0-bff8-3879ffaa6d30", "started_at": "2026-11-04T16:49:09.224254Z", "finished_at": "2026-11-04T16:50:04.917188Z", "discovery_id": "3c9e7730-6d12-44a4-8b69-53c37dfd49bd", "daemon_version": "0.19.6", "discovery_type": {"type": "Unified", "host_id": "f35cae23-aec3-4e90-8c72-278e1d887c57", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:50:04.968639Z", "hosts_discovered": 6, "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "f35cae23-aec3-4e90-8c72-278e1d887c57", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-04 16:49:09.224254+00	2026-11-04 16:50:04.975853+00	0	f	[]
cb47129a-2e6b-4188-a94b-4535c8f6bd46	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "ec9dc330-67d0-4ce5-94d8-506859c74940", "started_at": "2026-04-18T01:11:15.092803666Z", "finished_at": "2026-11-04T16:56:17.640229Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.748108Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:15.092803+00	2026-11-04 16:56:17.642833+00	0	f	[]
af47f099-1ed8-4665-a44f-56e94cd0ecfc	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "9dc720f8-1115-45ea-8b6d-f2e58154594d", "started_at": "2026-04-18T01:07:01.115515322Z", "finished_at": "2026-11-04T16:56:17.647698Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "1ce3a67d-4aeb-485a-9976-c67c4801fd30", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.585297Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "1ce3a67d-4aeb-485a-9976-c67c4801fd30", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:07:01.115515+00	2026-11-04 16:56:17.648954+00	0	f	[]
47ead246-b597-4158-a33d-2bc7f9fd2f68	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	1429b925-2df0-45db-8c88-73bf1c2829a0	{"type": "Scheduled", "enabled": true, "last_run": "2026-11-04T16:50:15.378800Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "cd5dc797-4d92-45ee-a10e-7f17db4c6422", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-04 16:50:05.385367+00	2026-11-04 16:53:15.409439+00	1	f	[]
ab9731b3-63f7-4766-9d5b-ec9c50f99ae0	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	1429b925-2df0-45db-8c88-73bf1c2829a0	{"type": "AdHoc", "last_run": "2026-11-04T16:50:15.862315Z"}	{"type": "Unified", "host_id": "cd5dc797-4d92-45ee-a10e-7f17db4c6422", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	ServerPoll Integration Test Discovery	2026-11-04 16:50:15.850024+00	2026-11-04 16:56:15.408928+00	1	f	[]
0a67f79e-a0f0-4740-97ab-3721f9bae1ee	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	1429b925-2df0-45db-8c88-73bf1c2829a0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 100, "warnings": [], "daemon_id": "1429b925-2df0-45db-8c88-73bf1c2829a0", "session_id": "8ffaafb6-6021-4837-bdc4-cf4774b62e52", "started_at": "2026-11-04T16:50:15.395911Z", "finished_at": "2026-11-04T16:52:46.280929Z", "discovery_id": "47ead246-b597-4158-a33d-2bc7f9fd2f68", "daemon_version": "0.19.6", "discovery_type": {"type": "Unified", "host_id": "cd5dc797-4d92-45ee-a10e-7f17db4c6422", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:53:15.403675Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "cd5dc797-4d92-45ee-a10e-7f17db4c6422", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-04 16:50:15.395911+00	2026-11-04 16:53:15.408436+00	0	f	[]
218b8a01-e8f8-4255-b671-fa6785c3f075	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	1429b925-2df0-45db-8c88-73bf1c2829a0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 100, "warnings": [], "daemon_id": "1429b925-2df0-45db-8c88-73bf1c2829a0", "session_id": "a426c7fc-950a-4b60-b7fb-f2f9b3df2b5d", "started_at": "2026-11-04T16:53:15.482879Z", "finished_at": "2026-11-04T16:55:46.017024Z", "discovery_id": "ab9731b3-63f7-4766-9d5b-ec9c50f99ae0", "daemon_version": "0.19.6", "discovery_type": {"type": "Unified", "host_id": "cd5dc797-4d92-45ee-a10e-7f17db4c6422", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:15.403212Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "cd5dc797-4d92-45ee-a10e-7f17db4c6422", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	ServerPoll Integration Test Discovery	2026-11-04 16:53:15.482879+00	2026-11-04 16:56:15.408124+00	0	f	[]
908f9b52-9c51-47e7-be8d-ef8eb733decd	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "5b19fece-cb82-45d8-a676-df53cc38a014", "started_at": "2026-04-18T01:11:16.444659954Z", "finished_at": "2026-11-04T16:56:17.644027Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}, "last_update_at": "2026-11-04T16:56:16.779504Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}	Self Report — My Site	2026-04-18 01:11:16.444659+00	2026-11-04 16:56:17.646568+00	0	f	[]
814bb697-b273-407a-85a0-bb664c06a7ae	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "27ff5779-08b0-4970-aebd-04649d27c725", "started_at": "2026-04-18T01:11:11.891664364Z", "finished_at": "2026-11-04T16:56:17.650056Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.687599Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:11.891664+00	2026-11-04 16:56:17.652636+00	0	f	[]
6e5e8a09-1d83-41e7-9461-03aeadff14bc	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "10fdd8f4-03b6-44ea-adb6-27e74136b365", "started_at": "2026-04-18T01:11:10.955161679Z", "finished_at": "2026-11-04T16:56:17.661050Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.669422Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:10.955161+00	2026-11-04 16:56:17.663562+00	0	f	[]
da351b32-18dd-4d65-8695-dbb27710aec6	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "857fbace-0358-4642-8ca0-b14bb999f353", "started_at": "2026-04-18T01:11:15.366415752Z", "finished_at": "2026-11-04T16:56:17.667390Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.754405Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:15.366415+00	2026-11-04 16:56:17.669998+00	0	f	[]
e8cf7ff8-332f-43a8-932f-0cda913dced1	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "f2d5414b-3369-4280-929d-5422dd11a4b4", "started_at": "2026-04-18T01:11:14.552476135Z", "finished_at": "2026-11-04T16:56:17.653685Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}, "last_update_at": "2026-11-04T16:56:16.741976Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}	Self Report — My Site	2026-04-18 01:11:14.552476+00	2026-11-04 16:56:17.656248+00	0	f	[]
b6b50b59-addb-47d9-a790-5adc2fe54333	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "bbe36108-56c4-4d82-9087-e577842ff202", "started_at": "2026-04-18T01:11:12.435139092Z", "finished_at": "2026-11-04T16:56:17.712857Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.700295Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:12.435139+00	2026-11-04 16:56:17.715535+00	0	f	[]
7d83c1d4-bffe-4f7e-b9ad-7e2358e536e3	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "6465c546-3856-4e33-9cd0-5b8a8d99d55b", "started_at": "2026-04-18T01:11:13.585680978Z", "finished_at": "2026-11-04T16:56:17.720197Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "5ef6e4af-f8ab-48d8-9b0e-6f4b93a49ef0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.717930Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "5ef6e4af-f8ab-48d8-9b0e-6f4b93a49ef0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:11:13.58568+00	2026-11-04 16:56:17.721642+00	0	f	[]
3997f7b5-aae8-4690-8e69-ea2ee1182ca8	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "df1323db-2f1d-4090-bd79-58d04eee33c1", "started_at": "2026-04-18T01:11:16.011978400Z", "finished_at": "2026-11-04T16:56:17.726330Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "a6d5229d-0b50-4243-a00d-c3d95bb17a36", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.766878Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a6d5229d-0b50-4243-a00d-c3d95bb17a36", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:11:16.011978+00	2026-11-04 16:56:17.72773+00	0	f	[]
b48f0c08-8b3a-4c32-848b-ead5a15940ca	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "032af69a-4335-4c54-ac9c-8941c44050a3", "started_at": "2026-04-18T01:11:11.692405958Z", "finished_at": "2026-11-04T16:56:17.732388Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.681390Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:11:11.692405+00	2026-11-04 16:56:17.733733+00	0	f	[]
4c55d6a8-5798-4973-beeb-3e56e6341dda	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "0ffe5496-d64e-41a0-ab86-ff28eed28819", "started_at": "2026-04-18T01:11:14.286588432Z", "finished_at": "2026-11-04T16:56:17.657329Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.735650Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:14.286588+00	2026-11-04 16:56:17.65978+00	0	f	[]
1a10182b-e5ad-4617-9bf6-485a3de799ff	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "34fe487d-22ef-418c-9099-42fa3376ded0", "started_at": "2026-04-18T01:08:24.495832378Z", "finished_at": "2026-11-04T16:56:17.664720Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "e9d19327-eb77-47c7-85d4-e590c7aa90a1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.640650Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e9d19327-eb77-47c7-85d4-e590c7aa90a1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:08:24.495832+00	2026-11-04 16:56:17.666263+00	0	f	[]
2a5fb788-a9ed-4268-b213-75f340c8da11	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "c10d2eae-1267-4023-997e-5a51fdf97281", "started_at": "2026-04-18T01:11:13.755644675Z", "finished_at": "2026-11-04T16:56:17.716607Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.723996Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:13.755644+00	2026-11-04 16:56:17.71904+00	0	f	[]
b303773f-c951-40b2-822d-cbdd656b3f38	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "969d115c-0112-4ba3-8757-bb1ece340112", "started_at": "2026-04-18T01:11:17.083410573Z", "finished_at": "2026-11-04T16:56:17.722718Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.792010Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:17.08341+00	2026-11-04 16:56:17.725115+00	0	f	[]
a7222ce1-7e74-4a78-883c-314441bd3321	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "7f54ecab-3771-4583-a7d0-f12569030e17", "started_at": "2026-04-18T01:11:18.430754762Z", "finished_at": "2026-11-04T16:56:17.728964Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}, "last_update_at": "2026-11-04T16:56:16.824061Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}	Self Report — My Site	2026-04-18 01:11:18.430754+00	2026-11-04 16:56:17.731406+00	0	f	[]
94a057eb-1d23-4f60-aa96-0f72f23e8ee7	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "3b0868af-d558-45e3-b688-2cf55472b6ee", "started_at": "2026-04-18T01:11:12.704194079Z", "finished_at": "2026-11-04T16:56:17.734821Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.706314Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:12.704194+00	2026-11-04 16:56:17.737261+00	0	f	[]
dd9a9ad2-3008-407b-aa04-ebc20b476289	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "5e741620-3de9-476c-86e7-7ce656d0a5a8", "started_at": "2026-04-18T01:11:17.892141982Z", "finished_at": "2026-11-04T16:56:17.671061Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.811270Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:17.892141+00	2026-11-04 16:56:17.673748+00	0	f	[]
d937e959-a222-4af9-bf14-ddb92f31ab57	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "abba33fb-bf1f-4e8b-985f-6a2d0b5d0380", "started_at": "2026-04-18T01:11:10.686796959Z", "finished_at": "2026-11-04T16:56:17.677356Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.663095Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:10.686796+00	2026-11-04 16:56:17.679714+00	0	f	[]
9713e1c2-5136-48cb-94f9-3c2a789b2efa	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "7c9364ea-b95f-4e96-b958-a71e1e3826df", "started_at": "2026-08-26T01:54:31.081144679Z", "finished_at": "2026-11-04T16:56:18.234508Z", "discovery_id": null, "daemon_version": "0.15.4", "discovery_type": {"type": "Unified", "host_id": "6a1dd7f3-9b36-49b1-b554-1ef783b14e5d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:17.868558Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6a1dd7f3-9b36-49b1-b554-1ef783b14e5d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-26 01:54:31.081144+00	2026-11-04 16:56:18.235917+00	0	f	[]
fbaf5e32-c46b-4028-9eff-c6c79c5a50c4	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "bd93b206-4e22-4e0a-9dd0-0f38664a4247", "started_at": "2026-02-02T14:21:21.225991527Z", "finished_at": "2026-11-04T16:56:24.814310Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:23.348234Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-02 14:21:21.225991+00	2026-11-04 16:56:24.816611+00	0	f	[]
d40c19dc-0538-4ffd-8688-08f4fb663f89	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "e5274c5c-4478-40d5-86c6-aabbe025f093", "started_at": "2026-02-02T19:01:57.807520644Z", "finished_at": "2026-11-04T16:56:24.820096Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "SelfReport", "host_id": "37e5888d-4a4e-4ca3-908c-a8939cc75bdd"}, "last_update_at": "2026-11-04T16:56:24.266555Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "37e5888d-4a4e-4ca3-908c-a8939cc75bdd"}	Self Report — My Site	2026-02-02 19:01:57.80752+00	2026-11-04 16:56:24.822463+00	0	f	[]
10ecdfa7-3b4c-4b0d-b07a-944301026afa	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "ee9f5e06-197c-4fbf-9d68-0ac7d7ec929e", "started_at": "2026-06-28T16:07:05.924226987Z", "finished_at": "2026-11-04T16:56:24.826731Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Unified", "host_id": "8d22037e-273c-4a29-9c5c-cdaaefe74227", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:24.306028Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8d22037e-273c-4a29-9c5c-cdaaefe74227", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:07:05.924226+00	2026-11-04 16:56:24.828072+00	0	f	[]
91596f1d-ff3f-48a8-891a-f29a690d9dc5	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "2abdbee0-ac03-472c-b137-4321f33f03ca", "started_at": "2026-04-18T01:11:17.727006742Z", "finished_at": "2026-11-04T16:56:17.675004Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.804821Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:11:17.727006+00	2026-11-04 16:56:17.676277+00	0	f	[]
2ce00b07-4f78-4d64-8d5b-980720559126	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "5bd5e93e-a606-4c6e-b159-b04879bdd801", "started_at": "2026-04-18T01:11:12.972982217Z", "finished_at": "2026-11-04T16:56:17.738318Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.712107Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:12.972982+00	2026-11-04 16:56:17.740987+00	0	f	[]
41b3c404-b7c2-464f-a93e-068a4ea8e206	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "4e1e43c0-b63f-41df-ba3a-71683fef4763", "started_at": "2026-02-04T01:54:22.209258404Z", "finished_at": "2026-11-04T16:56:19.906461Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:18.930374Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-04 01:54:22.209258+00	2026-11-04 16:56:19.90903+00	0	f	[]
e9c2d770-ce55-47e9-9834-84b368976b9f	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "b18c8b1b-202b-4081-9911-d4f395fe9344", "started_at": "2026-10-27T05:03:46.928849924Z", "finished_at": "2026-11-04T16:56:26.538083Z", "discovery_id": null, "daemon_version": "0.19.5", "discovery_type": {"type": "Unified", "host_id": "8913babd-75d5-4960-a597-9eb267cea952", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:26.103986Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8913babd-75d5-4960-a597-9eb267cea952", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-27 05:03:46.928849+00	2026-11-04 16:56:26.539523+00	0	f	[]
6c2b3f3d-8450-45f4-9534-6865b012c8a9	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "cd16f43c-4365-49c5-8fee-d081f3b53cd9", "started_at": "2026-02-23T00:07:19.655637880Z", "finished_at": "2026-11-04T16:56:30.036100Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "SelfReport", "host_id": "abd72866-b3c5-4e9a-89f6-491d6af2797b"}, "last_update_at": "2026-11-04T16:56:28.523123Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "abd72866-b3c5-4e9a-89f6-491d6af2797b"}	Self Report — My Site	2026-02-23 00:07:19.655637+00	2026-11-04 16:56:30.038313+00	0	f	[]
d28613fa-cdbc-4def-bfc1-59f73a2b7516	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "59fba01e-fefd-4c66-9ef2-a85c0e76a811", "started_at": "2026-04-18T01:11:12.160794534Z", "finished_at": "2026-11-04T16:56:17.680862Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.694227Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:12.160794+00	2026-11-04 16:56:17.683235+00	0	f	[]
726c3741-527d-4f8e-b007-b2a93c56ac03	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "22ea697f-065c-4d2e-a81f-0809764aad01", "started_at": "2026-04-18T01:11:15.640169759Z", "finished_at": "2026-11-04T16:56:17.684366Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.760504Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:15.640169+00	2026-11-04 16:56:17.68698+00	0	f	[]
633c6a9b-44da-4682-8c92-279e83f2eb8d	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "d2b26f9b-0329-4191-8a60-58cf3c796dba", "started_at": "2026-04-18T01:11:11.423409541Z", "finished_at": "2026-11-04T16:56:17.697416Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.675378Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:11:11.423409+00	2026-11-04 16:56:17.698817+00	0	f	[]
f5eb3c9b-0ed7-417b-8f16-3ff8ec865502	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "71cbed11-9ec5-4ae6-a3ce-3767032c2ab8", "started_at": "2026-04-17T22:09:29.738977360Z", "finished_at": "2026-11-04T16:56:17.699862Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "e4ddba2b-4c4f-4f89-a625-8f3f022d7581", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.647008Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e4ddba2b-4c4f-4f89-a625-8f3f022d7581", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-17 22:09:29.738977+00	2026-11-04 16:56:17.701117+00	0	f	[]
c2c41f61-7ba6-4503-9235-e850699bc01a	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "6845dcc8-6ea0-47af-9ce3-054b8cf1f667", "started_at": "2026-04-18T01:11:14.018637270Z", "finished_at": "2026-11-04T16:56:17.702377Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}, "last_update_at": "2026-11-04T16:56:16.729758Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}	Self Report — My Site	2026-04-18 01:11:14.018637+00	2026-11-04 16:56:17.704967+00	0	f	[]
ced26395-2e9a-4048-95ad-1178355d6473	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "e46bbbd5-df29-425e-b933-bd8b0a332217", "started_at": "2026-03-15T19:34:41.999288315Z", "finished_at": "2026-11-04T16:56:19.910299Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "SelfReport", "host_id": "67ddad71-cd34-47fe-ad5f-138fe7069c76"}, "last_update_at": "2026-11-04T16:56:19.338957Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "67ddad71-cd34-47fe-ad5f-138fe7069c76"}	Self Report — My Site	2026-03-15 19:34:41.999288+00	2026-11-04 16:56:19.912803+00	0	f	[]
4d1a4a80-11d8-4bea-b555-a1ddd135b9da	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "4db08e28-b456-4951-bd43-585d26cf9421", "started_at": "2026-01-07T19:14:20.246263005Z", "finished_at": "2026-11-04T16:56:17.688138Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:17.280116Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-07 19:14:20.246263+00	2026-11-04 16:56:17.690377+00	0	f	[]
f006ba4a-dc8c-460f-aec7-c17e4ded6695	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "d2c685dd-56ad-4d49-9d36-c64b0483a15a", "started_at": "2026-04-18T01:11:18.269780388Z", "finished_at": "2026-11-04T16:56:17.695039Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "5a5b21eb-2566-4e13-83d8-00cb2675bde6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.817522Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "5a5b21eb-2566-4e13-83d8-00cb2675bde6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:11:18.26978+00	2026-11-04 16:56:17.696261+00	0	f	[]
ff13920c-78aa-41ce-8c74-22ea29be02f8	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "34c804c2-c01c-4104-a3b3-e31ae6dd0b6f", "started_at": "2026-04-18T01:11:17.355650194Z", "finished_at": "2026-11-04T16:56:17.705974Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.798368Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:17.35565+00	2026-11-04 16:56:17.708297+00	0	f	[]
6283fcdd-a690-440b-a4ee-6ed5508a7782	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "890873b9-2054-463a-9fe8-11016f0d08f2", "started_at": "2026-07-29T04:17:17.562092815Z", "finished_at": "2026-11-04T16:56:21.935209Z", "discovery_id": null, "daemon_version": "0.19.0", "discovery_type": {"type": "Unified", "host_id": "f4930d3b-6bc0-42c1-b25c-bbf557bf56ce", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:21.518983Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f4930d3b-6bc0-42c1-b25c-bbf557bf56ce", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-29 04:17:17.562092+00	2026-11-04 16:56:21.936577+00	0	f	[]
5b6cccab-21cd-4dcc-a389-503faa0275f2	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "b5073709-1dc4-4f69-bab1-d17406b1b571", "started_at": "2026-07-29T04:22:21.659954694Z", "finished_at": "2026-11-04T16:56:21.941113Z", "discovery_id": null, "daemon_version": "0.19.0", "discovery_type": {"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:21.579015Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-29 04:22:21.659954+00	2026-11-04 16:56:21.942582+00	0	f	[]
9f94ae80-7b83-4c0b-b6fa-e6ccebe81329	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "c8c6530c-aaff-4d7c-a872-ba4457906d77", "started_at": "2026-04-18T01:11:19.267025681Z", "finished_at": "2026-11-04T16:56:17.691542Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}, "last_update_at": "2026-11-04T16:56:16.830409Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}	Self Report — My Site	2026-04-18 01:11:19.267025+00	2026-11-04 16:56:17.693934+00	0	f	[]
79b95f14-6e87-4ac4-8301-ab77689cecb2	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "4273fe18-d031-4977-950e-ad5a49d2ea8e", "started_at": "2026-04-18T01:11:16.182328068Z", "finished_at": "2026-11-04T16:56:17.709374Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:16.773223Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-04-18 01:11:16.182328+00	2026-11-04 16:56:17.711795+00	0	f	[]
cddc1e0f-395f-464d-ad0a-25e022da19e9	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "18c3c94f-80cb-48e9-aedc-76dbc938706e", "started_at": "2026-03-31T18:35:21.063363405Z", "finished_at": "2026-11-04T16:56:18.850787Z", "discovery_id": null, "daemon_version": "0.14.3", "discovery_type": {"type": "Unified", "host_id": "f5245920-25e1-4d8b-b567-0f935961690d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:18.377802Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f5245920-25e1-4d8b-b567-0f935961690d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:35:21.063363+00	2026-11-04 16:56:18.852236+00	0	f	[]
67770e4e-13b1-4aab-ad1a-fff6005e5a2c	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "f5a833cc-002f-41e1-b540-c0a623d6775c", "started_at": "2026-01-25T23:12:10.196614920Z", "finished_at": "2026-11-04T16:56:21.480938Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644"}, "last_update_at": "2026-11-04T16:56:20.617985Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644"}	Self Report — My Site	2026-01-25 23:12:10.196614+00	2026-11-04 16:56:21.483624+00	0	f	[]
44d17eef-8047-4493-afaf-ee8816f09fc6	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "466f3f06-3335-4195-94cd-7b1b85eceda4", "started_at": "2026-02-01T15:39:35.137282092Z", "finished_at": "2026-11-04T16:56:21.487194Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "SelfReport", "host_id": "d6e89254-a0d6-418b-9678-e75d11712fd1"}, "last_update_at": "2026-11-04T16:56:20.550216Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "d6e89254-a0d6-418b-9678-e75d11712fd1"}	Self Report — My Site	2026-02-01 15:39:35.137282+00	2026-11-04 16:56:21.489532+00	0	f	[]
1d12715e-6d66-490a-8399-499c59dbae49	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "26c0ac92-5d7b-41b7-969c-9e07d17bd1be", "started_at": "2026-10-15T03:38:40.733230767Z", "finished_at": "2026-11-04T16:56:21.493835Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:21.120744Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:38:40.73323+00	2026-11-04 16:56:21.495224+00	0	f	[]
45f1608b-df6d-4f13-b985-728426be2ee9	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "557486b8-0125-4f97-a586-fe9ae5f972d8", "started_at": "2026-09-03T18:01:02.050183807Z", "finished_at": "2026-11-04T16:56:18.231763Z", "discovery_id": null, "daemon_version": "0.15.4", "discovery_type": {"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:17.861864Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-03 18:01:02.050183+00	2026-11-04 16:56:18.233271+00	0	f	[]
73c75451-7abc-4203-8215-4267a423e12e	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "2f915185-76ea-4a3a-8f81-7ca0298ae158", "started_at": "2026-03-31T18:35:19.970481609Z", "finished_at": "2026-11-04T16:56:18.841693Z", "discovery_id": null, "daemon_version": "0.14.3", "discovery_type": {"type": "Unified", "host_id": "2a69c4df-31b5-4490-adc9-01e591b9f974", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:18.352140Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2a69c4df-31b5-4490-adc9-01e591b9f974", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:35:19.970481+00	2026-11-04 16:56:18.843194+00	0	f	[]
088e8206-0bcc-485c-a3f5-407826134916	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "1164516e-9bd1-4a5c-8fe8-182c9c6923d8", "started_at": "2026-03-31T18:30:47.557400890Z", "finished_at": "2026-11-04T16:56:18.848305Z", "discovery_id": null, "daemon_version": "0.14.3", "discovery_type": {"type": "Unified", "host_id": "52e0ccf9-3b92-408c-9a34-1bd2f053d047", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:18.261640Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "52e0ccf9-3b92-408c-9a34-1bd2f053d047", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:30:47.5574+00	2026-11-04 16:56:18.84968+00	0	f	[]
60c7e585-ad42-45f0-8f8e-0b56171df6da	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "9f3bb823-ded6-4cb7-957b-27c4af0ebff2", "started_at": "2026-03-15T19:35:11.987886037Z", "finished_at": "2026-11-04T16:56:19.917723Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:19.398519Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-15 19:35:11.987886+00	2026-11-04 16:56:19.920214+00	0	f	[]
de21550f-5264-4df9-ae14-e369f635d1e1	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "cd493a46-8909-4378-b924-ae1650a0f5af", "started_at": "2026-03-26T01:31:45.021343211Z", "finished_at": "2026-11-04T16:56:20.528056Z", "discovery_id": null, "daemon_version": "0.13.6", "discovery_type": {"type": "Unified", "host_id": "711a5546-8640-4ee2-bf0b-8b268c4e2044", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:19.940877Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "711a5546-8640-4ee2-bf0b-8b268c4e2044", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-26 01:31:45.021343+00	2026-11-04 16:56:20.529568+00	0	f	[]
039c54fa-3725-4f23-91c2-31421a6d7bbc	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "21fc2969-cba5-46cf-b381-1978e0c2959e", "started_at": "2026-09-03T17:55:43.577416616Z", "finished_at": "2026-11-04T16:56:18.237195Z", "discovery_id": null, "daemon_version": "0.15.4", "discovery_type": {"type": "Unified", "host_id": "3fd2970b-7217-4821-8bd7-6985174b6a22", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:17.761861Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "3fd2970b-7217-4821-8bd7-6985174b6a22", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-03 17:55:43.577416+00	2026-11-04 16:56:18.238544+00	0	f	[]
65dcad0a-67af-49d1-aaa1-cb4f88df79b2	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "adfad9b0-e25d-4bd4-88bc-e0ecc0f3f4a4", "started_at": "2026-01-26T13:51:30.181086877Z", "finished_at": "2026-11-04T16:56:18.844541Z", "discovery_id": null, "daemon_version": "0.14.3", "discovery_type": {"type": "SelfReport", "host_id": "01d97cc8-84d8-4806-877f-52bfd29791f8"}, "last_update_at": "2026-11-04T16:56:18.321337Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "01d97cc8-84d8-4806-877f-52bfd29791f8"}	Self Report — My Site	2026-01-26 13:51:30.181086+00	2026-11-04 16:56:18.847221+00	0	f	[]
d8cdfe53-73a4-4626-8e11-c11650f35a40	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "927175d4-7258-4b40-a656-8412141305c5", "started_at": "2026-02-04T01:53:52.212090885Z", "finished_at": "2026-11-04T16:56:19.902641Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "SelfReport", "host_id": "e0313f98-eeac-4055-beb3-7012a7bd7510"}, "last_update_at": "2026-11-04T16:56:18.872575Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "e0313f98-eeac-4055-beb3-7012a7bd7510"}	Self Report — My Site	2026-02-04 01:53:52.21209+00	2026-11-04 16:56:19.905321+00	0	f	[]
fe84d07b-e47d-49d0-b44a-b8768401ffe1	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "ef1a4d64-afd3-4762-b6ae-2ff80e8dbc2f", "started_at": "2026-03-26T01:38:51.825984709Z", "finished_at": "2026-11-04T16:56:20.525521Z", "discovery_id": null, "daemon_version": "0.13.6", "discovery_type": {"type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:20.070750Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-26 01:38:51.825984+00	2026-11-04 16:56:20.526895+00	0	f	[]
60a1a95b-0970-4803-b096-a14cfa3d3fe9	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "ae2ef896-c9ee-4c7d-92d0-2e940d717f51", "started_at": "2026-10-15T03:35:10.733740412Z", "finished_at": "2026-11-04T16:56:21.478376Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:21.114082Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:35:10.73374+00	2026-11-04 16:56:21.479772+00	0	f	[]
c63a683e-dfda-4dbe-85e5-791b8d7035d6	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "e7c3d9e2-4cad-4942-98d6-ae6053485366", "started_at": "2026-03-11T06:19:24.616705132Z", "finished_at": "2026-11-04T16:56:21.937698Z", "discovery_id": null, "daemon_version": "0.19.0", "discovery_type": {"type": "SelfReport", "host_id": "8c99dc12-cae3-41f2-a550-be436a112a35"}, "last_update_at": "2026-11-04T16:56:21.585083Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8c99dc12-cae3-41f2-a550-be436a112a35"}	Self Report — My Site	2026-03-11 06:19:24.616705+00	2026-11-04 16:56:21.940123+00	0	f	[]
b02c56a2-b512-43a2-b1a2-75b54ca7ef4a	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "030a56ec-f416-4d7f-864e-fde8841a379c", "started_at": "2026-09-03T17:57:32.046576969Z", "finished_at": "2026-11-04T16:56:18.239592Z", "discovery_id": null, "daemon_version": "0.15.4", "discovery_type": {"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:17.854996Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-03 17:57:32.046576+00	2026-11-04 16:56:18.240994+00	0	f	[]
b83c90f8-bf8f-4bc6-a7da-7d1ec8add5b3	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "1a182cd9-9710-482e-8645-d95c40db018a", "started_at": "2026-01-26T14:03:24.338877430Z", "finished_at": "2026-11-04T16:56:19.914098Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "SelfReport", "host_id": "7891ed81-377c-4eca-b05e-bc8a17129f90"}, "last_update_at": "2026-11-04T16:56:19.411826Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "7891ed81-377c-4eca-b05e-bc8a17129f90"}	Self Report — My Site	2026-01-26 14:03:24.338877+00	2026-11-04 16:56:19.916651+00	0	f	[]
ebafd331-757e-409a-b4af-284a1cfa142d	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "afde3201-cb3d-4b85-bb8f-470b83cba2a3", "started_at": "2026-02-01T15:48:58.546419422Z", "finished_at": "2026-11-04T16:56:21.490410Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:20.669664Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-01 15:48:58.546419+00	2026-11-04 16:56:21.49284+00	0	f	[]
42ba9817-67f6-433f-b2d1-e157653d078c	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "6b771cb0-0931-4c07-a677-efe6aa5bc5bd", "started_at": "2026-02-01T15:40:05.145636842Z", "finished_at": "2026-11-04T16:56:21.496203Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:20.611550Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-01 15:40:05.145636+00	2026-11-04 16:56:21.498772+00	0	f	[]
17182d18-e8a0-48b5-8316-49ce7b496db9	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "6740c1be-987b-48e8-b547-871b2ea811c7", "started_at": "2026-10-23T00:17:23.541441434Z", "finished_at": "2026-11-04T16:56:22.378634Z", "discovery_id": null, "daemon_version": "0.14.2", "discovery_type": {"type": "Unified", "host_id": "9134f9b8-a2ca-4af9-9f9d-f98529385af4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:21.957130Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9134f9b8-a2ca-4af9-9f9d-f98529385af4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-23 00:17:23.541441+00	2026-11-04 16:56:22.379987+00	0	f	[]
d7895e43-01b2-4e68-a4dc-4304c1140cd2	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "d03e3468-38c8-46e6-bd30-43693e9db1d7", "started_at": "2026-03-09T05:16:11.595722227Z", "finished_at": "2026-11-04T16:56:34.008895Z", "discovery_id": null, "daemon_version": "0.19.6", "discovery_type": {"type": "SelfReport", "host_id": "3ef5ad9b-872d-4fc7-8f8f-e5c0186d1841"}, "last_update_at": "2026-11-04T16:56:32.992700Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "3ef5ad9b-872d-4fc7-8f8f-e5c0186d1841"}	Self Report — My Site	2026-03-09 05:16:11.595722+00	2026-11-04 16:56:34.011579+00	0	f	[]
a79b68c3-bf82-41af-a515-2c1d26ad80e0	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "be760657-cb8b-4175-8484-00ea5633e0a3", "started_at": "2026-10-15T03:33:48.001242310Z", "finished_at": "2026-11-04T16:56:21.484687Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "Unified", "host_id": "6943745c-a092-47e4-9991-74ba816a3f6a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:21.022754Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6943745c-a092-47e4-9991-74ba816a3f6a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:33:48.001242+00	2026-11-04 16:56:21.486138+00	0	f	[]
52769b77-3ffc-480b-af43-343ba437e898	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "f36e9f5f-c1e1-4f52-b2b1-2de3128ff92c", "started_at": "2026-08-14T17:13:59.174760436Z", "finished_at": "2026-11-04T16:56:23.255468Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:22.892017Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-14 17:13:59.17476+00	2026-11-04 16:56:23.256827+00	0	f	[]
cfa1210b-9ce6-40b0-a7a6-bacaab6a97ab	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "52a4d1e0-c3b9-4b20-a4e8-0196198a1c15", "started_at": "2026-02-02T14:20:51.227652182Z", "finished_at": "2026-11-04T16:56:24.810593Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "SelfReport", "host_id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182"}, "last_update_at": "2026-11-04T16:56:23.286663Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182"}	Self Report — My Site	2026-02-02 14:20:51.227652+00	2026-11-04 16:56:24.813252+00	0	f	[]
ca86c7a1-25cd-4cb7-a17f-0c775047a74f	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "937bc019-8b5b-4d82-b8f2-134c36a700a8", "started_at": "2026-06-28T16:07:16.820412532Z", "finished_at": "2026-11-04T16:56:24.817708Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Unified", "host_id": "bb6764eb-c535-449f-90e5-a950f9304f32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:24.466510Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bb6764eb-c535-449f-90e5-a950f9304f32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:07:16.820412+00	2026-11-04 16:56:24.819116+00	0	f	[]
f8dc4fce-85c7-4e47-831a-f365a326c773	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "73503118-cf7f-4008-881e-7f39b457bf47", "started_at": "2026-06-28T16:07:12.405146675Z", "finished_at": "2026-11-04T16:56:24.829264Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Unified", "host_id": "e61c5740-38d0-46d2-a2d6-64ce68e6606f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:24.441573Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e61c5740-38d0-46d2-a2d6-64ce68e6606f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:07:12.405146+00	2026-11-04 16:56:24.830628+00	0	f	[]
0ff445d8-feb5-47e0-a929-bf2c55835bdf	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "30c1790e-6f3b-44f7-87be-9491e456edd0", "started_at": "2026-07-29T04:18:51.683205577Z", "finished_at": "2026-11-04T16:56:21.943635Z", "discovery_id": null, "daemon_version": "0.19.0", "discovery_type": {"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:21.572434Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-29 04:18:51.683205+00	2026-11-04 16:56:21.944905+00	0	f	[]
b7d0df1c-5a61-413f-9c99-e3a30a523d3d	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "f1a2c4ba-d124-4b5d-9448-729e2a28c186", "started_at": "2026-06-23T01:16:12.751429360Z", "finished_at": "2026-11-04T16:56:26.088414Z", "discovery_id": null, "daemon_version": "0.19.3", "discovery_type": {"type": "Unified", "host_id": "27a3457e-4127-4ee5-96e3-ba617595159b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:25.493707Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "27a3457e-4127-4ee5-96e3-ba617595159b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-23 01:16:12.751429+00	2026-11-04 16:56:26.089982+00	0	f	[]
500813e8-4e3d-4bdf-88af-780852964aad	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "d53d5b11-b914-4af1-bf04-5d6e47899de1", "started_at": "2026-01-23T03:40:39.007214962Z", "finished_at": "2026-11-04T16:56:30.020910Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:29.055347Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-23 03:40:39.007214+00	2026-11-04 16:56:30.023312+00	0	f	[]
28270da9-5f02-419b-babf-ce501700ac26	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "eb250b58-76a9-4335-a813-c2d71a75e483", "started_at": "2026-02-18T17:07:18.061512748Z", "finished_at": "2026-11-04T16:56:30.032777Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "SelfReport", "host_id": "25a3d274-a408-4ec8-b1fd-69eb88c7674d"}, "last_update_at": "2026-11-04T16:56:28.597224Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "25a3d274-a408-4ec8-b1fd-69eb88c7674d"}	Self Report — My Site	2026-02-18 17:07:18.061512+00	2026-11-04 16:56:30.03522+00	0	f	[]
160cd592-c31c-40a9-8e85-c814e8e5d882	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "8dab7a99-5955-498a-b597-5e7bb0e5d62a", "started_at": "2026-04-28T19:55:22.652516977Z", "finished_at": "2026-11-04T16:56:30.039258Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "Unified", "host_id": "c16a9026-e029-4161-8963-20186c87d56c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:29.582681Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c16a9026-e029-4161-8963-20186c87d56c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-28 19:55:22.652516+00	2026-11-04 16:56:30.040563+00	0	f	[]
0336adf0-c7de-479e-9f36-30046353bef4	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "13af9d3c-5935-4be7-90cf-7fb432a4db27", "started_at": "2026-08-06T01:38:34.928909970Z", "finished_at": "2026-11-04T16:56:23.249887Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "6be44535-29d4-4940-88c3-917ce0d69222", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:22.898028Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6be44535-29d4-4940-88c3-917ce0d69222", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-06 01:38:34.928909+00	2026-11-04 16:56:23.251322+00	0	f	[]
76d97e72-4fc5-49b6-9392-f10468930952	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "f0555e1a-a4e0-4d26-b3e1-2a1c581c337d", "started_at": "2026-02-09T15:41:37.848773295Z", "finished_at": "2026-11-04T16:56:23.261413Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "SelfReport", "host_id": "9e1eeb2e-e918-4dea-a437-6a61a65f4497"}, "last_update_at": "2026-11-04T16:56:22.400284Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "9e1eeb2e-e918-4dea-a437-6a61a65f4497"}	Self Report — My Site	2026-02-09 15:41:37.848773+00	2026-11-04 16:56:23.263986+00	0	f	[]
e5e4338f-f03c-4dba-840d-94f3cc99d3bf	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "c1eaa2e7-7631-418b-8d5f-74597c08a4f5", "started_at": "2026-10-31T00:21:22.175714347Z", "finished_at": "2026-11-04T16:56:26.969577Z", "discovery_id": null, "daemon_version": "0.13.6", "discovery_type": {"type": "Unified", "host_id": "a20cea9a-e475-4818-80d1-32a346c0085e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:26.551322Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a20cea9a-e475-4818-80d1-32a346c0085e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-31 00:21:22.175714+00	2026-11-04 16:56:26.971093+00	0	f	[]
7e46cfbb-fee0-43b0-a9eb-418577e254bb	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "7ad7978f-e4a7-4518-bb77-8a4c8c1dd0c6", "started_at": "2026-07-07T22:27:42.015566135Z", "finished_at": "2026-11-04T16:56:28.037122Z", "discovery_id": null, "daemon_version": "0.17.20", "discovery_type": {"type": "Unified", "host_id": "079502fe-1a72-44a6-ab38-040ead1355fb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:27.413872Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "079502fe-1a72-44a6-ab38-040ead1355fb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-07 22:27:42.015566+00	2026-11-04 16:56:28.038498+00	0	f	[]
6516898d-bf0a-4140-9d56-275b969ad52b	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "8473fe97-6e86-4b62-9b6d-2cd1349deeb8", "started_at": "2026-07-07T22:29:34.241374934Z", "finished_at": "2026-11-04T16:56:28.043298Z", "discovery_id": null, "daemon_version": "0.17.20", "discovery_type": {"type": "Unified", "host_id": "bb32f4ef-dc8e-4c79-89c6-04915971d7b0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:27.466598Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bb32f4ef-dc8e-4c79-89c6-04915971d7b0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-07 22:29:34.241374+00	2026-11-04 16:56:28.044778+00	0	f	[]
ff507415-90aa-499b-b0d2-7fb3bd484c3f	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "3326154e-ed93-4721-9226-b365a900ba24", "started_at": "2026-08-14T17:10:29.182979151Z", "finished_at": "2026-11-04T16:56:23.252688Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:22.885629Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-14 17:10:29.182979+00	2026-11-04 16:56:23.254197+00	0	f	[]
706f1117-d2e2-4fd1-9b76-cb5d40ae603a	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "b9089e15-81e6-435a-90fe-db5b24561924", "started_at": "2026-02-09T15:42:07.840272212Z", "finished_at": "2026-11-04T16:56:23.258012Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:22.461504Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-09 15:42:07.840272+00	2026-11-04 16:56:23.260451+00	0	f	[]
cd68bc0d-54e7-4a7d-936e-c87f6599b2b1	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "8c70cc2e-9120-44c5-8651-e6a5bad04bb9", "started_at": "2026-08-14T17:08:52.205106028Z", "finished_at": "2026-11-04T16:56:23.265094Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "b2d748df-ea85-45ad-8e23-79946b4b5772", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:22.822521Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b2d748df-ea85-45ad-8e23-79946b4b5772", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-14 17:08:52.205106+00	2026-11-04 16:56:23.266512+00	0	f	[]
8f1b3269-7fcf-4f26-bd17-4494679e7d83	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "7c20860a-b07c-4c4f-81bc-c7f44bee7a3d", "started_at": "2026-02-02T14:29:34.148055772Z", "finished_at": "2026-11-04T16:56:24.823462Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:23.411376Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-02 14:29:34.148055+00	2026-11-04 16:56:24.825781+00	0	f	[]
72766a88-bd47-48a6-9373-b8c27bb5be1c	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "5b2fda95-677f-4f93-a93f-daca41bf3af3", "started_at": "2026-07-01T17:45:06.329649066Z", "finished_at": "2026-11-04T16:56:25.472080Z", "discovery_id": null, "daemon_version": "0.17.0", "discovery_type": {"type": "Unified", "host_id": "e56072d7-8284-4090-9a48-d04892a27b80", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:24.859145Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e56072d7-8284-4090-9a48-d04892a27b80", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:45:06.329649+00	2026-11-04 16:56:25.473556+00	0	f	[]
1625efb5-3b64-4e5e-bc24-7d4463484b1b	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "fe678d3a-c2b0-4ee1-a912-7c07b0b6e1d7", "started_at": "2026-06-28T16:04:16.808363474Z", "finished_at": "2026-11-04T16:56:24.808061Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Unified", "host_id": "28b6b18e-69b9-450c-a795-d14a671bb34f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:24.260527Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "28b6b18e-69b9-450c-a795-d14a671bb34f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:04:16.808363+00	2026-11-04 16:56:24.80941+00	0	f	[]
f60bcb90-ab5b-4520-8e63-870cb273a7d0	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "b049b806-80fc-4dc5-9400-f19f9a9857ff", "started_at": "2026-06-28T16:07:10.003081910Z", "finished_at": "2026-11-04T16:56:24.831616Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Unified", "host_id": "37d3a1d8-b76b-4e2e-96e0-74362358f4f4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:24.388884Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "37d3a1d8-b76b-4e2e-96e0-74362358f4f4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:07:10.003081+00	2026-11-04 16:56:24.833111+00	0	f	[]
6a1bc6bf-b559-43e7-8836-5b7abf4b2066	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "06f71522-f4a4-4d87-9785-a442e797d994", "started_at": "2026-06-28T16:02:26.238749468Z", "finished_at": "2026-11-04T16:56:24.837594Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Unified", "host_id": "793047bd-2b38-4e62-8dd9-36ab1aa73797", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:24.210198Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "793047bd-2b38-4e62-8dd9-36ab1aa73797", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:02:26.238749+00	2026-11-04 16:56:24.838893+00	0	f	[]
8a90c561-bae5-436e-92b2-2222a64e5faf	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "bfc425bd-2840-4447-8d67-666671033cd6", "started_at": "2026-06-23T01:20:59.927972592Z", "finished_at": "2026-11-04T16:56:26.091116Z", "discovery_id": null, "daemon_version": "0.19.3", "discovery_type": {"type": "Unified", "host_id": "cf70c05a-1647-4958-806b-ff84119deb9a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:25.750502Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "cf70c05a-1647-4958-806b-ff84119deb9a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-23 01:20:59.927972+00	2026-11-04 16:56:26.092476+00	0	f	[]
2209810e-afa4-42b3-899f-90eeddfbd85a	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "6808c839-ea5d-4f6e-9662-60f24a6dd0c7", "started_at": "2026-03-06T07:04:08.169432273Z", "finished_at": "2026-11-04T16:56:43.646081Z", "discovery_id": null, "daemon_version": "0.14.16", "discovery_type": {"type": "SelfReport", "host_id": "07ca9676-4995-4fd4-b0a3-c60b0b356658"}, "last_update_at": "2026-11-04T16:56:42.757927Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "07ca9676-4995-4fd4-b0a3-c60b0b356658"}	Self Report — My Site	2026-03-06 07:04:08.169432+00	2026-11-04 16:56:43.648737+00	0	f	[]
f5c7805c-272a-422f-9a31-558e12c80138	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "7612c47a-db41-4c37-8d16-fbf4ace0e41e", "started_at": "2026-01-23T03:55:10.336667754Z", "finished_at": "2026-11-04T16:56:24.834269Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:23.842127Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-23 03:55:10.336667+00	2026-11-04 16:56:24.836651+00	0	f	[]
29db0549-e8de-4645-b9c7-f76eec1c2c45	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "a8a4a82b-01b9-4d9b-9bf2-cc44f54ff1d3", "started_at": "2026-10-03T03:54:08.695159586Z", "finished_at": "2026-11-04T16:56:28.502264Z", "discovery_id": null, "daemon_version": "0.14.6", "discovery_type": {"type": "Unified", "host_id": "0e0131ba-ba9b-4828-813f-4a8c181f5836", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:28.056553Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "0e0131ba-ba9b-4828-813f-4a8c181f5836", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-03 03:54:08.695159+00	2026-11-04 16:56:28.503639+00	0	f	[]
249c0f53-9509-47cd-b8c6-8ea4c78b871f	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "c30ae4d6-aa01-4253-94db-acdabf3a54e4", "started_at": "2026-04-28T19:09:10.681879638Z", "finished_at": "2026-11-04T16:56:30.024395Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "Unified", "host_id": "dd6f90d2-d1df-49c8-bba4-cf36f7343201", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:29.489071Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "dd6f90d2-d1df-49c8-bba4-cf36f7343201", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-28 19:09:10.681879+00	2026-11-04 16:56:30.025703+00	0	f	[]
0d25adcd-c339-4390-a40d-6d6f0866978c	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "34d1feb3-276c-4ea9-b889-7eab39b67a1c", "started_at": "2026-02-23T00:07:49.639240713Z", "finished_at": "2026-11-04T16:56:30.029332Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:28.584043Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-23 00:07:49.63924+00	2026-11-04 16:56:30.031783+00	0	f	[]
ce713656-2003-48b0-8ec7-6e6dbba7ea93	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "5d98fc6a-1bba-4985-a654-bf83407df1f2", "started_at": "2026-10-14T05:48:09.745795958Z", "finished_at": "2026-11-04T16:56:30.503005Z", "discovery_id": null, "daemon_version": "0.14.7", "discovery_type": {"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:30.148049Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-14 05:48:09.745795+00	2026-11-04 16:56:30.504538+00	0	f	[]
c6e767e2-fa88-476b-b97f-8abec7d69385	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "bb0bdfdc-1516-4579-a727-079afeb571ea", "started_at": "2026-07-01T17:46:57.844855087Z", "finished_at": "2026-11-04T16:56:25.469337Z", "discovery_id": null, "daemon_version": "0.17.0", "discovery_type": {"type": "Unified", "host_id": "23bba0c3-a3f9-49d7-ac3c-8696037b8b40", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:24.913152Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "23bba0c3-a3f9-49d7-ac3c-8696037b8b40", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:46:57.844855+00	2026-11-04 16:56:25.470743+00	0	f	[]
12c35dc5-eaa8-4571-a0dc-c72c10252d6c	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "e0ea6622-5f78-4cc3-b1f9-257946754dd8", "started_at": "2026-03-09T05:29:17.642983004Z", "finished_at": "2026-03-09T05:29:17.643045281Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:33.171688Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-09 05:29:17.642983+00	2026-11-04 16:56:33.174477+00	0	f	[]
f3c8d6ac-0a1c-4c65-bb7f-c60a51fc316b	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "9c3d4777-a8a7-40f4-9070-c83a719cd6b6", "started_at": "2026-07-23T13:33:48.918734857Z", "finished_at": "2026-11-04T16:56:36.978663Z", "discovery_id": null, "daemon_version": "0.15.0", "discovery_type": {"type": "Unified", "host_id": "95fbda32-d081-4ef7-86c5-ac175f9995b6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:36.611076Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "95fbda32-d081-4ef7-86c5-ac175f9995b6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-23 13:33:48.918734+00	2026-11-04 16:56:36.980156+00	0	f	[]
20ad7e18-78e5-448d-aa72-b3c0474b4785	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "b368c96b-6dbf-4523-bcf9-fee0229c2dfb", "started_at": "2026-10-02T13:49:47.412762700Z", "finished_at": "2026-11-04T16:56:38.959147Z", "discovery_id": null, "daemon_version": "0.15.3", "discovery_type": {"type": "Unified", "host_id": "6aeae378-cd4b-493a-8bff-471792dff268", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:38.506797Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6aeae378-cd4b-493a-8bff-471792dff268", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-02 13:49:47.412762+00	2026-11-04 16:56:38.960634+00	0	f	[]
49ae189b-12f3-411b-9654-5aa45335d2b7	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "2fdd805f-0e91-427a-a614-e15a2a15a0f0", "started_at": "2026-07-05T00:02:02.946186416Z", "finished_at": "2026-11-04T16:56:49.091446Z", "discovery_id": null, "daemon_version": "0.17.12", "discovery_type": {"type": "Unified", "host_id": "4ad32cd4-5017-4aa6-81fb-2a624a987ac2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:48.467201Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "4ad32cd4-5017-4aa6-81fb-2a624a987ac2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-05 00:02:02.946186+00	2026-11-04 16:56:49.092728+00	0	f	[]
e9ea679a-4d7a-439c-90c5-dba44f6e9728	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "feba9c5a-f04c-4e5c-97cd-994c4dffc382", "started_at": "2026-07-07T22:32:35.373947919Z", "finished_at": "2026-11-04T16:56:28.034535Z", "discovery_id": null, "daemon_version": "0.17.20", "discovery_type": {"type": "Unified", "host_id": "f1a03331-5149-4a21-91ed-42d9fd821651", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:27.636417Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f1a03331-5149-4a21-91ed-42d9fd821651", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-07 22:32:35.373947+00	2026-11-04 16:56:28.03595+00	0	f	[]
bd4e405d-d9ee-4fd2-890a-0cdb23f3cc25	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "580379b9-0101-428f-baef-843afbc5dfee", "started_at": "2026-01-26T14:03:54.326189222Z", "finished_at": "2026-11-04T16:56:28.039687Z", "discovery_id": null, "daemon_version": "0.17.20", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:27.050351Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-26 14:03:54.326189+00	2026-11-04 16:56:28.042254+00	0	f	[]
82de45e6-10fc-4a90-8308-26fef23aec7f	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "47b82122-5cf6-42a8-bf36-f15c9a6582f0", "started_at": "2026-04-28T19:50:40.314334020Z", "finished_at": "2026-11-04T16:56:30.026805Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "Unified", "host_id": "689f2fa8-dc17-4321-b680-7af464e5004f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:29.433491Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "689f2fa8-dc17-4321-b680-7af464e5004f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-28 19:50:40.314334+00	2026-11-04 16:56:30.02823+00	0	f	[]
4923d995-4524-45ee-8b5d-fea721389c31	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "d7b0c883-f1dd-45b9-8008-cb06d0194514", "started_at": "2026-10-14T05:43:36.061177700Z", "finished_at": "2026-11-04T16:56:30.505736Z", "discovery_id": null, "daemon_version": "0.14.7", "discovery_type": {"type": "Unified", "host_id": "d4dabb79-eb99-4045-8687-9ca8bd1e797a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:30.051902Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d4dabb79-eb99-4045-8687-9ca8bd1e797a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-14 05:43:36.061177+00	2026-11-04 16:56:30.507041+00	0	f	[]
eccb1771-16a0-4af2-8704-2b9ea4ceb447	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "13aa0c04-f207-47b3-9c03-0f3209efab76", "started_at": "2026-10-12T00:03:18.358288758Z", "finished_at": "2026-11-04T16:56:32.966468Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:32.620134Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-12 00:03:18.358288+00	2026-11-04 16:56:32.9678+00	0	f	[]
b5b2a599-3098-493d-a57b-0e41d268ba31	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "485c7d46-bc1d-4c32-a425-5904751efe34", "started_at": "2026-10-03T03:55:31.060040280Z", "finished_at": "2026-11-04T16:56:28.497044Z", "discovery_id": null, "daemon_version": "0.14.6", "discovery_type": {"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:28.147147Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-03 03:55:31.06004+00	2026-11-04 16:56:28.498487+00	0	f	[]
e4a63a42-4748-46bc-bec1-b6cede08f58f	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "69664faf-d5dd-440a-b554-0af052ce9005", "started_at": "2026-08-01T21:20:27.123050289Z", "finished_at": "2026-11-04T16:56:32.509448Z", "discovery_id": null, "daemon_version": "0.18.0", "discovery_type": {"type": "Unified", "host_id": "b6b48830-db6a-4d8f-8c70-db5516f3e458", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:32.069239Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b6b48830-db6a-4d8f-8c70-db5516f3e458", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-01 21:20:27.12305+00	2026-11-04 16:56:32.510833+00	0	f	[]
f4b02152-1baf-4880-b6fd-b04168d117ac	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "37559eea-aa20-4b8c-ba39-ee899c2fc556", "started_at": "2026-01-05T04:59:56.694889798Z", "finished_at": "2026-11-04T16:56:34.013081Z", "discovery_id": null, "daemon_version": "0.19.6", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:33.644752Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-05 04:59:56.694889+00	2026-11-04 16:56:34.015719+00	0	f	[]
c6dd81b8-dc2d-44c3-82e2-ef921799aede	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "271037a5-7bfa-49d0-bff8-3879ffaa6d30", "started_at": "2026-11-04T16:49:09.224254Z", "finished_at": "2026-11-04T16:56:34.447736Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Unified", "host_id": "f35cae23-aec3-4e90-8c72-278e1d887c57", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:34.027867Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f35cae23-aec3-4e90-8c72-278e1d887c57", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-04 16:49:09.224254+00	2026-11-04 16:56:34.449278+00	0	f	[]
4bc5e8eb-8a17-42d1-8ad4-07e4b076ee47	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "df28970e-d903-4f00-8b2a-1dfa823e410c", "started_at": "2026-07-28T03:25:25.938837651Z", "finished_at": "2026-11-04T16:56:35.464677Z", "discovery_id": null, "daemon_version": "0.14.13", "discovery_type": {"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:35.103961Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-28 03:25:25.938837+00	2026-11-04 16:56:35.46605+00	0	f	[]
415ece9e-7ee8-4a61-86dc-c850fc9e84b1	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "20717e93-7bdb-4413-9112-9ba79c38289f", "started_at": "2026-10-03T03:59:01.064308022Z", "finished_at": "2026-11-04T16:56:28.499827Z", "discovery_id": null, "daemon_version": "0.14.6", "discovery_type": {"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:28.153994Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-03 03:59:01.064308+00	2026-11-04 16:56:28.501187+00	0	f	[]
81ab48ea-f29b-4c67-9c41-4ac405f074a9	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "b3287603-7c63-47ac-886a-4b6bb9cb2151", "started_at": "2026-08-01T21:25:32.482042816Z", "finished_at": "2026-11-04T16:56:32.506808Z", "discovery_id": null, "daemon_version": "0.18.0", "discovery_type": {"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:32.137297Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-01 21:25:32.482042+00	2026-11-04 16:56:32.508355+00	0	f	[]
f644f124-a880-4fdf-9cc7-c1c0b1e6f75b	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "80837ade-3bd8-4bfa-a94b-ec249e6517fa", "started_at": "2026-03-11T06:19:54.610393414Z", "finished_at": "2026-11-04T16:56:35.020924Z", "discovery_id": null, "daemon_version": "0.17.6", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:34.529962Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:19:54.610393+00	2026-11-04 16:56:35.023555+00	0	f	[]
35c8a16d-bea1-41b7-a472-9d27f10e1b3a	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "f9ec8267-d3bb-4170-80d5-9c5a41add7eb", "started_at": "2026-07-28T03:20:23.724249500Z", "finished_at": "2026-11-04T16:56:35.462158Z", "discovery_id": null, "daemon_version": "0.14.13", "discovery_type": {"type": "Unified", "host_id": "24f17e21-d958-4c17-ad9d-3f058f1bfd07", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:35.044227Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "24f17e21-d958-4c17-ad9d-3f058f1bfd07", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-28 03:20:23.724249+00	2026-11-04 16:56:35.463501+00	0	f	[]
2f45bba5-7a73-48e7-b58f-73049377a1a9	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "d033c5f0-e137-4b0a-b42e-78daa430f113", "started_at": "2026-07-23T13:32:18.163549314Z", "finished_at": "2026-11-04T16:56:36.973356Z", "discovery_id": null, "daemon_version": "0.15.0", "discovery_type": {"type": "Unified", "host_id": "c37e16d2-78cb-4ad7-bffc-f8eaec8ac967", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:36.556059Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c37e16d2-78cb-4ad7-bffc-f8eaec8ac967", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-23 13:32:18.163549+00	2026-11-04 16:56:36.974824+00	0	f	[]
a90781f0-9081-488c-9c60-2f4fded61c5e	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "833d88c3-2acf-4989-86de-466d44e47a7e", "started_at": "2026-10-14T05:44:39.739395480Z", "finished_at": "2026-11-04T16:56:30.508243Z", "discovery_id": null, "daemon_version": "0.14.7", "discovery_type": {"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:30.141103Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-14 05:44:39.739395+00	2026-11-04 16:56:30.509786+00	0	f	[]
401c4a59-20b3-49ca-b299-f115c0abd7bc	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "faf2ae16-9ccd-4ba2-8be6-077b30ff3ae7", "started_at": "2026-08-26T01:59:44.730528896Z", "finished_at": "2026-11-04T16:56:31.460772Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:31.088314Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-26 01:59:44.730528+00	2026-11-04 16:56:31.462279+00	0	f	[]
11f1a16f-6a78-4d40-81db-bd1ec5c965c0	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "e8b2155f-5622-4046-aff2-4f8de2f868f0", "started_at": "2026-02-23T01:47:07.653242727Z", "finished_at": "2026-11-04T16:56:31.466313Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "SelfReport", "host_id": "49b8f936-67c8-4835-bc8c-5d897e43cdb0"}, "last_update_at": "2026-11-04T16:56:30.530529Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "49b8f936-67c8-4835-bc8c-5d897e43cdb0"}	Self Report — My Site	2026-02-23 01:47:07.653242+00	2026-11-04 16:56:31.468845+00	0	f	[]
c5d4516e-0954-4dd8-a99e-8dacf4925881	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "c17fbe8e-8128-45b1-b2b5-afef7e2c082f", "started_at": "2026-02-23T01:47:37.649663614Z", "finished_at": "2026-11-04T16:56:31.472447Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:30.590387Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-23 01:47:37.649663+00	2026-11-04 16:56:31.474945+00	0	f	[]
a9c5cde5-de10-4c73-8c8f-86e6d2692f32	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "ed48cfed-4a98-4473-9c83-33781243c790", "started_at": "2026-03-09T05:16:41.592520993Z", "finished_at": "2026-11-04T16:56:34.005178Z", "discovery_id": null, "daemon_version": "0.19.6", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:33.052983Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-09 05:16:41.59252+00	2026-11-04 16:56:34.007763+00	0	f	[]
ff927a60-cdd7-4aa2-8df0-5e01ff54dc52	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "69f77b5e-1350-4d55-ae3e-a0aa03bb47cd", "started_at": "2026-08-25T02:50:04.633793664Z", "finished_at": "2026-11-04T16:56:31.463456Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Unified", "host_id": "802b4d69-588c-452a-83c3-1e47ca5d84d1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:31.094628Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "802b4d69-588c-452a-83c3-1e47ca5d84d1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 02:50:04.633793+00	2026-11-04 16:56:31.464765+00	0	f	[]
1599e2f4-828a-4aa7-b5d1-1ff0afc75157	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "3bba0463-9815-4685-8b2e-93eacc318f67", "started_at": "2026-08-26T01:56:14.716285279Z", "finished_at": "2026-11-04T16:56:31.469827Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:31.081616Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-26 01:56:14.716285+00	2026-11-04 16:56:31.471327+00	0	f	[]
042ef3cc-275a-4939-9807-9d6f6b7802e1	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "9e24c5ae-ab53-4e0f-93db-281f3614fd56", "started_at": "2026-10-11T23:58:36.269774581Z", "finished_at": "2026-11-04T16:56:32.971684Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "Unified", "host_id": "8d1f0138-018b-4734-9af4-5f196dc2a028", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:32.522977Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8d1f0138-018b-4734-9af4-5f196dc2a028", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-11 23:58:36.269774+00	2026-11-04 16:56:32.973077+00	0	f	[]
46c62723-b28b-4fd2-9b29-0cad9d081d42	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "b43a3d5a-af6a-42f8-bf72-70c87206ab39", "started_at": "2026-07-22T23:04:06.688284640Z", "finished_at": "2026-11-04T16:56:36.976019Z", "discovery_id": null, "daemon_version": "0.15.0", "discovery_type": {"type": "Unified", "host_id": "ea19a9cd-5a65-4620-bff8-eb890602e187", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:36.617727Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "ea19a9cd-5a65-4620-bff8-eb890602e187", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-22 23:04:06.688284+00	2026-11-04 16:56:36.977534+00	0	f	[]
c393ad86-297e-4d18-b3a6-0d4f1eb0ae2e	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "00342129-a6d7-490e-84ba-2bd11b17a768", "started_at": "2026-02-26T00:44:02.257915046Z", "finished_at": "2026-11-04T16:56:45.494103Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:44.702105Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-26 00:44:02.257915+00	2026-11-04 16:56:45.496716+00	0	f	[]
991ad557-8cc3-4def-bb33-5622b66dd146	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "d10e30a1-1b5e-4a25-abe9-566d427e3186", "started_at": "2026-03-24T03:12:35.661709995Z", "finished_at": "2026-11-04T16:56:32.046580Z", "discovery_id": null, "daemon_version": "0.17.8", "discovery_type": {"type": "Unified", "host_id": "80008868-cd5c-4f1e-92ea-0bd73728c9a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:31.495013Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "80008868-cd5c-4f1e-92ea-0bd73728c9a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-24 03:12:35.661709+00	2026-11-04 16:56:32.048031+00	0	f	[]
4c594704-d362-4740-ae08-85ae13560ebe	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "2453f28d-e87c-4753-80ad-0416d47c71f8", "started_at": "2026-10-11T23:59:48.356682576Z", "finished_at": "2026-11-04T16:56:32.969086Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:32.613340Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-11 23:59:48.356682+00	2026-11-04 16:56:32.970503+00	0	f	[]
26f4375d-8ac0-49e1-9a73-c9e520e9136e	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "457a544b-3c09-45bf-90d1-ab9e5e4a144d", "started_at": "2026-04-01T14:32:54.762125416Z", "finished_at": "2026-11-04T16:56:36.526406Z", "discovery_id": null, "daemon_version": "0.17.5", "discovery_type": {"type": "Unified", "host_id": "a33f8415-bb39-43da-81c3-dfafa82595da", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:35.947758Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a33f8415-bb39-43da-81c3-dfafa82595da", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-01 14:32:54.762125+00	2026-11-04 16:56:36.527763+00	0	f	[]
2e04dae8-d61d-4ac1-85ec-a9b2bb0536ee	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "ae3a3d90-03a9-4961-a1d4-840e661d29a8", "started_at": "2026-02-11T02:46:52.998653452Z", "finished_at": "2026-11-04T16:56:41.314128Z", "discovery_id": null, "daemon_version": "0.14.4", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:40.002918Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-11 02:46:52.998653+00	2026-11-04 16:56:41.316839+00	0	f	[]
39aec144-a352-48cf-a9fe-dd2f2b575959	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "e39a2b83-3f31-4943-abbc-ba31d154ce16", "started_at": "2026-02-11T02:46:23.000125066Z", "finished_at": "2026-11-04T16:56:41.320929Z", "discovery_id": null, "daemon_version": "0.14.4", "discovery_type": {"type": "SelfReport", "host_id": "d4234791-d4ee-4257-a9d3-a257b5e1e3b9"}, "last_update_at": "2026-11-04T16:56:39.943976Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "d4234791-d4ee-4257-a9d3-a257b5e1e3b9"}	Self Report — My Site	2026-02-11 02:46:23.000125+00	2026-11-04 16:56:41.323552+00	0	f	[]
016e951a-9481-4f93-ad81-45be59ce0686	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "7e06b95e-3b67-41c4-9049-978411591e7c", "started_at": "2026-08-01T21:22:02.500665824Z", "finished_at": "2026-11-04T16:56:32.503929Z", "discovery_id": null, "daemon_version": "0.18.0", "discovery_type": {"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:32.130863Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-01 21:22:02.500665+00	2026-11-04 16:56:32.505486+00	0	f	[]
43dfe8b1-9274-471a-bb0a-ac75caa2c8fd	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "44932e97-76ed-4c08-b694-4e6aa6bca1be", "started_at": "2026-03-05T09:33:11.114723268Z", "finished_at": "2026-11-04T16:56:36.532943Z", "discovery_id": null, "daemon_version": "0.17.5", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:35.546763Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-05 09:33:11.114723+00	2026-11-04 16:56:36.535341+00	0	f	[]
c1d35b2a-4c04-469b-a456-000c9ae1ffc8	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "ca5b8138-b6c5-41bb-928c-59ab96c38227", "started_at": "2026-09-16T20:13:01.385050881Z", "finished_at": "2026-11-04T16:56:38.024899Z", "discovery_id": null, "daemon_version": "0.14.12", "discovery_type": {"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:37.658987Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 20:13:01.38505+00	2026-11-04 16:56:38.02638+00	0	f	[]
00c24f27-35b5-42fa-a92d-6594e33ec1ad	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "c063e961-df38-4fbb-b437-0e6e25593003", "started_at": "2026-09-24T21:00:16.922463583Z", "finished_at": "2026-11-04T16:56:42.261627Z", "discovery_id": null, "daemon_version": "0.14.10", "discovery_type": {"type": "Unified", "host_id": "ad3c2bd6-9bf9-4c4d-8534-3689c0e013c3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:41.822237Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "ad3c2bd6-9bf9-4c4d-8534-3689c0e013c3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-24 21:00:16.922463+00	2026-11-04 16:56:42.263073+00	0	f	[]
cec86af5-f694-4ac6-80b8-dc9683f9d785	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "3dec6b93-d9d7-46ab-94e2-ef221564d563", "started_at": "2026-09-24T21:01:31.815231601Z", "finished_at": "2026-11-04T16:56:42.268096Z", "discovery_id": null, "daemon_version": "0.14.10", "discovery_type": {"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:41.909703Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-24 21:01:31.815231+00	2026-11-04 16:56:42.269514+00	0	f	[]
7ac79d62-9f98-4bd4-a16d-950bf58b0f6e	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "b3439cfa-af27-47ae-8b9d-8c79b9d149fa", "started_at": "2026-07-28T03:21:55.956404860Z", "finished_at": "2026-11-04T16:56:35.467287Z", "discovery_id": null, "daemon_version": "0.14.13", "discovery_type": {"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:35.097470Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-28 03:21:55.956404+00	2026-11-04 16:56:35.468661+00	0	f	[]
81d04f58-f617-455c-b2f4-5ad49f8d7bd4	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "a1b350a1-59b8-4caf-ae55-c9f3f8ca89fb", "started_at": "2026-03-05T09:32:41.117544068Z", "finished_at": "2026-11-04T16:56:36.529031Z", "discovery_id": null, "daemon_version": "0.17.5", "discovery_type": {"type": "SelfReport", "host_id": "2a76c091-9847-4916-bc73-8668758f6f7d"}, "last_update_at": "2026-11-04T16:56:35.488672Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "2a76c091-9847-4916-bc73-8668758f6f7d"}	Self Report — My Site	2026-03-05 09:32:41.117544+00	2026-11-04 16:56:36.531873+00	0	f	[]
cb3fef22-72f3-4ad8-b5af-5ab73424f477	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "947c4ec5-0754-4116-8282-283b6a09f803", "started_at": "2026-03-05T07:38:50.115878953Z", "finished_at": "2026-11-04T16:56:38.949995Z", "discovery_id": null, "daemon_version": "0.15.3", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:38.111333Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-05 07:38:50.115878+00	2026-11-04 16:56:38.952839+00	0	f	[]
2d33762a-0a41-4565-8c66-3eb51eba61ba	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "901cf5c8-1989-4bf1-bf5d-e587ee211cda", "started_at": "2026-03-26T17:23:47.082705682Z", "finished_at": "2026-11-04T16:56:39.921988Z", "discovery_id": null, "daemon_version": "0.14.5", "discovery_type": {"type": "Unified", "host_id": "2bbce8eb-5555-41ef-b999-68a5af971278", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:38.984492Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2bbce8eb-5555-41ef-b999-68a5af971278", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-26 17:23:47.082705+00	2026-11-04 16:56:39.923527+00	0	f	[]
41bc96ff-8c7c-4d92-8483-73c19bd7cce0	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "0400691f-2af1-4b20-bade-e50044271bfe", "started_at": "2026-10-07T13:13:08.735400435Z", "finished_at": "2026-11-04T16:56:41.318074Z", "discovery_id": null, "daemon_version": "0.14.4", "discovery_type": {"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:40.970251Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-07 13:13:08.7354+00	2026-11-04 16:56:41.319682+00	0	f	[]
ba46b3ee-707a-4b1c-8ffe-7ea3cc1a72f3	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "f071713f-3367-4111-8f7b-7b7b50d33aa9", "started_at": "2026-03-23T23:57:20.088532334Z", "finished_at": "2026-11-04T16:56:37.547708Z", "discovery_id": null, "daemon_version": "0.17.16", "discovery_type": {"type": "Unified", "host_id": "14fcf158-caea-47c9-82ad-38dcc2907791", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:37.000822Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "14fcf158-caea-47c9-82ad-38dcc2907791", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-23 23:57:20.088532+00	2026-11-04 16:56:37.549114+00	0	f	[]
fb1a7664-1fea-43c4-8a36-7bc21f83ddea	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "64824f2d-3081-44a6-a631-23dbe65e4721", "started_at": "2026-09-16T20:11:28.193921546Z", "finished_at": "2026-11-04T16:56:38.027494Z", "discovery_id": null, "daemon_version": "0.14.12", "discovery_type": {"type": "Unified", "host_id": "505ad895-0816-4969-a72c-161ebc636002", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:37.569805Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "505ad895-0816-4969-a72c-161ebc636002", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 20:11:28.193921+00	2026-11-04 16:56:38.028814+00	0	f	[]
5f741da7-afdd-4d2d-95eb-a2ff92dce128	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "b5591b69-0dfe-41fa-87c1-3d5e140cca15", "started_at": "2026-03-06T07:04:38.164982988Z", "finished_at": "2026-11-04T16:56:43.649919Z", "discovery_id": null, "daemon_version": "0.14.16", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:42.817937Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-06 07:04:38.164982+00	2026-11-04 16:56:43.652302+00	0	f	[]
0ca8dea7-06e4-471f-9d41-569e0a15c785	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "68d1cd6d-9bbc-4af3-9202-ecf0db0dee57", "started_at": "2026-08-06T01:43:36.139858287Z", "finished_at": "2026-11-04T16:56:43.656882Z", "discovery_id": null, "daemon_version": "0.14.16", "discovery_type": {"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:43.290443Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-06 01:43:36.139858+00	2026-11-04 16:56:43.658278+00	0	f	[]
c92dd2d9-216a-4a55-82ef-334f9ca5a1a3	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "ce6f84d7-a1a5-468b-887d-1a0c67e78012", "started_at": "2026-02-28T21:18:57.228697745Z", "finished_at": "2026-11-04T16:56:43.661960Z", "discovery_id": null, "daemon_version": "0.14.16", "discovery_type": {"type": "SelfReport", "host_id": "b52f72ce-8332-4d4f-90a6-596cbbb0ba73"}, "last_update_at": "2026-11-04T16:56:42.293094Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "b52f72ce-8332-4d4f-90a6-596cbbb0ba73"}	Self Report — My Site	2026-02-28 21:18:57.228697+00	2026-11-04 16:56:43.664287+00	0	f	[]
ce04dbbe-4a2a-42db-a089-3c391a27f2a0	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "24dd8f5a-4866-4462-a9ff-b65cb5a3061a", "started_at": "2026-09-16T20:16:31.390119148Z", "finished_at": "2026-11-04T16:56:38.022288Z", "discovery_id": null, "daemon_version": "0.14.12", "discovery_type": {"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:37.665700Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 20:16:31.390119+00	2026-11-04 16:56:38.023675+00	0	f	[]
b7e561c3-5f00-4fe5-99e4-b5d6e3684639	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "1e8cf04d-e26d-4e84-a90f-4a534c217faf", "started_at": "2026-10-02T13:54:28.548758981Z", "finished_at": "2026-11-04T16:56:38.956836Z", "discovery_id": null, "daemon_version": "0.15.3", "discovery_type": {"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:38.600944Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-02 13:54:28.548758+00	2026-11-04 16:56:38.958083+00	0	f	[]
6540ff7a-9de7-456e-8e9b-2e24b18862b5	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "110d0739-6875-498e-9ed5-5c7c0dec84ca", "started_at": "2026-03-05T07:38:20.117782332Z", "finished_at": "2026-11-04T16:56:38.961846Z", "discovery_id": null, "daemon_version": "0.15.3", "discovery_type": {"type": "SelfReport", "host_id": "9b3486eb-3e41-4b56-8062-c98ad29f0a3a"}, "last_update_at": "2026-11-04T16:56:38.050901Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "9b3486eb-3e41-4b56-8062-c98ad29f0a3a"}	Self Report — My Site	2026-03-05 07:38:20.117782+00	2026-11-04 16:56:38.964325+00	0	f	[]
dcc47153-7ab1-4839-9025-fdbc036d55af	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "c50795be-e20e-483e-857d-f04a9f103f60", "started_at": "2026-08-17T00:25:40.701352183Z", "finished_at": "2026-11-04T16:56:45.497807Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:45.125320Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-17 00:25:40.701352+00	2026-11-04 16:56:45.499322+00	0	f	[]
93079a1e-f183-40c4-8066-437ac8dd8132	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "bbdc17b9-0273-4a91-8a0e-268a6fcf0277", "started_at": "2026-02-26T00:43:32.254923713Z", "finished_at": "2026-11-04T16:56:45.502946Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "SelfReport", "host_id": "4c158030-00c1-49eb-b7e2-4e803057a3dd"}, "last_update_at": "2026-11-04T16:56:44.642372Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "4c158030-00c1-49eb-b7e2-4e803057a3dd"}	Self Report — My Site	2026-02-26 00:43:32.254923+00	2026-11-04 16:56:45.505586+00	0	f	[]
e4689315-e3ad-4b31-9b78-a37d214eb4c9	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "68690167-4bf9-40db-9756-207731c78629", "started_at": "2026-09-16T19:13:38.792500966Z", "finished_at": "2026-11-04T16:56:38.029847Z", "discovery_id": null, "daemon_version": "0.14.12", "discovery_type": {"type": "Unified", "host_id": "fe9f2b88-655a-44e9-b2b2-b89b797b34ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:37.672139Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "fe9f2b88-655a-44e9-b2b2-b89b797b34ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 19:13:38.7925+00	2026-11-04 16:56:38.031291+00	0	f	[]
ccee9e66-daea-453c-b532-c5b77ad87f30	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "9d715165-15d9-4c53-8eeb-adadd14656ec", "started_at": "2026-10-26T16:01:31.798722266Z", "finished_at": "2026-11-04T16:56:50.447441Z", "discovery_id": null, "daemon_version": "0.17.18", "discovery_type": {"type": "Unified", "host_id": "9b66035e-43fc-4de9-a9fa-9febda413452", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:50.008158Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9b66035e-43fc-4de9-a9fa-9febda413452", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-26 16:01:31.798722+00	2026-11-04 16:56:50.448883+00	0	f	[]
7550d36f-1b2e-4706-acb7-48744e181c41	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "93db1812-9fa5-4c4b-a36b-d687b363811c", "started_at": "2026-10-02T13:50:58.545317669Z", "finished_at": "2026-11-04T16:56:38.953941Z", "discovery_id": null, "daemon_version": "0.15.3", "discovery_type": {"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:38.594307Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-02 13:50:58.545317+00	2026-11-04 16:56:38.95562+00	0	f	[]
f02e5ef9-9fe3-4123-9140-e9faf4847ee9	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "b39cff2d-d385-4ab8-8097-66c2190dae09", "started_at": "2026-03-06T23:20:38.018012792Z", "finished_at": "2026-11-04T16:56:41.324618Z", "discovery_id": null, "daemon_version": "0.14.4", "discovery_type": {"type": "SelfReport", "host_id": "6fd7ccd6-6ea7-4880-b5e5-ad545df36544"}, "last_update_at": "2026-11-04T16:56:40.410067Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "6fd7ccd6-6ea7-4880-b5e5-ad545df36544"}	Self Report — My Site	2026-03-06 23:20:38.018012+00	2026-11-04 16:56:41.327201+00	0	f	[]
33812c73-8224-43a8-8071-1210ccb2ffba	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "72d38ac6-a392-41f5-96f5-17c22a19e8f5", "started_at": "2026-03-06T23:21:08.018383672Z", "finished_at": "2026-11-04T16:56:41.330822Z", "discovery_id": null, "daemon_version": "0.14.4", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:40.470867Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-06 23:21:08.018383+00	2026-11-04 16:56:41.333457+00	0	f	[]
7d4b4462-9bda-470f-b4ec-2bfe57ad83a6	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "44164544-aa86-4db3-ad9a-503d5933df88", "started_at": "2026-09-24T21:05:01.816315928Z", "finished_at": "2026-11-04T16:56:42.258879Z", "discovery_id": null, "daemon_version": "0.14.10", "discovery_type": {"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:41.916335Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-24 21:05:01.816315+00	2026-11-04 16:56:42.26039+00	0	f	[]
ba2a0cc8-c99d-4587-8f5a-f969868f6653	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "14a48b0e-4297-410e-b019-b1d98130f2c0", "started_at": "2026-02-10T04:10:50.083304178Z", "finished_at": "2026-11-04T16:56:42.264356Z", "discovery_id": null, "daemon_version": "0.14.10", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:41.422376Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-10 04:10:50.083304+00	2026-11-04 16:56:42.267043+00	0	f	[]
d8fb3b3e-0c85-4b1a-ada2-6ea390f132b6	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "61efb9ca-2ac2-4ab8-858e-07da2d3ef161", "started_at": "2026-02-10T04:10:20.090437467Z", "finished_at": "2026-11-04T16:56:42.270567Z", "discovery_id": null, "daemon_version": "0.14.10", "discovery_type": {"type": "SelfReport", "host_id": "45cb5c93-df81-41f1-9047-97867efb47bb"}, "last_update_at": "2026-11-04T16:56:41.357077Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "45cb5c93-df81-41f1-9047-97867efb47bb"}	Self Report — My Site	2026-02-10 04:10:20.090437+00	2026-11-04 16:56:42.272896+00	0	f	[]
9c0045e0-3338-4137-a877-6f258b79e999	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "56d2df9f-197b-49f8-b4ac-e9d4674dca7a", "started_at": "2026-10-07T13:08:30.855227146Z", "finished_at": "2026-11-04T16:56:41.328336Z", "discovery_id": null, "daemon_version": "0.14.4", "discovery_type": {"type": "Unified", "host_id": "1be4e31e-a229-440d-b0f4-20c31ee11254", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:40.878710Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "1be4e31e-a229-440d-b0f4-20c31ee11254", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-07 13:08:30.855227+00	2026-11-04 16:56:41.329671+00	0	f	[]
df62cb1b-3595-46d9-bce3-082b41709b24	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "e903943a-3b77-43c1-a9ad-824cb543557b", "started_at": "2026-10-07T13:09:38.731246701Z", "finished_at": "2026-11-04T16:56:41.334602Z", "discovery_id": null, "daemon_version": "0.14.4", "discovery_type": {"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:40.963430Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-07 13:09:38.731246+00	2026-11-04 16:56:41.335911+00	0	f	[]
6b207e9a-2f1a-48e2-ab49-5e8b39c51ecd	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "d36fca8d-dbbc-4cad-a19f-f495eacf6234", "started_at": "2026-02-28T21:19:27.224801293Z", "finished_at": "2026-11-04T16:56:43.653430Z", "discovery_id": null, "daemon_version": "0.14.16", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:42.354472Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-28 21:19:27.224801+00	2026-11-04 16:56:43.655898+00	0	f	[]
ed6164d2-edaa-4406-a877-8d97795ba002	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "a49f4dc9-0185-4793-9f4f-0843c7021f81", "started_at": "2026-03-08T06:54:18.293920451Z", "finished_at": "2026-11-04T16:56:44.610321Z", "discovery_id": null, "daemon_version": "0.14.8", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:43.742701Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-08 06:54:18.29392+00	2026-11-04 16:56:44.613047+00	0	f	[]
647597ed-4139-4afe-b24c-92f11685793a	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "bfb7da47-98f1-4957-9f13-5b4f6f440e6a", "started_at": "2026-03-08T06:53:48.342722812Z", "finished_at": "2026-11-04T16:56:44.616781Z", "discovery_id": null, "daemon_version": "0.14.8", "discovery_type": {"type": "SelfReport", "host_id": "caeb1c88-3a79-4e24-8a5a-fedd841b3a98"}, "last_update_at": "2026-11-04T16:56:43.684105Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "caeb1c88-3a79-4e24-8a5a-fedd841b3a98"}	Self Report — My Site	2026-03-08 06:53:48.342722+00	2026-11-04 16:56:44.619317+00	0	f	[]
d6f07b7f-53b0-4a2c-9846-26af44b08925	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "c9568c2f-1ae3-48c4-9136-6eb8fcd496a8", "started_at": "2026-08-06T01:40:06.139857038Z", "finished_at": "2026-11-04T16:56:43.659386Z", "discovery_id": null, "daemon_version": "0.14.16", "discovery_type": {"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:43.284223Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-06 01:40:06.139857+00	2026-11-04 16:56:43.660874+00	0	f	[]
4b5d1fab-1671-4d65-9291-6ac46eb3637f	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "128947fe-516b-47a0-897f-975edb8d4122", "started_at": "2026-04-20T17:24:17.714515792Z", "finished_at": "2026-11-04T16:56:46.141658Z", "discovery_id": null, "daemon_version": "0.19.1", "discovery_type": {"type": "Unified", "host_id": "41167b70-ef54-45fb-ac1c-df4f8be5f312", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:45.587587Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "41167b70-ef54-45fb-ac1c-df4f8be5f312", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-20 17:24:17.714515+00	2026-11-04 16:56:46.143225+00	0	f	[]
1a896abf-1110-421e-849f-f101790cdf6b	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "0073f594-456f-4861-8285-c8eb1345c48c", "started_at": "2026-10-25T01:45:04.782037127Z", "finished_at": "2026-11-04T16:56:46.601812Z", "discovery_id": null, "daemon_version": "0.19.4", "discovery_type": {"type": "Unified", "host_id": "2d0259ed-6b96-4930-847a-fa3ebdbf38a3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:46.160247Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2d0259ed-6b96-4930-847a-fa3ebdbf38a3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-25 01:45:04.782037+00	2026-11-04 16:56:46.603204+00	0	f	[]
822b70a8-b364-4778-a659-6118739b88a0	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "6bbb8f0d-ed69-4df8-81d6-8bc44cdf4380", "started_at": "2026-03-02T09:02:56.919081491Z", "finished_at": "2026-11-04T16:56:48.436322Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:47.121Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-02 09:02:56.919081+00	2026-11-04 16:56:48.438825+00	0	f	[]
7c661a6b-d541-43c0-8c18-a6def7106b80	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "1a2a6d39-262b-45ff-80c8-1f8742f766af", "started_at": "2026-02-28T03:10:51.549945495Z", "finished_at": "2026-11-04T16:56:48.442339Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "SelfReport", "host_id": "422f12c5-aed6-4cdc-aab7-a281212ef855"}, "last_update_at": "2026-11-04T16:56:47.522574Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "422f12c5-aed6-4cdc-aab7-a281212ef855"}	Self Report — My Site	2026-02-28 03:10:51.549945+00	2026-11-04 16:56:48.44483+00	0	f	[]
c98db2ee-9ba8-4ad5-a8a1-341ab8ad6bf5	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "c8b7e98c-df16-4cc6-88f2-82866bbb959f", "started_at": "2026-09-15T05:38:49.198136490Z", "finished_at": "2026-11-04T16:56:44.607545Z", "discovery_id": null, "daemon_version": "0.14.8", "discovery_type": {"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:44.239583Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-15 05:38:49.198136+00	2026-11-04 16:56:44.609001+00	0	f	[]
173279a7-4f63-4867-ac38-bf9e897203c1	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "289721de-ae03-40cd-95d5-6af379282969", "started_at": "2026-09-15T05:37:18.553708846Z", "finished_at": "2026-11-04T16:56:44.614150Z", "discovery_id": null, "daemon_version": "0.14.8", "discovery_type": {"type": "Unified", "host_id": "2a9b1a5e-51d9-4f13-b6e4-0b1e3e27e0dc", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:44.153118Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2a9b1a5e-51d9-4f13-b6e4-0b1e3e27e0dc", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-15 05:37:18.553708+00	2026-11-04 16:56:44.61562+00	0	f	[]
026ba5e2-6a2b-41f9-b1d2-8eb9173852b9	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "01486396-5c85-4268-b3c8-b83da1042232", "started_at": "2026-09-15T05:42:19.268112358Z", "finished_at": "2026-11-04T16:56:44.620394Z", "discovery_id": null, "daemon_version": "0.14.8", "discovery_type": {"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:44.245883Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-15 05:42:19.268112+00	2026-11-04 16:56:44.621892+00	0	f	[]
d073b296-dcb0-40e7-a308-4e198fbf6074	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "ff3dfff8-b772-4147-bc5c-c2806e8e84a4", "started_at": "2026-02-28T03:11:21.545902350Z", "finished_at": "2026-11-04T16:56:48.426670Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:47.582283Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-28 03:11:21.545902+00	2026-11-04 16:56:48.429048+00	0	f	[]
440a3fa3-d057-467f-bd34-944d3e209db5	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "2b6e6d22-4644-4cad-a850-04fb5896df10", "started_at": "2026-03-02T09:02:26.922912054Z", "finished_at": "2026-11-04T16:56:48.432969Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "SelfReport", "host_id": "b6dcbdab-8ba5-4b12-8dee-5177af2cf702"}, "last_update_at": "2026-11-04T16:56:47.061045Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "b6dcbdab-8ba5-4b12-8dee-5177af2cf702"}	Self Report — My Site	2026-03-02 09:02:26.922912+00	2026-11-04 16:56:48.435356+00	0	f	[]
a87c06bb-6b3a-480b-8a31-b7998dfcc661	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "42d8b7a7-6a5c-409b-b8c2-d33c0af1e9c0", "started_at": "2026-08-17T00:29:10.692540259Z", "finished_at": "2026-11-04T16:56:45.500390Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:45.131999Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-17 00:29:10.69254+00	2026-11-04 16:56:45.501839+00	0	f	[]
4555c353-2c5d-442f-be56-04db1f232261	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "2b8a5125-8bbc-4f50-9fd7-baad3f93a89f", "started_at": "2026-08-17T00:24:03.071614457Z", "finished_at": "2026-11-04T16:56:45.506600Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Unified", "host_id": "51d0c941-a5a3-4d6c-b3a9-5afbcaed2d3d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:45.063037Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "51d0c941-a5a3-4d6c-b3a9-5afbcaed2d3d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-17 00:24:03.071614+00	2026-11-04 16:56:45.508086+00	0	f	[]
2b3f8683-da87-4d19-8151-6379beec1b43	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "dae6845a-5c1c-4537-8549-2cb0bb052dd7", "started_at": "2026-04-20T18:00:40.095617293Z", "finished_at": "2026-11-04T16:56:46.144350Z", "discovery_id": null, "daemon_version": "0.19.1", "discovery_type": {"type": "Unified", "host_id": "c1107482-106c-4d18-8ee2-07081aef2afb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:45.643811Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c1107482-106c-4d18-8ee2-07081aef2afb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-20 18:00:40.095617+00	2026-11-04 16:56:46.145624+00	0	f	[]
af38e069-0908-418a-9edc-7849db5ee447	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "675b82cb-527e-4541-8702-fef2a8eb5552", "started_at": "2026-08-25T02:55:08.502675512Z", "finished_at": "2026-11-04T16:56:49.994750Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:49.181966Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 02:55:08.502675+00	2026-11-04 16:56:49.996124+00	0	f	[]
da5bf036-9c0a-4afd-ad7f-05bf5bb7278d	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "04ade0d1-6874-420a-a1d3-6ece53cd5f6f", "started_at": "2026-04-20T17:56:03.901696836Z", "finished_at": "2026-11-04T16:56:46.146649Z", "discovery_id": null, "daemon_version": "0.19.1", "discovery_type": {"type": "Unified", "host_id": "2551f8f8-f3cf-43d9-9d28-dd99c9468591", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:45.528413Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2551f8f8-f3cf-43d9-9d28-dd99c9468591", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-20 17:56:03.901696+00	2026-11-04 16:56:46.148134+00	0	f	[]
dcd7783a-4391-47ac-8ce2-0aa7e024fab3	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "6a03df7f-6023-4f74-893b-f9bb39cb8d98", "started_at": "2026-10-29T03:47:06.824079209Z", "finished_at": "2026-11-04T16:56:47.038624Z", "discovery_id": null, "daemon_version": "0.14.11", "discovery_type": {"type": "Unified", "host_id": "f591b8f3-e44c-4fab-bb81-9b67c29df370", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:46.615101Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f591b8f3-e44c-4fab-bb81-9b67c29df370", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-29 03:47:06.824079+00	2026-11-04 16:56:47.040232+00	0	f	[]
460947f2-0c82-4167-b8a4-42493d755043	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "075ee07b-1779-47db-8c94-50c17ce5f66d", "started_at": "2026-10-01T04:03:53.917292009Z", "finished_at": "2026-11-04T16:57:02.248376Z", "discovery_id": null, "daemon_version": "0.19.6", "discovery_type": {"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:50.550377Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-01 04:03:53.917292+00	2026-11-04 16:57:02.249928+00	0	f	[]
49c96a5a-776d-4a4b-8e66-a2b84ea3d839	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "9478634a-d980-4211-b0fa-ac0e884de1ad", "started_at": "2026-10-05T14:23:50.058326236Z", "finished_at": "2026-11-04T16:56:48.430144Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:48.077288Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-05 14:23:50.058326+00	2026-11-04 16:56:48.431651+00	0	f	[]
b6687e86-0bec-46d7-91ac-17801dc93179	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "1811938e-d0f9-4568-bc69-ef3f2931e9ef", "started_at": "2026-08-25T01:24:07.586140654Z", "finished_at": "2026-11-04T16:56:49.986132Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Unified", "host_id": "5f0170e1-a84c-4207-b391-c13297a54d48", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:49.188248Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "5f0170e1-a84c-4207-b391-c13297a54d48", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 01:24:07.58614+00	2026-11-04 16:56:49.987601+00	0	f	[]
0a631dec-4e14-47f3-a250-acd1c5ad5f96	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "6fa2c508-5f31-44cd-9153-00c853afe9a7", "started_at": "2026-08-25T02:51:38.519652606Z", "finished_at": "2026-11-04T16:56:49.992301Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:49.175514Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 02:51:38.519652+00	2026-11-04 16:56:49.993719+00	0	f	[]
c1a4b7b7-15af-42fb-80c1-ff4d687419f0	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "5d811057-9d57-4a37-997d-6d9ff0bde3f4", "started_at": "2026-10-05T14:20:20.057223827Z", "finished_at": "2026-11-04T16:56:48.439850Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:48.070240Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-05 14:20:20.057223+00	2026-11-04 16:56:48.441183+00	0	f	[]
84cc05b8-cc99-4ed6-852e-0873e1264561	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "8625c9a3-252c-4570-9b6a-e4459dde2e43", "started_at": "2026-10-05T14:18:58.075801174Z", "finished_at": "2026-11-04T16:56:48.445773Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Unified", "host_id": "38b72a6c-0aea-4147-b599-5af096f07623", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:47.984240Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "38b72a6c-0aea-4147-b599-5af096f07623", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-05 14:18:58.075801+00	2026-11-04 16:56:48.447197+00	0	f	[]
ef8a0f1e-fa05-442b-a156-b49b46915f12	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "c540d032-e83e-4050-ab10-97635f0968a2", "started_at": "2026-10-01T04:07:23.928243114Z", "finished_at": "2026-11-04T16:57:02.251181Z", "discovery_id": null, "daemon_version": "0.19.6", "discovery_type": {"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:50.557023Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-01 04:07:23.928243+00	2026-11-04 16:57:02.252765+00	0	f	[]
5f606998-d0f3-4f95-b110-abe9c1521b10	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "11299ee7-3101-4bae-be7a-097db8407dd7", "started_at": "2026-01-23T03:48:00.568581680Z", "finished_at": "2026-11-04T16:56:49.988734Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:49.630219Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-23 03:48:00.568581+00	2026-11-04 16:56:49.991053+00	0	f	[]
cec54bde-5cad-466e-a278-516979a78841	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	51c5b65c-3dc1-4c23-a049-42eac5bf85de	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "c45ecd9e-e3a2-43fb-931d-25fd1afbc034", "progress": 0, "warnings": [], "daemon_id": "51c5b65c-3dc1-4c23-a049-42eac5bf85de", "session_id": "cbdca757-31a8-4a33-b45a-5ad52b4de60b", "started_at": "2026-10-01T04:02:39.535834801Z", "finished_at": "2026-11-04T16:57:02.253917Z", "discovery_id": null, "daemon_version": "0.19.6", "discovery_type": {"type": "Unified", "host_id": "bd69aa7e-1ffb-4e1d-8ac6-adc468be6771", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-04T16:56:50.460808Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bd69aa7e-1ffb-4e1d-8ac6-adc468be6771", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-01 04:02:39.535834+00	2026-11-04 16:57:02.255433+00	0	f	[]
\.


--
-- Data for Name: discovery_warnings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.discovery_warnings (id, discovery_id, site_id, code, credential_id, host_id, remote_host_id, subnet_id, details, created_at) FROM stdin;
b778cbe8-0f7c-490a-bf74-b743a76d43a3	242dde5c-d8f6-4d73-9a3d-762943697337	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	OutdatedDaemonFormat	\N	\N	\N	\N	{"code": "OutdatedDaemonFormat", "daemon_version": "0.16.0"}	2026-11-04 16:56:17.257218+00
\.


--
-- Data for Name: entity_tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.entity_tags (id, entity_id, entity_type, tag_id, created_at, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
3274144f-631f-4999-b5f8-62fc69c0c946	a8d47c19-9ec6-438f-bce9-915362bc8e11	"Service"	1cbdcb6b-9b2d-4577-8309-16bfaf104741	2026-11-04 16:56:15.458624+00	2026-11-04 16:56:15.458624+00	\N	\N	\N
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
cd5dc797-4d92-45ee-a10e-7f17db4c6422	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	scanopy-daemon-serverpoll	684d9cb23a63	\N	{"type": "Discovery"}	null	2026-11-04 16:50:05.374387+00	2026-11-04 16:50:05.374387+00	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-11-04 16:50:05.374387+00	\N	\N	2026-11-04 16:54:54.316267+00	218b8a01-e8f8-4255-b671-fa6785c3f075	0a67f79e-a0f0-4740-97ab-3721f9bae1ee	\N	\N	\N	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	\N	"Unspecified"	"DaemonSelfReport"	{"name": "Debian", "family": "Linux", "version": "12.0.0", "codename": "bookworm", "kernel_version": "7.0.0-1017-azure"}	"DaemonSelfReport"	\N	\N	"Unspecified"	\N	"Unspecified"	\N
f35cae23-aec3-4e90-8c72-278e1d887c57	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	Scanopy Daemon	8a06deacf59d	\N	{"type": "Discovery"}	null	2026-11-04 16:48:48.639442+00	2026-11-04 16:48:48.639442+00	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-11-04 16:48:48.639442+00	\N	\N	2026-11-04 16:56:34.045063+00	dba0d6aa-70d4-400b-a22b-1872f4020249	dba0d6aa-70d4-400b-a22b-1872f4020249	\N	\N	\N	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"ServiceMatch"	\N	"Unspecified"	"DaemonSelfReport"	{"name": "Debian", "family": "Linux", "version": "12.0.0", "codename": "bookworm", "kernel_version": "7.0.0-1017-azure"}	"DaemonSelfReport"	\N	\N	"Unspecified"	\N	"Unspecified"	\N
bd69aa7e-1ffb-4e1d-8ac6-adc468be6771	c45ecd9e-e3a2-43fb-931d-25fd1afbc034		c01a23384611	Scanopy daemon	{"type": "Discovery"}	null	2026-11-04 16:56:50.477875+00	2026-11-04 16:56:50.477875+00	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-11-04 16:56:50.477875+00	\N	\N	2026-11-04 16:56:50.477875+00	\N	\N	\N	\N	\N	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	\N	"Unspecified"	"DaemonSelfReport"	\N	"Unspecified"	\N	\N	"Unspecified"	\N	"Unspecified"	\N
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
275bec1c-89c4-4411-8e5b-43c805a94d91	bd69aa7e-1ffb-4e1d-8ac6-adc468be6771	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	2026-11-04 16:56:50.477875+00	2026-11-04 16:56:50.477875+00	1	lo	\N	24	\N	1	1	\N	fc3c5432-a2cb-4049-85e6-64b4d71473b3	lo	\N	\N	\N	2026-11-04 16:56:50.477875+00	\N	\N	2026-11-04 16:56:50.477875+00	\N	\N	\N	"Unspecified"	f	"Unspecified"	"Unspecified"	"Unspecified"
9777cbce-1d35-422f-8abb-793f21ff7c13	bd69aa7e-1ffb-4e1d-8ac6-adc468be6771	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	2026-11-04 16:56:50.477875+00	2026-11-04 16:56:50.477875+00	2	eth0	\N	53	\N	1	1	d6:d6:66:fc:6c:c2	ad5a60f7-f8f2-4b78-99f7-a43668ba7558	eth0	\N	\N	\N	2026-11-04 16:56:50.477875+00	\N	\N	2026-11-04 16:56:50.477875+00	\N	\N	\N	"DaemonSelfReport"	f	"Unspecified"	"Unspecified"	"Unspecified"
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
fc3c5432-a2cb-4049-85e6-64b4d71473b3	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	bd69aa7e-1ffb-4e1d-8ac6-adc468be6771	bf720fd4-edfb-4341-b9d4-97b0f30185d3	127.0.0.1	\N	lo	0	2026-11-04 16:56:50.477875+00	2026-11-04 16:56:50.477875+00	2026-11-04 16:56:50.477875+00	\N	\N	2026-11-04 16:56:50.477875+00	\N	\N	\N	"Unspecified"	"Unspecified"
ad5a60f7-f8f2-4b78-99f7-a43668ba7558	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	bd69aa7e-1ffb-4e1d-8ac6-adc468be6771	d0b8a284-36a1-4e60-a131-20992f033ee5	172.25.0.4	d6:d6:66:fc:6c:c2	eth0	1	2026-11-04 16:56:50.477875+00	2026-11-04 16:56:50.477875+00	2026-11-04 16:56:50.477875+00	\N	\N	2026-11-04 16:56:50.477875+00	\N	\N	\N	"DaemonSelfReport"	"Unspecified"
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
b708babc-d623-484f-8d9f-67f46c1b72d1	My Organization	\N	{"rate": "Month", "type": "Community", "base_cents": 0, "host_cents": null, "seat_cents": null, "site_cents": null, "trial_days": 0, "included_orgs": null, "included_hosts": null, "included_seats": null, "included_sites": null}	active	2026-11-04 16:48:48.454068+00	2026-11-04 16:48:48.454068+00	["OrgCreated", "OnboardingModalCompleted", "FirstDaemonRegistered", "FirstHostDiscovered", "FirstDiscoveryCompleted", "FirstTagCreated", "FirstDependencyCreated", "FirstUserApiKeyCreated", "SecondSiteCreated"]	f	\N	other	\N	f	\N	null	\N	\N	\N	\N	{"hosts": "None", "seats": "None", "sites": "None", "license_over_instance_limit": false, "license_silence_reported_for": null, "airgap_expiry_notified_through": null, "sunset_notified_last_unsupported": null}	\N	\N	0	\N	\N	\N	\N	f	\N
\.


--
-- Data for Name: ports; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ports (id, site_id, host_id, port_number, protocol, port_type, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id, port_source) FROM stdin;
c58a15f4-b1c5-4d91-83d6-72d9069b6eb7	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	bd69aa7e-1ffb-4e1d-8ac6-adc468be6771	60073	Tcp	Custom	2026-11-04 16:56:50.477875+00	2026-11-04 16:56:50.477875+00	2026-11-04 16:56:50.477875+00	\N	\N	2026-11-04 16:56:50.477875+00	\N	\N	\N	"Unspecified"
\.


--
-- Data for Name: release_announcements; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.release_announcements (id, version, email_sent_at, created_at, updated_at) FROM stdin;
2372e7d8-9977-42e5-b1e6-bc6497874fd9	0.19.6	\N	2026-11-04 16:48:45.447764+00	2026-11-04 16:48:45.447764+00
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
9ff69b61-4ac2-4522-82fb-85cd0b229403	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	2026-11-04 16:56:50.477875+00	2026-11-04 16:56:50.477875+00	Scanopy Daemon	bd69aa7e-1ffb-4e1d-8ac6-adc468be6771	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}}	0	2026-11-04 16:56:50.477875+00	\N	\N	2026-11-04 16:56:50.477875+00	\N	\N	\N	\N
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
9aea999a-8408-4ecd-aa17-18bc3c381fab	b708babc-d623-484f-8d9f-67f46c1b72d1	d06bee5b-4fed-42b4-b27a-1abd1baca9ff	\N	\N	Admin	\N	2026-11-04 16:57:04.929752+00
9610facf-1897-451d-9c31-6e4aa23dbc0e	b708babc-d623-484f-8d9f-67f46c1b72d1	8780e358-9c95-4a15-a3a4-9f3e0f92c5e1	\N	\N	Admin	\N	2026-11-04 16:57:04.929752+00
\.


--
-- Data for Name: site_credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.site_credentials (site_id, credential_id) FROM stdin;
c45ecd9e-e3a2-43fb-931d-25fd1afbc034	5619c59f-06d2-44b0-8195-474b377f1da5
\.


--
-- Data for Name: sites; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.sites (id, name, created_at, updated_at, organization_id, stale_after_hours) FROM stdin;
c45ecd9e-e3a2-43fb-931d-25fd1afbc034	My Site	2026-11-04 16:48:48.499938+00	2026-11-04 16:48:48.499938+00	b708babc-d623-484f-8d9f-67f46c1b72d1	\N
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
732673f7-0dda-4954-aded-258db05064df	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	2026-11-04 16:57:04.192834+00	2026-11-04 16:57:04.192834+00	"10.1.0.0/24"	Blocked Subnet	\N	Lan	{"type": "Manual"}	2026-11-04 16:57:04.192834+00	\N	\N	2026-11-04 16:57:04.192834+00	\N	\N	\N	\N	"Manual"	"Manual"	"Manual"
d0b8a284-36a1-4e60-a131-20992f033ee5	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	2026-10-01 04:02:39.48952+00	2026-10-01 04:02:39.48952+00	"172.25.0.0/28"	172.25.0.0/28	\N	Lan	{"type": "Discovery"}	2026-10-01 04:02:39.48952+00	\N	\N	2026-11-04 16:57:02.226333+00	\N	\N	\N	\N	"DaemonSelfReport"	"Unspecified"	"InterfaceNameMatch"
bf720fd4-edfb-4341-b9d4-97b0f30185d3	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	2026-10-01 04:02:39.48948+00	2026-10-01 04:02:39.48948+00	"127.0.0.0/8"	127.0.0.0/8	\N	Loopback	{"type": "Discovery"}	2026-10-01 04:02:39.48948+00	\N	\N	2026-11-04 16:57:02.2263+00	\N	\N	\N	\N	"DaemonSelfReport"	"Unspecified"	"InterfaceNameMatch"
\.


--
-- Data for Name: tag_groups; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tag_groups (id, organization_id, name, description, group_type, entity_types, ordered, created_at, updated_at) FROM stdin;
7af4a6bd-aa6b-47c3-b9f2-250d6ac02858	b708babc-d623-484f-8d9f-67f46c1b72d1	Application	What each entity is part of. A service inherits its host's application tag, and the Application view groups by it.	Application	\N	f	2026-11-04 16:48:48.485249+00	2026-11-04 16:48:48.485249+00
07bb405c-a31d-425b-989a-3cf349742520	b708babc-d623-484f-8d9f-67f46c1b72d1	Status	Where an asset is in its lifecycle.	Status	\N	t	2026-11-04 16:48:48.488461+00	2026-11-04 16:48:48.488461+00
76886701-27fb-44e5-a961-6eb731fad07d	b708babc-d623-484f-8d9f-67f46c1b72d1	Visibility	Entities holding a tag of this group are left out of every topology.	Visibility	\N	f	2026-11-04 16:48:48.490162+00	2026-11-04 16:48:48.490162+00
fd16ea8f-daf6-4186-80cd-7867e6225d22	b708babc-d623-484f-8d9f-67f46c1b72d1	Environment	Which stage of delivery an asset serves.	Custom	\N	t	2026-11-04 16:48:48.577737+00	2026-11-04 16:48:48.577737+00
fd1dd872-1aed-45cc-8445-80ddcac4458d	b708babc-d623-484f-8d9f-67f46c1b72d1	Criticality	How much an outage of the asset costs.	Custom	\N	t	2026-11-04 16:48:48.57898+00	2026-11-04 16:48:48.57898+00
7e8810ae-9dbe-437e-888f-1e402ef5f0ba	b708babc-d623-484f-8d9f-67f46c1b72d1	Management	Who administers the asset.	Custom	\N	t	2026-11-04 16:48:48.579902+00	2026-11-04 16:48:48.579902+00
\.


--
-- Data for Name: tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tags (id, organization_id, name, description, created_at, updated_at, color, valid_from, valid_to, lineage_id, snapshot_id, icon, tag_group_id, "position") FROM stdin;
542d2c77-abf2-4a4f-aa1a-43e4031d908c	b708babc-d623-484f-8d9f-67f46c1b72d1	Hidden	Left out of topology.	2026-11-04 16:48:48.49189+00	2026-11-04 16:48:48.49189+00	Gray	2026-11-04 16:48:48.49189+00	\N	\N	\N	eye-off	76886701-27fb-44e5-a961-6eb731fad07d	\N
1a6c5bb1-250c-49b8-8d4d-e8ac9928e755	b708babc-d623-484f-8d9f-67f46c1b72d1	Planned	Ordered or scheduled, not yet in service	2026-11-04 16:48:48.584753+00	2026-11-04 16:48:48.584753+00	Blue	2026-11-04 16:48:48.584753+00	\N	\N	\N	calendar-clock	07bb405c-a31d-425b-989a-3cf349742520	1
0f3736e6-bf34-432d-8b66-ba8430a855a6	b708babc-d623-484f-8d9f-67f46c1b72d1	Active	In service	2026-11-04 16:48:48.584767+00	2026-11-04 16:48:48.584767+00	Green	2026-11-04 16:48:48.584767+00	\N	\N	\N	circle-check	07bb405c-a31d-425b-989a-3cf349742520	2
6aa31af9-10a9-409f-9eb5-ba6f8156063e	b708babc-d623-484f-8d9f-67f46c1b72d1	Decommissioning	Being retired; still on the network	2026-11-04 16:48:48.584768+00	2026-11-04 16:48:48.584768+00	Orange	2026-11-04 16:48:48.584768+00	\N	\N	\N	hourglass	07bb405c-a31d-425b-989a-3cf349742520	3
4eae36d1-53f3-4ca2-a93a-240f7caa71de	b708babc-d623-484f-8d9f-67f46c1b72d1	Decommissioned	Retired from service; kept for the record	2026-11-04 16:48:48.584769+00	2026-11-04 16:48:48.584769+00	Gray	2026-11-04 16:48:48.584769+00	\N	\N	\N	archive	07bb405c-a31d-425b-989a-3cf349742520	4
33e79cdd-8655-4de8-bd71-bc01b1c88e9a	b708babc-d623-484f-8d9f-67f46c1b72d1	Production	Serves live users and business traffic	2026-11-04 16:48:48.584775+00	2026-11-04 16:48:48.584775+00	Red	2026-11-04 16:48:48.584775+00	\N	\N	\N	server	fd16ea8f-daf6-4186-80cd-7867e6225d22	1
52f40db1-f77d-4baa-8a0b-e281ed7bb6d2	b708babc-d623-484f-8d9f-67f46c1b72d1	Staging	A pre-release copy of production for final testing	2026-11-04 16:48:48.584776+00	2026-11-04 16:48:48.584776+00	Amber	2026-11-04 16:48:48.584776+00	\N	\N	\N	flask-conical	fd16ea8f-daf6-4186-80cd-7867e6225d22	2
8811a4ac-8558-4e27-bc5e-2ab60c6a4c3d	b708babc-d623-484f-8d9f-67f46c1b72d1	Development	Used to build and test changes	2026-11-04 16:48:48.584778+00	2026-11-04 16:48:48.584778+00	Blue	2026-11-04 16:48:48.584778+00	\N	\N	\N	code	fd16ea8f-daf6-4186-80cd-7867e6225d22	3
a7193d68-5560-4fbd-80ed-42c6672367d9	b708babc-d623-484f-8d9f-67f46c1b72d1	Lab	Test bench or experimental equipment	2026-11-04 16:48:48.584779+00	2026-11-04 16:48:48.584779+00	Purple	2026-11-04 16:48:48.584779+00	\N	\N	\N	test-tube	fd16ea8f-daf6-4186-80cd-7867e6225d22	4
2fcd8d6c-e7da-40e2-a883-d389d2e0e2d9	b708babc-d623-484f-8d9f-67f46c1b72d1	Critical	An outage stops the business	2026-11-04 16:48:48.584783+00	2026-11-04 16:48:48.584783+00	Red	2026-11-04 16:48:48.584783+00	\N	\N	\N	siren	fd1dd872-1aed-45cc-8445-80ddcac4458d	1
49bf273b-1954-4e02-a0c6-7cb50aca516e	b708babc-d623-484f-8d9f-67f46c1b72d1	High	An outage disrupts many people	2026-11-04 16:48:48.584785+00	2026-11-04 16:48:48.584785+00	Orange	2026-11-04 16:48:48.584785+00	\N	\N	\N	arrow-up	fd1dd872-1aed-45cc-8445-80ddcac4458d	2
cb5c2bed-9fd6-4d2e-b28a-5129ddae0b2d	b708babc-d623-484f-8d9f-67f46c1b72d1	Medium	An outage disrupts some people	2026-11-04 16:48:48.584786+00	2026-11-04 16:48:48.584786+00	Yellow	2026-11-04 16:48:48.584786+00	\N	\N	\N	minus	fd1dd872-1aed-45cc-8445-80ddcac4458d	3
08db00d9-8014-45bf-9fb6-afa1ffc4a79b	b708babc-d623-484f-8d9f-67f46c1b72d1	Low	An outage has little impact	2026-11-04 16:48:48.584787+00	2026-11-04 16:48:48.584787+00	Gray	2026-11-04 16:48:48.584787+00	\N	\N	\N	arrow-down	fd1dd872-1aed-45cc-8445-80ddcac4458d	4
8578a92f-be50-4c44-9d0f-7cc61376efc0	b708babc-d623-484f-8d9f-67f46c1b72d1	Managed	Administered by your IT team	2026-11-04 16:48:48.584793+00	2026-11-04 16:48:48.584793+00	Green	2026-11-04 16:48:48.584793+00	\N	\N	\N	shield-check	7e8810ae-9dbe-437e-888f-1e402ef5f0ba	1
0c6ed27f-59a1-4bb6-a371-1d39ace70ca7	b708babc-d623-484f-8d9f-67f46c1b72d1	Unmanaged	Administered by no one, such as a personal device	2026-11-04 16:48:48.584794+00	2026-11-04 16:48:48.584794+00	Orange	2026-11-04 16:48:48.584794+00	\N	\N	\N	shield-question-mark	7e8810ae-9dbe-437e-888f-1e402ef5f0ba	2
1da0b83c-c8c5-466a-b94c-0f0483ae975f	b708babc-d623-484f-8d9f-67f46c1b72d1	Third-party	Administered by a vendor or another organization	2026-11-04 16:48:48.584796+00	2026-11-04 16:48:48.584796+00	Blue	2026-11-04 16:48:48.584796+00	\N	\N	\N	handshake	7e8810ae-9dbe-437e-888f-1e402ef5f0ba	3
1cbdcb6b-9b2d-4577-8309-16bfaf104741	b708babc-d623-484f-8d9f-67f46c1b72d1	Integration Test Tag	\N	2026-11-04 16:56:15.427587+00	2026-11-04 16:56:15.427587+00	Yellow	2026-11-04 16:56:15.427587+00	\N	\N	\N	\N	\N	\N
07d8699b-a84d-4139-a9f1-6e070effd95c	b708babc-d623-484f-8d9f-67f46c1b72d1	Test Tag	\N	2026-11-04 16:57:02.983316+00	2026-11-04 16:57:02.983316+00	Yellow	2026-11-04 16:57:02.983316+00	2026-11-04 16:57:03.011318+00	42b02a55-38b1-4fdc-a896-a4113569cb32	\N	\N	\N	\N
42b02a55-38b1-4fdc-a896-a4113569cb32	b708babc-d623-484f-8d9f-67f46c1b72d1	Updated Tag	\N	2026-11-04 16:57:02.983316+00	2026-11-04 16:57:02.983316+00	Yellow	2026-11-04 16:57:03.011318+00	2026-11-04 16:57:03.032591+00	\N	\N	\N	\N	\N
\.


--
-- Data for Name: topologies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.topologies (id, site_id, options, created_at, updated_at) FROM stdin;
55500dba-23ce-470b-a696-33b76b273979	c45ecd9e-e3a2-43fb-931d-25fd1afbc034	{"local": {"tag_filter": {}, "bundle_edges": true, "show_minimap": true, "no_fade_edges": false, "hide_edge_types": {"L3Logical": ["Hypervisor", "SameContainer"], "Workloads": ["RequestPath", "HubAndSpoke", "PhysicalLink", "NeighborLink"], "L2Physical": ["SameHost"], "Application": ["ContainerRuntime"]}}, "request": {"element_sort": {}, "element_rules": [{"id": "85c59d21-f862-4b09-8d2e-23e1ed9ea836", "rule": "ByTrunkPort"}, {"id": "840533b4-ee50-4809-b084-2d9fc8a2070c", "rule": "ByVLAN"}, {"id": "6b5601b7-d06e-4bc7-972c-2e4f8f3f745f", "rule": "ByPortOpStatus"}, {"id": "2b72f8e9-cb7d-469d-a8dd-19eb553927ba", "rule": {"ByServiceCategory": {"title": "Infrastructure", "categories": ["NetworkCore", "NetworkAccess", "NetworkAppliance", "RemoteAccess", "Workstation", "Mobile", "IoT", "Printer", "OpenPorts", "NetworkIdentities"], "is_infra_rule": true}}}, {"id": "3569de9f-7a40-4b66-a614-907ffe787052", "rule": {"ByTag": {"title": null, "tag_ids": []}}}, {"id": "b5013016-c973-420a-9d66-5ea19d0625b5", "rule": "ByDeviceType"}, {"id": "feb37aec-fe11-43d8-8610-be2bbf10cf65", "rule": "ByHypervisor"}, {"id": "3dc66a9e-4ed8-4564-8f64-b714101034e0", "rule": "ByContainerRuntime"}, {"id": "1d9a5cd6-50f6-4b85-ae64-382f98309056", "rule": "ByStack"}], "hide_entities": {}, "container_rules": {"L3Logical": [{"id": "abfbb62d-9475-436a-bf06-2fe807dfb2e5", "rule": "BySubnet"}, {"id": "a9131d50-024c-484f-ac58-e47b20890f35", "rule": "MergeContainerBridges"}], "Workloads": [{"id": "b7844a71-be69-46d0-8904-3e5fe2eca2dc", "rule": "ByHost"}], "L2Physical": [{"id": "b7844a71-be69-46d0-8904-3e5fe2eca2dc", "rule": "ByHost"}], "Application": [{"id": "18f1a273-6c93-423a-b291-5723b61db6ad", "rule": {"ByApplication": {"tag_ids": []}}}]}, "hide_metadata_values": {"L3Logical": {"Service": {"Category": ["OpenPorts"]}}, "Workloads": {"Service": {"Category": ["OpenPorts"]}}, "L2Physical": {"Service": {"Category": ["OpenPorts"]}, "Interface": {"LinkState": ["Unlinked"]}}, "Application": {"Service": {"Category": ["OpenPorts"]}}}}}	2026-11-04 16:48:48.52078+00	2026-11-04 16:48:48.52078+00
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
d06bee5b-4fed-42b4-b27a-1abd1baca9ff	2026-11-04 16:48:48.455602+00	2026-11-04 16:48:48.455602+00	$argon2id$v=19$m=19456,t=2,p=1$xqouZgk+REuVCgNAwgTttg$KS9PPqevamAppSEWF+b+anw0mHHOGftz2iCXz9wL1QE	\N	\N	\N	user@gmail.com	b708babc-d623-484f-8d9f-67f46c1b72d1	Owner	{}	\N	t	\N	\N	\N	\N	\N	{"daemon_alerts": true, "product_updates": true, "trial_and_usage": true, "discovery_digest": true, "product_onboarding": true}	0	{"clock": "browser_default", "time_zone": null, "date_order": "browser_default", "timestamps": "relative", "week_start": "monday", "table_density": "compact"}	\N	0.19.6	\N
8780e358-9c95-4a15-a3a4-9f3e0f92c5e1	2026-11-04 16:57:03.839766+00	2026-11-04 16:57:03.839766+00	\N	\N	\N	\N	user@example.com	b708babc-d623-484f-8d9f-67f46c1b72d1	Owner	{}	\N	f	\N	\N	\N	\N	\N	{"daemon_alerts": true, "product_updates": true, "trial_and_usage": true, "discovery_digest": true, "product_onboarding": true}	0	{"clock": "browser_default", "time_zone": null, "date_order": "browser_default", "timestamps": "relative", "week_start": "monday", "table_density": "compact"}	\N	\N	\N
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
8frG-DzdhY-l4aWASeQX8A	\\x93c410f017e44980a5e1a58f85dd3cf8c6faf182ad73657373696f6e5f65706f636800a7757365725f6964d92464303662656535622d346665642d343262342d623237612d31616264316261636139666699cd07eacd013b103030ce1c2d29e8000000	2026-11-11 16:48:48.472721+00
7focXmkM4gqyDDopLMoJ2Q	\\x93c410d909ca2c293a0cb20ae20c695e1cfaed83ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92461313264636161642d643233612d343734382d613038382d336536303831313162346433a87573655f63617365a56f74686572a7757365725f6964d92464303662656535622d346665642d343262342d623237612d316162643162616361396666ad73657373696f6e5f65706f63680099cd07eacd013b103810ce0d824156000000	2026-11-11 16:56:16.22664+00
7px9Kup_8q4fMYMsqXLkHg	\\x93c4101ee472a92c83311faef27fea2a7d9cee83ad73657373696f6e5f65706f636800ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92463306466663132322d306462392d343766352d613061662d396163333435656562643335a87573655f63617365a56f74686572a7757365725f6964d92464303662656535622d346665642d343262342d623237612d31616264316261636139666699cd07eacd013b103832ce392fe866000000	2026-11-11 16:56:50.95944+00
DIK158RAYNgKabGa55GuCQ	\\x93c41009ae91e79ab1690ad86040c4e7b5820c83ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92466313837613931652d323133642d343463322d623936352d636465643939386539663361a87573655f63617365a56f74686572ad73657373696f6e5f65706f636800a7757365725f6964d92464303662656535622d346665642d343262342d623237612d31616264316261636139666699cd07eacd013b103902ce2660d3ef000000	2026-11-11 16:57:02.643879+00
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

\unrestrict G3wKpFQsW5syLKLkic0HnJhgEKkRF2DPNxAQQG5AbvSz7JF7tfLMv205qMcW7Vb

