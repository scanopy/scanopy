--
-- PostgreSQL database dump
--

\restrict fCIJ7e6JdTRNWkFwQyWycsIfSoCOurx7PPEzhYzsPH0TYyaqx1aZrIQqc0At4vU

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
20251006215000	users	2026-11-14 15:18:17.774283+00	t	\\x4f13ce14ff67ef0b7145987c7b22b588745bf9fbb7b673450c26a0f2f9a36ef8ca980e456c8d77cfb1b2d7a4577a64d7	-1
20251006215100	networks	2026-11-14 15:18:17.781244+00	t	\\xeaa5a07a262709f64f0c59f31e25519580c79e2d1a523ce72736848946a34b17dd9adc7498eaf90551af6b7ec6d4e0e3	-1
20251006215151	create_hosts	2026-11-14 15:18:17.786823+00	t	\\x6ec7487074c0724932d21df4cf1ed66645313cf62c159a7179e39cbc261bcb81a24f7933a0e3cf58504f2a90fc5c1962	-1
20251006215155	create_subnets	2026-11-14 15:18:17.792152+00	t	\\xefb5b25742bd5f4489b67351d9f2494a95f307428c911fd8c5f475bfb03926347bdc269bbd048d2ddb06336945b27926	-1
20251006215201	create_groups	2026-11-14 15:18:17.797812+00	t	\\x0a7032bf4d33a0baf020e905da865cde240e2a09dda2f62aa535b2c5d4b26b20be30a3286f1b5192bd94cd4a5dbb5bcd	-1
20251006215204	create_daemons	2026-11-14 15:18:17.803814+00	t	\\xcfea93403b1f9cf9aac374711d4ac72d8a223e3c38a1d2a06d9edb5f94e8a557debac3668271f8176368eadc5105349f	-1
20251006215212	create_services	2026-11-14 15:18:17.810822+00	t	\\xd5b07f82fc7c9da2782a364d46078d7d16b5c08df70cfbf02edcfe9b1b24ab6024ad159292aeea455f15cfd1f4740c1d	-1
20251029193448	user-auth	2026-11-14 15:18:17.818752+00	t	\\xfde8161a8db89d51eeade7517d90a41d560f19645620f2298f78f116219a09728b18e91251ae31e46a47f6942d5a9032	-1
20251030044828	daemon_api	2026-11-14 15:18:17.821138+00	t	\\x181eb3541f51ef5b038b2064660370775d1b364547a214a20dde9c9d4bb95a1c273cd4525ef29e61fa65a3eb4fee0400	-1
20251030170438	host-hide	2026-11-14 15:18:17.823051+00	t	\\x87c6fda7f8456bf610a78e8e98803158caa0e12857c5bab466a5bb0004d41b449004a68e728ca13f17e051f662a15454	-1
20251102224919	create_discovery	2026-11-14 15:18:17.837456+00	t	\\xb32a04abb891aba48f92a059fae7341442355ca8e4af5d109e28e2a4f79ee8e11b2a8f40453b7f6725c2dd6487f26573	-1
20251106235621	normalize-daemon-cols	2026-11-14 15:18:17.840273+00	t	\\x5b137118d506e2708097c432358bf909265b3cf3bacd662b02e2c81ba589a9e0100631c7801cffd9c57bb10a6674fb3b	-1
20251107034459	api_keys	2026-11-14 15:18:17.851607+00	t	\\x3133ec043c0c6e25b6e55f7da84cae52b2a72488116938a2c669c8512c2efe72a74029912bcba1f2a2a0a8b59ef01dde	-1
20251107222650	oidc-auth	2026-11-14 15:18:17.887666+00	t	\\xd349750e0298718cbcd98eaff6e152b3fb45c3d9d62d06eedeb26c75452e9ce1af65c3e52c9f2de4bd532939c2f31096	-1
20251110181948	orgs-billing	2026-11-14 15:18:17.905858+00	t	\\x5bbea7a2dfc9d00213bd66b473289ddd66694eff8a4f3eaab937c985b64c5f8c3ad2d64e960afbb03f335ac6766687aa	-1
20251113223656	group-enhancements	2026-11-14 15:18:17.907886+00	t	\\xbe0699486d85df2bd3edc1f0bf4f1f096d5b6c5070361702c4d203ec2bb640811be88bb1979cfe51b40805ad84d1de65	-1
20251117032720	daemon-mode	2026-11-14 15:18:17.909778+00	t	\\xdd0d899c24b73d70e9970e54b2c748d6b6b55c856ca0f8590fe990da49cc46c700b1ce13f57ff65abd6711f4bd8a6481	-1
20251118143058	set-default-plan	2026-11-14 15:18:17.911827+00	t	\\xd19142607aef84aac7cfb97d60d29bda764d26f513f2c72306734c03cec2651d23eee3ce6cacfd36ca52dbddc462f917	-1
20251118225043	save-topology	2026-11-14 15:18:17.923931+00	t	\\x011a594740c69d8d0f8b0149d49d1b53cfbf948b7866ebd84403394139cb66a44277803462846b06e762577adc3e61a3	-1
20251123232748	network-permissions	2026-11-14 15:18:17.928098+00	t	\\x161be7ae5721c06523d6488606f1a7b1f096193efa1183ecdd1c2c9a4a9f4cad4884e939018917314aaf261d9a3f97ae	-1
20251125001342	billing-updates	2026-11-14 15:18:17.929712+00	t	\\xa235d153d95aeb676e3310a52ccb69dfbd7ca36bba975d5bbca165ceeec7196da12119f23597ea5276c364f90f23db1e	-1
20251128035448	org-onboarding-status	2026-11-14 15:18:17.932149+00	t	\\x1d7a7e9bf23b5078250f31934d1bc47bbaf463ace887e7746af30946e843de41badfc2b213ed64912a18e07b297663d8	-1
20251129180942	nfs-consolidate	2026-11-14 15:18:17.934077+00	t	\\xb38f41d30699a475c2b967f8e43156f3b49bb10341bddbde01d9fb5ba805f6724685e27e53f7e49b6c8b59e29c74f98e	-1
20251206052641	discovery-progress	2026-11-14 15:18:17.936442+00	t	\\x9d433b7b8c58d0d5437a104497e5e214febb2d1441a3ad7c28512e7497ed14fb9458e0d4ff786962a59954cb30da1447	-1
20251206202200	plan-fix	2026-11-14 15:18:17.937932+00	t	\\x242f6699dbf485cf59a8d1b8cd9d7c43aeef635a9316be815a47e15238c5e4af88efaa0daf885be03572948dc0c9edac	-1
20251207061341	daemon-url	2026-11-14 15:18:17.941466+00	t	\\x01172455c4f2d0d57371d18ef66d2ab3b7a8525067ef8a86945c616982e6ce06f5ea1e1560a8f20dadcd5be2223e6df1	-1
20251210045929	tags	2026-11-14 15:18:17.95417+00	t	\\xe3dde83d39f8552b5afcdc1493cddfeffe077751bf55472032bc8b35fc8fc2a2caa3b55b4c2354ace7de03c3977982db	-1
20251210175035	terms	2026-11-14 15:18:17.955789+00	t	\\xe47f0cf7aba1bffa10798bede953da69fd4bfaebf9c75c76226507c558a3595c6bfc6ac8920d11398dbdf3b762769992	-1
20251213025048	hash-keys	2026-11-14 15:18:17.966342+00	t	\\xfc7cbb8ce61f0c225322297f7459dcbe362242b9001c06cb874b7f739cea7ae888d8f0cfaed6623bcbcb9ec54c8cd18b	-1
20251214050638	scanopy	2026-11-14 15:18:17.968654+00	t	\\x0108bb39832305f024126211710689adc48d973ff66e5e59ff49468389b75c1ff95d1fbbb7bdb50e33ec1333a1f29ea6	-1
20251215215724	topo-scanopy-fix	2026-11-14 15:18:17.969881+00	t	\\xed88a4b71b3c9b61d46322b5053362e5a25a9293cd3c420c9df9fcaeb3441254122b8a18f58c297f535c842b8a8b0a38	-1
20251217153736	category_rename	2026-11-14 15:18:17.97271+00	t	\\x03af7ec905e11a77e25038a3c272645da96014da7c50c585a25cea3f9a7579faba3ff45114a5e589d144c9550ba42421	-1
20251218053111	invite-persistence	2026-11-14 15:18:17.980007+00	t	\\x21d12f48b964acfd600f88e70ceb14abd9cf2a8a10db2eae2a6d8f44cf7d20749f93293631e6123e92b7c3c1793877c2	-1
20251219211216	create_shares	2026-11-14 15:18:17.989346+00	t	\\x036485debd3536f9e58ead728f461b925585911acf565970bf3b2ab295b12a2865606d6a56d334c5641dcd42adeb3d68	-1
20251220170928	permissions-cleanup	2026-11-14 15:18:17.991475+00	t	\\x632f7b6702b494301e0d36fd3b900686b1a7f9936aef8c084b5880f1152b8256a125566e2b5ac40216eaadd3c4c64a03	-1
20251220180000	commercial-to-community	2026-11-14 15:18:17.992776+00	t	\\x26fc298486c225f2f01271d611418377c403183ae51daf32fef104ec07c027f2017d138910c4fbfb5f49819a5f4194d6	-1
20251221010000	cleanup_subnet_type	2026-11-14 15:18:17.994123+00	t	\\xb521121f3fd3a10c0de816977ac2a2ffb6118f34f8474ffb9058722abc0dc4cf5cbec83bc6ee49e79a68e6b715087f40	-1
20251221020000	remove_host_target	2026-11-14 15:18:17.995828+00	t	\\x77b5f8872705676ca81a5704bd1eaee90b9a52b404bdaa27a23da2ffd4858d3e131680926a5a00ad2a0d7a24ba229046	-1
20251221030000	user_network_access	2026-11-14 15:18:18.005081+00	t	\\x5c23f5bb6b0b8ca699a17eee6730c4197a006ca21fecc79136a5e5697b9211a81b4cd08ceda70dace6a26408d021ff3a	-1
20251221040000	interfaces_table	2026-11-14 15:18:18.018648+00	t	\\xf7977b6f1e7e5108c614397d03a38c9bd9243fdc422575ec29610366a0c88f443de2132185878d8e291f06a50a8c3244	-1
20251221050000	ports_table	2026-11-14 15:18:18.030859+00	t	\\xdf72f9306b405be7be62c39003ef38408115e740b120f24e8c78b8e136574fff7965c52023b3bc476899613fa5f4fe35	-1
20251221060000	bindings_table	2026-11-14 15:18:18.045721+00	t	\\x933648a724bd179c7f47305e4080db85342d48712cde39374f0f88cde9d7eba8fe5fafba360937331e2a8178dec420c4	-1
20251221070000	group_bindings	2026-11-14 15:18:18.054433+00	t	\\x697475802f6c42e38deee6596f4ba786b09f7b7cd91742fbc5696dd0f9b3ddfce90dd905153f2b1a9e82f959f5a88302	-1
20251222020000	tag_cascade_delete	2026-11-14 15:18:18.056596+00	t	\\xabfb48c0da8522f5c8ea6d482eb5a5f4562ed41f6160a5915f0fd477c7dd0517aa84760ef99ab3a5db3e0f21b0c69b5f	-1
20251223232524	network_remove_default	2026-11-14 15:18:18.058449+00	t	\\x7099fe4e52405e46269d7ce364050da930b481e72484ad3c4772fd2911d2d505476d659fa9f400c63bc287512d033e18	-1
20251225100000	color_enum	2026-11-14 15:18:18.060518+00	t	\\x62cecd9d79a49835a3bea68a7959ab62aa0c1aaa7e2940dec6a7f8a714362df3649f0c1f9313672d9268295ed5a1cfa9	-1
20251227010000	topology_snapshot_migration	2026-11-14 15:18:18.067431+00	t	\\xc042591d254869c0e79c8b52a9ede680fd26f094e2c385f5f017e115f5e3f31ad155f4885d095344f2642ebb70755d54	-1
20251228010000	user_api_keys	2026-11-14 15:18:18.08317+00	t	\\xa41adb558a5b9d94a4e17af3f16839b83f7da072dbeac9251b12d8a84c7bec6df008009acf246468712a975bb36bb5f5	-1
20251230160000	daemon_version_and_maintainer	2026-11-14 15:18:18.087159+00	t	\\xafed3d9f00adb8c1b0896fb663af801926c218472a0a197f90ecdaa13305a78846a9e15af0043ec010328ba533fca68f	-1
20260103000000	service_position	2026-11-14 15:18:18.089969+00	t	\\x19d00e8c8b300d1c74d721931f4d771ec7bc4e06db0d6a78126e00785586fdc4bcff5b832eeae2fce0cb8d01e12a7fb5	-1
20260106000000	interface_mac_index	2026-11-14 15:18:18.092583+00	t	\\xa26248372a1e31af46a9c6fbdaef178982229e2ceeb90cc6a289d5764f87a38747294b3adf5f21276b5d171e42bdb6ac	-1
20260106204402	entity_tags_junction	2026-11-14 15:18:18.109028+00	t	\\xf73c604f9f0b8db065d990a861684b0dbd62c3ef9bead120c68431c933774de56491a53f021e79f09801680152f5a08a	-1
20260108033856	fix_entity_tags_json_format	2026-11-14 15:18:18.111304+00	t	\\x197eaa063d4f96dd0e897ad8fd96cc1ba9a54dda40a93a5c12eac14597e4dea4c806dd0a527736fb5807b7a8870d9916	-1
20260110000000	email_verification	2026-11-14 15:18:18.115984+00	t	\\xb8da8433f58ba4ce846b9fa0c2551795747a8473ad10266b19685504847458ea69d27a0ce430151cfb426f5f5fb6ac3a	-1
20260114145808	daemon_user_fk_set_null	2026-11-14 15:18:18.117779+00	t	\\x57b060be9fc314d7c5851c75661ca8269118feea6cf7ee9c61b147a0e117c4d39642cf0d1acdf7a723a9a76066c1b8ff	-1
20260116010000	snmp_credentials	2026-11-14 15:18:18.126597+00	t	\\x6f3971cf194d56883c61fa795406a8ab568307ed86544920d098b32a6a1ebb7effcb5ec38a70fdc9b617eff92d63d51e	-1
20260116020000	host_snmp_fields	2026-11-14 15:18:18.131931+00	t	\\xf2f088c13ab0dd34e1cb1e5327b0b4137440b0146e5ce1e78b8d2dfa05d9b5a12a328eeb807988453a8a43ad8a1c95ba	-1
20260116030000	if_entries	2026-11-14 15:18:18.149496+00	t	\\xa58391708f8b21901ab9250af528f638a6055462f70ffddfd7c451433aacdabd62825546fa8be108f23a3cae78b8ae28	-1
20260116100000	daemon_api_key_link	2026-11-14 15:18:18.153877+00	t	\\x41088aa314ab173344a6b416280721806b2f296a32a8d8cae58c7e5717f389fe599134ed03980ed97e4b7659e99c4f82	-1
20260131190000	add_hubspot_company_id	2026-11-14 15:18:18.155493+00	t	\\x4326f95f4954e176157c1c3e034074a3e5c44da4d60bbd7a9e4b6238c9ef52a30f8b38d3c887864b6e4c1163dc062beb	-1
20260201021238	fix_service_acronym_capitalization	2026-11-14 15:18:18.157663+00	t	\\x88b010ac8f0223d880ea6a730f11dc6d27fa5de9d8747de3431e46d59f1dbf2f72ae4a87c2e52c32152549f5c1f96bb2	-1
20260204004436	add_entity_tags_to_topology	2026-11-14 15:18:18.159376+00	t	\\x3eff1a1490e77065ec861ef1b9aad8c55de0170106a42720f7931b3929b179122b16e44390b2652771bf91bba32a7757	-1
20260205120000	billing_overhaul	2026-11-14 15:18:18.162215+00	t	\\xbf850cfa0c40a3c65f574efd15fd55a4b702296203d28077a09d1c22076fee8601f2b78345aef370ab9163657de767ab	-1
20260205183207	rename_hubspot_to_brevo	2026-11-14 15:18:18.16387+00	t	\\x4678a7d80215e5eafb5e80af0daa20e2868a3b4f2112e88cb1b2b9efc87d63de3fb96c133f359b224c658789ae4b0d13	-1
20260221120000	add_plan_limit_notifications	2026-11-14 15:18:18.165797+00	t	\\xef770dac07e1d80888832f33184dc46c1d3b8185b91c507cb404468d6ad8c29cacf455178801c67aa27b6a626d3ad82d	-1
20260222120000	add_pending_email	2026-11-14 15:18:18.167225+00	t	\\xddd220f7602c44548d56849c0a8d081ecd1da1383374a11e3e227c7d9becb73a49f5e5bb09ed65901c16df4c16e913e5	-1
20260301120000	add_if_name_to_if_entries	2026-11-14 15:18:18.168798+00	t	\\xc9fc0a2b77ecbf0e1d5ab292c4fe162a26113468c878dfd26a3c63d89c0ee1957ca328ecfe25c611867a0e73780f0cb6	-1
20260306002816	cleanup_standby	2026-11-14 15:18:18.170344+00	t	\\x01b0c236a8a4d0d97f0f633b18f8cbdb92b6d72063289989b90a1b7b6b303e65e0557eb09927b2580dcb7e8ee5966c75	-1
20260309120000	add_org_use_case	2026-11-14 15:18:18.171808+00	t	\\xdb8c8a2f0f9416ba3b687fc75453d7c12c50a6f386b4784d21bd6adfc4a4a7556c637c25cf116118402bbd12c0d5aafe	-1
20260313120000	snmp_extended_discovery	2026-11-14 15:18:18.174258+00	t	\\xc4e72539099de1b830d87a169bfbabba4b8fb378a3c4c4a1dfca698adf3e403d750040d784c26d9fa343be2908064c9d	-1
20260315120000	universal_credentials	2026-11-14 15:18:18.198807+00	t	\\x87dc6f39202e81d5555df78a9d056b143f11bd22e6d7f483065f605e242a360902c72c4d5a49717e7fcc24a366bb5ff5	-1
20260315120001	discovery_scan_settings	2026-11-14 15:18:18.200644+00	t	\\xe9da183fdd8e04e574f553f61f6f33efa046cdae38c846c8077b06c5260446fb4aa39da2449bda7f1d8cf3aa9f16e158	-1
20260315120002	backfill_org_created_milestone	2026-11-14 15:18:18.20211+00	t	\\x14f886a19773cd2263d86f88479be460d21f071d5212e3789c5c40b6415c293fc7d06c7b138351cc42108f89a14fe745	-1
20260316120000	fix_jsonb_null_if_entries	2026-11-14 15:18:18.204482+00	t	\\x65c358069710f7f86d6a3e257e658c2f241cc376433c3a0317b0ec9e1876a66f9738cb65c6ab1a5c197fe40d5aa2aa2b	-1
20260319120000	rename_snmp_to_snmpv2c	2026-11-14 15:18:18.206082+00	t	\\xdce5c9461f402e1672607078b2c571f0eb30b51d46f8e9414d8909efb40693f543e49e560cb7d703db274515043aa08e	-1
20260321120000	add_discovery_scan_count	2026-11-14 15:18:18.208959+00	t	\\x6c8201ab453a51632176d534c6604e0818e28a8a4a153e33e254f4dac0f9b67c9db394082cb663ff1b25941229cf96fc	-1
20260329120000	backfill_subnet_virtualization	2026-11-14 15:18:18.211955+00	t	\\xeac50ded27603dbb5e8773604a52143c9fa8654263e7dd12d3d128ce972c2feed84600e36b2e7a79525b58c44d2ad9d3	-1
20260402120000	rename_topology_node_types	2026-11-14 15:18:18.213303+00	t	\\xc4ba06868add823f83ff1948091bdfe17dbdde80bbec6fe2cf8da2b3689aeeebbe9e9de01b1292bff3c98a74d9e6279f	-1
20260403120000	topology_grouping_rules	2026-11-14 15:18:18.215692+00	t	\\x00799da1206d7c3b3c3db90b7d14437cc054ed2d7273020342e562c619a671e008ff4fdf0365170440b392956949e730	-1
20260405120000	rename_groups_to_dependencies	2026-11-14 15:18:18.22695+00	t	\\x9ce895b456366bf6e54316b22cabd2803aa542dd3733fffa680f0a3af5c4c55a612c5ee511371206921869b7f271c35b	-1
20260406120000	add_tag_is_application_group	2026-11-14 15:18:18.228813+00	t	\\xb7a71e5fdd96ca46c9c7577003309050a93bc53ad192ac5df78e7621f3ed64f07fb29b4658f17af55732cf6dfb7958c2	-1
20260406130000	add_vlans	2026-11-14 15:18:18.242939+00	t	\\x5b3e5d10578d90b5175e5718a28d7147a21b99af2fb3e0ed171d20ee8fd8838c290f648dafdd3b72ef60ff487f7f2494	-1
20260409000000	add_vlans_to_topologies	2026-11-14 15:18:18.24497+00	t	\\x5e0b9dc670580ceec3aa6eae005a39f98733fc27dc574b7f3922f4297813facd5d610af953dfec13e09d0b99eceb3865	-1
20260410000000	rename_interfaces_and_if_entries	2026-11-14 15:18:18.254223+00	t	\\x07f54a59869f458f41f45d75f250aee26b20a426f1ec29930606841770194d6aea0e9e6253a6375fbeebcf9b49121224	-1
20260414000000	add_share_enabled_views	2026-11-14 15:18:18.255875+00	t	\\xc56514355a5977c3242e728e7f5a2533e7b4a5cf8a7ce7757e412e51f1ad85e96d65c13ccd96d050be4a07799b9aef57	-1
20260415120000	rename_onboarding_first_group_created	2026-11-14 15:18:18.257178+00	t	\\x2c17035835d3ead105b76d98688c0b7bd328abdaf9f721d70d057c8afdf438819e93da56707deea5b469b81a7b84d5d7	-1
20260417000000	reindex_interfaces_identity	2026-11-14 15:18:18.262463+00	t	\\x10701e13bc3d838e2ec4a856555ebf338173792f220c405996d3c77e7987e9806798ca0328eb6259e4a62b7e05665b25	-1
20260418000000	add_standby_cleared_at_to_daemons	2026-11-14 15:18:18.263915+00	t	\\x547807de451d015a4ce1438796d5b95e2b98043c521015a21239f6778d10a8d3bf7d8b14e278e09aa0105f1935ad4181	-1
20260501000000	add_organization_billing_flags	2026-11-14 15:18:18.267186+00	t	\\x2de34c4af667d4cd8bc263c27f0526a4a2132022e2eb71ae94fd89edebbfb40cda840055a94e89a21925a317cdff285f	-1
20260502000000	scd2_add_columns	2026-11-14 15:18:18.288966+00	t	\\xe78a73574d86320c0de7fcd43682ee3cdd436dc64d1371f98cb67ef0cdf33097df6e8a92c9a26c5702795144103b085e	-1
20260502000001	scd2_add_discovery_fks_not_valid	2026-11-14 15:18:18.305374+00	t	\\xb49114eb6f8d77cb0c5062619da5c829e5bd65898710efb6207d2804be3677a95eebe88d41d21ae8cba21344fcd63f97	-1
20260502000002	scd2_validate_discovery_fks	2026-11-14 15:18:18.318244+00	t	\\x500201842b9f486d1397c0f0f3ee1a36bc101440d4d97fe0f0bfc971672f1f53f7945aed93c34e2e53d2d9cb913419f1	-1
20260502000003	scd2_backfill_with_metadata	2026-11-14 15:18:18.324186+00	t	\\xd299c1dbf7ec284995dc8a5fcf4b264e4f1ff970085a8631e6d00aa0bc6d6708a5e41d515bb9299c22ef128a958e68e9	-1
20260502000004	scd2_partial_unique_indexes	2026-11-14 15:18:18.353634+00	t	\\xbc30e554c37ec3ca72baa7fc634920257909944dc841231a966845fd4c5bc27ae20606c98867934c872e8883bd56efa1	-1
20260502000005	scd2_drop_old_unique_constraints	2026-11-14 15:18:18.35845+00	t	\\x660c0237299796c6e70a21780d529453a2ac8645f7b122a0b8aa4f362ff780de8849f53a66b71b3ea29ecc66877f47ad	-1
20260502000006	scd2_supporting_indexes	2026-11-14 15:18:18.45182+00	t	\\x4394aaf6951c4b7048f66334fb8c550c55d4cdc06beb29eb7f086a717ee9f26ddeb28e73a7b298633ef517f6c0f84ee0	-1
20260502000007	entitysource_metadata_strip	2026-11-14 15:18:18.456403+00	t	\\x1cfd1a00b100ca0b007be6e8b55f67dde412108f299f94b419a80af45b74b75fdead95637dccb13fdd57fed2f1132446	-1
20260502100000	add_user_email_settings	2026-11-14 15:18:18.458393+00	t	\\xe886b44ca9d3d6461d82b573f617391b8b5f22f18b8113a333aa8c275545ec6455060f68ecbfdd17af86fccc9a500d7a	-1
20260502120000	create_snapshots_table	2026-11-14 15:18:18.464021+00	t	\\x18f829b255b8e11e385903e51182f04fb835aa78ef0090865ce7743f747e4aaaa7b1a7b806a377e9b12af6205aed527a	-1
20260502120001	add_snapshot_id_fks	2026-11-14 15:18:18.47849+00	t	\\x0a080ceaa2922301464b30ea7f25e1f0de5493d15424d07a8dd1f2b4bf1880cf2d25f64d5bf1863f705a1557d867afa2	-1
20260502120002	add_snapshot_id_indexes	2026-11-14 15:18:18.509294+00	t	\\xb7514ff0cf637241d21e90b48214a3a3a5076d0142c99aca39479d2a82cf35e1abdbce8ad5663ca8272d5f74004ae5f9	-1
20260502120003	topology_snapshots_backfill	2026-11-14 15:18:18.518736+00	t	\\x5435da11acf8a9fc37e0520a5b2a959fce84b16080b2a5b67adcf249dea07ae533f858c56f87542152cfc271933c1428	-1
20260502120004	drop_legacy_topology_columns	2026-11-14 15:18:18.534792+00	t	\\xfe45e73be308e6267840664a662cd679fb43469a11f06a168b8b8cfc64b6e86eb10d948271b67a0541481dcc60447769	-1
20260627120000	add_discovery_integration_targets	2026-11-14 15:18:18.53707+00	t	\\x9adf9ea4c36868a3b86067214f8550b8e866aff5d3d6c6ab3d71d1a15ea5e4be0b4349db6d15308fefbceca8403f0cbb	-1
20260703120000	daemon_interfaced_subnets_junction	2026-11-14 15:18:18.543313+00	t	\\xf1399bf8104b1d9cff6a7f66c6040829f01dd64614294295f0ecc27412acc19fd6dd20eff66ce9eadf0cf0c0e8c4353e	-1
20260703120001	drop_discovery_pending_credential_ids	2026-11-14 15:18:18.545641+00	t	\\xc2cfe6f450d000ae36149b042ebcfed938c01487d78ca56a125d72bbeca0de0548824a2396761b3dc647dc80e2e43e80	-1
20260705120000	add_user_session_epoch	2026-11-14 15:18:18.547644+00	t	\\xb296babe8e50ecdd1f50770e12d4234b47ed41cdd826c545debf5f94226672cc2d6fd28eef0483a5fdc8837f5d280d88	-1
20260706120000	drop_credentials_target_ips_and_daemons_capabilities	2026-11-14 15:18:18.550367+00	t	\\xc544c88df68334ac0b3ec180da7ba2030837707f9e3cf2b75b4d56012646c24f8f60f094cc00f05c0f446a2a845e5e98	-1
20260716120000	api_keys_daemon_id	2026-11-14 15:18:18.554646+00	t	\\x484c3b2553bcf81c95aabce4be70b1e2048e097529e04a176a0b75c2edf28ecd0892c959bbd8e8166f621a9dc2dba0dc	-1
20260716120001	api_keys_daemon_id_validate	2026-11-14 15:18:18.557034+00	t	\\x723190451e98f9394b2134dccb89f8eeeb15f6a5644fb88cb24e9e0e7a4ebf5a0c455659930ce2d3096e20c004d7da89	-1
20260716120002	api_keys_daemon_id_unique	2026-11-14 15:18:18.560873+00	t	\\xf788be5a414917ec5c8b9040f85acd352ae3f580dab7e8833db5df19b7d2cf3b43b8a021c03a5082da27eca3bb7a07f0	-1
20260722120000	add_networks_stale_after_hours	2026-11-14 15:18:18.562664+00	t	\\x308d9e04ef943560d3e420c59a3c8017064d59bd28cb63f99c4b562124e2fa038630e71405171901d731420bd9de4e2f	-1
20260727120000	rename_plan_limit_notifications_to_notifications	2026-11-14 15:18:18.565308+00	t	\\x2447c735b8ac6885ae4341738294392f367abcc7c0f5e857bb08523dc5d5a8eccf77db5462b531370a0795135757098d	-1
20260728120000	rename_stale_if_entries_constraints	2026-11-14 15:18:18.570262+00	t	\\x577c54966f8c146a6e5101eb8a2f7f006ef7899ba95ec876be10970b3483b622b254d00dc9c9582efb1862404315764b	-1
20260803120000	drop_organizations_plan_limit_notifications	2026-11-14 15:18:18.573773+00	t	\\xc4c9b44a4cce6cf419cffcbbeade3d5eb119b51f321bdcd3c51a1046d8ec8df81d32f6e76c6472eefdcfae9899ef3107	-1
20260803120001	virtualization_service_id_column	2026-11-14 15:18:18.587142+00	t	\\xe07f6fee751430c94141be6d7ba17390f08f60bdcd01a65072708382b27e9543a9f27fb0cc76bb824940794d6ea67553	-1
20260819120000	hosts_name_source	2026-11-14 15:18:18.594168+00	t	\\x6526859d261bf724e6ac3bb2a51c8588764aab15ecdfad964e06e98bb59f05a753ea2950bdce40cbf7f945d157a230aa	-1
20260824120000	interfaces_neighbor_seen_at	2026-11-14 15:18:18.596363+00	t	\\x70a26934dcc110484dc95100cbc96879504a9bc0ae0fb12ca0effe032816e3282e255d90145c4d2c6bfe2eea351cb4c3	-1
20260826120000	subnets_cidr_source	2026-11-14 15:18:18.60034+00	t	\\x1d8c6a9258f1fd07a08a268cddf5f164bb3b8bb0af34d6d167e480d50b649cbdc5a7b89aa419a218266e9aa1b3d3b497	-1
20260827120000	hosts_firmware_revision	2026-11-14 15:18:18.604086+00	t	\\xa9b3a89986c699c70232d37f5909ab5fecd9c4d2c8708a11b2cad4775494a12080c6664964f942b0079a883f3ecd6ea2	-1
20260827130000	interfaces_unknown_port_facts	2026-11-14 15:18:18.606681+00	t	\\xfa35944166a6c60219a07a6792cd3abbaf909ba78e5494482d73d3d10ddacea618845822dc3dc875542772af742ac38d	-1
20260828120000	hosts_attribute_sources	2026-11-14 15:18:18.637821+00	t	\\x29220b8fb99e5ffb55ca71ea3c05f375214dd77d247d4a54dc7855c5654dbfbbbefceafeb5ddf10710c27085f0a866c4	-1
20260828120001	mac_address_sources	2026-11-14 15:18:18.640914+00	t	\\x39ea28fed133ae2c317bd5fad54e4d16f402ee895b75a6290322e75b434f5502aeddc9f6359d7e95625bb0c0d4ca1d4e	-1
20260828130000	hosts_software_revision	2026-11-14 15:18:18.643474+00	t	\\x83cdafd5691f1bc5fc2345a696ec9fd5c75700693f0011c65d352276d806a6f207f335189153ee8e0fa3fb7e60c18234	-1
20260828140000	ip_addresses_network_mac_index	2026-11-14 15:18:18.647262+00	t	\\x838b853bbb63bcfe0d0976f1c7289960e3133563151c47904ddda0ce5a0078bdc4af9ce07de63d3c024d89b831995535	-1
20260903120000	interfaces_if_descr_optional	2026-11-14 15:18:18.649097+00	t	\\x54495748b0271deebaf7cb06e9e4da9241563e83a162e4bfffaa9e571516223565ab239fcc8bb51fac9e37cbb7625757	-1
20260903120001	interface_neighbor_tables	2026-11-14 15:18:18.68512+00	t	\\xa8352a3ed9b845d96229adcb3e3eb8e796d22a8b6aa794bbc0d2ecafcbf76d68aa59a6e7713f134a57af0acceca3865a	-1
20260908120000	interfaces_ip_configured	2026-11-14 15:18:18.687755+00	t	\\x2467c0b912e78e9cac3818ffdec4157356f6b4a3c96e87a0171d88496439943eacbafefc7a4b51a2b68cc08bbbd29656	-1
20261005120001	rename_networks_to_sites_backfill	2026-11-14 15:18:18.75797+00	t	\\x085c1f2e6ec6b548e7f47fb092519f5f8889629e6198b30bd4a91e4e1d6b5e502513efb213c0601abf444d1f39183f81	-1
20261006120000	host_asset_tag	2026-11-14 15:18:18.760729+00	t	\\x1a9ef01df34f62321f6fdc26f4f75d952000cc366b54217a3107132643258e44957033c5f37b3618ff8d895d01eae918	-1
20261006120100	tag_group_and_icon	2026-11-14 15:18:18.762609+00	t	\\x3bad6e7e42b3a23b29a94462ddefba434ba2e7e92d1d9464db16644d40e45b97d01a83716203f5cda27d7bb5623dd436	-1
20261009120000	user_email_unsubscribe_token	2026-11-14 15:18:18.764836+00	t	\\x41aa0b879e77d264d1d6fc17c65bebe7e2d7796b9785ad79a85b928e23cc93c3cb8a0532ecbc8d72e34fc6e202d25ce1	-1
20261009120001	user_email_unsubscribe_token_index	2026-11-14 15:18:18.768388+00	t	\\x3cdfe6e7f6fe847ff4f18c7ee1d2430ee9833e48c6aa4eb3585c873529bb007b8786c26e6bad1c193ee8cf0dc0b23f0c	-1
20261009120100	tag_groups	2026-11-14 15:18:18.808896+00	t	\\xbdae7fe6cd73793312edbf2c987d3425f1e2c88c9fa98717a9e21550503734aaf67b3a1b804e145269962cd361aaa16a	-1
20261011120000	backfill_interface_credential_milestone	2026-11-14 15:18:18.811004+00	t	\\xfcad57a2a59537e0c9033a6ff906abf7115789554bc06c00c0caa388c220a6a5d3dc880c7aec0df9c6f561b3b94d7477	-1
20261011130000	release_notes	2026-11-14 15:18:18.816192+00	t	\\x46e5e77f7da9f2a01cb042045c01461eeb40b09477a5d2a40f302582b17e2b02e40f6e9dbe40ed1b24e667227ba3a2b3	-1
20261011130001	merge_and_seed_default_credentials	2026-11-14 15:18:18.840113+00	t	\\xc5fc006e4c216dd10154418fd2d52cf3e58b70039498b21a2fc74a1b3e8d9ce34af25da2b6bc197d0c7dff4f9ca910eb	-1
20261012120000	collection_sources	2026-11-14 15:18:18.845993+00	t	\\xfa0a6238fae38ae0c45b9657d540271cf8201a4981d6a16d84c856e8ba192e312b22472db6e2f734e1cd0a56eae723de	-1
20261029120000	unconfirmed_entities	2026-11-14 15:18:18.932837+00	t	\\xb2882cb53e9225fec0318c7ed531021d6aef20dda71386d972c97095771a885b6ff0edea3a785c0b60716fa88af31a28	-1
20261030120000	site_assignments	2026-11-14 15:18:18.962758+00	t	\\xdda847f8830d4d1b6e54d872e8ab95392fe2dcb0628e5ff26d8a6fd90d13c93af9682c8afe8011b1b34d304db68afdae	-1
20261031120000	hypervisor_guest_warnings_and_ssh_logins	2026-11-14 15:18:18.973156+00	t	\\xc2f3e4ab34a0fe5834edc0e0a6df3e67d5e59437be44e35e10b3df45894993ac8c43422b16b059df0561032e41b102eb	-1
20260914120000	hosts_hostname_source	2026-11-14 15:18:18.692348+00	t	\\xa57a5dc26c000ec5b0b4aaf66486018f637b1f1228ae994dafa06a653b5afae3d8dd2bb1264f5695c50053d656bccdc2	-1
20260915120000	organization_license_columns	2026-11-14 15:18:18.69506+00	t	\\xd46a3e083f8179128aae52f68541627b57131d66c93d84c2ad0002a7bf6c025f757345490b47d810511d49cd9528525d	-1
20260915120001	organizations_license_entitlement	2026-11-14 15:18:18.696895+00	t	\\x1da434d08d6afb9f0bbe4165f112cae53d3046c8578d4feb88d5f3ea0fa0649cd4bf2a69ac6c2c24978a561a6333fcb9	-1
20260916120000	organization_license_key_issued_at	2026-11-14 15:18:18.698635+00	t	\\x99d66ef126e64532e4e87c1a99b0ac630a2031b1b4c9ba41cbe257b8e3a83e47ec0fe1360a409b5cbb57401326d7a9d3	-1
20260917120000	organization_license_key_type	2026-11-14 15:18:18.700208+00	t	\\xac327476c465df672532ce71bfbe03a0d92ff5cc3053fae9a15fa24100308cda6b08e118267c3c1009118eeb90f46038	-1
20260917130000	organization_bills_by_invoice	2026-11-14 15:18:18.702266+00	t	\\x33bf32d3b3edce4fe6803c326785210f322b9cdfd065440098b53424df7042ba90da1ac6f99ade774f894da1b67b0979	-1
20260930120000	clear_zero_mac_addresses	2026-11-14 15:18:18.708069+00	t	\\xef8416f55b61953f00f1fd249a04c82d63d0d8d510e9db6cae96f79db7a70a61dfc5a22029a6878e74da05e60b63b7dd	-1
20261001120000	add_user_display_settings	2026-11-14 15:18:18.71034+00	t	\\xd7f9afb0762c40085ced713496cf39e9778c4257196e1ba02a0feec1af67eab309df78ce5b753ebde6d250f6fbf3ef32	-1
20261001120001	credential_daemon_os	2026-11-14 15:18:18.713104+00	t	\\xe679b2c2caea51e608200d064b81b02f1f4cee20cfa60a929db29b87666ba8a5d01e5d0739a2b9953f401f4171c11c9d	-1
20261002120000	daemon_os	2026-11-14 15:18:18.714698+00	t	\\x931ef66a80cd751db47512b5f9397a5d7df21aff77cbf28c5a190c69b27ce6040d2e2f2ed3c08e85faf13d28e5cba25f	-1
20261003120000	hosts_os	2026-11-14 15:18:18.717097+00	t	\\xb8a3b3a3e6ec9e21cc64468fa5b691dba7d39556aab7cd28fee01a21fcbde0782952b4b73cbdbd334a17c554769fbdce	-1
20261003130000	organization_license_server_version	2026-11-14 15:18:18.718818+00	t	\\xe2f1bf9692ed8bb1a08a1ae13b53f632fd5f2112d53225bbf13332af26639b9518edb28daa4b3ed98bf71ed4b1002d34	-1
20261004120000	host_virtualization_interface	2026-11-14 15:18:18.722447+00	t	\\x79224f29c669331336f577a5c9f7e5f674dec637f1a879e9c4016e8705af2e921e63cf4ab178cb21af1026317d6e57be	-1
20261004120001	host_virtualization_interface_validate	2026-11-14 15:18:18.725133+00	t	\\xf03d5b6826e7aca5a5efe1ab57878591bfd12ac5c097b1d71e5539348cf9a5e794c2ef19c5fa3ee8aac039920b9f1f4e	-1
20261004120002	host_virtualization_interface_index	2026-11-14 15:18:18.728637+00	t	\\x64e39691a98e9ed761812e1bfdeaa7b8d9aa13eb0401a39fb40dd36f21222ac47dad7f1b4ff9465a3cd0da1af42039ee	-1
20261005120000	rename_networks_to_sites	2026-11-14 15:18:18.748771+00	t	\\x5e08911ddc549e9d522eece781caa57fe42ee4cb6f7d16fb25b63bf6c7f95e897d2cbd12db345040f5a4035d420ace43	-1
20261012130000	discovery_warnings	2026-11-14 15:18:18.86941+00	t	\\xc760d410eaa7d4d14214bf602cd9ba7423c1a328b71768d79bcef0cee51b9b685c37438f74091fb9c3afd6483a98e46c	-1
20261013120000	license_instances	2026-11-14 15:18:18.877683+00	t	\\x86796dc3b14fc5805b84f6c6fd92fa8aaa560a004ea4ca8f4affa76f55b830f8d2403393cddfcf215070e84d499bb5cb	-1
20261013140000	terms_notice_version	2026-11-14 15:18:18.879461+00	t	\\x624bbb290511d7491b8816b152b06194417047e2b9b19eeb8b1d954ec17e6fee77c73d9fd19fb74d62f8c44d66bb1a93	-1
20261019120000	dependency_members_keep_on_delete	2026-11-14 15:18:18.883787+00	t	\\x2391912be8b3a07910ba2240ffe21065efe5836453ee2ba9ee7ca5fda5dea58d701795486c7e03a70909c88e5cb5d595	-1
20261021120000	vlan_and_subnet_sources	2026-11-14 15:18:18.887193+00	t	\\x2535fce0c0903904a1d9bfa86132278e9238231438d3b15be9863c47a362c284cc26d4b4ecb7830ee128a004d3d99878	-1
20261022120000	credential_needs_newer_daemon	2026-11-14 15:18:18.898102+00	t	\\xfcc14b3c03c4c930f7ffa6a0086ff9fe12d35c77ccc331b4469ebaf19ddcff05a1a21801b4f2c36fd7d14df5c403a1a1	-1
20261023120000	snmp_community_credential	2026-11-14 15:18:18.911897+00	t	\\x71e050db43cceee2a66e206cbd87f478cc8a6932440def65d5ea5c79f55ae7339bff4a9e7727b4bd7a9696dc535816cf	-1
20261024120000	topology_device_type_rule	2026-11-14 15:18:18.914094+00	t	\\xb5f60015b76437698dbd4381b285195bc4b866a3d20f1cfb588bd675b92aab51dedce82bcbc29749c5e04ed4e6050d87	-1
20261024120001	google_nest_capitalization	2026-11-14 15:18:18.916584+00	t	\\x204cc0286eea5cba45dded8756cd98207c511b505eef07ce0470fd35104d69092038adc091536ffb2dda65e288d73262	-1
20261025120000	every_org_has_a_plan	2026-11-14 15:18:18.921509+00	t	\\x403a5bebde12227dbbfa913490de00196184c947a72da3d86f39809db010406715561f96eac6839546713c7c9aaf90c4	-1
20261026120000	strip_discovery_run_type_warnings	2026-11-14 15:18:18.926078+00	t	\\xffd0edff777c52348517fd47b5b87eb9406eea878e1168f23c2660786bf1352031ebb04761ed43dc3e6f6886dc5d4f52	-1
20261027120000	hosts_matched_device_type	2026-11-14 15:18:18.928852+00	t	\\xe970e578c2fad70d71c4e1d4a882d5e477e6372d090d93dad467ab8f656738deff5354b05868b59d8c808a2bfb61d8bd	-1
20261108120000	organization_setup_and_entity_source	2026-11-14 15:18:18.982853+00	t	\\xb6a079ad40e031c99f12bb9f5e5911ea84716a1cefcc9429a3f04174acaea84c0851e87665eee8ec171c9f7c64dee5b8	-1
20261114120000	resync_setup_and_entity_source	2026-11-14 15:18:18.987616+00	t	\\x3e11f795ab7866698ab3b9fe978e44c0bba6a794ed325bf842e900f48012caa8b4038772d5e718f142417a651f6b496e	-1
\.


--
-- Data for Name: api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.api_keys (id, key, site_id, name, created_at, updated_at, last_used, expires_at, is_enabled, plaintext, daemon_id) FROM stdin;
a1cd5c48-acf2-4e83-98b6-767a1a590c2f	b5b0a73e1ea4f20bed9c5e466a6ffee466472839a0b439a6acfa30365fdee6be	8dc5a375-7025-4afa-b80b-5db6acaf56d8	Compat Test API Key	2026-11-14 15:27:20.074439+00	2026-11-14 15:27:20.074439+00	2026-11-14 15:27:51.106793+00	\N	t	\N	\N
562ad3c2-fa4c-4a17-991d-732c4b86d0d6	3071e067524bb3ddda0e89b3b3cb41cc252e2be112b98605f3cfbe792618a6b6	8dc5a375-7025-4afa-b80b-5db6acaf56d8	scanopy-daemon API Key	2026-11-14 15:18:25.587142+00	2026-11-14 15:18:25.587142+00	2026-11-14 15:28:12.755442+00	\N	t	\N	e405bf21-7d94-4fa9-af78-83d4c6405b13
4d760397-8cb0-4dbd-b27e-369c8fabcca2	1dc268debc07a6d8ecb36641c4705eab38f8b554ebc1cf9d8b58ba4247815b15	8dc5a375-7025-4afa-b80b-5db6acaf56d8	scanopy-daemon-serverpoll API Key	2026-11-14 15:20:01.420781+00	2026-11-14 15:20:01.420781+00	2026-11-14 15:28:16.039315+00	\N	t	scp_d_meFthQ8bXucJCWMuW1AVHfeK1krZmDIf	4d0b4af4-2ec8-4fa6-9678-fe4509fd654a
\.


--
-- Data for Name: bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.bindings (id, site_id, service_id, binding_type, ip_address_id, port_id, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id) FROM stdin;
116ba20d-e20d-4619-9b95-5466171405f3	8dc5a375-7025-4afa-b80b-5db6acaf56d8	a6bceb85-8a66-414b-93ad-e7a855b019b8	Port	81412bd9-bca5-4e10-9ab3-ddba0f10d91b	59eca2e1-5650-44fc-bcc0-a436c484f2da	2026-03-15 19:34:42.017404+00	2026-03-15 19:34:42.017404+00	2026-11-14 15:28:00.356893+00	\N	\N	2026-11-14 15:28:00.356896+00	\N	\N	\N
\.


--
-- Data for Name: credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.credentials (id, organization_id, name, credential_type, created_at, updated_at, daemon_os, description, login_credential_id, source) FROM stdin;
30ddaec3-9d76-48d1-a89d-cd4cb9bb4063	4588b37a-3195-4908-9679-32e8f045faa8	SNMP (public)	{"type": "SnmpCommunity", "community": {"mode": "Inline", "value": "public"}}	2026-11-14 15:18:25.397313+00	2026-11-14 15:18:25.397313+00	\N	Assigned to every new site. Tries the SNMP community "public" on every host, over v2c and then v1.	\N	{"type": "System"}
6f624f30-b6e7-438c-b1cf-4b5e4937f206	4588b37a-3195-4908-9679-32e8f045faa8	Docker socket	{"type": "DockerSocket"}	2026-11-14 15:18:25.402334+00	2026-11-14 15:18:25.402334+00	\N	Added to every new daemon. Reads the local Docker socket at its default path.	\N	{"type": "System"}
c454a2be-110b-4fa5-b8de-00eecd14b27e	4588b37a-3195-4908-9679-32e8f045faa8	Podman socket	{"type": "PodmanSocket"}	2026-11-14 15:18:25.407756+00	2026-11-14 15:18:25.407756+00	\N	Added to every new daemon. Reads the local Podman socket at its default path.	\N	{"type": "System"}
\.


--
-- Data for Name: daemon_interfaced_subnets; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemon_interfaced_subnets (daemon_id, subnet_id) FROM stdin;
e405bf21-7d94-4fa9-af78-83d4c6405b13	4ad11d12-d29a-4f8d-87e9-dc2c288a785d
4d0b4af4-2ec8-4fa6-9678-fe4509fd654a	b2945287-f947-4f7d-ae4a-ddf05523c732
4d0b4af4-2ec8-4fa6-9678-fe4509fd654a	4ad11d12-d29a-4f8d-87e9-dc2c288a785d
\.


--
-- Data for Name: daemons; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemons (id, site_id, host_id, created_at, last_seen, updated_at, mode, url, name, version, user_id, api_key_id, is_unreachable, standby, standby_cleared_at, os) FROM stdin;
e405bf21-7d94-4fa9-af78-83d4c6405b13	8dc5a375-7025-4afa-b80b-5db6acaf56d8	3806ccb4-5580-4e86-96d4-305bcc95aa18	2026-11-14 15:18:25.605312+00	2026-11-14 15:28:12.761687+00	2026-11-14 15:18:25.605312+00	"daemon_poll"		scanopy-daemon	0.20.0	e866d437-0fab-4ecb-ae06-f0de46edc63c	562ad3c2-fa4c-4a17-991d-732c4b86d0d6	f	f	\N	"linux"
4d0b4af4-2ec8-4fa6-9678-fe4509fd654a	8dc5a375-7025-4afa-b80b-5db6acaf56d8	0ad3aed3-81f0-472c-867f-33bb0c77fbb8	2026-11-14 15:20:01.43207+00	2026-11-14 15:28:16.044406+00	2026-11-14 15:20:01.43207+00	"server_poll"	http://daemon-serverpoll:60074	scanopy-daemon-serverpoll	0.20.0	e866d437-0fab-4ecb-ae06-f0de46edc63c	4d760397-8cb0-4dbd-b27e-369c8fabcca2	f	f	2026-11-14 15:28:16.044409+00	"linux"
\.


--
-- Data for Name: dependencies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.dependencies (id, site_id, name, description, created_at, updated_at, source, color, edge_style, dependency_type, member_type, valid_from, valid_to, lineage_id, snapshot_id) FROM stdin;
b9d1e560-c379-455a-9366-72cae7e5ba4f	8dc5a375-7025-4afa-b80b-5db6acaf56d8		\N	2026-11-14 15:27:19.116024+00	2026-11-14 15:27:19.116024+00	{"type": "Manual"}	Yellow	"Bezier"	RequestPath	Services	2026-11-14 15:27:19.116024+00	\N	\N	\N
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
b38b0172-ef9c-4586-96d8-d16ec194a328	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "b368c96b-6dbf-4523-bcf9-fee0229c2dfb", "started_at": "2026-10-02T13:49:47.412762700Z", "finished_at": "2026-11-14T15:27:21.074411Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "Unified", "host_id": "6aeae378-cd4b-493a-8bff-471792dff268", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:20.520639Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6aeae378-cd4b-493a-8bff-471792dff268", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-02 13:49:47.412762+00	2026-11-14 15:27:21.075674+00	0	f	[]
e0725832-2ae0-4a52-a306-4978687c2def	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Scheduled", "enabled": true, "last_run": "2026-11-14T15:18:31.080730Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "3806ccb4-5580-4e86-96d4-305bcc95aa18", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-14 15:18:25.608242+00	2026-11-14 15:20:00.908614+00	1	f	[]
5b3b9c56-bc30-4de0-9179-8ca22e5a6bbe	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 100, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "a02baba2-d793-499f-a314-7dd54efbfc05", "started_at": "2026-11-14T15:19:01.111454Z", "finished_at": "2026-11-14T15:20:00.833040Z", "discovery_id": "e0725832-2ae0-4a52-a306-4978687c2def", "daemon_version": "0.20.0", "discovery_type": {"type": "Unified", "host_id": "3806ccb4-5580-4e86-96d4-305bcc95aa18", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:20:00.899160Z", "hosts_discovered": 6, "last_activity_at": "2026-11-14T15:20:00.823Z", "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "3806ccb4-5580-4e86-96d4-305bcc95aa18", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-14 15:19:01.111454+00	2026-11-14 15:20:00.907581+00	0	f	[]
944f2777-4aae-42d5-a5d5-92f80302e841	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "e7c3d9e2-4cad-4942-98d6-ae6053485366", "started_at": "2026-03-11T06:19:24.616705132Z", "finished_at": "2026-11-14T15:27:21.623675Z", "discovery_id": null, "daemon_version": "0.17.17", "discovery_type": {"type": "SelfReport", "host_id": "8c99dc12-cae3-41f2-a550-be436a112a35"}, "last_update_at": "2026-11-14T15:27:21.154891Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8c99dc12-cae3-41f2-a550-be436a112a35"}	Self Report — My Site	2026-03-11 06:19:24.616705+00	2026-11-14 15:27:21.625801+00	0	f	[]
28ebc38a-f8df-4fc4-9ef5-eeeb683a56cd	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "c063e961-df38-4fbb-b437-0e6e25593003", "started_at": "2026-09-24T21:00:16.922463583Z", "finished_at": "2026-11-14T15:27:22.185812Z", "discovery_id": null, "daemon_version": "0.17.16", "discovery_type": {"type": "Unified", "host_id": "ad3c2bd6-9bf9-4c4d-8534-3689c0e013c3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:21.639147Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "ad3c2bd6-9bf9-4c4d-8534-3689c0e013c3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-24 21:00:16.922463+00	2026-11-14 15:27:22.186929+00	0	f	[]
c7f26443-d5db-44c9-baa7-cabe2382305e	8dc5a375-7025-4afa-b80b-5db6acaf56d8	4d0b4af4-2ec8-4fa6-9678-fe4509fd654a	{"type": "Scheduled", "enabled": true, "last_run": "2026-11-14T15:20:19.042834Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "0ad3aed3-81f0-472c-867f-33bb0c77fbb8", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-14 15:20:01.43848+00	2026-11-14 15:24:19.070496+00	1	f	[]
71a7b541-8015-457e-b44e-55db72e2a885	8dc5a375-7025-4afa-b80b-5db6acaf56d8	4d0b4af4-2ec8-4fa6-9678-fe4509fd654a	{"type": "AdHoc", "last_run": "2026-11-14T15:20:20.045659Z"}	{"type": "Unified", "host_id": "0ad3aed3-81f0-472c-867f-33bb0c77fbb8", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	ServerPoll Integration Test Discovery	2026-11-14 15:20:20.033878+00	2026-11-14 15:27:19.073904+00	1	f	[]
e542f7b2-5bc2-4893-84bc-c5e2e9cc472f	8dc5a375-7025-4afa-b80b-5db6acaf56d8	4d0b4af4-2ec8-4fa6-9678-fe4509fd654a	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 100, "warnings": [], "daemon_id": "4d0b4af4-2ec8-4fa6-9678-fe4509fd654a", "session_id": "99f42ff7-61ed-40f0-a822-cb35a3dbebc9", "started_at": "2026-11-14T15:21:19.052130Z", "finished_at": "2026-11-14T15:23:49.907203Z", "discovery_id": "c7f26443-d5db-44c9-baa7-cabe2382305e", "daemon_version": "0.20.0", "discovery_type": {"type": "Unified", "host_id": "0ad3aed3-81f0-472c-867f-33bb0c77fbb8", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:24:19.064624Z", "hosts_discovered": null, "last_activity_at": "2026-11-14T15:23:43.905Z", "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "0ad3aed3-81f0-472c-867f-33bb0c77fbb8", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-14 15:21:19.05213+00	2026-11-14 15:24:19.069304+00	0	f	[]
8a31afe4-22e3-45f4-adb6-b0f6f6092758	8dc5a375-7025-4afa-b80b-5db6acaf56d8	4d0b4af4-2ec8-4fa6-9678-fe4509fd654a	{"type": "Historical", "results": {"error": null, "phase": "Complete", "reason": "Completed", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 100, "warnings": [], "daemon_id": "4d0b4af4-2ec8-4fa6-9678-fe4509fd654a", "session_id": "2fa8a29e-6e84-47de-a889-b0bf23df1c4e", "started_at": "2026-11-14T15:24:19.148076Z", "finished_at": "2026-11-14T15:26:49.611203Z", "discovery_id": "71a7b541-8015-457e-b44e-55db72e2a885", "daemon_version": "0.20.0", "discovery_type": {"type": "Unified", "host_id": "0ad3aed3-81f0-472c-867f-33bb0c77fbb8", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:19.068801Z", "hosts_discovered": null, "last_activity_at": "2026-11-14T15:26:43.610Z", "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "0ad3aed3-81f0-472c-867f-33bb0c77fbb8", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	ServerPoll Integration Test Discovery	2026-11-14 15:24:19.148076+00	2026-11-14 15:27:19.072951+00	0	f	[]
33bea135-30cf-4678-8ec4-196664acf70d	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "1e8cf04d-e26d-4e84-a90f-4a534c217faf", "started_at": "2026-10-02T13:54:28.548758981Z", "finished_at": "2026-11-14T15:27:21.069315Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:20.613667Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-02 13:54:28.548758+00	2026-11-14 15:27:21.070612+00	0	f	[]
2b5538af-5c7a-446c-a696-0fd846b8a2ce	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "93db1812-9fa5-4c4b-a36b-d687b363811c", "started_at": "2026-10-02T13:50:58.545317669Z", "finished_at": "2026-11-14T15:27:21.071850Z", "discovery_id": null, "daemon_version": "0.17.7", "discovery_type": {"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:20.608084Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "164154f4-2a78-4cea-be86-6d4548e4765b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-02 13:50:58.545317+00	2026-11-14 15:27:21.072987+00	0	f	[]
192c4b00-955a-412b-9001-fe23d1521d8b	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "0ffe5496-d64e-41a0-ab86-ff28eed28819", "started_at": "2026-02-04T02:00:38.542462976Z", "finished_at": "2026-11-14T15:27:24.451895Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:23.407090Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-04 02:00:38.542462+00	2026-11-14 15:27:24.453844+00	0	f	[]
4770e1e2-0cbd-43d3-bda9-3589656c6de6	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "b5073709-1dc4-4f69-bab1-d17406b1b571", "started_at": "2026-07-29T04:22:21.659954694Z", "finished_at": "2026-11-14T15:27:21.618164Z", "discovery_id": null, "daemon_version": "0.17.17", "discovery_type": {"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:21.149473Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-29 04:22:21.659954+00	2026-11-14 15:27:21.619263+00	0	f	[]
0eb3157e-0977-4256-825b-1c7e8ae39cd2	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "30c1790e-6f3b-44f7-87be-9491e456edd0", "started_at": "2026-07-29T04:18:51.683205577Z", "finished_at": "2026-11-14T15:27:21.620893Z", "discovery_id": null, "daemon_version": "0.17.17", "discovery_type": {"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:21.143379Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c9547878-e75d-4a56-9e0e-9dfd4b3e37ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-29 04:18:51.683205+00	2026-11-14 15:27:21.622077+00	0	f	[]
893574a6-239d-4206-8771-5a885acaf929	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "890873b9-2054-463a-9fe8-11016f0d08f2", "started_at": "2026-07-29T04:17:17.562092815Z", "finished_at": "2026-11-14T15:27:21.626943Z", "discovery_id": null, "daemon_version": "0.17.17", "discovery_type": {"type": "Unified", "host_id": "f4930d3b-6bc0-42c1-b25c-bbf557bf56ce", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:21.094837Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f4930d3b-6bc0-42c1-b25c-bbf557bf56ce", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-29 04:17:17.562092+00	2026-11-14 15:27:21.628019+00	0	f	[]
1ffbd317-2d1b-439c-b5bc-06e6bd111881	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "44164544-aa86-4db3-ad9a-503d5933df88", "started_at": "2026-09-24T21:05:01.816315928Z", "finished_at": "2026-11-14T15:27:22.190460Z", "discovery_id": null, "daemon_version": "0.17.16", "discovery_type": {"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:21.727497Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-24 21:05:01.816315+00	2026-11-14 15:27:22.191772+00	0	f	[]
fa1cfa7d-acc9-4988-9035-573811af18ab	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "6b771cb0-0931-4c07-a677-efe6aa5bc5bd", "started_at": "2026-02-01T15:40:05.145636842Z", "finished_at": "2026-11-14T15:27:24.442080Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:22.832234Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-01 15:40:05.145636+00	2026-11-14 15:27:24.444143+00	0	f	[]
515d7935-7108-4316-95ab-966da360e8c0	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "3dec6b93-d9d7-46ab-94e2-ef221564d563", "started_at": "2026-09-24T21:01:31.815231601Z", "finished_at": "2026-11-14T15:27:22.188134Z", "discovery_id": null, "daemon_version": "0.17.16", "discovery_type": {"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:21.721341Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "fd55fcfc-bdcf-4039-902e-d171b34fb740", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-24 21:01:31.815231+00	2026-11-14 15:27:22.189289+00	0	f	[]
1a35f361-7fa8-4509-8ecc-00ce341bab16	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "4e1e43c0-b63f-41df-ba3a-71683fef4763", "started_at": "2026-02-04T01:54:22.209258404Z", "finished_at": "2026-11-14T15:27:24.445327Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:23.401187Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-04 01:54:22.209258+00	2026-11-14 15:27:24.447343+00	0	f	[]
c017155d-0dad-43f5-bcab-e24a9423f9be	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "5b19fece-cb82-45d8-a676-df53cc38a014", "started_at": "2026-02-01T15:46:28.576568381Z", "finished_at": "2026-11-14T15:27:24.468737Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}, "last_update_at": "2026-11-14T15:27:22.870239Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}	Self Report — My Site	2026-02-01 15:46:28.576568+00	2026-11-14 15:27:24.470636+00	0	f	[]
73dc7838-89ea-4412-a3c6-e205c0be9c95	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "c8c6530c-aaff-4d7c-a872-ba4457906d77", "started_at": "2026-02-04T02:06:45.972533343Z", "finished_at": "2026-11-14T15:27:24.479919Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}, "last_update_at": "2026-11-14T15:27:23.439792Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}	Self Report — My Site	2026-02-04 02:06:45.972533+00	2026-11-14 15:27:24.481813+00	0	f	[]
202849fe-79e6-4f70-bba4-9b1dd562cf54	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "9d715165-15d9-4c53-8eeb-adadd14656ec", "started_at": "2026-10-26T16:01:31.798722266Z", "finished_at": "2026-11-14T15:27:25.033408Z", "discovery_id": null, "daemon_version": "0.17.22", "discovery_type": {"type": "Unified", "host_id": "9b66035e-43fc-4de9-a9fa-9febda413452", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:24.500698Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9b66035e-43fc-4de9-a9fa-9febda413452", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-26 16:01:31.798722+00	2026-11-14 15:27:25.034567+00	0	f	[]
4ea1ff75-f882-4d5d-9e8b-22978ce8466a	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "27ff5779-08b0-4970-aebd-04649d27c725", "started_at": "2026-03-24T00:04:21.695856405Z", "finished_at": "2026-11-14T15:27:26.807050Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.284268Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:21.695856+00	2026-11-14 15:27:26.809009+00	0	f	[]
a7f9667c-f661-49ab-b145-346bf9a99db2	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "ca5b8138-b6c5-41bb-928c-59ab96c38227", "started_at": "2026-09-16T20:13:01.385050881Z", "finished_at": "2026-11-14T15:27:22.756670Z", "discovery_id": null, "daemon_version": "0.13.6", "discovery_type": {"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:22.292860Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 20:13:01.38505+00	2026-11-14 15:27:22.757824+00	0	f	[]
acb520c2-ac9f-4fa4-a15a-855094d0090e	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "24dd8f5a-4866-4462-a9ff-b65cb5a3061a", "started_at": "2026-09-16T20:16:31.390119148Z", "finished_at": "2026-11-14T15:27:22.763650Z", "discovery_id": null, "daemon_version": "0.13.6", "discovery_type": {"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:22.298806Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "49271e8b-f7ca-452e-94e6-24ce7613e14e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 20:16:31.390119+00	2026-11-14 15:27:22.764742+00	0	f	[]
9ee1db7a-263c-4689-a813-4613b528ef8d	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "c540d032-e83e-4050-ab10-97635f0968a2", "started_at": "2026-10-01T04:07:23.928243114Z", "finished_at": "2026-11-14T15:27:24.471629Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:23.986273Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-01 04:07:23.928243+00	2026-11-14 15:27:24.472659+00	0	f	[]
a5773f21-6115-4d43-801e-61b374a38a65	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "e903943a-3b77-43c1-a9ad-824cb543557b", "started_at": "2026-10-07T13:09:38.731246701Z", "finished_at": "2026-11-14T15:27:25.589669Z", "discovery_id": null, "daemon_version": "0.18.2", "discovery_type": {"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:25.127564Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-07 13:09:38.731246+00	2026-11-14 15:27:25.590944+00	0	f	[]
73af6c83-a96c-4a27-bb5f-61dab682438d	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "969d115c-0112-4ba3-8757-bb1ece340112", "started_at": "2026-03-24T00:04:20.606954763Z", "finished_at": "2026-11-14T15:27:26.796261Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.263320Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:20.606954+00	2026-11-14 15:27:26.798138+00	0	f	[]
b5d6339b-6fca-40dd-a812-50076859a65d	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "64824f2d-3081-44a6-a631-23dbe65e4721", "started_at": "2026-09-16T20:11:28.193921546Z", "finished_at": "2026-11-14T15:27:22.759105Z", "discovery_id": null, "daemon_version": "0.13.6", "discovery_type": {"type": "Unified", "host_id": "505ad895-0816-4969-a72c-161ebc636002", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:22.211481Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "505ad895-0816-4969-a72c-161ebc636002", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 20:11:28.193921+00	2026-11-14 15:27:22.760274+00	0	f	[]
9e58911d-9743-4e8b-9277-c9ba36032b6f	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "afde3201-cb3d-4b85-bb8f-470b83cba2a3", "started_at": "2026-02-01T15:48:58.546419422Z", "finished_at": "2026-11-14T15:27:24.448970Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:22.888496Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-01 15:48:58.546419+00	2026-11-14 15:27:24.450852+00	0	f	[]
82da62f6-44e3-4398-96d6-4bc70bab993a	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "1a182cd9-9710-482e-8645-d95c40db018a", "started_at": "2026-01-26T14:03:24.338877430Z", "finished_at": "2026-11-14T15:27:24.482699Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "7891ed81-377c-4eca-b05e-bc8a17129f90"}, "last_update_at": "2026-11-14T15:27:23.412670Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "7891ed81-377c-4eca-b05e-bc8a17129f90"}	Self Report — My Site	2026-01-26 14:03:24.338877+00	2026-11-14 15:27:24.484516+00	0	f	[]
8f871b8e-b1c0-4a13-8ec6-cd4569ca22e0	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "56d2df9f-197b-49f8-b4ac-e9d4674dca7a", "started_at": "2026-10-07T13:08:30.855227146Z", "finished_at": "2026-11-14T15:27:25.592258Z", "discovery_id": null, "daemon_version": "0.18.2", "discovery_type": {"type": "Unified", "host_id": "1be4e31e-a229-440d-b0f4-20c31ee11254", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:25.045132Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "1be4e31e-a229-440d-b0f4-20c31ee11254", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-07 13:08:30.855227+00	2026-11-14 15:27:25.59331+00	0	f	[]
759e4c87-dabb-4782-bdd5-b1b8e1785c64	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "c10d2eae-1267-4023-997e-5a51fdf97281", "started_at": "2026-03-24T00:04:18.984158426Z", "finished_at": "2026-11-14T15:27:26.799195Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.237666Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:18.984158+00	2026-11-14 15:27:26.801069+00	0	f	[]
d96c0a81-36a6-4d11-9c5f-a77b3b4ab3d0	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "59fba01e-fefd-4c66-9ef2-a85c0e76a811", "started_at": "2026-03-24T00:04:22.788677537Z", "finished_at": "2026-11-14T15:27:26.809938Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.304481Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:22.788677+00	2026-11-14 15:27:26.811912+00	0	f	[]
fb221ce7-1336-4da0-9303-38482421aa61	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "68690167-4bf9-40db-9756-207731c78629", "started_at": "2026-09-16T19:13:38.792500966Z", "finished_at": "2026-11-14T15:27:22.761449Z", "discovery_id": null, "daemon_version": "0.13.6", "discovery_type": {"type": "Unified", "host_id": "fe9f2b88-655a-44e9-b2b2-b89b797b34ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:22.304643Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "fe9f2b88-655a-44e9-b2b2-b89b797b34ad", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-16 19:13:38.7925+00	2026-11-14 15:27:22.762545+00	0	f	[]
f0444a32-56d6-49d8-be53-05780e7413e6	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "7f54ecab-3771-4583-a7d0-f12569030e17", "started_at": "2026-02-04T02:06:45.451848584Z", "finished_at": "2026-11-14T15:27:24.454897Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}, "last_update_at": "2026-11-14T15:27:23.428938Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}	Self Report — My Site	2026-02-04 02:06:45.451848+00	2026-11-14 15:27:24.456785+00	0	f	[]
4ae2eb4b-248e-4bab-bbec-791ee373e9d4	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "927175d4-7258-4b40-a656-8412141305c5", "started_at": "2026-02-04T01:53:52.212090885Z", "finished_at": "2026-11-14T15:27:24.488354Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "e0313f98-eeac-4055-beb3-7012a7bd7510"}, "last_update_at": "2026-11-14T15:27:23.349509Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "e0313f98-eeac-4055-beb3-7012a7bd7510"}	Self Report — My Site	2026-02-04 01:53:52.21209+00	2026-11-14 15:27:24.49022+00	0	f	[]
f130e787-1b83-4f99-954a-63c29c5feba1	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "4273fe18-d031-4977-950e-ad5a49d2ea8e", "started_at": "2026-03-24T00:04:21.150690914Z", "finished_at": "2026-11-14T15:27:26.793344Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.273936Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:21.15069+00	2026-11-14 15:27:26.795222+00	0	f	[]
f6a0c898-444b-4778-a9f3-95afc322a550	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "f071713f-3367-4111-8f7b-7b7b50d33aa9", "started_at": "2026-03-23T23:57:20.088532334Z", "finished_at": "2026-11-14T15:27:26.804968Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "Unified", "host_id": "14fcf158-caea-47c9-82ad-38dcc2907791", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:26.168841Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "14fcf158-caea-47c9-82ad-38dcc2907791", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-23 23:57:20.088532+00	2026-11-14 15:27:26.806003+00	0	f	[]
800e5c32-8983-43a2-b36f-767a7aac1935	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "d2b26f9b-0329-4191-8a60-58cf3c796dba", "started_at": "2026-03-24T00:00:06.445031185Z", "finished_at": "2026-11-14T15:27:26.815673Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:26.211891Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-24 00:00:06.445031+00	2026-11-14 15:27:26.816694+00	0	f	[]
0ac7d11e-48b6-40a4-8b98-b00111878c34	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "f5a833cc-002f-41e1-b540-c0a623d6775c", "started_at": "2026-01-25T23:12:10.196614920Z", "finished_at": "2026-11-14T15:27:24.457827Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644"}, "last_update_at": "2026-11-14T15:27:22.837654Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644"}	Self Report — My Site	2026-01-25 23:12:10.196614+00	2026-11-14 15:27:24.459786+00	0	f	[]
f3a55fca-4c3e-4803-84ec-75102b6e807e	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "be760657-cb8b-4175-8484-00ea5633e0a3", "started_at": "2026-10-15T03:33:48.001242310Z", "finished_at": "2026-11-14T15:27:26.145465Z", "discovery_id": null, "daemon_version": "0.15.0", "discovery_type": {"type": "Unified", "host_id": "6943745c-a092-47e4-9991-74ba816a3f6a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:25.606211Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6943745c-a092-47e4-9991-74ba816a3f6a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:33:48.001242+00	2026-11-14 15:27:26.146569+00	0	f	[]
480e2c90-cdba-4293-9d16-77aae41a6ce4	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "ec9dc330-67d0-4ce5-94d8-506859c74940", "started_at": "2026-03-24T00:04:23.061291004Z", "finished_at": "2026-11-14T15:27:26.787171Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.309559Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:23.061291+00	2026-11-14 15:27:26.789149+00	0	f	[]
4c3a4b52-934b-4943-971e-0623ea75bd09	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "466f3f06-3335-4195-94cd-7b1b85eceda4", "started_at": "2026-02-01T15:39:35.137282092Z", "finished_at": "2026-11-14T15:27:24.460897Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "d6e89254-a0d6-418b-9678-e75d11712fd1"}, "last_update_at": "2026-11-14T15:27:22.782841Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "d6e89254-a0d6-418b-9678-e75d11712fd1"}	Self Report — My Site	2026-02-01 15:39:35.137282+00	2026-11-14 15:27:24.462922+00	0	f	[]
5042fad6-e278-455e-af0f-a1d471a2f64a	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "bbe36108-56c4-4d82-9087-e577842ff202", "started_at": "2026-03-24T00:04:22.515854945Z", "finished_at": "2026-11-14T15:27:26.777766Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.299438Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:22.515854+00	2026-11-14 15:27:26.779807+00	0	f	[]
dbd3f2d6-09dd-4b1f-90b3-05cab8b64082	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "reason": "UserCancelled", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "e0ea6622-5f78-4cc3-b1f9-257946754dd8", "started_at": "2026-03-09T05:29:17.642983004Z", "finished_at": "2026-03-09T05:29:17.643045281Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:26.995821Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-09 05:29:17.642983+00	2026-11-14 15:27:26.997887+00	0	f	[]
40dfa1e8-fdda-4d55-8a98-1a6766ccf928	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "cbdca757-31a8-4a33-b45a-5ad52b4de60b", "started_at": "2026-10-01T04:02:39.535834801Z", "finished_at": "2026-11-14T15:27:24.464186Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Unified", "host_id": "bd69aa7e-1ffb-4e1d-8ac6-adc468be6771", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:23.900129Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bd69aa7e-1ffb-4e1d-8ac6-adc468be6771", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-01 04:02:39.535834+00	2026-11-14 15:27:24.465239+00	0	f	[]
58348ff1-66e0-4cde-8450-fced92ad412e	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "6845dcc8-6ea0-47af-9ce3-054b8cf1f667", "started_at": "2026-02-04T02:06:44.655234805Z", "finished_at": "2026-11-14T15:27:24.485479Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}, "last_update_at": "2026-11-14T15:27:23.423471Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}	Self Report — My Site	2026-02-04 02:06:44.655234+00	2026-11-14 15:27:24.48739+00	0	f	[]
30a84857-d886-4d39-9135-f31ab53314ec	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "5e741620-3de9-476c-86e7-7ce656d0a5a8", "started_at": "2026-03-24T00:04:23.604406719Z", "finished_at": "2026-11-14T15:27:26.790211Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.320168Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:23.604406+00	2026-11-14 15:27:26.792205+00	0	f	[]
1fdfeb1d-3620-4fde-a3a7-b4218c547c28	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "d03e3468-38c8-46e6-bd30-43693e9db1d7", "started_at": "2026-03-09T05:16:11.595722227Z", "finished_at": "2026-11-14T15:27:27.940882Z", "discovery_id": null, "daemon_version": "0.20.0", "discovery_type": {"type": "SelfReport", "host_id": "3ef5ad9b-872d-4fc7-8f8f-e5c0186d1841"}, "last_update_at": "2026-11-14T15:27:26.843550Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "3ef5ad9b-872d-4fc7-8f8f-e5c0186d1841"}	Self Report — My Site	2026-03-09 05:16:11.595722+00	2026-11-14 15:27:27.942944+00	0	f	[]
036d7d13-6e9d-43c3-a83a-0fe8ce8e0864	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "075ee07b-1779-47db-8c94-50c17ce5f66d", "started_at": "2026-10-01T04:03:53.917292009Z", "finished_at": "2026-11-14T15:27:24.466401Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:23.980255Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "74639d80-f428-433e-8ecc-d7de25ca8e25", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-01 04:03:53.917292+00	2026-11-14 15:27:24.467616+00	0	f	[]
dfa462e4-0756-4694-9ca1-82943838391a	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "b64df0ca-f173-4ba8-a48b-edf7e372974a", "started_at": "2026-02-04T02:06:45.712590434Z", "finished_at": "2026-11-14T15:27:24.476858Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}, "last_update_at": "2026-11-14T15:27:23.434522Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}	Self Report — My Site	2026-02-04 02:06:45.71259+00	2026-11-14 15:27:24.478745+00	0	f	[]
20a28d68-49f2-4ee4-8089-1db92e0ca57d	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "5bd5e93e-a606-4c6e-b159-b04879bdd801", "started_at": "2026-03-24T00:04:23.332055633Z", "finished_at": "2026-11-14T15:27:26.781025Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.314848Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:23.332055+00	2026-11-14 15:27:26.783122+00	0	f	[]
790eba65-cc4a-43e1-bf25-cef48fee6aa1	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "f2d5414b-3369-4280-929d-5422dd11a4b4", "started_at": "2026-02-04T02:06:44.396505189Z", "finished_at": "2026-11-14T15:27:24.473753Z", "discovery_id": null, "daemon_version": "0.19.2", "discovery_type": {"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}, "last_update_at": "2026-11-14T15:27:23.418100Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}	Self Report — My Site	2026-02-04 02:06:44.396505+00	2026-11-14 15:27:24.475755+00	0	f	[]
d995bd5e-5bba-4b83-b342-e03c65583fa3	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "26c0ac92-5d7b-41b7-969c-9e07d17bd1be", "started_at": "2026-10-15T03:38:40.733230767Z", "finished_at": "2026-11-14T15:27:26.150101Z", "discovery_id": null, "daemon_version": "0.15.0", "discovery_type": {"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:25.688647Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:38:40.73323+00	2026-11-14 15:27:26.151156+00	0	f	[]
1ec1bc51-3606-491c-9947-e30767840ea2	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "34c804c2-c01c-4104-a3b3-e31ae6dd0b6f", "started_at": "2026-03-24T00:04:19.801043433Z", "finished_at": "2026-11-14T15:27:26.801940Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.252964Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:19.801043+00	2026-11-14 15:27:26.803864+00	0	f	[]
61426ddb-7a76-46c9-87ef-3f0bbc2814a4	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "3b0868af-d558-45e3-b688-2cf55472b6ee", "started_at": "2026-03-24T00:04:24.139743985Z", "finished_at": "2026-11-14T15:27:26.812872Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.330515Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:24.139743+00	2026-11-14 15:27:26.814697+00	0	f	[]
b47d9af7-f7e3-48cb-9eeb-dc3d1b57e06d	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "abba33fb-bf1f-4e8b-985f-6a2d0b5d0380", "started_at": "2026-03-24T00:04:21.423333702Z", "finished_at": "2026-11-14T15:27:26.823730Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.279084Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:21.423333+00	2026-11-14 15:27:26.82558+00	0	f	[]
2ab929cd-7d6a-43a3-a694-cdbd0d30d177	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "0400691f-2af1-4b20-bade-e50044271bfe", "started_at": "2026-10-07T13:13:08.735400435Z", "finished_at": "2026-11-14T15:27:25.594376Z", "discovery_id": null, "daemon_version": "0.18.2", "discovery_type": {"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:25.133397Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bd485d9c-d591-4aae-a253-1eca3ff34bbb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-07 13:13:08.7354+00	2026-11-14 15:27:25.595616+00	0	f	[]
d3ada1d0-b68f-4073-8656-63ed6a700007	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "ae2ef896-c9ee-4c7d-92d0-2e940d717f51", "started_at": "2026-10-15T03:35:10.733740412Z", "finished_at": "2026-11-14T15:27:26.147848Z", "discovery_id": null, "daemon_version": "0.15.0", "discovery_type": {"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:25.682804Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "71257747-2cda-461d-be2e-37ddacd1f60c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-15 03:35:10.73374+00	2026-11-14 15:27:26.149092+00	0	f	[]
dac1729d-619a-4592-b9eb-c24f8393ce19	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "857fbace-0358-4642-8ca0-b14bb999f353", "started_at": "2026-03-24T00:04:21.968361234Z", "finished_at": "2026-11-14T15:27:26.784293Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.289291Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:21.968361+00	2026-11-14 15:27:26.786271+00	0	f	[]
b8f40e4e-919d-4541-a244-05b7fa62c1c4	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "ed48cfed-4a98-4473-9c83-33781243c790", "started_at": "2026-03-09T05:16:41.592520993Z", "finished_at": "2026-11-14T15:27:27.944131Z", "discovery_id": null, "daemon_version": "0.20.0", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:26.893609Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-09 05:16:41.59252+00	2026-11-14 15:27:27.946215+00	0	f	[]
d10e53b1-e9f0-4985-b60d-d2388339bb25	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "10fdd8f4-03b6-44ea-adb6-27e74136b365", "started_at": "2026-03-24T00:04:20.878919695Z", "finished_at": "2026-11-14T15:27:26.817641Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.268708Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:20.878919+00	2026-11-14 15:27:26.819695+00	0	f	[]
8fd2ccf5-f1bd-4aed-ba7b-37cb3bfd3d9d	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "22ea697f-065c-4d2e-a81f-0809764aad01", "started_at": "2026-03-24T00:04:22.243245356Z", "finished_at": "2026-11-14T15:27:26.820693Z", "discovery_id": null, "daemon_version": "0.14.17", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "last_update_at": "2026-11-14T15:27:26.294281Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Site	2026-03-24 00:04:22.243245+00	2026-11-14 15:27:26.822768+00	0	f	[]
225b6967-e7c2-487e-9a68-cebe9f68aa7d	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "a02baba2-d793-499f-a314-7dd54efbfc05", "started_at": "2026-11-14T15:19:01.111454Z", "finished_at": "2026-11-14T15:27:29.032767Z", "discovery_id": null, "daemon_version": "0.15.4", "discovery_type": {"type": "Unified", "host_id": "3806ccb4-5580-4e86-96d4-305bcc95aa18", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:27.970051Z", "hosts_discovered": null, "last_activity_at": "2026-11-14T15:19:01.111Z", "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "3806ccb4-5580-4e86-96d4-305bcc95aa18", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-14 15:19:01.111454+00	2026-11-14 15:27:29.034065+00	0	f	[]
7254335a-dc91-45e0-84ee-f95a7878d719	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "d53d5b11-b914-4af1-bf04-5d6e47899de1", "started_at": "2026-01-23T03:40:39.007214962Z", "finished_at": "2026-11-14T15:27:29.035710Z", "discovery_id": null, "daemon_version": "0.15.4", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:28.574760Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-23 03:40:39.007214+00	2026-11-14 15:27:29.037971+00	0	f	[]
64bfd2b5-8e52-484d-9e41-45193aee206b	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "1164516e-9bd1-4a5c-8fe8-182c9c6923d8", "started_at": "2026-03-31T18:30:47.557400890Z", "finished_at": "2026-11-14T15:27:29.725714Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Unified", "host_id": "52e0ccf9-3b92-408c-9a34-1bd2f053d047", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:29.060233Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "52e0ccf9-3b92-408c-9a34-1bd2f053d047", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:30:47.5574+00	2026-11-14 15:27:29.726791+00	0	f	[]
40fc79cd-1ca8-4266-b72e-1ebe185bfd4b	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "2f915185-76ea-4a3a-8f81-7ca0298ae158", "started_at": "2026-03-31T18:35:19.970481609Z", "finished_at": "2026-11-14T15:27:29.728260Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Unified", "host_id": "2a69c4df-31b5-4490-adc9-01e591b9f974", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:29.147583Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2a69c4df-31b5-4490-adc9-01e591b9f974", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:35:19.970481+00	2026-11-14 15:27:29.729368+00	0	f	[]
947ce5ff-7066-42bc-951e-dd536bce8504	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "18c3c94f-80cb-48e9-aedc-76dbc938706e", "started_at": "2026-03-31T18:35:21.063363405Z", "finished_at": "2026-11-14T15:27:29.730699Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Unified", "host_id": "f5245920-25e1-4d8b-b567-0f935961690d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:29.169468Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f5245920-25e1-4d8b-b567-0f935961690d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:35:21.063363+00	2026-11-14 15:27:29.73186+00	0	f	[]
80a216ed-7ad6-4e35-a8db-b2282d1a6e07	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "e8b2155f-5622-4046-aff2-4f8de2f868f0", "started_at": "2026-02-23T01:47:07.653242727Z", "finished_at": "2026-11-14T15:27:31.535968Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "SelfReport", "host_id": "49b8f936-67c8-4835-bc8c-5d897e43cdb0"}, "last_update_at": "2026-11-14T15:27:30.487926Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "49b8f936-67c8-4835-bc8c-5d897e43cdb0"}	Self Report — My Site	2026-02-23 01:47:07.653242+00	2026-11-14 15:27:31.537866+00	0	f	[]
023f71b5-e9d0-43c9-a2a8-0f238e2b06ed	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "fe678d3a-c2b0-4ee1-a912-7c07b0b6e1d7", "started_at": "2026-07-01T17:49:50.486612447Z", "finished_at": "2026-11-14T15:27:32.217773Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "28b6b18e-69b9-450c-a795-d14a671bb34f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:31.642336Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "28b6b18e-69b9-450c-a795-d14a671bb34f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:49:50.486612+00	2026-11-14 15:27:32.21884+00	0	f	[]
972fab36-71e1-4f90-8364-c95af91c30bb	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "7c20860a-b07c-4c4f-81bc-c7f44bee7a3d", "started_at": "2026-02-02T14:29:34.148055772Z", "finished_at": "2026-11-14T15:27:33.337124Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:32.350390Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-02 14:29:34.148055+00	2026-11-14 15:27:33.339932+00	0	f	[]
34637c81-7377-4b35-8ccf-ccbbab03c3c9	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "f36e9f5f-c1e1-4f52-b2b1-2de3128ff92c", "started_at": "2026-08-14T17:13:59.174760436Z", "finished_at": "2026-11-14T15:27:37.402873Z", "discovery_id": null, "daemon_version": "0.14.5", "discovery_type": {"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:36.938870Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-14 17:13:59.17476+00	2026-11-14 15:27:37.404053+00	0	f	[]
ecbdd873-1e5d-465d-9608-99f4e01d83a1	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "6465c546-3856-4e33-9cd0-5b8a8d99d55b", "started_at": "2026-03-31T18:32:33.461505006Z", "finished_at": "2026-11-14T15:27:29.733242Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Unified", "host_id": "5ef6e4af-f8ab-48d8-9b0e-6f4b93a49ef0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:29.114441Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "5ef6e4af-f8ab-48d8-9b0e-6f4b93a49ef0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:32:33.461505+00	2026-11-14 15:27:29.734474+00	0	f	[]
192aa5bb-019c-49d0-abc4-5968b95bf9e3	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "73503118-cf7f-4008-881e-7f39b457bf47", "started_at": "2026-07-01T17:49:57.001482027Z", "finished_at": "2026-11-14T15:27:32.215502Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "e61c5740-38d0-46d2-a2d6-64ce68e6606f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:31.762346Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e61c5740-38d0-46d2-a2d6-64ce68e6606f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:49:57.001482+00	2026-11-14 15:27:32.216673+00	0	f	[]
3b5c29a7-7c83-4594-8f1c-15f3935c4e69	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "b049b806-80fc-4dc5-9400-f19f9a9857ff", "started_at": "2026-07-01T17:49:54.577783598Z", "finished_at": "2026-11-14T15:27:32.222073Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "37d3a1d8-b76b-4e2e-96e0-74362358f4f4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:31.715842Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "37d3a1d8-b76b-4e2e-96e0-74362358f4f4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:49:54.577783+00	2026-11-14 15:27:32.223148+00	0	f	[]
0cf30c06-a1e6-42b3-bd70-49148b3dae63	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "bd93b206-4e22-4e0a-9dd0-0f38664a4247", "started_at": "2026-02-02T14:21:21.225991527Z", "finished_at": "2026-11-14T15:27:33.324488Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:32.300503Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-02 14:21:21.225991+00	2026-11-14 15:27:33.327084+00	0	f	[]
7aa7fe8b-e548-42f9-929d-47e76de11392	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "485c7d46-bc1d-4c32-a425-5904751efe34", "started_at": "2026-10-03T03:55:31.060040280Z", "finished_at": "2026-11-14T15:27:33.334192Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:32.869718Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-03 03:55:31.06004+00	2026-11-14 15:27:33.336009+00	0	f	[]
036b75cc-2c01-4bfa-9717-1323b77528c4	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "032af69a-4335-4c54-ac9c-8941c44050a3", "started_at": "2026-03-31T18:35:21.335253166Z", "finished_at": "2026-11-14T15:27:29.735588Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:29.175218Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:35:21.335253+00	2026-11-14 15:27:29.736769+00	0	f	[]
ab9a4679-a750-4a9e-98bc-18b1228b3cc2	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "adfad9b0-e25d-4bd4-88bc-e0ecc0f3f4a4", "started_at": "2026-01-26T13:51:30.181086877Z", "finished_at": "2026-11-14T15:27:29.737848Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "SelfReport", "host_id": "01d97cc8-84d8-4806-877f-52bfd29791f8"}, "last_update_at": "2026-11-14T15:27:29.120302Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "01d97cc8-84d8-4806-877f-52bfd29791f8"}	Self Report — My Site	2026-01-26 13:51:30.181086+00	2026-11-14 15:27:29.739816+00	0	f	[]
d20e30f1-132a-4c04-ad88-58de72486d18	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "2abdbee0-ac03-472c-b137-4321f33f03ca", "started_at": "2026-03-31T18:35:25.967010190Z", "finished_at": "2026-11-14T15:27:29.741274Z", "discovery_id": null, "daemon_version": "0.15.2", "discovery_type": {"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:29.262482Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-31 18:35:25.96701+00	2026-11-14 15:27:29.742459+00	0	f	[]
78153a22-7aa2-49c1-8979-d0f9cc574e30	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "cd493a46-8909-4378-b924-ae1650a0f5af", "started_at": "2026-03-26T01:31:45.021343211Z", "finished_at": "2026-11-14T15:27:30.465657Z", "discovery_id": null, "daemon_version": "0.14.7", "discovery_type": {"type": "Unified", "host_id": "711a5546-8640-4ee2-bf0b-8b268c4e2044", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:29.762491Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "711a5546-8640-4ee2-bf0b-8b268c4e2044", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-26 01:31:45.021343+00	2026-11-14 15:27:30.466879+00	0	f	[]
e2d5bb17-95e6-4ea6-adb1-922a740fbccc	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "ef1a4d64-afd3-4762-b6ae-2ff80e8dbc2f", "started_at": "2026-03-26T01:38:51.825984709Z", "finished_at": "2026-11-14T15:27:30.468025Z", "discovery_id": null, "daemon_version": "0.14.7", "discovery_type": {"type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:29.874397Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-26 01:38:51.825984+00	2026-11-14 15:27:30.469181+00	0	f	[]
9f4c7388-db6d-47cf-80fe-afb54a782b8f	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "c17fbe8e-8128-45b1-b2b5-afef7e2c082f", "started_at": "2026-02-23T01:47:37.649663614Z", "finished_at": "2026-11-14T15:27:31.532489Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:30.542739Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-23 01:47:37.649663+00	2026-11-14 15:27:31.534497+00	0	f	[]
516a20ce-dcd8-48ae-87d4-e1bc5a54bf7b	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "9c3d4777-a8a7-40f4-9070-c83a719cd6b6", "started_at": "2026-07-23T13:33:48.918734857Z", "finished_at": "2026-11-14T15:27:35.635991Z", "discovery_id": null, "daemon_version": "0.14.2", "discovery_type": {"type": "Unified", "host_id": "95fbda32-d081-4ef7-86c5-ac175f9995b6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:34.643443Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "95fbda32-d081-4ef7-86c5-ac175f9995b6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-23 13:33:48.918734+00	2026-11-14 15:27:35.637199+00	0	f	[]
38cc44de-cc33-46d1-b6fe-bb0e8aa16bc9	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "d033c5f0-e137-4b0a-b42e-78daa430f113", "started_at": "2026-07-23T13:32:18.163549314Z", "finished_at": "2026-11-14T15:27:35.646884Z", "discovery_id": null, "daemon_version": "0.14.2", "discovery_type": {"type": "Unified", "host_id": "c37e16d2-78cb-4ad7-bffc-f8eaec8ac967", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:34.590968Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c37e16d2-78cb-4ad7-bffc-f8eaec8ac967", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-23 13:32:18.163549+00	2026-11-14 15:27:35.647968+00	0	f	[]
6e35ccce-5c24-48a9-98d7-613dcf83b377	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "ee9f5e06-197c-4fbf-9d68-0ac7d7ec929e", "started_at": "2026-06-28T16:07:05.924226987Z", "finished_at": "2026-11-14T15:27:36.859143Z", "discovery_id": null, "daemon_version": "0.17.10", "discovery_type": {"type": "Unified", "host_id": "8d22037e-273c-4a29-9c5c-cdaaefe74227", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:36.250983Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8d22037e-273c-4a29-9c5c-cdaaefe74227", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:07:05.924226+00	2026-11-14 15:27:36.86025+00	0	f	[]
18940b40-880d-4d0a-a3cb-2f2b711448b2	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "eb250b58-76a9-4335-a813-c2d71a75e483", "started_at": "2026-02-18T17:07:18.061512748Z", "finished_at": "2026-11-14T15:27:39.044471Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "SelfReport", "host_id": "25a3d274-a408-4ec8-b1fd-69eb88c7674d"}, "last_update_at": "2026-11-14T15:27:38.020036Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "25a3d274-a408-4ec8-b1fd-69eb88c7674d"}	Self Report — My Site	2026-02-18 17:07:18.061512+00	2026-11-14 15:27:39.046513+00	0	f	[]
f1ace54e-ed7c-4fef-beb6-04db22552897	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "0073f594-456f-4861-8285-c8eb1345c48c", "started_at": "2026-10-25T01:45:04.782037127Z", "finished_at": "2026-11-14T15:27:31.538859Z", "discovery_id": null, "daemon_version": "0.17.2", "discovery_type": {"type": "Unified", "host_id": "2d0259ed-6b96-4930-847a-fa3ebdbf38a3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:31.021263Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2d0259ed-6b96-4930-847a-fa3ebdbf38a3", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-25 01:45:04.782037+00	2026-11-14 15:27:31.539953+00	0	f	[]
eac968c7-256e-4c1a-b341-4a22ca9b0e4c	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "bb0bdfdc-1516-4579-a727-079afeb571ea", "started_at": "2026-07-01T17:46:57.844855087Z", "finished_at": "2026-11-14T15:27:32.219848Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "23bba0c3-a3f9-49d7-ac3c-8696037b8b40", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:31.602452Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "23bba0c3-a3f9-49d7-ac3c-8696037b8b40", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:46:57.844855+00	2026-11-14 15:27:32.22104+00	0	f	[]
4937d3fd-4ad6-4dc5-b9ec-bde9fc05b536	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "52a4d1e0-c3b9-4b20-a4e8-0196198a1c15", "started_at": "2026-02-02T14:20:51.227652182Z", "finished_at": "2026-11-14T15:27:33.341041Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "SelfReport", "host_id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182"}, "last_update_at": "2026-11-14T15:27:32.249970Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "53cf4d7a-0ed6-452f-8bc6-4e4a3d611182"}	Self Report — My Site	2026-02-02 14:20:51.227652+00	2026-11-14 15:27:33.343201+00	0	f	[]
a9a791bb-5402-4075-902a-1f1abbed17ab	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "4db08e28-b456-4951-bd43-585d26cf9421", "started_at": "2026-01-07T19:14:20.246263005Z", "finished_at": "2026-11-14T15:27:35.638304Z", "discovery_id": null, "daemon_version": "0.14.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:35.171148Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-07 19:14:20.246263+00	2026-11-14 15:27:35.640247+00	0	f	[]
80d7f2fd-3556-4899-a056-0f10dc840f22	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "833d88c3-2acf-4989-86de-466d44e47a7e", "started_at": "2026-10-14T05:44:39.739395480Z", "finished_at": "2026-11-14T15:27:40.840149Z", "discovery_id": null, "daemon_version": "0.17.0", "discovery_type": {"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:40.378094Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-14 05:44:39.739395+00	2026-11-14 15:27:40.841259+00	0	f	[]
b912fcb6-75a0-4321-80d3-cd0ea938cd0f	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "e5274c5c-4478-40d5-86c6-aabbe025f093", "started_at": "2026-02-02T19:01:57.807520644Z", "finished_at": "2026-11-14T15:27:32.224186Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "SelfReport", "host_id": "37e5888d-4a4e-4ca3-908c-a8939cc75bdd"}, "last_update_at": "2026-11-14T15:27:31.607736Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "37e5888d-4a4e-4ca3-908c-a8939cc75bdd"}	Self Report — My Site	2026-02-02 19:01:57.80752+00	2026-11-14 15:27:32.22608+00	0	f	[]
9ff4f7d3-2e29-4ea0-b1a9-9e1f6e64bbdd	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "d2c685dd-56ad-4d49-9d36-c64b0483a15a", "started_at": "2026-07-01T17:49:49.938683105Z", "finished_at": "2026-11-14T15:27:32.231209Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "5a5b21eb-2566-4e13-83d8-00cb2675bde6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:31.631839Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "5a5b21eb-2566-4e13-83d8-00cb2675bde6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:49:49.938683+00	2026-11-14 15:27:32.232221+00	0	f	[]
4f3d91c1-046a-4295-afd2-5312182663b7	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "a8a4a82b-01b9-4d9b-9bf2-cc44f54ff1d3", "started_at": "2026-10-03T03:54:08.695159586Z", "finished_at": "2026-11-14T15:27:33.331557Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Unified", "host_id": "0e0131ba-ba9b-4828-813f-4a8c181f5836", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:32.796208Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "0e0131ba-ba9b-4828-813f-4a8c181f5836", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-03 03:54:08.695159+00	2026-11-14 15:27:33.333045+00	0	f	[]
71b0c212-9234-49b4-a13e-fd77f7d2ff52	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "937bc019-8b5b-4d82-b8f2-134c36a700a8", "started_at": "2026-07-05T00:06:52.008699875Z", "finished_at": "2026-11-14T15:27:34.028205Z", "discovery_id": null, "daemon_version": "0.14.14", "discovery_type": {"type": "Unified", "host_id": "bb6764eb-c535-449f-90e5-a950f9304f32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:33.595641Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bb6764eb-c535-449f-90e5-a950f9304f32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-05 00:06:52.008699+00	2026-11-14 15:27:34.029459+00	0	f	[]
40779bcd-3678-43e9-92cf-5bec49382730	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "f0555e1a-a4e0-4d26-b3e1-2a1c581c337d", "started_at": "2026-02-09T15:41:37.848773295Z", "finished_at": "2026-11-14T15:27:36.856388Z", "discovery_id": null, "daemon_version": "0.17.10", "discovery_type": {"type": "SelfReport", "host_id": "9e1eeb2e-e918-4dea-a437-6a61a65f4497"}, "last_update_at": "2026-11-14T15:27:35.665581Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "9e1eeb2e-e918-4dea-a437-6a61a65f4497"}	Self Report — My Site	2026-02-09 15:41:37.848773+00	2026-11-14 15:27:36.858228+00	0	f	[]
046ddfee-7136-4a86-9324-05735da65c03	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "bbdc17b9-0273-4a91-8a0e-268a6fcf0277", "started_at": "2026-02-26T00:43:32.254923713Z", "finished_at": "2026-11-14T15:27:46.511876Z", "discovery_id": null, "daemon_version": "0.14.13", "discovery_type": {"type": "SelfReport", "host_id": "4c158030-00c1-49eb-b7e2-4e803057a3dd"}, "last_update_at": "2026-11-14T15:27:45.482948Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "4c158030-00c1-49eb-b7e2-4e803057a3dd"}	Self Report — My Site	2026-02-26 00:43:32.254923+00	2026-11-14 15:27:46.514505+00	0	f	[]
93cba5bd-b88e-43be-870c-c9c994ea318c	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "34fe487d-22ef-418c-9099-42fa3376ded0", "started_at": "2026-07-01T17:49:52.939865021Z", "finished_at": "2026-11-14T15:27:32.227076Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "e9d19327-eb77-47c7-85d4-e590c7aa90a1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:31.684755Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e9d19327-eb77-47c7-85d4-e590c7aa90a1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:49:52.939865+00	2026-11-14 15:27:32.228163+00	0	f	[]
ee221183-f13c-4c5f-ad77-a26b4992d5ae	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "2fdd805f-0e91-427a-a614-e15a2a15a0f0", "started_at": "2026-07-05T00:02:02.946186416Z", "finished_at": "2026-11-14T15:27:34.026157Z", "discovery_id": null, "daemon_version": "0.14.14", "discovery_type": {"type": "Unified", "host_id": "4ad32cd4-5017-4aa6-81fb-2a624a987ac2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:33.363995Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "4ad32cd4-5017-4aa6-81fb-2a624a987ac2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-05 00:02:02.946186+00	2026-11-14 15:27:34.027238+00	0	f	[]
5585d0d1-1b5b-4471-9868-c1adc104571d	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "b43a3d5a-af6a-42f8-bf72-70c87206ab39", "started_at": "2026-07-22T23:04:06.688284640Z", "finished_at": "2026-11-14T15:27:35.633589Z", "discovery_id": null, "daemon_version": "0.14.2", "discovery_type": {"type": "Unified", "host_id": "ea19a9cd-5a65-4620-bff8-eb890602e187", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:34.649230Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "ea19a9cd-5a65-4620-bff8-eb890602e187", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-22 23:04:06.688284+00	2026-11-14 15:27:35.634823+00	0	f	[]
a091b3e1-0900-4c54-9ffa-0ca07576d302	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "6808c839-ea5d-4f6e-9662-60f24a6dd0c7", "started_at": "2026-03-06T07:04:08.169432273Z", "finished_at": "2026-11-14T15:27:35.644178Z", "discovery_id": null, "daemon_version": "0.14.2", "discovery_type": {"type": "SelfReport", "host_id": "07ca9676-4995-4fd4-b0a3-c60b0b356658"}, "last_update_at": "2026-11-14T15:27:34.047163Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "07ca9676-4995-4fd4-b0a3-c60b0b356658"}	Self Report — My Site	2026-03-06 07:04:08.169432+00	2026-11-14 15:27:35.646005+00	0	f	[]
34484763-de4b-4b95-816b-4ed942195713	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "6a03df7f-6023-4f74-893b-f9bb39cb8d98", "started_at": "2026-10-29T03:47:06.824079209Z", "finished_at": "2026-11-14T15:27:42.065217Z", "discovery_id": null, "daemon_version": "0.19.3", "discovery_type": {"type": "Unified", "host_id": "f591b8f3-e44c-4fab-bb81-9b67c29df370", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:41.549394Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f591b8f3-e44c-4fab-bb81-9b67c29df370", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-29 03:47:06.824079+00	2026-11-14 15:27:42.0663+00	0	f	[]
4f54b049-39e4-4409-963d-c58889bf660f	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "5b2fda95-677f-4f93-a93f-daca41bf3af3", "started_at": "2026-07-01T17:45:06.329649066Z", "finished_at": "2026-11-14T15:27:32.229200Z", "discovery_id": null, "daemon_version": "0.14.1", "discovery_type": {"type": "Unified", "host_id": "e56072d7-8284-4090-9a48-d04892a27b80", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:31.558101Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e56072d7-8284-4090-9a48-d04892a27b80", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-01 17:45:06.329649+00	2026-11-14 15:27:32.230227+00	0	f	[]
2563c91b-3542-46b2-aaf7-bd6e7f252121	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "b9089e15-81e6-435a-90fe-db5b24561924", "started_at": "2026-02-09T15:42:07.840272212Z", "finished_at": "2026-11-14T15:27:36.851145Z", "discovery_id": null, "daemon_version": "0.17.10", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:35.713117Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-09 15:42:07.840272+00	2026-11-14 15:27:36.853169+00	0	f	[]
9eb4f392-25c7-4ccb-8fc8-69e96b8067e2	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "47b82122-5cf6-42a8-bf36-f15c9a6582f0", "started_at": "2026-04-28T19:50:40.314334020Z", "finished_at": "2026-11-14T15:27:39.740834Z", "discovery_id": null, "daemon_version": "0.18.0", "discovery_type": {"type": "Unified", "host_id": "689f2fa8-dc17-4321-b680-7af464e5004f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:39.081641Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "689f2fa8-dc17-4321-b680-7af464e5004f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-28 19:50:40.314334+00	2026-11-14 15:27:39.741978+00	0	f	[]
f50ed0e4-de33-41b5-a2a5-fe31577205eb	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "2453f28d-e87c-4753-80ad-0416d47c71f8", "started_at": "2026-10-11T23:59:48.356682576Z", "finished_at": "2026-11-14T15:27:40.287499Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:39.827010Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-11 23:59:48.356682+00	2026-11-14 15:27:40.288862+00	0	f	[]
3c346a09-973e-4e7f-aa66-244234e07ea6	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "8473fe97-6e86-4b62-9b6d-2cd1349deeb8", "started_at": "2026-07-07T22:29:34.241374934Z", "finished_at": "2026-11-14T15:27:43.299032Z", "discovery_id": null, "daemon_version": "0.17.6", "discovery_type": {"type": "Unified", "host_id": "bb32f4ef-dc8e-4c79-89c6-04915971d7b0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:42.660922Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "bb32f4ef-dc8e-4c79-89c6-04915971d7b0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-07 22:29:34.241374+00	2026-11-14 15:27:43.300161+00	0	f	[]
7b4f8fb4-8872-45b3-be6b-097dce06a3c0	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "20717e93-7bdb-4413-9112-9ba79c38289f", "started_at": "2026-10-03T03:59:01.064308022Z", "finished_at": "2026-11-14T15:27:33.328335Z", "discovery_id": null, "daemon_version": "0.17.3", "discovery_type": {"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:32.875189Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8cbf3b63-69a1-4338-ad0d-29660d89568d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-03 03:59:01.064308+00	2026-11-14 15:27:33.32998+00	0	f	[]
a7235791-5fbb-49e0-8faa-e607cfc8b980	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "feba9c5a-f04c-4e5c-97cd-994c4dffc382", "started_at": "2026-07-05T00:03:52.021458299Z", "finished_at": "2026-11-14T15:27:34.023862Z", "discovery_id": null, "daemon_version": "0.14.14", "discovery_type": {"type": "Unified", "host_id": "f1a03331-5149-4a21-91ed-42d9fd821651", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:33.411940Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f1a03331-5149-4a21-91ed-42d9fd821651", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-05 00:03:52.021458+00	2026-11-14 15:27:34.025015+00	0	f	[]
2ba57e22-4d24-4315-9791-7855a897fc31	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "06f71522-f4a4-4d87-9785-a442e797d994", "started_at": "2026-06-28T16:02:26.238749468Z", "finished_at": "2026-11-14T15:27:36.854281Z", "discovery_id": null, "daemon_version": "0.17.10", "discovery_type": {"type": "Unified", "host_id": "793047bd-2b38-4e62-8dd9-36ab1aa73797", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:36.167228Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "793047bd-2b38-4e62-8dd9-36ab1aa73797", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-28 16:02:26.238749+00	2026-11-14 15:27:36.85534+00	0	f	[]
68e2db79-eb62-43ce-ad11-9a58ad833e58	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "34d1feb3-276c-4ea9-b889-7eab39b67a1c", "started_at": "2026-02-23T00:07:49.639240713Z", "finished_at": "2026-11-14T15:27:39.053084Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:38.009759Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-23 00:07:49.63924+00	2026-11-14 15:27:39.054927+00	0	f	[]
f395feec-7fb8-4bb2-b1bf-fef1c7113849	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "69f77b5e-1350-4d55-ae3e-a0aa03bb47cd", "started_at": "2026-08-25T02:50:04.633793664Z", "finished_at": "2026-11-14T15:27:39.060934Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "Unified", "host_id": "802b4d69-588c-452a-83c3-1e47ca5d84d1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:38.587052Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "802b4d69-588c-452a-83c3-1e47ca5d84d1", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 02:50:04.633793+00	2026-11-14 15:27:39.062033+00	0	f	[]
baeed963-3048-435c-b9b4-0364c80519ef	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "b5591b69-0dfe-41fa-87c1-3d5e140cca15", "started_at": "2026-03-06T07:04:38.164982988Z", "finished_at": "2026-11-14T15:27:35.641189Z", "discovery_id": null, "daemon_version": "0.14.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:34.095437Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-06 07:04:38.164982+00	2026-11-14 15:27:35.643071+00	0	f	[]
1e071a89-ab60-42eb-83f1-81f0b9ccf7c6	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "8c70cc2e-9120-44c5-8651-e6a5bad04bb9", "started_at": "2026-08-14T17:08:52.205106028Z", "finished_at": "2026-11-14T15:27:37.405234Z", "discovery_id": null, "daemon_version": "0.14.5", "discovery_type": {"type": "Unified", "host_id": "b2d748df-ea85-45ad-8e23-79946b4b5772", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:36.879682Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b2d748df-ea85-45ad-8e23-79946b4b5772", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-14 17:08:52.205106+00	2026-11-14 15:27:37.406463+00	0	f	[]
7ba585de-9f22-402a-aece-38284db4848d	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "e39a2b83-3f31-4943-abbc-ba31d154ce16", "started_at": "2026-02-11T02:46:23.000125066Z", "finished_at": "2026-11-14T15:27:39.050291Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "SelfReport", "host_id": "d4234791-d4ee-4257-a9d3-a257b5e1e3b9"}, "last_update_at": "2026-11-14T15:27:37.430121Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "d4234791-d4ee-4257-a9d3-a257b5e1e3b9"}	Self Report — My Site	2026-02-11 02:46:23.000125+00	2026-11-14 15:27:39.052171+00	0	f	[]
ddcc21cb-87f3-4fdc-a5fc-3f054377d729	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "13aa0c04-f207-47b3-9c03-0f3209efab76", "started_at": "2026-10-12T00:03:18.358288758Z", "finished_at": "2026-11-14T15:27:40.284981Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:39.832346Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a2f4c68b-164e-497e-8245-525705dbc63f", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-12 00:03:18.358288+00	2026-11-14 15:27:40.286205+00	0	f	[]
c35ef632-2fbb-4013-834b-d43e5f8d5bd1	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "d7b0c883-f1dd-45b9-8008-cb06d0194514", "started_at": "2026-10-14T05:43:36.061177700Z", "finished_at": "2026-11-14T15:27:40.837927Z", "discovery_id": null, "daemon_version": "0.17.0", "discovery_type": {"type": "Unified", "host_id": "d4dabb79-eb99-4045-8687-9ca8bd1e797a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:40.302583Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d4dabb79-eb99-4045-8687-9ca8bd1e797a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-14 05:43:36.061177+00	2026-11-14 15:27:40.839035+00	0	f	[]
01cfb6fc-b392-43ae-8de0-95ef8659e495	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "3326154e-ed93-4721-9226-b365a900ba24", "started_at": "2026-08-14T17:10:29.182979151Z", "finished_at": "2026-11-14T15:27:37.407538Z", "discovery_id": null, "daemon_version": "0.14.5", "discovery_type": {"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:36.933071Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6af9c31b-1876-455a-a2c0-4dcab5710ad7", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-14 17:10:29.182979+00	2026-11-14 15:27:37.408706+00	0	f	[]
a446b167-1a3e-4f87-8da2-901189bd8582	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "faf2ae16-9ccd-4ba2-8be6-077b30ff3ae7", "started_at": "2026-08-26T01:59:44.730528896Z", "finished_at": "2026-11-14T15:27:39.058708Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:38.581553Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-26 01:59:44.730528+00	2026-11-14 15:27:39.059831+00	0	f	[]
ca862f76-971c-4338-b9a2-5cf07780026c	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "c30ae4d6-aa01-4253-94db-acdabf3a54e4", "started_at": "2026-04-28T19:09:10.681879638Z", "finished_at": "2026-11-14T15:27:39.736499Z", "discovery_id": null, "daemon_version": "0.18.0", "discovery_type": {"type": "Unified", "host_id": "dd6f90d2-d1df-49c8-bba4-cf36f7343201", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:39.128012Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "dd6f90d2-d1df-49c8-bba4-cf36f7343201", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-28 19:09:10.681879+00	2026-11-14 15:27:39.737576+00	0	f	[]
96581324-9d63-4f41-a28f-c05412639b08	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "5d98fc6a-1bba-4985-a654-bf83407df1f2", "started_at": "2026-10-14T05:48:09.745795958Z", "finished_at": "2026-11-14T15:27:40.842334Z", "discovery_id": null, "daemon_version": "0.17.0", "discovery_type": {"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:40.383975Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "469e24e9-4644-4372-ba07-8915ce6219a9", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-14 05:48:09.745795+00	2026-11-14 15:27:40.843443+00	0	f	[]
8bde330f-1e45-4b8e-8470-a51c4ceff2a4	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "11299ee7-3101-4bae-be7a-097db8407dd7", "started_at": "2026-01-23T03:48:00.568581680Z", "finished_at": "2026-11-14T15:27:44.912230Z", "discovery_id": null, "daemon_version": "0.17.12", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:44.451101Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-23 03:48:00.568581+00	2026-11-14 15:27:44.91426+00	0	f	[]
3b59e38c-36d8-4ea6-b649-3b2b1fd6b5c3	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "13af9d3c-5935-4be7-90cf-7fb432a4db27", "started_at": "2026-08-06T01:38:34.928909970Z", "finished_at": "2026-11-14T15:27:37.409870Z", "discovery_id": null, "daemon_version": "0.14.5", "discovery_type": {"type": "Unified", "host_id": "6be44535-29d4-4940-88c3-917ce0d69222", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:36.944493Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6be44535-29d4-4940-88c3-917ce0d69222", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-06 01:38:34.928909+00	2026-11-14 15:27:37.411062+00	0	f	[]
544a630e-551f-4583-8934-4a09ddc1b1b4	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "cd16f43c-4365-49c5-8fee-d081f3b53cd9", "started_at": "2026-02-23T00:07:19.655637880Z", "finished_at": "2026-11-14T15:27:39.047507Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "SelfReport", "host_id": "abd72866-b3c5-4e9a-89f6-491d6af2797b"}, "last_update_at": "2026-11-14T15:27:37.963674Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "abd72866-b3c5-4e9a-89f6-491d6af2797b"}	Self Report — My Site	2026-02-23 00:07:19.655637+00	2026-11-14 15:27:39.04938+00	0	f	[]
38b06e8c-5dd1-40b5-b393-e12a64def7b1	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "8dab7a99-5955-498a-b597-5e7bb0e5d62a", "started_at": "2026-04-28T19:55:22.652516977Z", "finished_at": "2026-11-14T15:27:39.738655Z", "discovery_id": null, "daemon_version": "0.18.0", "discovery_type": {"type": "Unified", "host_id": "c16a9026-e029-4161-8963-20186c87d56c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:39.211590Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c16a9026-e029-4161-8963-20186c87d56c", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-28 19:55:22.652516+00	2026-11-14 15:27:39.739777+00	0	f	[]
be941728-2c94-4076-ad3e-d534254b0b91	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "7ad7978f-e4a7-4518-bb77-8a4c8c1dd0c6", "started_at": "2026-07-07T22:27:42.015566135Z", "finished_at": "2026-11-14T15:27:43.301229Z", "discovery_id": null, "daemon_version": "0.17.6", "discovery_type": {"type": "Unified", "host_id": "079502fe-1a72-44a6-ab38-040ead1355fb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:42.618328Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "079502fe-1a72-44a6-ab38-040ead1355fb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-07 22:27:42.015566+00	2026-11-14 15:27:43.302307+00	0	f	[]
e4e403b4-c6d4-4364-bb03-a5103d10cec0	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "b3287603-7c63-47ac-886a-4b6bb9cb2151", "started_at": "2026-08-01T21:25:32.482042816Z", "finished_at": "2026-11-14T15:27:44.915295Z", "discovery_id": null, "daemon_version": "0.17.12", "discovery_type": {"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:43.925952Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-01 21:25:32.482042+00	2026-11-14 15:27:44.916336+00	0	f	[]
36011d4f-4ff5-4c41-ac7a-e4aa4e544626	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "ae3a3d90-03a9-4961-a1d4-840e661d29a8", "started_at": "2026-02-11T02:46:52.998653452Z", "finished_at": "2026-11-14T15:27:39.041079Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:37.479031Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-11 02:46:52.998653+00	2026-11-14 15:27:39.043263+00	0	f	[]
6cd972f9-d545-4bf6-9643-3789524c505b	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "9e24c5ae-ab53-4e0f-93db-281f3614fd56", "started_at": "2026-10-11T23:58:36.269774581Z", "finished_at": "2026-11-14T15:27:40.290027Z", "discovery_id": null, "daemon_version": "0.18.1", "discovery_type": {"type": "Unified", "host_id": "8d1f0138-018b-4734-9af4-5f196dc2a028", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:39.752197Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8d1f0138-018b-4734-9af4-5f196dc2a028", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-11 23:58:36.269774+00	2026-11-14 15:27:40.291349+00	0	f	[]
923d721a-22b0-430f-a7f6-4a743218c8ab	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "bfc425bd-2840-4447-8d67-666671033cd6", "started_at": "2026-06-23T01:20:59.927972592Z", "finished_at": "2026-11-14T15:27:41.537463Z", "discovery_id": null, "daemon_version": "0.19.4", "discovery_type": {"type": "Unified", "host_id": "cf70c05a-1647-4958-806b-ff84119deb9a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:41.085495Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "cf70c05a-1647-4958-806b-ff84119deb9a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-23 01:20:59.927972+00	2026-11-14 15:27:41.538533+00	0	f	[]
2aac14da-dbb9-48b2-9017-9daa6642a0d2	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "675b82cb-527e-4541-8702-fef2a8eb5552", "started_at": "2026-08-25T02:55:08.502675512Z", "finished_at": "2026-11-14T15:27:45.457671Z", "discovery_id": null, "daemon_version": "0.14.8", "discovery_type": {"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:44.994846Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 02:55:08.502675+00	2026-11-14 15:27:45.458795+00	0	f	[]
bf6e2ec0-f310-4fa5-bcd5-dca41fcb3216	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "6bbb8f0d-ed69-4df8-81d6-8bc44cdf4380", "started_at": "2026-03-02T09:02:56.919081491Z", "finished_at": "2026-11-14T15:27:52.846152Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:51.178646Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-02 09:02:56.919081+00	2026-11-14 15:27:52.848183+00	0	f	[]
dcf81fc2-23d0-4801-ae2f-16ed5831728d	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "3bba0463-9815-4685-8b2e-93eacc318f67", "started_at": "2026-08-26T01:56:14.716285279Z", "finished_at": "2026-11-14T15:27:39.056668Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:38.575984Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2dc4cb80-0289-41fd-92b7-fc61c77ed3e6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-26 01:56:14.716285+00	2026-11-14 15:27:39.057718+00	0	f	[]
72357cc2-5034-4ce0-ae96-ea06f4316750	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "7c9364ea-b95f-4e96-b958-a71e1e3826df", "started_at": "2026-08-26T01:54:31.081144679Z", "finished_at": "2026-11-14T15:27:39.062951Z", "discovery_id": null, "daemon_version": "0.16.2", "discovery_type": {"type": "Unified", "host_id": "6a1dd7f3-9b36-49b1-b554-1ef783b14e5d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:38.502789Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "6a1dd7f3-9b36-49b1-b554-1ef783b14e5d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-26 01:54:31.081144+00	2026-11-14 15:27:39.064005+00	0	f	[]
7fe62f01-fa42-4c49-8f93-f172cfcfe0f9	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "f1a2c4ba-d124-4b5d-9448-729e2a28c186", "started_at": "2026-06-23T01:16:12.751429360Z", "finished_at": "2026-11-14T15:27:41.535117Z", "discovery_id": null, "daemon_version": "0.19.4", "discovery_type": {"type": "Unified", "host_id": "27a3457e-4127-4ee5-96e3-ba617595159b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:40.862318Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "27a3457e-4127-4ee5-96e3-ba617595159b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-06-23 01:16:12.751429+00	2026-11-14 15:27:41.536246+00	0	f	[]
29c458d3-0b1f-40dd-b0ff-beaab0cd97ab	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "1811938e-d0f9-4568-bc69-ef3f2931e9ef", "started_at": "2026-08-25T01:24:07.586140654Z", "finished_at": "2026-11-14T15:27:45.459940Z", "discovery_id": null, "daemon_version": "0.14.8", "discovery_type": {"type": "Unified", "host_id": "5f0170e1-a84c-4207-b391-c13297a54d48", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:45.000136Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "5f0170e1-a84c-4207-b391-c13297a54d48", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 01:24:07.58614+00	2026-11-14 15:27:45.461064+00	0	f	[]
1eb7f89d-f39d-4d52-8cea-9e9019ae9de7	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "00342129-a6d7-490e-84ba-2bd11b17a768", "started_at": "2026-02-26T00:44:02.257915046Z", "finished_at": "2026-11-14T15:27:46.508369Z", "discovery_id": null, "daemon_version": "0.14.13", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:45.532949Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-26 00:44:02.257915+00	2026-11-14 15:27:46.51069+00	0	f	[]
cc3dc6ba-b12f-4085-9ed4-97efb6b257e0	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "b18c8b1b-202b-4081-9911-d4f395fe9344", "started_at": "2026-10-27T05:03:46.928849924Z", "finished_at": "2026-11-14T15:27:42.598506Z", "discovery_id": null, "daemon_version": "0.17.4", "discovery_type": {"type": "Unified", "host_id": "8913babd-75d5-4960-a597-9eb267cea952", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:42.078207Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "8913babd-75d5-4960-a597-9eb267cea952", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-27 05:03:46.928849+00	2026-11-14 15:27:42.599718+00	0	f	[]
d8f0e76f-a26c-4c34-a7e9-9180a650fd8b	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "44932e97-76ed-4c08-b694-4e6aa6bca1be", "started_at": "2026-03-05T09:33:11.114723268Z", "finished_at": "2026-11-14T15:27:47.715333Z", "discovery_id": null, "daemon_version": "0.17.21", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:46.584187Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-05 09:33:11.114723+00	2026-11-14 15:27:47.717159+00	0	f	[]
ec0c5911-ae9f-4c6c-829d-35b9debdf812	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "8625c9a3-252c-4570-9b6a-e4459dde2e43", "started_at": "2026-10-05T14:18:58.075801174Z", "finished_at": "2026-11-14T15:27:48.252731Z", "discovery_id": null, "daemon_version": "0.19.6", "discovery_type": {"type": "Unified", "host_id": "38b72a6c-0aea-4147-b599-5af096f07623", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:47.729960Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "38b72a6c-0aea-4147-b599-5af096f07623", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-05 14:18:58.075801+00	2026-11-14 15:27:48.253901+00	0	f	[]
c70b5b09-0cc5-4230-814f-864c5a3d6310	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "df1323db-2f1d-4090-bd79-58d04eee33c1", "started_at": "2026-04-18T01:11:16.011978400Z", "finished_at": "2026-11-14T15:27:52.840970Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Unified", "host_id": "a6d5229d-0b50-4243-a00d-c3d95bb17a36", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:52.342099Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a6d5229d-0b50-4243-a00d-c3d95bb17a36", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:11:16.011978+00	2026-11-14 15:27:52.842152+00	0	f	[]
7a4e28c7-2368-4e81-bf7c-20e317130f86	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "2b6e6d22-4644-4cad-a850-04fb5896df10", "started_at": "2026-03-02T09:02:26.922912054Z", "finished_at": "2026-11-14T15:27:52.851510Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "SelfReport", "host_id": "b6dcbdab-8ba5-4b12-8dee-5177af2cf702"}, "last_update_at": "2026-11-14T15:27:51.130369Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "b6dcbdab-8ba5-4b12-8dee-5177af2cf702"}	Self Report — My Site	2026-03-02 09:02:26.922912+00	2026-11-14 15:27:52.85338+00	0	f	[]
ca6f0393-59fd-4ee2-bcfe-ad7e1d80017d	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "b3439cfa-af27-47ae-8b9d-8c79b9d149fa", "started_at": "2026-07-28T03:21:55.956404860Z", "finished_at": "2026-11-14T15:27:43.843964Z", "discovery_id": null, "daemon_version": "0.17.8", "discovery_type": {"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:43.367136Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-28 03:21:55.956404+00	2026-11-14 15:27:43.845073+00	0	f	[]
c95790c4-10fb-44a9-8374-5d3900ee15a0	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "69664faf-d5dd-440a-b554-0af052ce9005", "started_at": "2026-08-01T21:20:27.123050289Z", "finished_at": "2026-11-14T15:27:44.917332Z", "discovery_id": null, "daemon_version": "0.17.12", "discovery_type": {"type": "Unified", "host_id": "b6b48830-db6a-4d8f-8c70-db5516f3e458", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:43.867930Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b6b48830-db6a-4d8f-8c70-db5516f3e458", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-01 21:20:27.12305+00	2026-11-14 15:27:44.918589+00	0	f	[]
758e54a4-f2f1-4344-bd1a-4fc8d92b9646	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "42d8b7a7-6a5c-409b-b8c2-d33c0af1e9c0", "started_at": "2026-08-17T00:29:10.692540259Z", "finished_at": "2026-11-14T15:27:50.559023Z", "discovery_id": null, "daemon_version": "0.14.9", "discovery_type": {"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:50.099034Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-17 00:29:10.69254+00	2026-11-14 15:27:50.560337+00	0	f	[]
be9962de-8429-4341-ab55-f7fa72337788	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "bfb7da47-98f1-4957-9f13-5b4f6f440e6a", "started_at": "2026-03-08T06:53:48.342722812Z", "finished_at": "2026-11-14T15:27:59.038287Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "SelfReport", "host_id": "caeb1c88-3a79-4e24-8a5a-fedd841b3a98"}, "last_update_at": "2026-11-14T15:27:57.434726Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "caeb1c88-3a79-4e24-8a5a-fedd841b3a98"}	Self Report — My Site	2026-03-08 06:53:48.342722+00	2026-11-14 15:27:59.040078+00	0	f	[]
c54040ec-b440-4ec6-89a7-a1347701555e	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "b39cff2d-d385-4ab8-8097-66c2190dae09", "started_at": "2026-03-06T23:20:38.018012792Z", "finished_at": "2026-11-14T15:27:59.048723Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "SelfReport", "host_id": "6fd7ccd6-6ea7-4880-b5e5-ad545df36544"}, "last_update_at": "2026-11-14T15:27:56.890637Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "6fd7ccd6-6ea7-4880-b5e5-ad545df36544"}	Self Report — My Site	2026-03-06 23:20:38.018012+00	2026-11-14 15:27:59.05061+00	0	f	[]
3b2a676e-0bfd-4ea2-a279-c67798ad83f0	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "df28970e-d903-4f00-8b2a-1dfa823e410c", "started_at": "2026-07-28T03:25:25.938837651Z", "finished_at": "2026-11-14T15:27:43.846237Z", "discovery_id": null, "daemon_version": "0.17.8", "discovery_type": {"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:43.373128Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e936ae7b-5e53-48ac-ab95-8478021a9cda", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-28 03:25:25.938837+00	2026-11-14 15:27:43.847408+00	0	f	[]
a8cbf73e-e4f0-459b-8796-c8f5d367c8eb	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "a1b350a1-59b8-4caf-ae55-c9f3f8ca89fb", "started_at": "2026-03-05T09:32:41.117544068Z", "finished_at": "2026-11-14T15:27:47.712239Z", "discovery_id": null, "daemon_version": "0.17.21", "discovery_type": {"type": "SelfReport", "host_id": "2a76c091-9847-4916-bc73-8668758f6f7d"}, "last_update_at": "2026-11-14T15:27:46.534946Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "2a76c091-9847-4916-bc73-8668758f6f7d"}	Self Report — My Site	2026-03-05 09:32:41.117544+00	2026-11-14 15:27:47.714363+00	0	f	[]
2bb4cda2-2052-4368-a1f1-d9969af6476f	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "01486396-5c85-4268-b3c8-b83da1042232", "started_at": "2026-09-15T05:42:19.268112358Z", "finished_at": "2026-11-14T15:27:50.019982Z", "discovery_id": null, "daemon_version": "0.17.11", "discovery_type": {"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:49.546611Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-15 05:42:19.268112+00	2026-11-14 15:27:50.02119+00	0	f	[]
85a7e150-b34c-48ac-b4d1-301ad2cae7ee	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "289721de-ae03-40cd-95d5-6af379282969", "started_at": "2026-09-15T05:37:18.553708846Z", "finished_at": "2026-11-14T15:27:50.026688Z", "discovery_id": null, "daemon_version": "0.17.11", "discovery_type": {"type": "Unified", "host_id": "2a9b1a5e-51d9-4f13-b6e4-0b1e3e27e0dc", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:49.468980Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2a9b1a5e-51d9-4f13-b6e4-0b1e3e27e0dc", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-15 05:37:18.553708+00	2026-11-14 15:27:50.027858+00	0	f	[]
4b4e9218-6701-4adc-9395-16a93022e0d8	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "580379b9-0101-428f-baef-843afbc5dfee", "started_at": "2026-01-26T14:03:54.326189222Z", "finished_at": "2026-11-14T15:27:52.843239Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:51.720030Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-26 14:03:54.326189+00	2026-11-14 15:27:52.845207+00	0	f	[]
7dcb886b-6e5e-43e9-901a-5a50717d1a51	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "f9ec8267-d3bb-4170-80d5-9c5a41add7eb", "started_at": "2026-07-28T03:20:23.724249500Z", "finished_at": "2026-11-14T15:27:43.848483Z", "discovery_id": null, "daemon_version": "0.17.8", "discovery_type": {"type": "Unified", "host_id": "24f17e21-d958-4c17-ad9d-3f058f1bfd07", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:43.322409Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "24f17e21-d958-4c17-ad9d-3f058f1bfd07", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-07-28 03:20:23.724249+00	2026-11-14 15:27:43.849591+00	0	f	[]
38e00eef-a524-4593-aaba-39272c28f386	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "5d811057-9d57-4a37-997d-6d9ff0bde3f4", "started_at": "2026-10-05T14:20:20.057223827Z", "finished_at": "2026-11-14T15:27:48.250359Z", "discovery_id": null, "daemon_version": "0.19.6", "discovery_type": {"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:47.799717Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-05 14:20:20.057223+00	2026-11-14 15:27:48.251516+00	0	f	[]
c9fc0269-bfc2-4c96-aa32-ec84211389bf	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "271037a5-7bfa-49d0-bff8-3879ffaa6d30", "started_at": "2026-11-04T16:49:09.224254Z", "finished_at": "2026-11-14T15:27:48.787378Z", "discovery_id": null, "daemon_version": "0.15.5", "discovery_type": {"type": "Unified", "host_id": "f35cae23-aec3-4e90-8c72-278e1d887c57", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:48.267173Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "f35cae23-aec3-4e90-8c72-278e1d887c57", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-11-04 16:49:09.224254+00	2026-11-14 15:27:48.788615+00	0	f	[]
6332b27a-d4b3-4714-96d2-4efdd68c7ab8	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "457a544b-3c09-45bf-90d1-ab9e5e4a144d", "started_at": "2026-04-01T14:32:54.762125416Z", "finished_at": "2026-11-14T15:27:49.449116Z", "discovery_id": null, "daemon_version": "0.17.15", "discovery_type": {"type": "Unified", "host_id": "a33f8415-bb39-43da-81c3-dfafa82595da", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:48.807278Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a33f8415-bb39-43da-81c3-dfafa82595da", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-01 14:32:54.762125+00	2026-11-14 15:27:49.450332+00	0	f	[]
d4032c33-21d1-4b6a-b8e3-24a288b10bb2	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "80837ade-3bd8-4bfa-a94b-ec249e6517fa", "started_at": "2026-03-11T06:19:54.610393414Z", "finished_at": "2026-11-14T15:27:54.129980Z", "discovery_id": null, "daemon_version": "0.19.5", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:53.556398Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-11 06:19:54.610393+00	2026-11-14 15:27:54.132004+00	0	f	[]
958c935f-57b7-4640-93fe-fcab9cad0242	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "7e06b95e-3b67-41c4-9049-978411591e7c", "started_at": "2026-08-01T21:22:02.500665824Z", "finished_at": "2026-11-14T15:27:44.919677Z", "discovery_id": null, "daemon_version": "0.17.12", "discovery_type": {"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:43.920082Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "b9ed7c60-90f9-4aea-ba6d-089f82acd280", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-01 21:22:02.500665+00	2026-11-14 15:27:44.920781+00	0	f	[]
456884a9-20e8-48b7-ad10-5f7fead5a9f8	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "901cf5c8-1989-4bf1-bf5d-e587ee211cda", "started_at": "2026-03-26T17:23:47.082705682Z", "finished_at": "2026-11-14T15:27:47.718152Z", "discovery_id": null, "daemon_version": "0.17.21", "discovery_type": {"type": "Unified", "host_id": "2bbce8eb-5555-41ef-b999-68a5af971278", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:47.073528Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2bbce8eb-5555-41ef-b999-68a5af971278", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-26 17:23:47.082705+00	2026-11-14 15:27:47.719209+00	0	f	[]
a7b9f08a-66d0-4c6f-9657-dccb6a0bf9a5	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "9478634a-d980-4211-b0fa-ac0e884de1ad", "started_at": "2026-10-05T14:23:50.058326236Z", "finished_at": "2026-11-14T15:27:48.254933Z", "discovery_id": null, "daemon_version": "0.19.6", "discovery_type": {"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:47.805324Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a93aa532-b5e3-4e61-b44e-7701d1d5416b", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-05 14:23:50.058326+00	2026-11-14 15:27:48.256027+00	0	f	[]
e25fa5e0-96fb-428a-b5a6-c9b9d7176f6c	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "d36fca8d-dbbc-4cad-a19f-f495eacf6234", "started_at": "2026-02-28T21:19:27.224801293Z", "finished_at": "2026-11-14T15:27:59.026924Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:55.298385Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-28 21:19:27.224801+00	2026-11-14 15:27:59.028756+00	0	f	[]
32165482-cf3d-492f-8b71-09d15c898cf2	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "dae6845a-5c1c-4537-8549-2cb0bb052dd7", "started_at": "2026-04-20T18:00:40.095617293Z", "finished_at": "2026-11-14T15:28:00.320518Z", "discovery_id": null, "daemon_version": "0.14.19", "discovery_type": {"type": "Unified", "host_id": "c1107482-106c-4d18-8ee2-07081aef2afb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:59.178714Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "c1107482-106c-4d18-8ee2-07081aef2afb", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-20 18:00:40.095617+00	2026-11-14 15:28:00.321613+00	0	f	[]
eb8865c6-4429-4b1c-9c78-c10ee4fdae18	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "6fa2c508-5f31-44cd-9153-00c853afe9a7", "started_at": "2026-08-25T02:51:38.519652606Z", "finished_at": "2026-11-14T15:27:45.462087Z", "discovery_id": null, "daemon_version": "0.14.8", "discovery_type": {"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:44.989483Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "0d2cffae-1d39-467b-91f5-fa201d8bec32", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-25 02:51:38.519652+00	2026-11-14 15:27:45.463239+00	0	f	[]
605cc52a-eb22-48f7-9e39-bc8e8f3f64e3	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "6740c1be-987b-48e8-b547-871b2ea811c7", "started_at": "2026-10-23T00:17:23.541441434Z", "finished_at": "2026-11-14T15:27:46.505781Z", "discovery_id": null, "daemon_version": "0.14.13", "discovery_type": {"type": "Unified", "host_id": "9134f9b8-a2ca-4af9-9f9d-f98529385af4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:45.986106Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9134f9b8-a2ca-4af9-9f9d-f98529385af4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-23 00:17:23.541441+00	2026-11-14 15:27:46.507014+00	0	f	[]
2c3c9394-6751-44ec-853b-2d76f46033af	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "c8b7e98c-df16-4cc6-88f2-82866bbb959f", "started_at": "2026-09-15T05:38:49.198136490Z", "finished_at": "2026-11-14T15:27:50.024587Z", "discovery_id": null, "daemon_version": "0.17.11", "discovery_type": {"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:49.540972Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "07c81ee1-8335-4c3c-9ae4-9df184fe7497", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-15 05:38:49.198136+00	2026-11-14 15:27:50.025714+00	0	f	[]
7d66d4cc-dcb7-4595-909b-efdaf916ee3f	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "2b8a5125-8bbc-4f50-9fd7-baad3f93a89f", "started_at": "2026-08-17T00:24:03.071614457Z", "finished_at": "2026-11-14T15:27:50.563992Z", "discovery_id": null, "daemon_version": "0.14.9", "discovery_type": {"type": "Unified", "host_id": "51d0c941-a5a3-4d6c-b3a9-5afbcaed2d3d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:50.045487Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "51d0c941-a5a3-4d6c-b3a9-5afbcaed2d3d", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-17 00:24:03.071614+00	2026-11-14 15:27:50.565115+00	0	f	[]
c574b7cb-ba22-4ba9-a32f-6821c3c731e0	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "1a2a6d39-262b-45ff-80c8-1f8742f766af", "started_at": "2026-02-28T03:10:51.549945495Z", "finished_at": "2026-11-14T15:27:52.854315Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "SelfReport", "host_id": "422f12c5-aed6-4cdc-aab7-a281212ef855"}, "last_update_at": "2026-11-14T15:27:50.584310Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "422f12c5-aed6-4cdc-aab7-a281212ef855"}	Self Report — My Site	2026-02-28 03:10:51.549945+00	2026-11-14 15:27:52.856119+00	0	f	[]
c891d60f-5670-4453-9977-0281fc7de4d2	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "21fc2969-cba5-46cf-b381-1978e0c2959e", "started_at": "2026-09-03T17:55:43.577416616Z", "finished_at": "2026-11-14T15:27:50.022297Z", "discovery_id": null, "daemon_version": "0.17.11", "discovery_type": {"type": "Unified", "host_id": "3fd2970b-7217-4821-8bd7-6985174b6a22", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:49.552118Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "3fd2970b-7217-4821-8bd7-6985174b6a22", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-03 17:55:43.577416+00	2026-11-14 15:27:50.023543+00	0	f	[]
c914b930-e7bc-419b-992d-536a3e8e38f0	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "c50795be-e20e-483e-857d-f04a9f103f60", "started_at": "2026-08-17T00:25:40.701352183Z", "finished_at": "2026-11-14T15:27:50.561670Z", "discovery_id": null, "daemon_version": "0.14.9", "discovery_type": {"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:50.093844Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "72944247-e7a7-4b25-9a55-37b0e5183ecd", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-17 00:25:40.701352+00	2026-11-14 15:27:50.56286+00	0	f	[]
61286f5e-d330-49c3-ae22-219934098630	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "030a56ec-f416-4d7f-864e-fde8841a379c", "started_at": "2026-09-03T17:57:32.046576969Z", "finished_at": "2026-11-14T15:27:55.232409Z", "discovery_id": null, "daemon_version": "0.14.10", "discovery_type": {"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:54.769646Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-03 17:57:32.046576+00	2026-11-14 15:27:55.233515+00	0	f	[]
6096d3fb-8a92-496f-b0d1-99a7e1c2d872	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "a49f4dc9-0185-4793-9f4f-0843c7021f81", "started_at": "2026-03-08T06:54:18.293920451Z", "finished_at": "2026-11-14T15:27:59.023715Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:57.480665Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-08 06:54:18.29392+00	2026-11-14 15:27:59.0259+00	0	f	[]
ee1afcbc-dece-4952-bb1d-a561f4b76092	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "9dc720f8-1115-45ea-8b6d-f2e58154594d", "started_at": "2026-04-18T01:07:01.115515322Z", "finished_at": "2026-11-14T15:27:52.849124Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Unified", "host_id": "1ce3a67d-4aeb-485a-9976-c67c4801fd30", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:52.185302Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "1ce3a67d-4aeb-485a-9976-c67c4801fd30", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-18 01:07:01.115515+00	2026-11-14 15:27:52.850324+00	0	f	[]
260ae5f8-d306-48b9-a29b-25521b2e0c47	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "71cbed11-9ec5-4ae6-a3ce-3767032c2ab8", "started_at": "2026-04-17T22:09:29.738977360Z", "finished_at": "2026-11-14T15:27:52.860262Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Unified", "host_id": "e4ddba2b-4c4f-4f89-a625-8f3f022d7581", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:52.233368Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "e4ddba2b-4c4f-4f89-a625-8f3f022d7581", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-17 22:09:29.738977+00	2026-11-14 15:27:52.861345+00	0	f	[]
e22ca7b8-09bb-4e90-a807-0d91b865c2f6	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "37559eea-aa20-4b8c-ba39-ee899c2fc556", "started_at": "2026-01-05T04:59:56.694889798Z", "finished_at": "2026-11-14T15:27:59.040963Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:58.032139Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-05 04:59:56.694889+00	2026-11-14 15:27:59.042795+00	0	f	[]
76e22a48-8d45-4c67-953f-366775778295	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "ce6f84d7-a1a5-468b-887d-1a0c67e78012", "started_at": "2026-02-28T21:18:57.228697745Z", "finished_at": "2026-11-14T15:27:59.051527Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "SelfReport", "host_id": "b52f72ce-8332-4d4f-90a6-596cbbb0ba73"}, "last_update_at": "2026-11-14T15:27:55.252740Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "b52f72ce-8332-4d4f-90a6-596cbbb0ba73"}	Self Report — My Site	2026-02-28 21:18:57.228697+00	2026-11-14 15:27:59.053386+00	0	f	[]
0f89d8f8-8b8b-4bd5-b052-00b50661e7b7	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "7612c47a-db41-4c37-8d16-fbf4ace0e41e", "started_at": "2026-01-23T03:55:10.336667754Z", "finished_at": "2026-11-14T15:28:00.315033Z", "discovery_id": null, "daemon_version": "0.14.19", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:59.861902Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-01-23 03:55:10.336667+00	2026-11-14 15:28:00.317067+00	0	f	[]
3b6e9f55-35ff-4a2d-88b9-87b02b36cdc8	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "ff3dfff8-b772-4147-bc5c-c2806e8e84a4", "started_at": "2026-02-28T03:11:21.545902350Z", "finished_at": "2026-11-14T15:27:52.857099Z", "discovery_id": null, "daemon_version": "0.15.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:50.635250Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-28 03:11:21.545902+00	2026-11-14 15:27:52.859243+00	0	f	[]
3b989ea6-d132-4eb9-9644-fec4030df475	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "557486b8-0125-4f97-a586-fe9ae5f972d8", "started_at": "2026-09-03T18:01:02.050183807Z", "finished_at": "2026-11-14T15:27:55.234719Z", "discovery_id": null, "daemon_version": "0.14.10", "discovery_type": {"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:54.775353Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "cea1b136-864f-46f0-8684-69205c308fd2", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-09-03 18:01:02.050183+00	2026-11-14 15:27:55.235869+00	0	f	[]
1488fc16-51d3-45e6-89a8-4ce7dab4137b	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "d10e30a1-1b5e-4a25-abe9-566d427e3186", "started_at": "2026-03-24T03:12:35.661709995Z", "finished_at": "2026-11-14T15:27:53.490271Z", "discovery_id": null, "daemon_version": "0.14.18", "discovery_type": {"type": "Unified", "host_id": "80008868-cd5c-4f1e-92ea-0bd73728c9a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:52.879177Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "80008868-cd5c-4f1e-92ea-0bd73728c9a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-03-24 03:12:35.661709+00	2026-11-14 15:27:53.491542+00	0	f	[]
7aeab5ae-b8bb-4a48-895d-a767a2900a20	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "c9568c2f-1ae3-48c4-9136-6eb8fcd496a8", "started_at": "2026-08-06T01:40:06.139857038Z", "finished_at": "2026-11-14T15:27:59.021591Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:58.550018Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-06 01:40:06.139857+00	2026-11-14 15:27:59.022691+00	0	f	[]
628f3aba-07f1-49e2-b31d-00c0f223a9db	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "128947fe-516b-47a0-897f-975edb8d4122", "started_at": "2026-04-20T17:24:17.714515792Z", "finished_at": "2026-11-14T15:28:00.318204Z", "discovery_id": null, "daemon_version": "0.14.19", "discovery_type": {"type": "Unified", "host_id": "41167b70-ef54-45fb-ac1c-df4f8be5f312", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:59.122883Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "41167b70-ef54-45fb-ac1c-df4f8be5f312", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-20 17:24:17.714515+00	2026-11-14 15:28:00.319375+00	0	f	[]
417a81e6-1a61-47cc-88fc-6df24a170336	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "c1eaa2e7-7631-418b-8d5f-74597c08a4f5", "started_at": "2026-10-31T00:21:22.175714347Z", "finished_at": "2026-11-14T15:27:54.663980Z", "discovery_id": null, "daemon_version": "0.17.14", "discovery_type": {"type": "Unified", "host_id": "a20cea9a-e475-4818-80d1-32a346c0085e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:54.143061Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "a20cea9a-e475-4818-80d1-32a346c0085e", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-10-31 00:21:22.175714+00	2026-11-14 15:27:54.665085+00	0	f	[]
23e023aa-507e-4b42-8ae4-4b3569498662	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "110d0739-6875-498e-9ed5-5c7c0dec84ca", "started_at": "2026-03-05T07:38:20.117782332Z", "finished_at": "2026-11-14T15:27:59.035482Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "SelfReport", "host_id": "9b3486eb-3e41-4b56-8062-c98ad29f0a3a"}, "last_update_at": "2026-11-14T15:27:55.795514Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "9b3486eb-3e41-4b56-8062-c98ad29f0a3a"}	Self Report — My Site	2026-03-05 07:38:20.117782+00	2026-11-14 15:27:59.037314+00	0	f	[]
4acd8505-5fd0-48f6-9035-e2d7dd81547b	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "72d38ac6-a392-41f5-96f5-17c22a19e8f5", "started_at": "2026-03-06T23:21:08.018383672Z", "finished_at": "2026-11-14T15:27:59.045864Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:56.936226Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-06 23:21:08.018383+00	2026-11-14 15:27:59.047742+00	0	f	[]
ed743c90-60ee-4981-8a95-8ee548baef2d	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "9f3bb823-ded6-4cb7-957b-27c4af0ebff2", "started_at": "2026-03-15T19:35:11.987886037Z", "finished_at": "2026-11-14T15:28:12.768148Z", "discovery_id": null, "daemon_version": "0.20.0", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:28:00.389911Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-15 19:35:11.987886+00	2026-11-14 15:28:12.770361+00	0	f	[]
e03a79a0-9bfc-4f13-b7be-cd69423f0100	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "947c4ec5-0754-4116-8282-283b6a09f803", "started_at": "2026-03-05T07:38:50.115878953Z", "finished_at": "2026-11-14T15:27:59.018244Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:55.843108Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-03-05 07:38:50.115878+00	2026-11-14 15:27:59.020358+00	0	f	[]
0aff3c0a-ebbf-41d1-a091-e0974efc1713	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "61efb9ca-2ac2-4ab8-858e-07da2d3ef161", "started_at": "2026-02-10T04:10:20.090437467Z", "finished_at": "2026-11-14T15:27:59.029703Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "SelfReport", "host_id": "45cb5c93-df81-41f1-9047-97867efb47bb"}, "last_update_at": "2026-11-14T15:27:56.340970Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "45cb5c93-df81-41f1-9047-97867efb47bb"}	Self Report — My Site	2026-02-10 04:10:20.090437+00	2026-11-14 15:27:59.031671+00	0	f	[]
4c090654-8860-471d-b70c-7bebe9156131	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "14a48b0e-4297-410e-b019-b1d98130f2c0", "started_at": "2026-02-10T04:10:50.083304178Z", "finished_at": "2026-11-14T15:27:59.032559Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:56.396513Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Network", "subnet_ids": null, "snmp_credentials": {"daemon_os": null, "ip_overrides": [], "default_credential": null, "default_credential_id": null}, "host_naming_fallback": "BestService"}	Network Discovery — My Site	2026-02-10 04:10:50.083304+00	2026-11-14 15:27:59.034382+00	0	f	[]
999ac5aa-ce82-47f3-890d-0a61744f9bea	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "68d1cd6d-9bbc-4af3-9202-ecf0db0dee57", "started_at": "2026-08-06T01:43:36.139858287Z", "finished_at": "2026-11-14T15:27:59.043828Z", "discovery_id": null, "daemon_version": "0.16.1", "discovery_type": {"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:58.555873Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d2ea0ffb-433c-4824-bc41-008482cd88c0", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-08-06 01:43:36.139858+00	2026-11-14 15:27:59.044888+00	0	f	[]
105fd70a-92a7-4b69-bf93-46aa99cdc19a	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "04ade0d1-6874-420a-a1d3-6ece53cd5f6f", "started_at": "2026-04-20T17:56:03.901696836Z", "finished_at": "2026-11-14T15:28:00.312711Z", "discovery_id": null, "daemon_version": "0.14.19", "discovery_type": {"type": "Unified", "host_id": "2551f8f8-f3cf-43d9-9d28-dd99c9468591", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}, "last_update_at": "2026-11-14T15:27:59.072691Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "2551f8f8-f3cf-43d9-9d28-dd99c9468591", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "arp_scan_cutoff": null, "full_scan_interval": null, "port_scan_batch_size": null, "max_discovery_duration": null, "probe_raw_socket_ports": false, "trust_port_only_detections": false}, "host_naming_fallback": "BestService"}	Discovery	2026-04-20 17:56:03.901696+00	2026-11-14 15:28:00.313887+00	0	f	[]
c3487801-a7f6-4182-a35e-787d287c109e	8dc5a375-7025-4afa-b80b-5db6acaf56d8	e405bf21-7d94-4fa9-af78-83d4c6405b13	{"type": "Historical", "results": {"error": "The daemon was idle while the server had it running this scan, and never reported how the scan ended", "phase": "Failed", "reason": "DaemonDroppedSession", "site_id": "8dc5a375-7025-4afa-b80b-5db6acaf56d8", "progress": 0, "warnings": [], "daemon_id": "e405bf21-7d94-4fa9-af78-83d4c6405b13", "session_id": "e46bbbd5-df29-425e-b933-bd8b0a332217", "started_at": "2026-03-15T19:34:41.999288315Z", "finished_at": "2026-11-14T15:28:12.764716Z", "discovery_id": null, "daemon_version": "0.20.0", "discovery_type": {"type": "SelfReport", "host_id": "67ddad71-cd34-47fe-ad5f-138fe7069c76"}, "last_update_at": "2026-11-14T15:28:00.340398Z", "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "67ddad71-cd34-47fe-ad5f-138fe7069c76"}	Self Report — My Site	2026-03-15 19:34:41.999288+00	2026-11-14 15:28:12.766977+00	0	f	[]
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
e8a8bfcc-2867-4eba-992d-080da408d194	3d575668-8608-485f-b4b5-8a5329efc592	"Service"	cf0c4ec2-e05d-406c-b5a8-b9baa148050f	2026-11-14 15:27:19.109284+00	2026-11-14 15:27:19.109284+00	\N	\N	\N
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
0ad3aed3-81f0-472c-867f-33bb0c77fbb8	8dc5a375-7025-4afa-b80b-5db6acaf56d8	scanopy-daemon-serverpoll	423216d9854b	\N	{"type": "Discovery"}	null	2026-11-14 15:20:01.422985+00	2026-11-14 15:20:01.422985+00	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-11-14 15:20:01.422985+00	\N	\N	2026-11-14 15:25:57.89189+00	8a31afe4-22e3-45f4-adb6-b0f6f6092758	e542f7b2-5bc2-4893-84bc-c5e2e9cc472f	\N	\N	\N	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	\N	"Unspecified"	"DaemonSelfReport"	{"name": "Debian", "family": "Linux", "version": "12.0.0", "codename": "bookworm", "kernel_version": "7.0.0-1019-azure"}	"DaemonSelfReport"	\N	\N	"Unspecified"	\N	"Unspecified"	\N
3806ccb4-5580-4e86-96d4-305bcc95aa18	8dc5a375-7025-4afa-b80b-5db6acaf56d8	Scanopy Daemon	e521a7cd35c1	\N	{"type": "Discovery"}	null	2026-11-14 15:18:25.589792+00	2026-11-14 15:18:25.589792+00	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-11-14 15:18:25.589792+00	\N	\N	2026-11-14 15:27:27.977868+00	5b3b9c56-bc30-4de0-9179-8ca22e5a6bbe	5b3b9c56-bc30-4de0-9179-8ca22e5a6bbe	\N	\N	\N	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"ServiceMatch"	\N	"Unspecified"	"DaemonSelfReport"	{"name": "Debian", "family": "Linux", "version": "12.0.0", "codename": "bookworm", "kernel_version": "7.0.0-1019-azure"}	"DaemonSelfReport"	\N	\N	"Unspecified"	\N	"Unspecified"	\N
67ddad71-cd34-47fe-ad5f-138fe7069c76	8dc5a375-7025-4afa-b80b-5db6acaf56d8	aea74b9b3d22	aea74b9b3d22	Scanopy daemon	{"type": "Discovery"}	null	2026-11-14 15:28:00.35907+00	2026-11-14 15:28:00.35907+00	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N	2026-11-14 15:28:00.35907+00	\N	\N	2026-11-14 15:28:00.35907+00	\N	\N	\N	\N	\N	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	"Unspecified"	\N	"Unspecified"	"Unspecified"	\N	"Unspecified"	\N	\N	"Unspecified"	\N	"Unspecified"	\N
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
81412bd9-bca5-4e10-9ab3-ddba0f10d91b	8dc5a375-7025-4afa-b80b-5db6acaf56d8	67ddad71-cd34-47fe-ad5f-138fe7069c76	4ad11d12-d29a-4f8d-87e9-dc2c288a785d	172.25.0.4	36:4d:0a:e4:6f:0d	eth0	0	2026-11-14 15:28:00.35907+00	2026-11-14 15:28:00.35907+00	2026-11-14 15:28:00.35907+00	\N	\N	2026-11-14 15:28:00.35907+00	\N	\N	\N	"Unspecified"	"Unspecified"
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
4588b37a-3195-4908-9679-32e8f045faa8	My Organization	\N	{"rate": "Month", "type": "Community", "base_cents": 0, "trial_days": 0}	active	2026-11-14 15:18:25.376581+00	2026-11-14 15:18:25.376581+00	[]	f	\N	other	\N	f	\N	null	\N	\N	\N	\N	{"hosts": "None", "seats": "None", "sites": "None", "setup_items_open": [], "setup_items_reported": [], "license_over_instance_limit": false, "license_silence_reported_for": null, "airgap_expiry_notified_through": null, "sunset_notified_last_unsupported": null}	\N	\N	0	\N	\N	\N	\N	f	\N	["InstallDaemon", "RunFirstScan"]
\.


--
-- Data for Name: ports; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ports (id, site_id, host_id, port_number, protocol, port_type, created_at, updated_at, valid_from, valid_to, lineage_id, last_seen_at, last_discovery_id, first_discovery_id, snapshot_id, port_source) FROM stdin;
59eca2e1-5650-44fc-bcc0-a436c484f2da	8dc5a375-7025-4afa-b80b-5db6acaf56d8	67ddad71-cd34-47fe-ad5f-138fe7069c76	60073	Tcp	Custom	2026-11-14 15:28:00.35907+00	2026-11-14 15:28:00.35907+00	2026-11-14 15:28:00.35907+00	\N	\N	2026-11-14 15:28:00.35907+00	\N	\N	\N	"Unspecified"
\.


--
-- Data for Name: release_announcements; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.release_announcements (id, version, email_sent_at, created_at, updated_at) FROM stdin;
07947f2e-90b3-4e60-9c35-b4d0b30279f1	0.20.0	\N	2026-11-14 15:18:19.13961+00	2026-11-14 15:18:19.13961+00
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
a6bceb85-8a66-414b-93ad-e7a855b019b8	8dc5a375-7025-4afa-b80b-5db6acaf56d8	2026-11-14 15:28:00.35907+00	2026-11-14 15:28:00.35907+00	Scanopy Daemon	67ddad71-cd34-47fe-ad5f-138fe7069c76	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}}	0	2026-11-14 15:28:00.35907+00	\N	\N	2026-11-14 15:28:00.35907+00	\N	\N	\N	\N
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
959635a7-c962-4991-9539-8a48464b1088	4588b37a-3195-4908-9679-32e8f045faa8	e866d437-0fab-4ecb-ae06-f0de46edc63c	\N	\N	Admin	\N	2026-11-14 15:28:15.801898+00
0b65985f-7348-4f83-9a6e-2fbf058ba128	4588b37a-3195-4908-9679-32e8f045faa8	978fe15d-3005-4044-9207-1a2dd8d4dcaf	\N	\N	Admin	\N	2026-11-14 15:28:15.801898+00
\.


--
-- Data for Name: site_credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.site_credentials (site_id, credential_id) FROM stdin;
8dc5a375-7025-4afa-b80b-5db6acaf56d8	30ddaec3-9d76-48d1-a89d-cd4cb9bb4063
\.


--
-- Data for Name: sites; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.sites (id, name, created_at, updated_at, organization_id, stale_after_hours) FROM stdin;
8dc5a375-7025-4afa-b80b-5db6acaf56d8	My Site	2026-11-14 15:18:25.434102+00	2026-11-14 15:18:25.434102+00	4588b37a-3195-4908-9679-32e8f045faa8	\N
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
b2945287-f947-4f7d-ae4a-ddf05523c732	8dc5a375-7025-4afa-b80b-5db6acaf56d8	2026-11-14 15:28:03.818598+00	2026-11-14 15:28:03.818598+00	"127.0.0.0/8"	127.0.0.0/8	\N	Loopback	{"type": "Discovery"}	2026-11-14 15:28:03.818598+00	\N	\N	2026-11-14 15:28:03.818598+00	\N	\N	\N	\N	"DaemonSelfReport"	"Unspecified"	"InterfaceNameMatch"
4ad11d12-d29a-4f8d-87e9-dc2c288a785d	8dc5a375-7025-4afa-b80b-5db6acaf56d8	1970-01-01 00:00:00+00	2026-03-15 19:34:41.989523+00	"172.25.0.0/28"	172.25.0.0/28	\N	Lan	{"type": "Discovery"}	1970-01-01 00:00:00+00	\N	\N	2026-11-14 15:28:03.818626+00	\N	\N	\N	\N	"DaemonSelfReport"	"Unspecified"	"InterfaceNameMatch"
27338808-83fb-4891-a9f0-2e38beee588a	8dc5a375-7025-4afa-b80b-5db6acaf56d8	2026-11-14 15:28:14.932394+00	2026-11-14 15:28:14.932394+00	"10.1.0.0/24"	Blocked Subnet	\N	Lan	{"type": "Manual"}	2026-11-14 15:28:14.932394+00	\N	\N	2026-11-14 15:28:14.932394+00	\N	\N	\N	\N	"Manual"	"Manual"	"Manual"
\.


--
-- Data for Name: tag_groups; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tag_groups (id, organization_id, name, description, group_type, entity_types, ordered, created_at, updated_at) FROM stdin;
c351ade3-ef5a-4160-b76e-4f31cd663d9d	4588b37a-3195-4908-9679-32e8f045faa8	Application	What each entity is part of. A service inherits its host's application tag, and the Application view groups by it.	Application	\N	f	2026-11-14 15:18:25.415685+00	2026-11-14 15:18:25.415685+00
490383e7-be69-4cbc-aa06-4b20720b4f6a	4588b37a-3195-4908-9679-32e8f045faa8	Status	Where an asset is in its lifecycle.	Status	\N	t	2026-11-14 15:18:25.418455+00	2026-11-14 15:18:25.418455+00
6c0f3908-27ca-4609-9a97-737745edeef3	4588b37a-3195-4908-9679-32e8f045faa8	Visibility	Entities holding a tag of this group are left out of every topology.	Visibility	\N	f	2026-11-14 15:18:25.42012+00	2026-11-14 15:18:25.42012+00
f8d3570c-9e18-47b5-8f09-f9e249b3980e	4588b37a-3195-4908-9679-32e8f045faa8	Environment	Which stage of delivery an asset serves.	Custom	\N	t	2026-11-14 15:18:25.517971+00	2026-11-14 15:18:25.517971+00
5adbc505-10e9-412a-a50c-fc961b427d82	4588b37a-3195-4908-9679-32e8f045faa8	Criticality	How much an outage of the asset costs.	Custom	\N	t	2026-11-14 15:18:25.519307+00	2026-11-14 15:18:25.519307+00
65b030ca-b8a4-4d30-bc48-86f92d19d414	4588b37a-3195-4908-9679-32e8f045faa8	Management	Who administers the asset.	Custom	\N	t	2026-11-14 15:18:25.520279+00	2026-11-14 15:18:25.520279+00
\.


--
-- Data for Name: tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tags (id, organization_id, name, description, created_at, updated_at, color, valid_from, valid_to, lineage_id, snapshot_id, icon, tag_group_id, "position", source) FROM stdin;
d46ec649-c6ea-42d2-8ca3-96d4ba6aa179	4588b37a-3195-4908-9679-32e8f045faa8	Hidden	Left out of topology.	2026-11-14 15:18:25.422171+00	2026-11-14 15:18:25.422171+00	Gray	2026-11-14 15:18:25.422171+00	\N	\N	\N	eye-off	6c0f3908-27ca-4609-9a97-737745edeef3	\N	{"type": "System"}
7b7a3f08-f258-4219-87af-f9e37027e372	4588b37a-3195-4908-9679-32e8f045faa8	Planned	Ordered or scheduled, not yet in service	2026-11-14 15:18:25.524668+00	2026-11-14 15:18:25.524668+00	Blue	2026-11-14 15:18:25.524668+00	\N	\N	\N	calendar-clock	490383e7-be69-4cbc-aa06-4b20720b4f6a	1	{"type": "System"}
94f999ab-20f0-4ab3-8d85-cd78da3b4a67	4588b37a-3195-4908-9679-32e8f045faa8	Active	In service	2026-11-14 15:18:25.524689+00	2026-11-14 15:18:25.524689+00	Green	2026-11-14 15:18:25.524689+00	\N	\N	\N	circle-check	490383e7-be69-4cbc-aa06-4b20720b4f6a	2	{"type": "System"}
a26e1357-c47f-4933-a18d-270e15910cf0	4588b37a-3195-4908-9679-32e8f045faa8	Decommissioning	Being retired; still on the network	2026-11-14 15:18:25.524691+00	2026-11-14 15:18:25.524691+00	Orange	2026-11-14 15:18:25.524691+00	\N	\N	\N	hourglass	490383e7-be69-4cbc-aa06-4b20720b4f6a	3	{"type": "System"}
23b0fe3e-7829-4cdf-af39-00c03e983a77	4588b37a-3195-4908-9679-32e8f045faa8	Decommissioned	Retired from service; kept for the record	2026-11-14 15:18:25.524693+00	2026-11-14 15:18:25.524693+00	Gray	2026-11-14 15:18:25.524693+00	\N	\N	\N	archive	490383e7-be69-4cbc-aa06-4b20720b4f6a	4	{"type": "System"}
cdf6859b-7223-4589-93d3-8b2437692bbb	4588b37a-3195-4908-9679-32e8f045faa8	Production	Serves live users and business traffic	2026-11-14 15:18:25.524699+00	2026-11-14 15:18:25.524699+00	Red	2026-11-14 15:18:25.524699+00	\N	\N	\N	server	f8d3570c-9e18-47b5-8f09-f9e249b3980e	1	{"type": "System"}
28f05185-162b-4bcf-9e2e-6d8354c9b410	4588b37a-3195-4908-9679-32e8f045faa8	Staging	A pre-release copy of production for final testing	2026-11-14 15:18:25.524702+00	2026-11-14 15:18:25.524702+00	Amber	2026-11-14 15:18:25.524702+00	\N	\N	\N	flask-conical	f8d3570c-9e18-47b5-8f09-f9e249b3980e	2	{"type": "System"}
cc78873a-37b8-4dcb-aa50-6a45e1d0947c	4588b37a-3195-4908-9679-32e8f045faa8	Development	Used to build and test changes	2026-11-14 15:18:25.524704+00	2026-11-14 15:18:25.524704+00	Blue	2026-11-14 15:18:25.524704+00	\N	\N	\N	code	f8d3570c-9e18-47b5-8f09-f9e249b3980e	3	{"type": "System"}
964731ee-7a4b-40de-86b9-91d6fe1896e2	4588b37a-3195-4908-9679-32e8f045faa8	Lab	Test bench or experimental equipment	2026-11-14 15:18:25.524706+00	2026-11-14 15:18:25.524706+00	Purple	2026-11-14 15:18:25.524706+00	\N	\N	\N	test-tube	f8d3570c-9e18-47b5-8f09-f9e249b3980e	4	{"type": "System"}
b1d079a5-f0b6-45ab-a77c-099f73cb90cf	4588b37a-3195-4908-9679-32e8f045faa8	Critical	An outage stops the business	2026-11-14 15:18:25.524711+00	2026-11-14 15:18:25.524711+00	Red	2026-11-14 15:18:25.524711+00	\N	\N	\N	siren	5adbc505-10e9-412a-a50c-fc961b427d82	1	{"type": "System"}
3485ffc0-c512-4889-ab5d-48e5d9b9281c	4588b37a-3195-4908-9679-32e8f045faa8	High	An outage disrupts many people	2026-11-14 15:18:25.524713+00	2026-11-14 15:18:25.524713+00	Orange	2026-11-14 15:18:25.524713+00	\N	\N	\N	arrow-up	5adbc505-10e9-412a-a50c-fc961b427d82	2	{"type": "System"}
e615116b-1d76-4046-928f-d741214a23dc	4588b37a-3195-4908-9679-32e8f045faa8	Medium	An outage disrupts some people	2026-11-14 15:18:25.524715+00	2026-11-14 15:18:25.524715+00	Yellow	2026-11-14 15:18:25.524715+00	\N	\N	\N	minus	5adbc505-10e9-412a-a50c-fc961b427d82	3	{"type": "System"}
43907a86-be0f-4b0c-8e54-a50723d2bd40	4588b37a-3195-4908-9679-32e8f045faa8	Low	An outage has little impact	2026-11-14 15:18:25.524717+00	2026-11-14 15:18:25.524717+00	Gray	2026-11-14 15:18:25.524717+00	\N	\N	\N	arrow-down	5adbc505-10e9-412a-a50c-fc961b427d82	4	{"type": "System"}
be30bd12-4aed-4b91-b809-fe5f9d4fe3bc	4588b37a-3195-4908-9679-32e8f045faa8	Managed	Administered by your IT team	2026-11-14 15:18:25.524724+00	2026-11-14 15:18:25.524724+00	Green	2026-11-14 15:18:25.524724+00	\N	\N	\N	shield-check	65b030ca-b8a4-4d30-bc48-86f92d19d414	1	{"type": "System"}
5ab5471e-c89b-4abf-b8ab-659233334dd6	4588b37a-3195-4908-9679-32e8f045faa8	Unmanaged	Administered by no one, such as a personal device	2026-11-14 15:18:25.524726+00	2026-11-14 15:18:25.524726+00	Orange	2026-11-14 15:18:25.524726+00	\N	\N	\N	shield-question-mark	65b030ca-b8a4-4d30-bc48-86f92d19d414	2	{"type": "System"}
6b142020-e3c0-41f0-b477-20b34907a287	4588b37a-3195-4908-9679-32e8f045faa8	Third-party	Administered by a vendor or another organization	2026-11-14 15:18:25.524728+00	2026-11-14 15:18:25.524728+00	Blue	2026-11-14 15:18:25.524728+00	\N	\N	\N	handshake	65b030ca-b8a4-4d30-bc48-86f92d19d414	3	{"type": "System"}
cf0c4ec2-e05d-406c-b5a8-b9baa148050f	4588b37a-3195-4908-9679-32e8f045faa8	Integration Test Tag	\N	2026-11-14 15:27:19.089859+00	2026-11-14 15:27:19.089859+00	Yellow	2026-11-14 15:27:19.089859+00	\N	\N	\N	\N	\N	\N	{"type": "Manual"}
2eaf13d2-ef36-47aa-affc-2c50d62c2a59	4588b37a-3195-4908-9679-32e8f045faa8	Test Tag	\N	2026-11-14 15:28:13.596774+00	2026-11-14 15:28:13.596774+00	Yellow	2026-11-14 15:28:13.596774+00	2026-11-14 15:28:13.620737+00	e0af0ad5-d493-46f6-99e0-d598df875a37	\N	\N	\N	\N	{"type": "Manual"}
e0af0ad5-d493-46f6-99e0-d598df875a37	4588b37a-3195-4908-9679-32e8f045faa8	Updated Tag	\N	2026-11-14 15:28:13.596774+00	2026-11-14 15:28:13.596774+00	Yellow	2026-11-14 15:28:13.620737+00	2026-11-14 15:28:13.641591+00	\N	\N	\N	\N	\N	{"type": "Manual"}
\.


--
-- Data for Name: topologies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.topologies (id, site_id, options, created_at, updated_at) FROM stdin;
d39c3f96-e52e-4db5-b544-6aeb976280f2	8dc5a375-7025-4afa-b80b-5db6acaf56d8	{"local": {"tag_filter": {}, "bundle_edges": true, "show_minimap": true, "no_fade_edges": false, "hide_edge_types": {"L3Logical": ["Hypervisor", "SameContainer"], "Workloads": ["RequestPath", "HubAndSpoke", "PhysicalLink", "NeighborLink"], "L2Physical": ["SameHost"], "Application": ["ContainerRuntime"]}}, "request": {"element_sort": {}, "element_rules": [{"id": "0ad74163-c169-4997-8560-1f81e4ba2c4a", "rule": "ByTrunkPort"}, {"id": "e6dab4e0-b189-41b6-8c3d-a7eac24b867d", "rule": "ByVLAN"}, {"id": "4fe3a19f-9e84-447d-a964-4b3515d5250e", "rule": "ByPortOpStatus"}, {"id": "51c509c3-1f86-44d1-ab9e-4e04094d135e", "rule": {"ByServiceCategory": {"title": "Infrastructure", "categories": ["NetworkCore", "NetworkAccess", "NetworkAppliance", "RemoteAccess", "Workstation", "Mobile", "IoT", "Printer", "OpenPorts", "NetworkIdentities"], "is_infra_rule": true}}}, {"id": "7341600e-7732-4d07-805c-771d02558842", "rule": {"ByTag": {"title": null, "tag_ids": []}}}, {"id": "ae6fa17f-bc3b-4512-acfe-7234f23d5f48", "rule": "ByDeviceType"}, {"id": "d802e307-14d8-47aa-92fe-8734a7ffdc73", "rule": "ByHypervisor"}, {"id": "e31fe1c0-17b3-4403-bcdf-e1523f19cd2a", "rule": "ByContainerRuntime"}, {"id": "fa37dffe-729e-402c-8ad2-340d07cae25b", "rule": "ByStack"}], "hide_entities": {}, "container_rules": {"L3Logical": [{"id": "52d6cee4-acba-4545-856d-bdb985ed974f", "rule": "BySubnet"}, {"id": "adf1711d-d1af-4a4e-b157-1e3ca69eb427", "rule": "MergeContainerBridges"}], "Workloads": [{"id": "3eb78ca9-6147-4dd4-baa7-41673bf5f0df", "rule": "ByHost"}], "L2Physical": [{"id": "3eb78ca9-6147-4dd4-baa7-41673bf5f0df", "rule": "ByHost"}], "Application": [{"id": "df7a9919-db80-48e7-a854-dfea4c86ab03", "rule": {"ByApplication": {"tag_ids": []}}}]}, "hide_metadata_values": {"L3Logical": {"Service": {"Category": ["OpenPorts"]}}, "Workloads": {"Service": {"Category": ["OpenPorts"]}}, "L2Physical": {"Service": {"Category": ["OpenPorts"]}, "Interface": {"LinkState": ["Unlinked"]}}, "Application": {"Service": {"Category": ["OpenPorts"]}}}}}	2026-11-14 15:18:25.457536+00	2026-11-14 15:18:25.457536+00
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
e866d437-0fab-4ecb-ae06-f0de46edc63c	2026-11-14 15:18:25.379037+00	2026-11-14 15:18:25.379037+00	$argon2id$v=19$m=19456,t=2,p=1$MAtR7WoczGesCdMEeSCw8Q$ghK1DHg00I2E/ikQuH6vCBTcthiivkYAxzsI8gpY5AY	\N	\N	\N	user@gmail.com	4588b37a-3195-4908-9679-32e8f045faa8	Owner	{}	\N	t	\N	\N	\N	\N	\N	{"daemon_alerts": true, "product_updates": true, "trial_and_usage": true, "discovery_digest": true, "product_onboarding": true}	0	{"clock": "browser_default", "time_zone": null, "date_order": "browser_default", "timestamps": "relative", "week_start": "monday", "table_density": "compact"}	\N	0.20.0	\N
978fe15d-3005-4044-9207-1a2dd8d4dcaf	2026-11-14 15:28:14.494198+00	2026-11-14 15:28:14.494198+00	\N	\N	\N	\N	user@example.com	4588b37a-3195-4908-9679-32e8f045faa8	Owner	{}	\N	f	\N	\N	\N	\N	\N	{"daemon_alerts": true, "product_updates": true, "trial_and_usage": true, "discovery_digest": true, "product_onboarding": true}	0	{"clock": "browser_default", "time_zone": null, "date_order": "browser_default", "timestamps": "relative", "week_start": "monday", "table_density": "compact"}	\N	\N	\N
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
U2mYGOVlf5fsXRUIS0NAtw	\\x93c410b740434b08155dec977f65e51898695382a7757365725f6964d92465383636643433372d306661622d346563622d616530362d663064653436656463363363ad73657373696f6e5f65706f63680099cd07eacd01450f1219ce17d10f18000000	2026-11-21 15:18:25.399576+00
J847qdd0xENRUVD-wwgDgA	\\x93c410800308c3fe50515143c474d7a93bce2783ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92436363039623231642d386635612d343866352d616434332d663235326563306331343465a87573655f63617365a56f74686572ad73657373696f6e5f65706f636800a7757365725f6964d92465383636643433372d306661622d346563622d616530362d66306465343665646336336399cd07eacd01450f1b14ce043c56af000000	2026-11-21 15:27:20.071063+00
kGc5B8jP3OEkLjyylXGK2A	\\x93c410d88a7195b23c2e24e1dccfc80739679083ad73657373696f6e5f65706f636800ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92465646437366138302d333261612d343030382d623635372d393862646664636431343238a87573655f63617365a56f74686572a7757365725f6964d92465383636643433372d306661622d346563622d616530362d66306465343665646336336399cd07eacd01450f1c01ce01b3e453000000	2026-11-21 15:28:01.028566+00
JGqXeQv6dzrybqo5EJmy2A	\\x93c410d8b2991039aa6ef23a77fa0b79976a2483ad70656e64696e675f736574757083a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea47369746582a46e616d65a74d792053697465a7736974655f6964d92461373530313363642d366233372d346232372d383266342d613137353434333939636337a87573655f63617365a56f74686572a7757365725f6964d92465383636643433372d306661622d346563622d616530362d663064653436656463363363ad73657373696f6e5f65706f63680099cd07eacd01450f1c0dce1014132a000000	2026-11-21 15:28:13.269751+00
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

\unrestrict fCIJ7e6JdTRNWkFwQyWycsIfSoCOurx7PPEzhYzsPH0TYyaqx1aZrIQqc0At4vU

