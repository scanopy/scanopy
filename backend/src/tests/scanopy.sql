--
-- PostgreSQL database dump
--

\restrict PJ3btamkYVOtmTuYU7kCTMvkO0oaN26fjXu3b93b62QeEiAObF4G7oAE5uTbOAG

-- Dumped from database version 17.9
-- Dumped by pg_dump version 17.9

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

ALTER TABLE IF EXISTS ONLY public.users DROP CONSTRAINT IF EXISTS users_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_network_access DROP CONSTRAINT IF EXISTS user_network_access_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_network_access DROP CONSTRAINT IF EXISTS user_network_access_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_key_network_access DROP CONSTRAINT IF EXISTS user_api_key_network_access_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_key_network_access DROP CONSTRAINT IF EXISTS user_api_key_network_access_api_key_id_fkey;
ALTER TABLE IF EXISTS ONLY public.topologies DROP CONSTRAINT IF EXISTS topologies_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.tags DROP CONSTRAINT IF EXISTS tags_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_topology_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_created_by_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.networks DROP CONSTRAINT IF EXISTS organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.network_credentials DROP CONSTRAINT IF EXISTS network_credentials_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.network_credentials DROP CONSTRAINT IF EXISTS network_credentials_credential_id_fkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_created_by_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_subnet_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_neighbor_if_entry_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_neighbor_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_interface_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.host_credentials DROP CONSTRAINT IF EXISTS host_credentials_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.host_credentials DROP CONSTRAINT IF EXISTS host_credentials_credential_id_fkey;
ALTER TABLE IF EXISTS ONLY public.groups DROP CONSTRAINT IF EXISTS groups_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_group_id_fkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_binding_id_fkey;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_tag_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_daemon_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_api_key_id_fkey;
ALTER TABLE IF EXISTS ONLY public.credentials DROP CONSTRAINT IF EXISTS credentials_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_service_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_port_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_interface_id_fkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_network_id_fkey;
DROP TRIGGER IF EXISTS reassign_daemons_before_user_delete ON public.users;
DROP INDEX IF EXISTS public.idx_users_password_reset_token;
DROP INDEX IF EXISTS public.idx_users_organization;
DROP INDEX IF EXISTS public.idx_users_oidc_provider_subject;
DROP INDEX IF EXISTS public.idx_users_email_verification_token;
DROP INDEX IF EXISTS public.idx_users_email_lower;
DROP INDEX IF EXISTS public.idx_user_network_access_user;
DROP INDEX IF EXISTS public.idx_user_network_access_network;
DROP INDEX IF EXISTS public.idx_user_api_keys_user;
DROP INDEX IF EXISTS public.idx_user_api_keys_org;
DROP INDEX IF EXISTS public.idx_user_api_keys_key;
DROP INDEX IF EXISTS public.idx_user_api_key_network_access_network;
DROP INDEX IF EXISTS public.idx_user_api_key_network_access_key;
DROP INDEX IF EXISTS public.idx_topologies_network;
DROP INDEX IF EXISTS public.idx_tags_organization;
DROP INDEX IF EXISTS public.idx_tags_org_name;
DROP INDEX IF EXISTS public.idx_subnets_network;
DROP INDEX IF EXISTS public.idx_shares_topology;
DROP INDEX IF EXISTS public.idx_shares_network;
DROP INDEX IF EXISTS public.idx_shares_enabled;
DROP INDEX IF EXISTS public.idx_services_network;
DROP INDEX IF EXISTS public.idx_services_host_position;
DROP INDEX IF EXISTS public.idx_services_host_id;
DROP INDEX IF EXISTS public.idx_ports_number;
DROP INDEX IF EXISTS public.idx_ports_network;
DROP INDEX IF EXISTS public.idx_ports_host;
DROP INDEX IF EXISTS public.idx_organizations_stripe_customer;
DROP INDEX IF EXISTS public.idx_networks_owner_organization;
DROP INDEX IF EXISTS public.idx_invites_organization;
DROP INDEX IF EXISTS public.idx_invites_expires_at;
DROP INDEX IF EXISTS public.idx_interfaces_subnet;
DROP INDEX IF EXISTS public.idx_interfaces_network;
DROP INDEX IF EXISTS public.idx_interfaces_host_mac;
DROP INDEX IF EXISTS public.idx_interfaces_host;
DROP INDEX IF EXISTS public.idx_if_entries_network;
DROP INDEX IF EXISTS public.idx_if_entries_neighbor_if_entry;
DROP INDEX IF EXISTS public.idx_if_entries_neighbor_host;
DROP INDEX IF EXISTS public.idx_if_entries_mac_address;
DROP INDEX IF EXISTS public.idx_if_entries_interface;
DROP INDEX IF EXISTS public.idx_if_entries_host;
DROP INDEX IF EXISTS public.idx_hosts_network;
DROP INDEX IF EXISTS public.idx_hosts_chassis_id;
DROP INDEX IF EXISTS public.idx_groups_network;
DROP INDEX IF EXISTS public.idx_group_bindings_group;
DROP INDEX IF EXISTS public.idx_group_bindings_binding;
DROP INDEX IF EXISTS public.idx_entity_tags_tag_id;
DROP INDEX IF EXISTS public.idx_entity_tags_entity;
DROP INDEX IF EXISTS public.idx_discovery_network;
DROP INDEX IF EXISTS public.idx_discovery_daemon;
DROP INDEX IF EXISTS public.idx_daemons_network;
DROP INDEX IF EXISTS public.idx_daemons_api_key;
DROP INDEX IF EXISTS public.idx_daemon_host_id;
DROP INDEX IF EXISTS public.idx_credentials_type;
DROP INDEX IF EXISTS public.idx_credentials_org;
DROP INDEX IF EXISTS public.idx_bindings_service;
DROP INDEX IF EXISTS public.idx_bindings_port;
DROP INDEX IF EXISTS public.idx_bindings_network;
DROP INDEX IF EXISTS public.idx_bindings_interface;
DROP INDEX IF EXISTS public.idx_api_keys_network;
DROP INDEX IF EXISTS public.idx_api_keys_key;
ALTER TABLE IF EXISTS ONLY tower_sessions.session DROP CONSTRAINT IF EXISTS session_pkey;
ALTER TABLE IF EXISTS ONLY public.users DROP CONSTRAINT IF EXISTS users_pkey;
ALTER TABLE IF EXISTS ONLY public.user_network_access DROP CONSTRAINT IF EXISTS user_network_access_user_id_network_id_key;
ALTER TABLE IF EXISTS ONLY public.user_network_access DROP CONSTRAINT IF EXISTS user_network_access_pkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_pkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_key_key;
ALTER TABLE IF EXISTS ONLY public.user_api_key_network_access DROP CONSTRAINT IF EXISTS user_api_key_network_access_pkey;
ALTER TABLE IF EXISTS ONLY public.user_api_key_network_access DROP CONSTRAINT IF EXISTS user_api_key_network_access_api_key_id_network_id_key;
ALTER TABLE IF EXISTS ONLY public.topologies DROP CONSTRAINT IF EXISTS topologies_pkey;
ALTER TABLE IF EXISTS ONLY public.tags DROP CONSTRAINT IF EXISTS tags_pkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_pkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_pkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_pkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_pkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_host_id_port_number_protocol_key;
ALTER TABLE IF EXISTS ONLY public.organizations DROP CONSTRAINT IF EXISTS organizations_pkey;
ALTER TABLE IF EXISTS ONLY public.networks DROP CONSTRAINT IF EXISTS networks_pkey;
ALTER TABLE IF EXISTS ONLY public.network_credentials DROP CONSTRAINT IF EXISTS network_credentials_pkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_pkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_pkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_host_id_subnet_id_ip_address_key;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_pkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_host_id_if_index_key;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_pkey;
ALTER TABLE IF EXISTS ONLY public.host_credentials DROP CONSTRAINT IF EXISTS host_credentials_pkey;
ALTER TABLE IF EXISTS ONLY public.groups DROP CONSTRAINT IF EXISTS groups_pkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_pkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_group_id_binding_id_key;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_pkey;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_entity_id_entity_type_tag_id_key;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_pkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_pkey;
ALTER TABLE IF EXISTS ONLY public.credentials DROP CONSTRAINT IF EXISTS credentials_pkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_pkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_pkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_key_key;
ALTER TABLE IF EXISTS ONLY public._sqlx_migrations DROP CONSTRAINT IF EXISTS _sqlx_migrations_pkey;
DROP TABLE IF EXISTS tower_sessions.session;
DROP TABLE IF EXISTS public.users;
DROP TABLE IF EXISTS public.user_network_access;
DROP TABLE IF EXISTS public.user_api_keys;
DROP TABLE IF EXISTS public.user_api_key_network_access;
DROP TABLE IF EXISTS public.topologies;
DROP TABLE IF EXISTS public.tags;
DROP TABLE IF EXISTS public.subnets;
DROP TABLE IF EXISTS public.shares;
DROP TABLE IF EXISTS public.services;
DROP TABLE IF EXISTS public.ports;
DROP TABLE IF EXISTS public.organizations;
DROP TABLE IF EXISTS public.networks;
DROP TABLE IF EXISTS public.network_credentials;
DROP TABLE IF EXISTS public.invites;
DROP TABLE IF EXISTS public.interfaces;
DROP TABLE IF EXISTS public.if_entries;
DROP TABLE IF EXISTS public.hosts;
DROP TABLE IF EXISTS public.host_credentials;
DROP TABLE IF EXISTS public.groups;
DROP TABLE IF EXISTS public.group_bindings;
DROP TABLE IF EXISTS public.entity_tags;
DROP TABLE IF EXISTS public.discovery;
DROP TABLE IF EXISTS public.daemons;
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
    network_id uuid NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used timestamp with time zone,
    expires_at timestamp with time zone,
    is_enabled boolean DEFAULT true NOT NULL,
    plaintext text
);


ALTER TABLE public.api_keys OWNER TO postgres;

--
-- Name: bindings; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.bindings (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    service_id uuid NOT NULL,
    binding_type text NOT NULL,
    interface_id uuid,
    port_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT bindings_binding_type_check CHECK ((binding_type = ANY (ARRAY['Interface'::text, 'Port'::text]))),
    CONSTRAINT valid_binding CHECK ((((binding_type = 'Interface'::text) AND (interface_id IS NOT NULL) AND (port_id IS NULL)) OR ((binding_type = 'Port'::text) AND (port_id IS NOT NULL))))
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
    target_ips inet[],
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.credentials OWNER TO postgres;

--
-- Name: daemons; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.daemons (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    host_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    last_seen timestamp with time zone,
    capabilities jsonb DEFAULT '{}'::jsonb,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    mode text DEFAULT '"Push"'::text,
    url text NOT NULL,
    name text,
    version text,
    user_id uuid NOT NULL,
    api_key_id uuid,
    is_unreachable boolean DEFAULT false NOT NULL,
    standby boolean DEFAULT false NOT NULL
);


ALTER TABLE public.daemons OWNER TO postgres;

--
-- Name: discovery; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.discovery (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    daemon_id uuid NOT NULL,
    run_type jsonb NOT NULL,
    discovery_type jsonb NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    scan_count integer DEFAULT 0 NOT NULL,
    force_full_scan boolean DEFAULT false NOT NULL,
    pending_credential_ids uuid[] DEFAULT '{}'::uuid[] NOT NULL
);


ALTER TABLE public.discovery OWNER TO postgres;

--
-- Name: entity_tags; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.entity_tags (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    entity_id uuid NOT NULL,
    entity_type character varying(50) NOT NULL,
    tag_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.entity_tags OWNER TO postgres;

--
-- Name: group_bindings; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.group_bindings (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    group_id uuid NOT NULL,
    binding_id uuid NOT NULL,
    "position" integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.group_bindings OWNER TO postgres;

--
-- Name: groups; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.groups (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    name text NOT NULL,
    description text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    source jsonb NOT NULL,
    color text NOT NULL,
    edge_style text DEFAULT '"SmoothStep"'::text,
    group_type text NOT NULL
);


ALTER TABLE public.groups OWNER TO postgres;

--
-- Name: host_credentials; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.host_credentials (
    host_id uuid NOT NULL,
    credential_id uuid NOT NULL,
    interface_ids uuid[]
);


ALTER TABLE public.host_credentials OWNER TO postgres;

--
-- Name: hosts; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.hosts (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    name text NOT NULL,
    hostname text,
    description text,
    source jsonb NOT NULL,
    virtualization jsonb,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    hidden boolean DEFAULT false,
    sys_descr text,
    sys_object_id text,
    sys_location text,
    sys_contact text,
    management_url text,
    chassis_id text,
    manufacturer text,
    model text,
    serial_number text,
    sys_name text
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
-- Name: if_entries; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.if_entries (
    id uuid NOT NULL,
    host_id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    if_index integer NOT NULL,
    if_descr text NOT NULL,
    if_alias text,
    if_type integer NOT NULL,
    speed_bps bigint,
    admin_status integer NOT NULL,
    oper_status integer NOT NULL,
    mac_address macaddr,
    interface_id uuid,
    neighbor_if_entry_id uuid,
    neighbor_host_id uuid,
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
    if_name text,
    fdb_macs jsonb,
    CONSTRAINT chk_neighbor_exclusive CHECK (((neighbor_if_entry_id IS NULL) OR (neighbor_host_id IS NULL)))
);


ALTER TABLE public.if_entries OWNER TO postgres;

--
-- Name: TABLE if_entries; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON TABLE public.if_entries IS 'SNMP ifTable entries - physical/logical interfaces on network devices';


--
-- Name: COLUMN if_entries.if_index; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.if_index IS 'SNMP ifIndex - stable identifier within device';


--
-- Name: COLUMN if_entries.if_descr; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.if_descr IS 'SNMP ifDescr - interface description (e.g., GigabitEthernet0/1)';


--
-- Name: COLUMN if_entries.if_alias; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.if_alias IS 'SNMP ifAlias - user-configured description';


--
-- Name: COLUMN if_entries.if_type; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.if_type IS 'SNMP ifType - IANAifType integer (6=ethernet, 24=loopback, etc.)';


--
-- Name: COLUMN if_entries.speed_bps; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.speed_bps IS 'Interface speed from ifSpeed/ifHighSpeed in bits per second';


--
-- Name: COLUMN if_entries.admin_status; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.admin_status IS 'SNMP ifAdminStatus: 1=up, 2=down, 3=testing';


--
-- Name: COLUMN if_entries.oper_status; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.oper_status IS 'SNMP ifOperStatus: 1=up, 2=down, 3=testing, 4=unknown, 5=dormant, 6=notPresent, 7=lowerLayerDown';


--
-- Name: COLUMN if_entries.interface_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.interface_id IS 'FK to Interface entity when this ifEntry has an IP address (must be on same host)';


--
-- Name: COLUMN if_entries.neighbor_if_entry_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.neighbor_if_entry_id IS 'Full neighbor resolution: FK to remote IfEntry discovered via LLDP/CDP';


--
-- Name: COLUMN if_entries.neighbor_host_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.neighbor_host_id IS 'Partial neighbor resolution: FK to remote Host when specific port is unknown';


--
-- Name: COLUMN if_entries.lldp_mgmt_addr; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.lldp_mgmt_addr IS 'LLDP remote management address (lldpRemManAddr)';


--
-- Name: COLUMN if_entries.lldp_sys_desc; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.lldp_sys_desc IS 'LLDP remote system description (lldpRemSysDesc)';


--
-- Name: COLUMN if_entries.cdp_device_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.cdp_device_id IS 'CDP cache remote device ID (typically hostname)';


--
-- Name: COLUMN if_entries.cdp_port_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.cdp_port_id IS 'CDP cache remote port ID string';


--
-- Name: COLUMN if_entries.cdp_platform; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.cdp_platform IS 'CDP cache remote device platform (e.g., Cisco IOS)';


--
-- Name: COLUMN if_entries.cdp_address; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.cdp_address IS 'CDP cache remote device management IP address';


--
-- Name: interfaces; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.interfaces (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    host_id uuid NOT NULL,
    subnet_id uuid NOT NULL,
    ip_address inet NOT NULL,
    mac_address macaddr,
    name text,
    "position" integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.interfaces OWNER TO postgres;

--
-- Name: invites; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.invites (
    id uuid NOT NULL,
    organization_id uuid NOT NULL,
    permissions text NOT NULL,
    network_ids uuid[] NOT NULL,
    url text NOT NULL,
    created_by uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    send_to text
);


ALTER TABLE public.invites OWNER TO postgres;

--
-- Name: network_credentials; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.network_credentials (
    network_id uuid NOT NULL,
    credential_id uuid NOT NULL
);


ALTER TABLE public.network_credentials OWNER TO postgres;

--
-- Name: networks; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.networks (
    id uuid NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    organization_id uuid NOT NULL
);


ALTER TABLE public.networks OWNER TO postgres;

--
-- Name: COLUMN networks.organization_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.networks.organization_id IS 'The organization that owns and pays for this network';


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
    brevo_company_id text,
    has_payment_method boolean DEFAULT false NOT NULL,
    trial_end_date timestamp with time zone,
    plan_limit_notifications jsonb DEFAULT '{}'::jsonb NOT NULL,
    use_case text
);


ALTER TABLE public.organizations OWNER TO postgres;

--
-- Name: TABLE organizations; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON TABLE public.organizations IS 'Organizations that own networks and have Stripe subscriptions';


--
-- Name: COLUMN organizations.plan; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.organizations.plan IS 'The current billing plan for the organization (e.g., Community, Pro)';


--
-- Name: ports; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.ports (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    host_id uuid NOT NULL,
    port_number integer NOT NULL,
    protocol text NOT NULL,
    port_type text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT ports_port_number_check CHECK (((port_number >= 0) AND (port_number <= 65535))),
    CONSTRAINT ports_protocol_check CHECK ((protocol = ANY (ARRAY['Tcp'::text, 'Udp'::text])))
);


ALTER TABLE public.ports OWNER TO postgres;

--
-- Name: services; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.services (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    name text NOT NULL,
    host_id uuid NOT NULL,
    service_definition text NOT NULL,
    virtualization jsonb,
    source jsonb NOT NULL,
    "position" integer DEFAULT 0 NOT NULL
);


ALTER TABLE public.services OWNER TO postgres;

--
-- Name: shares; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.shares (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    topology_id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_by uuid NOT NULL,
    name text NOT NULL,
    is_enabled boolean DEFAULT true NOT NULL,
    expires_at timestamp with time zone,
    password_hash text,
    allowed_domains text[],
    options jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.shares OWNER TO postgres;

--
-- Name: subnets; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.subnets (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    cidr text NOT NULL,
    name text NOT NULL,
    description text,
    subnet_type text NOT NULL,
    source jsonb NOT NULL
);


ALTER TABLE public.subnets OWNER TO postgres;

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
    color text NOT NULL
);


ALTER TABLE public.tags OWNER TO postgres;

--
-- Name: topologies; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.topologies (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    network_id uuid NOT NULL,
    name text NOT NULL,
    edges jsonb NOT NULL,
    nodes jsonb NOT NULL,
    options jsonb NOT NULL,
    hosts jsonb NOT NULL,
    subnets jsonb NOT NULL,
    services jsonb NOT NULL,
    groups jsonb NOT NULL,
    is_stale boolean,
    last_refreshed timestamp with time zone DEFAULT now() NOT NULL,
    is_locked boolean,
    locked_at timestamp with time zone,
    locked_by uuid,
    removed_hosts uuid[],
    removed_services uuid[],
    removed_subnets uuid[],
    removed_groups uuid[],
    parent_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL,
    interfaces jsonb DEFAULT '[]'::jsonb NOT NULL,
    removed_interfaces uuid[] DEFAULT '{}'::uuid[],
    ports jsonb DEFAULT '[]'::jsonb NOT NULL,
    removed_ports uuid[] DEFAULT '{}'::uuid[],
    bindings jsonb DEFAULT '[]'::jsonb NOT NULL,
    removed_bindings uuid[] DEFAULT '{}'::uuid[],
    if_entries jsonb DEFAULT '[]'::jsonb NOT NULL,
    removed_if_entries uuid[] DEFAULT '{}'::uuid[],
    entity_tags jsonb DEFAULT '[]'::jsonb NOT NULL
);


ALTER TABLE public.topologies OWNER TO postgres;

--
-- Name: user_api_key_network_access; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.user_api_key_network_access (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    api_key_id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.user_api_key_network_access OWNER TO postgres;

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
-- Name: user_network_access; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.user_network_access (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.user_network_access OWNER TO postgres;

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
    pending_email text
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
20251006215000	users	2026-03-26 17:23:31.307833+00	t	\\x4f13ce14ff67ef0b7145987c7b22b588745bf9fbb7b673450c26a0f2f9a36ef8ca980e456c8d77cfb1b2d7a4577a64d7	3922823
20251006215100	networks	2026-03-26 17:23:31.313124+00	t	\\xeaa5a07a262709f64f0c59f31e25519580c79e2d1a523ce72736848946a34b17dd9adc7498eaf90551af6b7ec6d4e0e3	5099054
20251006215151	create hosts	2026-03-26 17:23:31.31866+00	t	\\x6ec7487074c0724932d21df4cf1ed66645313cf62c159a7179e39cbc261bcb81a24f7933a0e3cf58504f2a90fc5c1962	4416711
20251006215155	create subnets	2026-03-26 17:23:31.323458+00	t	\\xefb5b25742bd5f4489b67351d9f2494a95f307428c911fd8c5f475bfb03926347bdc269bbd048d2ddb06336945b27926	4088213
20251006215201	create groups	2026-03-26 17:23:31.32793+00	t	\\x0a7032bf4d33a0baf020e905da865cde240e2a09dda2f62aa535b2c5d4b26b20be30a3286f1b5192bd94cd4a5dbb5bcd	4242564
20251006215204	create daemons	2026-03-26 17:23:31.332613+00	t	\\xcfea93403b1f9cf9aac374711d4ac72d8a223e3c38a1d2a06d9edb5f94e8a557debac3668271f8176368eadc5105349f	4571192
20251006215212	create services	2026-03-26 17:23:31.33757+00	t	\\xd5b07f82fc7c9da2782a364d46078d7d16b5c08df70cfbf02edcfe9b1b24ab6024ad159292aeea455f15cfd1f4740c1d	5358912
20251029193448	user-auth	2026-03-26 17:23:31.343288+00	t	\\xfde8161a8db89d51eeade7517d90a41d560f19645620f2298f78f116219a09728b18e91251ae31e46a47f6942d5a9032	6444222
20251030044828	daemon api	2026-03-26 17:23:31.350048+00	t	\\x181eb3541f51ef5b038b2064660370775d1b364547a214a20dde9c9d4bb95a1c273cd4525ef29e61fa65a3eb4fee0400	1685788
20251030170438	host-hide	2026-03-26 17:23:31.352069+00	t	\\x87c6fda7f8456bf610a78e8e98803158caa0e12857c5bab466a5bb0004d41b449004a68e728ca13f17e051f662a15454	1248025
20251102224919	create discovery	2026-03-26 17:23:31.353626+00	t	\\xb32a04abb891aba48f92a059fae7341442355ca8e4af5d109e28e2a4f79ee8e11b2a8f40453b7f6725c2dd6487f26573	11451694
20251106235621	normalize-daemon-cols	2026-03-26 17:23:31.365424+00	t	\\x5b137118d506e2708097c432358bf909265b3cf3bacd662b02e2c81ba589a9e0100631c7801cffd9c57bb10a6674fb3b	1968360
20251107034459	api keys	2026-03-26 17:23:31.367717+00	t	\\x3133ec043c0c6e25b6e55f7da84cae52b2a72488116938a2c669c8512c2efe72a74029912bcba1f2a2a0a8b59ef01dde	9430846
20251107222650	oidc-auth	2026-03-26 17:23:31.377548+00	t	\\xd349750e0298718cbcd98eaff6e152b3fb45c3d9d62d06eedeb26c75452e9ce1af65c3e52c9f2de4bd532939c2f31096	31738534
20251110181948	orgs-billing	2026-03-26 17:23:31.40975+00	t	\\x5bbea7a2dfc9d00213bd66b473289ddd66694eff8a4f3eaab937c985b64c5f8c3ad2d64e960afbb03f335ac6766687aa	12347257
20251113223656	group-enhancements	2026-03-26 17:23:31.422498+00	t	\\xbe0699486d85df2bd3edc1f0bf4f1f096d5b6c5070361702c4d203ec2bb640811be88bb1979cfe51b40805ad84d1de65	1292750
20251117032720	daemon-mode	2026-03-26 17:23:31.424174+00	t	\\xdd0d899c24b73d70e9970e54b2c748d6b6b55c856ca0f8590fe990da49cc46c700b1ce13f57ff65abd6711f4bd8a6481	1223299
20251118143058	set-default-plan	2026-03-26 17:23:31.425702+00	t	\\xd19142607aef84aac7cfb97d60d29bda764d26f513f2c72306734c03cec2651d23eee3ce6cacfd36ca52dbddc462f917	1374374
20251118225043	save-topology	2026-03-26 17:23:31.427433+00	t	\\x011a594740c69d8d0f8b0149d49d1b53cfbf948b7866ebd84403394139cb66a44277803462846b06e762577adc3e61a3	10023332
20251123232748	network-permissions	2026-03-26 17:23:31.437893+00	t	\\x161be7ae5721c06523d6488606f1a7b1f096193efa1183ecdd1c2c9a4a9f4cad4884e939018917314aaf261d9a3f97ae	3101420
20251125001342	billing-updates	2026-03-26 17:23:31.441518+00	t	\\xa235d153d95aeb676e3310a52ccb69dfbd7ca36bba975d5bbca165ceeec7196da12119f23597ea5276c364f90f23db1e	1051929
20251128035448	org-onboarding-status	2026-03-26 17:23:31.443177+00	t	\\x1d7a7e9bf23b5078250f31934d1bc47bbaf463ace887e7746af30946e843de41badfc2b213ed64912a18e07b297663d8	1649470
20251129180942	nfs-consolidate	2026-03-26 17:23:31.445279+00	t	\\xb38f41d30699a475c2b967f8e43156f3b49bb10341bddbde01d9fb5ba805f6724685e27e53f7e49b6c8b59e29c74f98e	1399039
20251206052641	discovery-progress	2026-03-26 17:23:31.447026+00	t	\\x9d433b7b8c58d0d5437a104497e5e214febb2d1441a3ad7c28512e7497ed14fb9458e0d4ff786962a59954cb30da1447	1902987
20251206202200	plan-fix	2026-03-26 17:23:31.449229+00	t	\\x242f6699dbf485cf59a8d1b8cd9d7c43aeef635a9316be815a47e15238c5e4af88efaa0daf885be03572948dc0c9edac	1008884
20251207061341	daemon-url	2026-03-26 17:23:31.450573+00	t	\\x01172455c4f2d0d57371d18ef66d2ab3b7a8525067ef8a86945c616982e6ce06f5ea1e1560a8f20dadcd5be2223e6df1	2615168
20251210045929	tags	2026-03-26 17:23:31.453573+00	t	\\xe3dde83d39f8552b5afcdc1493cddfeffe077751bf55472032bc8b35fc8fc2a2caa3b55b4c2354ace7de03c3977982db	10083079
20251210175035	terms	2026-03-26 17:23:31.464069+00	t	\\xe47f0cf7aba1bffa10798bede953da69fd4bfaebf9c75c76226507c558a3595c6bfc6ac8920d11398dbdf3b762769992	1046998
20251213025048	hash-keys	2026-03-26 17:23:31.465474+00	t	\\xfc7cbb8ce61f0c225322297f7459dcbe362242b9001c06cb874b7f739cea7ae888d8f0cfaed6623bcbcb9ec54c8cd18b	9840275
20251214050638	scanopy	2026-03-26 17:23:31.475663+00	t	\\x0108bb39832305f024126211710689adc48d973ff66e5e59ff49468389b75c1ff95d1fbbb7bdb50e33ec1333a1f29ea6	1563890
20251215215724	topo-scanopy-fix	2026-03-26 17:23:31.477565+00	t	\\xed88a4b71b3c9b61d46322b5053362e5a25a9293cd3c420c9df9fcaeb3441254122b8a18f58c297f535c842b8a8b0a38	869484
20251217153736	category rename	2026-03-26 17:23:31.478747+00	t	\\x03af7ec905e11a77e25038a3c272645da96014da7c50c585a25cea3f9a7579faba3ff45114a5e589d144c9550ba42421	1885724
20251218053111	invite-persistence	2026-03-26 17:23:31.481021+00	t	\\x21d12f48b964acfd600f88e70ceb14abd9cf2a8a10db2eae2a6d8f44cf7d20749f93293631e6123e92b7c3c1793877c2	5706889
20251219211216	create shares	2026-03-26 17:23:31.487117+00	t	\\x036485debd3536f9e58ead728f461b925585911acf565970bf3b2ab295b12a2865606d6a56d334c5641dcd42adeb3d68	7429915
20251220170928	permissions-cleanup	2026-03-26 17:23:31.494945+00	t	\\x632f7b6702b494301e0d36fd3b900686b1a7f9936aef8c084b5880f1152b8256a125566e2b5ac40216eaadd3c4c64a03	1754365
20251220180000	commercial-to-community	2026-03-26 17:23:31.497064+00	t	\\x26fc298486c225f2f01271d611418377c403183ae51daf32fef104ec07c027f2017d138910c4fbfb5f49819a5f4194d6	1028062
20251221010000	cleanup subnet type	2026-03-26 17:23:31.498408+00	t	\\xb521121f3fd3a10c0de816977ac2a2ffb6118f34f8474ffb9058722abc0dc4cf5cbec83bc6ee49e79a68e6b715087f40	924498
20251221020000	remove host target	2026-03-26 17:23:31.499658+00	t	\\x77b5f8872705676ca81a5704bd1eaee90b9a52b404bdaa27a23da2ffd4858d3e131680926a5a00ad2a0d7a24ba229046	1049222
20251221030000	user network access	2026-03-26 17:23:31.501001+00	t	\\x5c23f5bb6b0b8ca699a17eee6730c4197a006ca21fecc79136a5e5697b9211a81b4cd08ceda70dace6a26408d021ff3a	7641292
20251221040000	interfaces table	2026-03-26 17:23:31.508967+00	t	\\xf7977b6f1e7e5108c614397d03a38c9bd9243fdc422575ec29610366a0c88f443de2132185878d8e291f06a50a8c3244	14045522
20251221050000	ports table	2026-03-26 17:23:31.52345+00	t	\\xdf72f9306b405be7be62c39003ef38408115e740b120f24e8c78b8e136574fff7965c52023b3bc476899613fa5f4fe35	9980949
20251221060000	bindings table	2026-03-26 17:23:31.533759+00	t	\\x933648a724bd179c7f47305e4080db85342d48712cde39374f0f88cde9d7eba8fe5fafba360937331e2a8178dec420c4	11762067
20251221070000	group bindings	2026-03-26 17:23:31.545978+00	t	\\x697475802f6c42e38deee6596f4ba786b09f7b7cd91742fbc5696dd0f9b3ddfce90dd905153f2b1a9e82f959f5a88302	6867518
20251222020000	tag cascade delete	2026-03-26 17:23:31.55323+00	t	\\xabfb48c0da8522f5c8ea6d482eb5a5f4562ed41f6160a5915f0fd477c7dd0517aa84760ef99ab3a5db3e0f21b0c69b5f	1280226
20251223232524	network remove default	2026-03-26 17:23:31.554804+00	t	\\x7099fe4e52405e46269d7ce364050da930b481e72484ad3c4772fd2911d2d505476d659fa9f400c63bc287512d033e18	1040625
20251225100000	color enum	2026-03-26 17:23:31.556162+00	t	\\x62cecd9d79a49835a3bea68a7959ab62aa0c1aaa7e2940dec6a7f8a714362df3649f0c1f9313672d9268295ed5a1cfa9	1686460
20251227010000	topology snapshot migration	2026-03-26 17:23:31.558207+00	t	\\xc042591d254869c0e79c8b52a9ede680fd26f094e2c385f5f017e115f5e3f31ad155f4885d095344f2642ebb70755d54	4665860
20251228010000	user api keys	2026-03-26 17:23:31.563227+00	t	\\xa41adb558a5b9d94a4e17af3f16839b83f7da072dbeac9251b12d8a84c7bec6df008009acf246468712a975bb36bb5f5	13169072
20251230160000	daemon version and maintainer	2026-03-26 17:23:31.576748+00	t	\\xafed3d9f00adb8c1b0896fb663af801926c218472a0a197f90ecdaa13305a78846a9e15af0043ec010328ba533fca68f	2992364
20260103000000	service position	2026-03-26 17:23:31.580157+00	t	\\x19d00e8c8b300d1c74d721931f4d771ec7bc4e06db0d6a78126e00785586fdc4bcff5b832eeae2fce0cb8d01e12a7fb5	2054862
20260106000000	interface mac index	2026-03-26 17:23:31.582598+00	t	\\xa26248372a1e31af46a9c6fbdaef178982229e2ceeb90cc6a289d5764f87a38747294b3adf5f21276b5d171e42bdb6ac	1911953
20260106204402	entity tags junction	2026-03-26 17:23:31.584899+00	t	\\xf73c604f9f0b8db065d990a861684b0dbd62c3ef9bead120c68431c933774de56491a53f021e79f09801680152f5a08a	14157329
20260108033856	fix entity tags json format	2026-03-26 17:23:31.599492+00	t	\\x197eaa063d4f96dd0e897ad8fd96cc1ba9a54dda40a93a5c12eac14597e4dea4c806dd0a527736fb5807b7a8870d9916	1585890
20260110000000	email verification	2026-03-26 17:23:31.601424+00	t	\\xb8da8433f58ba4ce846b9fa0c2551795747a8473ad10266b19685504847458ea69d27a0ce430151cfb426f5f5fb6ac3a	3789253
20260114145808	daemon user fk set null	2026-03-26 17:23:31.605639+00	t	\\x57b060be9fc314d7c5851c75661ca8269118feea6cf7ee9c61b147a0e117c4d39642cf0d1acdf7a723a9a76066c1b8ff	1113381
20260116010000	snmp credentials	2026-03-26 17:23:31.607051+00	t	\\x6f3971cf194d56883c61fa795406a8ab568307ed86544920d098b32a6a1ebb7effcb5ec38a70fdc9b617eff92d63d51e	7679574
20260116020000	host snmp fields	2026-03-26 17:23:31.615335+00	t	\\xf2f088c13ab0dd34e1cb1e5327b0b4137440b0146e5ce1e78b8d2dfa05d9b5a12a328eeb807988453a8a43ad8a1c95ba	4781987
20260116030000	if entries	2026-03-26 17:23:31.620436+00	t	\\xa58391708f8b21901ab9250af528f638a6055462f70ffddfd7c451433aacdabd62825546fa8be108f23a3cae78b8ae28	15347957
20260116100000	daemon api key link	2026-03-26 17:23:31.636326+00	t	\\x41088aa314ab173344a6b416280721806b2f296a32a8d8cae58c7e5717f389fe599134ed03980ed97e4b7659e99c4f82	3602521
20260131190000	add hubspot company id	2026-03-26 17:23:31.640421+00	t	\\x4326f95f4954e176157c1c3e034074a3e5c44da4d60bbd7a9e4b6238c9ef52a30f8b38d3c887864b6e4c1163dc062beb	906614
20260201021238	fix service acronym capitalization	2026-03-26 17:23:31.641653+00	t	\\x88b010ac8f0223d880ea6a730f11dc6d27fa5de9d8747de3431e46d59f1dbf2f72ae4a87c2e52c32152549f5c1f96bb2	1804892
20260204004436	add entity tags to topology	2026-03-26 17:23:31.643766+00	t	\\x3eff1a1490e77065ec861ef1b9aad8c55de0170106a42720f7931b3929b179122b16e44390b2652771bf91bba32a7757	1265459
20260205120000	billing overhaul	2026-03-26 17:23:31.64538+00	t	\\xbf850cfa0c40a3c65f574efd15fd55a4b702296203d28077a09d1c22076fee8601f2b78345aef370ab9163657de767ab	18138502
20260205183207	rename hubspot to brevo	2026-03-26 17:23:31.663967+00	t	\\x4678a7d80215e5eafb5e80af0daa20e2868a3b4f2112e88cb1b2b9efc87d63de3fb96c133f359b224c658789ae4b0d13	1143402
20260221120000	add plan limit notifications	2026-03-26 17:23:31.665441+00	t	\\xef770dac07e1d80888832f33184dc46c1d3b8185b91c507cb404468d6ad8c29cacf455178801c67aa27b6a626d3ad82d	1248637
20260222120000	add pending email	2026-03-26 17:23:31.667034+00	t	\\xddd220f7602c44548d56849c0a8d081ecd1da1383374a11e3e227c7d9becb73a49f5e5bb09ed65901c16df4c16e913e5	967148
20260301120000	add if name to if entries	2026-03-26 17:23:31.66833+00	t	\\xc9fc0a2b77ecbf0e1d5ab292c4fe162a26113468c878dfd26a3c63d89c0ee1957ca328ecfe25c611867a0e73780f0cb6	1029535
20260306002816	cleanup standby	2026-03-26 17:23:31.669664+00	t	\\x01b0c236a8a4d0d97f0f633b18f8cbdb92b6d72063289989b90a1b7b6b303e65e0557eb09927b2580dcb7e8ee5966c75	1037370
20260309120000	add org use case	2026-03-26 17:23:31.671056+00	t	\\xdb8c8a2f0f9416ba3b687fc75453d7c12c50a6f386b4784d21bd6adfc4a4a7556c637c25cf116118402bbd12c0d5aafe	956668
20260313120000	snmp extended discovery	2026-03-26 17:23:31.672603+00	t	\\xc4e72539099de1b830d87a169bfbabba4b8fb378a3c4c4a1dfca698adf3e403d750040d784c26d9fa343be2908064c9d	1885484
20260315120000	universal credentials	2026-03-26 17:23:31.674789+00	t	\\x87dc6f39202e81d5555df78a9d056b143f11bd22e6d7f483065f605e242a360902c72c4d5a49717e7fcc24a366bb5ff5	21870938
20260315120001	discovery scan settings	2026-03-26 17:23:31.697099+00	t	\\xe9da183fdd8e04e574f553f61f6f33efa046cdae38c846c8077b06c5260446fb4aa39da2449bda7f1d8cf3aa9f16e158	1231585
20260315120002	backfill org created milestone	2026-03-26 17:23:31.69867+00	t	\\x14f886a19773cd2263d86f88479be460d21f071d5212e3789c5c40b6415c293fc7d06c7b138351cc42108f89a14fe745	922884
20260316120000	fix jsonb null if entries	2026-03-26 17:23:31.699935+00	t	\\x65c358069710f7f86d6a3e257e658c2f241cc376433c3a0317b0ec9e1876a66f9738cb65c6ab1a5c197fe40d5aa2aa2b	1830761
20260319120000	rename snmp to snmpv2c	2026-03-26 17:23:31.702117+00	t	\\xdce5c9461f402e1672607078b2c571f0eb30b51d46f8e9414d8909efb40693f543e49e560cb7d703db274515043aa08e	1176522
20260321120000	add discovery scan count	2026-03-26 17:23:31.703689+00	t	\\x6c8201ab453a51632176d534c6604e0818e28a8a4a153e33e254f4dac0f9b67c9db394082cb663ff1b25941229cf96fc	2076903
\.


--
-- Data for Name: api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.api_keys (id, key, network_id, name, created_at, updated_at, last_used, expires_at, is_enabled, plaintext) FROM stdin;
019c5022-3b5c-485d-99f9-412e6fdb7ade	3815d47961d03f13cb67ead618edffd51d13eee7db6d43359d7a7829ab710381	278ebd82-9714-4374-9e9d-6f36fc737e13	scanopy-daemon-serverpoll API Key	2026-03-26 17:25:52.627191+00	2026-03-26 17:25:52.627191+00	2026-03-26 17:30:53.025622+00	\N	t	scp_d_mJKkKBsKG9ZoOuTOPziP48WAAIy1BIgu
aa98beac-46e6-401d-87f9-9e69515f890c	cbb9306b07b4d893d47d3f438873dc1ee1099f1bba16080cafd306cc46c87642	278ebd82-9714-4374-9e9d-6f36fc737e13	Compat Test API Key	2026-03-26 17:30:32.835179+00	2026-03-26 17:30:32.835179+00	2026-03-26 17:30:45.465732+00	\N	t	\N
df24d87a-a50f-4af6-a6d1-12a2b021f249	f88817addcf94d11a264cb8afda94bca6e60bfbaeab03651663919bd770e65ca	278ebd82-9714-4374-9e9d-6f36fc737e13	Integrated Daemon API Key	2026-03-26 17:23:34.460596+00	2026-03-26 17:23:34.460596+00	2026-03-26 17:30:46.995707+00	\N	t	\N
\.


--
-- Data for Name: bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.bindings (id, network_id, service_id, binding_type, interface_id, port_id, created_at, updated_at) FROM stdin;
6cd7e64c-0085-4fcc-a5e3-e9442db3d35d	278ebd82-9714-4374-9e9d-6f36fc737e13	21028e37-a633-4035-a05b-14147e25b334	Port	dca211ac-a264-4669-882f-af7865a0e99a	0f930761-8e71-4cc8-bc48-ff6576bedfa9	2026-01-26 14:03:24.349538+00	2026-01-26 14:03:24.349538+00
\.


--
-- Data for Name: credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.credentials (id, organization_id, name, credential_type, target_ips, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: daemons; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemons (id, network_id, host_id, created_at, last_seen, capabilities, updated_at, mode, url, name, version, user_id, api_key_id, is_unreachable, standby) FROM stdin;
565678ea-6b27-45bf-940a-7cfbb3b598d5	278ebd82-9714-4374-9e9d-6f36fc737e13	2a69c4df-31b5-4490-adc9-01e591b9f974	2026-03-26 17:25:52.63082+00	2026-03-26 17:30:31.761442+00	{"has_docker_socket": false, "interfaced_subnet_ids": ["74e42142-f121-40a9-a484-15e632404bd4", "403c1922-9137-4ae5-9a4d-4f97bc09af5f"]}	2026-03-26 17:25:52.63082+00	"server_poll"	http://daemon-serverpoll:60074	scanopy-daemon-serverpoll	0.15.3	b203dc51-2299-4adb-bca0-ac40343ff5e3	019c5022-3b5c-485d-99f9-412e6fdb7ade	f	f
38843217-52f8-4b49-aad5-97b7a550c3cf	278ebd82-9714-4374-9e9d-6f36fc737e13	2bbce8eb-5555-41ef-b999-68a5af971278	2026-03-26 17:23:34.532837+00	2026-03-26 17:30:46.999816+00	{"has_docker_socket": false, "interfaced_subnet_ids": ["c0cc364b-d3fc-438f-9559-db5af7e44aa6", "ec56206f-e2c6-48d9-9c2a-bd7f52795206"]}	2026-03-26 17:23:34.532837+00	"daemon_poll"		scanopy-daemon	0.15.3	b203dc51-2299-4adb-bca0-ac40343ff5e3	\N	f	f
\.


--
-- Data for Name: discovery; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.discovery (id, network_id, daemon_id, run_type, discovery_type, name, created_at, updated_at, scan_count, force_full_scan, pending_credential_ids) FROM stdin;
2ebdb263-ef0a-46db-8a97-1603acf44af6	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Scheduled", "enabled": true, "last_run": "2026-03-26T17:23:34.545918089Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "2bbce8eb-5555-41ef-b999-68a5af971278", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-26 17:23:34.543636+00	2026-03-26 17:25:52.260969+00	1	f	{}
f2fecdcc-5ed2-4caf-802a-0dd7acbfd496	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "901cf5c8-1989-4bf1-bf5d-e587ee211cda", "started_at": "2026-03-26T17:23:47.082705682Z", "finished_at": "2026-03-26T17:25:52.248714209Z", "discovery_type": {"type": "Unified", "host_id": "2bbce8eb-5555-41ef-b999-68a5af971278", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}, "hosts_discovered": 5, "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "2bbce8eb-5555-41ef-b999-68a5af971278", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-26 17:23:47.082705+00	2026-03-26 17:25:52.25999+00	0	f	{}
6581b7d7-4353-40d9-8ba1-ff84fe046fa7	278ebd82-9714-4374-9e9d-6f36fc737e13	565678ea-6b27-45bf-940a-7cfbb3b598d5	{"type": "Scheduled", "enabled": true, "last_run": "2026-03-26T17:26:01.773808176Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "2a69c4df-31b5-4490-adc9-01e591b9f974", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-26 17:26:01.772312+00	2026-03-26 17:26:01.773808+00	0	f	{}
db0bce4b-974b-416b-925e-06cf4809b41b	278ebd82-9714-4374-9e9d-6f36fc737e13	565678ea-6b27-45bf-940a-7cfbb3b598d5	{"type": "AdHoc", "last_run": "2026-03-26T17:25:52.997341600Z"}	{"type": "Unified", "host_id": "2a69c4df-31b5-4490-adc9-01e591b9f974", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	ServerPoll Integration Test Discovery	2026-03-26 17:25:52.98665+00	2026-03-26 17:30:31.902835+00	1	f	{}
76477f4f-cb3b-40b3-991d-d6e9c86d4bdc	278ebd82-9714-4374-9e9d-6f36fc737e13	565678ea-6b27-45bf-940a-7cfbb3b598d5	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "565678ea-6b27-45bf-940a-7cfbb3b598d5", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "2f915185-76ea-4a3a-8f81-7ca0298ae158", "started_at": "2026-03-26T17:26:31.845707448Z", "finished_at": "2026-03-26T17:30:31.892955213Z", "discovery_type": {"type": "Unified", "host_id": "2a69c4df-31b5-4490-adc9-01e591b9f974", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}, "hosts_discovered": 5, "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "2a69c4df-31b5-4490-adc9-01e591b9f974", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-26 17:26:31.845707+00	2026-03-26 17:30:31.902136+00	0	f	{}
261408d9-a9c8-4b9e-bed5-e0ccbec7f731	278ebd82-9714-4374-9e9d-6f36fc737e13	565678ea-6b27-45bf-940a-7cfbb3b598d5	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "565678ea-6b27-45bf-940a-7cfbb3b598d5", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "6845dcc8-6ea0-47af-9ce3-054b8cf1f667", "started_at": "2026-03-26T17:30:46.230848295Z", "finished_at": "2026-03-26T17:30:46.237902556Z", "discovery_type": {"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}	Self Report — My Network	2026-03-26 17:30:46.230848+00	2026-03-26 17:30:46.244263+00	0	f	{}
e4e54792-defb-4e51-aa99-deb0cc5d307e	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "969d115c-0112-4ba3-8757-bb1ece340112", "started_at": "2026-03-26T17:30:48.960080125Z", "finished_at": "2026-03-26T17:30:48.967102744Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:48.96008+00	2026-03-26 17:30:48.973077+00	0	f	{}
64fdedd7-f7eb-49ed-ac1c-2776747ea943	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "27ff5779-08b0-4970-aebd-04649d27c725", "started_at": "2026-03-26T17:30:46.502051637Z", "finished_at": "2026-03-26T17:30:46.509667095Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:46.502051+00	2026-03-26 17:30:46.517062+00	0	f	{}
5adf8f9e-535d-4c8a-9302-cc4eae3f2b11	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "c10d2eae-1267-4023-997e-5a51fdf97281", "started_at": "2026-03-26T17:30:46.776067863Z", "finished_at": "2026-03-26T17:30:46.783727944Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:46.776067+00	2026-03-26 17:30:46.790802+00	0	f	{}
ddd9f33a-0c50-4fa2-9e79-bbc00701998a	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "857fbace-0358-4642-8ca0-b14bb999f353", "started_at": "2026-03-26T17:30:47.050392460Z", "finished_at": "2026-03-26T17:30:47.058732220Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:47.050392+00	2026-03-26 17:30:47.065221+00	0	f	{}
7d415201-c2ae-48e8-aa9b-7b874ef5c090	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "59fba01e-fefd-4c66-9ef2-a85c0e76a811", "started_at": "2026-03-26T17:30:47.325281396Z", "finished_at": "2026-03-26T17:30:47.334235555Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:47.325281+00	2026-03-26 17:30:47.34268+00	0	f	{}
8a8db72e-0201-46ae-8baf-724b31c1e8c7	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 0, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "032af69a-4335-4c54-ac9c-8941c44050a3", "started_at": "2026-03-26T17:30:47.701653167Z", "finished_at": "2026-03-26T17:30:47.708062592Z", "discovery_type": {"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-26 17:30:47.701653+00	2026-03-26 17:30:47.713036+00	0	f	{}
b55ab618-e556-47be-a442-f702684909f0	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "34c804c2-c01c-4104-a3b3-e31ae6dd0b6f", "started_at": "2026-03-26T17:30:47.871412855Z", "finished_at": "2026-03-26T17:30:47.879299031Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:47.871412+00	2026-03-26 17:30:47.885251+00	0	f	{}
c35f3963-316e-416d-b954-08191f2c8028	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "4273fe18-d031-4977-950e-ad5a49d2ea8e", "started_at": "2026-03-26T17:30:48.146980027Z", "finished_at": "2026-03-26T17:30:48.154435939Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:48.14698+00	2026-03-26 17:30:48.161201+00	0	f	{}
a935885a-7139-467c-936f-c014f5352c96	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "5b19fece-cb82-45d8-a676-df53cc38a014", "started_at": "2026-03-26T17:30:48.413220079Z", "finished_at": "2026-03-26T17:30:48.420526380Z", "discovery_type": {"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}	Self Report — My Network	2026-03-26 17:30:48.41322+00	2026-03-26 17:30:48.426353+00	0	f	{}
a01643bf-c161-4d24-9563-57ae04dce8ca	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "b64df0ca-f173-4ba8-a48b-edf7e372974a", "started_at": "2026-03-26T17:30:49.225771425Z", "finished_at": "2026-03-26T17:30:49.233310411Z", "discovery_type": {"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}	Self Report — My Network	2026-03-26 17:30:49.225771+00	2026-03-26 17:30:49.238801+00	0	f	{}
f2cfbf4d-104a-4337-b13c-6ed691ecddb4	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "f2d5414b-3369-4280-929d-5422dd11a4b4", "started_at": "2026-03-26T17:30:49.489140450Z", "finished_at": "2026-03-26T17:30:49.497097253Z", "discovery_type": {"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}	Self Report — My Network	2026-03-26 17:30:49.48914+00	2026-03-26 17:30:49.505728+00	0	f	{}
7bcfe653-5d1c-4185-9eb2-cd5fd04fd045	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "5bd5e93e-a606-4c6e-b159-b04879bdd801", "started_at": "2026-03-26T17:30:48.687113856Z", "finished_at": "2026-03-26T17:30:48.695206457Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:48.687113+00	2026-03-26 17:30:48.701062+00	0	f	{}
b62ecc12-1406-48e8-abbf-1bf54c07d206	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "0ffe5496-d64e-41a0-ab86-ff28eed28819", "started_at": "2026-03-26T17:30:49.771871373Z", "finished_at": "2026-03-26T17:30:49.780343867Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:49.771871+00	2026-03-26 17:30:49.786636+00	0	f	{}
4b4c4c3b-153a-4c74-90db-83d056f15c93	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 0, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "d2b26f9b-0329-4191-8a60-58cf3c796dba", "started_at": "2026-03-26T17:30:50.147694126Z", "finished_at": "2026-03-26T17:30:50.154641558Z", "discovery_type": {"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-26 17:30:50.147694+00	2026-03-26 17:30:50.160622+00	0	f	{}
2d92f518-045e-400b-825d-ad13b1070b21	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "ec9dc330-67d0-4ce5-94d8-506859c74940", "started_at": "2026-03-26T17:30:50.850719698Z", "finished_at": "2026-03-26T17:30:50.859112514Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:50.850719+00	2026-03-26 17:30:50.865612+00	0	f	{}
668b0819-93f6-4f47-bf70-841d29b29198	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "abba33fb-bf1f-4e8b-985f-6a2d0b5d0380", "started_at": "2026-03-26T17:30:51.384604759Z", "finished_at": "2026-03-26T17:30:51.393414365Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:51.384604+00	2026-03-26 17:30:51.399028+00	0	f	{}
ac882fc8-7f5c-4d95-8b8d-e0e58f88923a	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "5e741620-3de9-476c-86e7-7ce656d0a5a8", "started_at": "2026-03-26T17:30:51.656568719Z", "finished_at": "2026-03-26T17:30:51.665467495Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:51.656568+00	2026-03-26 17:30:51.670711+00	0	f	{}
b4b16a0d-ade6-48c5-bff4-fda3216edc3d	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 0, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "2abdbee0-ac03-472c-b137-4321f33f03ca", "started_at": "2026-03-26T17:30:52.303021919Z", "finished_at": "2026-03-26T17:30:52.310334334Z", "discovery_type": {"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-26 17:30:52.303021+00	2026-03-26 17:30:52.315292+00	0	f	{}
54d553d9-9915-49fd-b34a-dd9a4f2612f6	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "c8c6530c-aaff-4d7c-a872-ba4457906d77", "started_at": "2026-03-26T17:30:53.014094468Z", "finished_at": "2026-03-26T17:30:53.023846031Z", "discovery_type": {"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}	Self Report — My Network	2026-03-26 17:30:53.014094+00	2026-03-26 17:30:53.030649+00	0	f	{}
8918b32a-272b-486c-b1c0-047e435b3721	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "bbe36108-56c4-4d82-9087-e577842ff202", "started_at": "2026-03-26T17:30:50.316066279Z", "finished_at": "2026-03-26T17:30:50.323463037Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:50.316066+00	2026-03-26 17:30:50.328868+00	0	f	{}
4ab72901-7116-46a3-9924-5c9764696434	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "7f54ecab-3771-4583-a7d0-f12569030e17", "started_at": "2026-03-26T17:30:51.115606289Z", "finished_at": "2026-03-26T17:30:51.123400446Z", "discovery_type": {"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}	Self Report — My Network	2026-03-26 17:30:51.115606+00	2026-03-26 17:30:51.129022+00	0	f	{}
ec4db415-fc4b-486f-91ca-db2345d393d0	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "f2d5414b-3369-4280-929d-5422dd11a4b4", "started_at": "2026-03-26T17:30:50.580873723Z", "finished_at": "2026-03-26T17:30:50.588831263Z", "discovery_type": {"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}	Self Report — My Network	2026-03-26 17:30:50.580873+00	2026-03-26 17:30:50.595077+00	0	f	{}
0911bb18-6090-4a46-964b-01a3f958f70c	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "22ea697f-065c-4d2e-a81f-0809764aad01", "started_at": "2026-03-26T17:30:51.928295081Z", "finished_at": "2026-03-26T17:30:51.936712669Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:51.928295+00	2026-03-26 17:30:51.942163+00	0	f	{}
1b76bf52-78b4-4183-8568-2eabd32efd2d	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "10fdd8f4-03b6-44ea-adb6-27e74136b365", "started_at": "2026-03-26T17:30:52.472056622Z", "finished_at": "2026-03-26T17:30:52.480645436Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:52.472056+00	2026-03-26 17:30:52.486021+00	0	f	{}
a35f7e53-1e8d-48e3-9d38-11a33bccae49	278ebd82-9714-4374-9e9d-6f36fc737e13	38843217-52f8-4b49-aad5-97b7a550c3cf	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "session_id": "3b0868af-d558-45e3-b688-2cf55472b6ee", "started_at": "2026-03-26T17:30:52.743481525Z", "finished_at": "2026-03-26T17:30:52.752810304Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 17:30:52.743481+00	2026-03-26 17:30:52.758322+00	0	f	{}
\.


--
-- Data for Name: entity_tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.entity_tags (id, entity_id, entity_type, tag_id, created_at) FROM stdin;
43664bd8-2045-4684-8943-f65642a90a50	51f5de3e-afaa-447d-b10b-628186c4b2b0	"Service"	4d48aff7-4ab3-4f35-8968-4fffdfdc81d5	2026-03-26 17:30:31.927902+00
\.


--
-- Data for Name: group_bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.group_bindings (id, group_id, binding_id, "position", created_at) FROM stdin;
\.


--
-- Data for Name: groups; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.groups (id, network_id, name, description, created_at, updated_at, source, color, edge_style, group_type) FROM stdin;
83c743de-9862-4e42-858c-d235da3a15e4	278ebd82-9714-4374-9e9d-6f36fc737e13		\N	2026-03-26 17:30:31.932472+00	2026-03-26 17:30:31.932472+00	{"type": "Manual"}	Yellow	"SmoothStep"	RequestPath
\.


--
-- Data for Name: host_credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.host_credentials (host_id, credential_id, interface_ids) FROM stdin;
\.


--
-- Data for Name: hosts; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.hosts (id, network_id, name, hostname, description, source, virtualization, created_at, updated_at, hidden, sys_descr, sys_object_id, sys_location, sys_contact, management_url, chassis_id, manufacturer, model, serial_number, sys_name) FROM stdin;
7891ed81-377c-4eca-b05e-bc8a17129f90	278ebd82-9714-4374-9e9d-6f36fc737e13	bfc749035741	bfc749035741	Scanopy daemon	{"type": "Discovery", "metadata": [{"date": "2026-01-26T14:03:24.349517222Z", "type": "SelfReport", "host_id": "7891ed81-377c-4eca-b05e-bc8a17129f90", "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf"}]}	null	2026-01-26 14:03:24.349521+00	2026-01-26 14:03:24.349521+00	f	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
\.


--
-- Data for Name: if_entries; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.if_entries (id, host_id, network_id, created_at, updated_at, if_index, if_descr, if_alias, if_type, speed_bps, admin_status, oper_status, mac_address, interface_id, neighbor_if_entry_id, neighbor_host_id, lldp_chassis_id, lldp_port_id, lldp_sys_name, lldp_port_desc, lldp_mgmt_addr, lldp_sys_desc, cdp_device_id, cdp_port_id, cdp_platform, cdp_address, if_name, fdb_macs) FROM stdin;
\.


--
-- Data for Name: interfaces; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.interfaces (id, network_id, host_id, subnet_id, ip_address, mac_address, name, "position", created_at, updated_at) FROM stdin;
dca211ac-a264-4669-882f-af7865a0e99a	278ebd82-9714-4374-9e9d-6f36fc737e13	7891ed81-377c-4eca-b05e-bc8a17129f90	c0cc364b-d3fc-438f-9559-db5af7e44aa6	172.25.0.4	16:6c:97:10:88:ac	eth0	0	2026-01-26 14:03:24.343892+00	2026-01-26 14:03:24.343892+00
\.


--
-- Data for Name: invites; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.invites (id, organization_id, permissions, network_ids, url, created_by, created_at, updated_at, expires_at, send_to) FROM stdin;
\.


--
-- Data for Name: network_credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.network_credentials (network_id, credential_id) FROM stdin;
\.


--
-- Data for Name: networks; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.networks (id, name, created_at, updated_at, organization_id) FROM stdin;
278ebd82-9714-4374-9e9d-6f36fc737e13	My Network	2026-03-26 17:23:34.438101+00	2026-03-26 17:23:34.438101+00	ee302260-241e-4199-b849-849c6c7bb227
\.


--
-- Data for Name: organizations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.organizations (id, name, stripe_customer_id, plan, plan_status, created_at, updated_at, onboarding, brevo_company_id, has_payment_method, trial_end_date, plan_limit_notifications, use_case) FROM stdin;
ee302260-241e-4199-b849-849c6c7bb227	My Organization	\N	{"rate": "Month", "type": "Community", "base_cents": 0, "host_cents": null, "seat_cents": null, "trial_days": 0, "network_cents": null, "included_hosts": null, "included_seats": null, "included_networks": null}	active	2026-03-26 17:23:34.426302+00	2026-03-26 17:23:34.426302+00	["OnboardingModalCompleted", "OrgCreated", "FirstDaemonRegistered", "FirstHostDiscovered", "FirstDiscoveryCompleted", "FirstTagCreated", "FirstGroupCreated", "FirstUserApiKeyCreated", "SecondNetworkCreated"]	\N	f	\N	{"hosts": "None", "seats": "None", "networks": "None"}	\N
\.


--
-- Data for Name: ports; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ports (id, network_id, host_id, port_number, protocol, port_type, created_at, updated_at) FROM stdin;
0f930761-8e71-4cc8-bc48-ff6576bedfa9	278ebd82-9714-4374-9e9d-6f36fc737e13	7891ed81-377c-4eca-b05e-bc8a17129f90	60073	Tcp	Custom	2026-01-26 14:03:24.349194+00	2026-01-26 14:03:24.349194+00
\.


--
-- Data for Name: services; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.services (id, network_id, created_at, updated_at, name, host_id, service_definition, virtualization, source, "position") FROM stdin;
21028e37-a633-4035-a05b-14147e25b334	278ebd82-9714-4374-9e9d-6f36fc737e13	2026-01-26 14:03:24.349547+00	2026-01-26 14:03:24.349547+00	Scanopy Daemon	7891ed81-377c-4eca-b05e-bc8a17129f90	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2026-01-26T14:03:24.349543722Z", "type": "SelfReport", "host_id": "7891ed81-377c-4eca-b05e-bc8a17129f90", "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf"}]}	0
\.


--
-- Data for Name: shares; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.shares (id, topology_id, network_id, created_by, name, is_enabled, expires_at, password_hash, allowed_domains, options, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: subnets; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.subnets (id, network_id, created_at, updated_at, cidr, name, description, subnet_type, source) FROM stdin;
c0cc364b-d3fc-438f-9559-db5af7e44aa6	278ebd82-9714-4374-9e9d-6f36fc737e13	2026-01-26 14:03:24.323228+00	2026-01-26 14:03:24.323228+00	"172.25.0.0/28"	172.25.0.0/28	\N	Lan	{"type": "Discovery", "metadata": [{"date": "2026-01-26T14:03:24.323199055Z", "type": "SelfReport", "host_id": "7891ed81-377c-4eca-b05e-bc8a17129f90", "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf"}]}
ec56206f-e2c6-48d9-9c2a-bd7f52795206	278ebd82-9714-4374-9e9d-6f36fc737e13	2026-03-26 17:30:46.993386+00	2026-03-26 17:30:46.993386+00	"127.0.0.0/8"	127.0.0.0/8	\N	Loopback	{"type": "Discovery", "metadata": [{"date": "2026-03-26T17:30:46.993382400Z", "type": "SelfReport", "host_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf"}]}
caf1e1e0-6088-4627-bb80-b5e7b60265b2	278ebd82-9714-4374-9e9d-6f36fc737e13	2026-03-26 17:30:54.722116+00	2026-03-26 17:30:54.722116+00	"10.1.0.0/24"	Blocked Subnet	\N	Lan	{"type": "System"}
\.


--
-- Data for Name: tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tags (id, organization_id, name, description, created_at, updated_at, color) FROM stdin;
4d48aff7-4ab3-4f35-8968-4fffdfdc81d5	ee302260-241e-4199-b849-849c6c7bb227	Integration Test Tag	\N	2026-03-26 17:30:31.913419+00	2026-03-26 17:30:31.913419+00	Yellow
\.


--
-- Data for Name: topologies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.topologies (id, network_id, name, edges, nodes, options, hosts, subnets, services, groups, is_stale, last_refreshed, is_locked, locked_at, locked_by, removed_hosts, removed_services, removed_subnets, removed_groups, parent_id, created_at, updated_at, tags, interfaces, removed_interfaces, ports, removed_ports, bindings, removed_bindings, if_entries, removed_if_entries, entity_tags) FROM stdin;
8295aa65-e0af-477a-a603-406b5104eab7	278ebd82-9714-4374-9e9d-6f36fc737e13	My Topology	[]	[]	{"local": {"tag_filter": {"hidden_host_tag_ids": [], "hidden_subnet_tag_ids": [], "hidden_service_tag_ids": []}, "show_minimap": true, "no_fade_edges": false, "hide_edge_types": ["HostVirtualization"], "left_zone_title": "Infrastructure", "hide_resize_handles": false}, "request": {"hide_ports": false, "hide_service_categories": ["OpenPorts"], "show_gateway_in_left_zone": true, "group_docker_bridges_by_host": true, "left_zone_service_categories": ["DNS", "ReverseProxy"], "hide_vm_title_on_docker_container": false}}	[{"id": "2bbce8eb-5555-41ef-b999-68a5af971278", "name": "scanopy-daemon", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-03-26T17:23:47.092630228Z", "type": "Unified", "host_id": "2bbce8eb-5555-41ef-b999-68a5af971278", "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}]}, "hostname": "92dfc2d8a777", "created_at": "2026-03-26T17:23:34.525150Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:23:34.525150Z", "description": null, "virtualization": null, "credential_assignments": []}, {"id": "25c7e9d4-f667-43e3-88a9-d8b59b47b53a", "name": "scanopy-postgres-dev-1.scanopy_scanopy-dev", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-03-26T17:24:30.828801107Z", "type": "Network", "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "hostname": "scanopy-postgres-dev-1.scanopy_scanopy-dev", "created_at": "2026-03-26T17:23:47.241527Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:23:47.241527Z", "description": null, "virtualization": null, "credential_assignments": []}, {"id": "8a1c8fcb-e0dd-4561-a916-9c0c50b5ff50", "name": "scanopy-daemon-serverpoll-1.scanopy_scanopy-dev", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-03-26T17:23:58.222284115Z", "type": "Network", "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "hostname": "scanopy-daemon-serverpoll-1.scanopy_scanopy-dev", "created_at": "2026-03-26T17:23:47.342605Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:23:47.342605Z", "description": null, "virtualization": null, "credential_assignments": []}, {"id": "8110a97a-cc2a-4295-abcc-f5acd255fb42", "name": "172.25.0.1", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": []}, "hostname": null, "created_at": "2026-03-26T17:23:47.443593Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:23:47.443593Z", "description": null, "virtualization": null, "credential_assignments": []}, {"id": "141e399a-6cb1-45fa-b2ae-dc997a16fd2a", "name": "homeassistant-discovery.scanopy_scanopy-dev", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-03-26T17:24:47.175858554Z", "type": "Network", "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "hostname": "homeassistant-discovery.scanopy_scanopy-dev", "created_at": "2026-03-26T17:23:47.544995Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:23:47.544995Z", "description": null, "virtualization": null, "credential_assignments": []}, {"id": "fdfecae4-b11e-493c-bafc-f6bca3365c7d", "name": "scanopy-server-1.scanopy_scanopy-dev", "tags": [], "hidden": false, "source": {"type": "Discovery", "metadata": [{"date": "2026-03-26T17:24:14.657225968Z", "type": "Network", "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}, {"date": "2026-03-26T17:24:14.657225968Z", "type": "Network", "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "hostname": "scanopy-server-1.scanopy_scanopy-dev", "created_at": "2026-03-26T17:23:47.646461Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:23:47.646461Z", "description": null, "virtualization": null, "credential_assignments": []}]	[{"id": "c0cc364b-d3fc-438f-9559-db5af7e44aa6", "cidr": "172.25.0.0/28", "name": "172.25.0.0/28", "tags": [], "source": {"type": "Discovery", "metadata": [{"date": "2026-01-26T14:03:24.323199055Z", "type": "SelfReport", "host_id": "7891ed81-377c-4eca-b05e-bc8a17129f90", "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf"}]}, "created_at": "2026-01-26T14:03:24.323228Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-01-26T14:03:24.323228Z", "description": null, "subnet_type": "Lan"}, {"id": "ec56206f-e2c6-48d9-9c2a-bd7f52795206", "cidr": "127.0.0.0/8", "name": "127.0.0.0/8", "tags": [], "source": {"type": "Discovery", "metadata": [{"date": "2026-03-26T17:30:46.993382400Z", "type": "SelfReport", "host_id": "38843217-52f8-4b49-aad5-97b7a550c3cf", "daemon_id": "38843217-52f8-4b49-aad5-97b7a550c3cf"}]}, "created_at": "2026-03-26T17:30:46.993386Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:30:46.993386Z", "description": null, "subnet_type": "Loopback"}, {"id": "caf1e1e0-6088-4627-bb80-b5e7b60265b2", "cidr": "10.1.0.0/24", "name": "Blocked Subnet", "tags": [], "source": {"type": "System"}, "created_at": "2026-03-26T17:30:54.722116Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:30:54.722116Z", "description": null, "subnet_type": "Lan"}]	[{"id": "f4264228-e4ea-46f5-bfd6-6d1a3ecba86a", "name": "Scanopy Daemon", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2026-03-26T17:26:31.854236308Z", "type": "Unified", "host_id": "2a69c4df-31b5-4490-adc9-01e591b9f974", "daemon_id": "565678ea-6b27-45bf-940a-7cfbb3b598d5", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}]}, "host_id": "2a69c4df-31b5-4490-adc9-01e591b9f974", "bindings": [{"id": "95ceec0a-e70a-4c2c-bac5-166e482e5f78", "type": "Port", "port_id": "b53794d0-b65e-48bb-b915-faf3616bfc57", "created_at": "2026-03-26T17:26:31.854231Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "service_id": "f4264228-e4ea-46f5-bfd6-6d1a3ecba86a", "updated_at": "2026-03-26T17:26:31.854231Z", "interface_id": "9775f1a1-6699-4349-b78d-023310150cac"}, {"id": "dc166917-2c49-4621-a52a-9fcfb1abf3bd", "type": "Port", "port_id": "b53794d0-b65e-48bb-b915-faf3616bfc57", "created_at": "2026-03-26T17:26:31.854233Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "service_id": "f4264228-e4ea-46f5-bfd6-6d1a3ecba86a", "updated_at": "2026-03-26T17:26:31.854233Z", "interface_id": "9f980730-80f0-480c-908f-95d92a6d1faf"}], "position": 0, "created_at": "2026-03-26T17:26:31.854261Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:26:31.854261Z", "virtualization": null, "service_definition": "Scanopy Daemon"}, {"id": "178c36a0-d5f3-4c35-808c-10e482b64907", "name": "Scanopy Daemon", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.4:60073/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-03-26T17:28:25.631610700Z", "type": "Network", "daemon_id": "565678ea-6b27-45bf-940a-7cfbb3b598d5", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "49daf480-920e-4f32-820d-ec9f88305a16", "bindings": [{"id": "030a726a-3c73-4102-ac67-d39dacea94f3", "type": "Port", "port_id": "e72cb4bd-f001-43b3-87c2-e4a710c069f0", "created_at": "2026-03-26T17:28:25.631630Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "service_id": "178c36a0-d5f3-4c35-808c-10e482b64907", "updated_at": "2026-03-26T17:28:25.631630Z", "interface_id": "6d1a2cb3-6935-44b6-9e06-2dfab6c6dec2"}], "position": 0, "created_at": "2026-03-26T17:28:25.631634Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:28:25.631634Z", "virtualization": null, "service_definition": "Scanopy Daemon"}, {"id": "27b908dc-dd4e-416c-ac52-9de8b0495553", "name": "PostgreSQL", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 5432/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-03-26T17:29:01.425184377Z", "type": "Network", "daemon_id": "565678ea-6b27-45bf-940a-7cfbb3b598d5", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "00fa6371-057e-4f1c-908b-f67259836173", "bindings": [{"id": "cb3e1bfa-6192-40b6-b69f-247a7d484d80", "type": "Port", "port_id": "fe16fe72-74e4-42e6-b5d7-ef2cb0444cd9", "created_at": "2026-03-26T17:29:01.425200Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "service_id": "27b908dc-dd4e-416c-ac52-9de8b0495553", "updated_at": "2026-03-26T17:29:01.425200Z", "interface_id": "5091793e-4b2b-4e4a-92b1-5fdd886b16b9"}], "position": 0, "created_at": "2026-03-26T17:29:01.425205Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:29:01.425205Z", "virtualization": null, "service_definition": "PostgreSQL"}, {"id": "51f5de3e-afaa-447d-b10b-628186c4b2b0", "name": "Home Assistant", "tags": ["4d48aff7-4ab3-4f35-8968-4fffdfdc81d5"], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.5:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-03-26T17:29:14.837314324Z", "type": "Network", "daemon_id": "565678ea-6b27-45bf-940a-7cfbb3b598d5", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "257087e6-56a5-4871-99f8-7cd1a10e4764", "bindings": [{"id": "aaa709ee-7835-43fb-b306-4e61babd266a", "type": "Port", "port_id": "854bb435-a508-401d-8461-58581e429db4", "created_at": "2026-03-26T17:29:14.837333Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "service_id": "51f5de3e-afaa-447d-b10b-628186c4b2b0", "updated_at": "2026-03-26T17:29:14.837333Z", "interface_id": "b4264754-d4ce-4a49-bf86-7d986a6600b0"}], "position": 0, "created_at": "2026-03-26T17:29:14.837337Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:29:14.837337Z", "virtualization": null, "service_definition": "Home Assistant"}, {"id": "da6723e0-e5e7-499a-9d72-f0ac4f55ced8", "name": "Scanopy Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-03-26T17:29:25.062039059Z", "type": "Network", "daemon_id": "565678ea-6b27-45bf-940a-7cfbb3b598d5", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "856d9408-482a-457a-8e2e-ecc74e3d651f", "bindings": [{"id": "b95f63a5-190c-42c8-b091-0f33e6755a90", "type": "Port", "port_id": "70453fb5-bcf1-4a42-88e7-5af368c60ccb", "created_at": "2026-03-26T17:29:25.062059Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "service_id": "da6723e0-e5e7-499a-9d72-f0ac4f55ced8", "updated_at": "2026-03-26T17:29:25.062059Z", "interface_id": "c9483535-0b2b-485b-a804-f8f33fe9940e"}], "position": 0, "created_at": "2026-03-26T17:29:25.062062Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:29:25.062062Z", "virtualization": null, "service_definition": "Scanopy Server"}, {"id": "306b5518-a9c3-41a6-a941-5253440446fa", "name": "Home Assistant", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-03-26T17:29:33.186984915Z", "type": "Network", "daemon_id": "565678ea-6b27-45bf-940a-7cfbb3b598d5", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "856d9408-482a-457a-8e2e-ecc74e3d651f", "bindings": [{"id": "4a6a90dd-bf76-402d-9364-7be60cfb8311", "type": "Port", "port_id": "aa58ff91-2749-4c58-b111-169fb0643593", "created_at": "2026-03-26T17:29:33.187004Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "service_id": "306b5518-a9c3-41a6-a941-5253440446fa", "updated_at": "2026-03-26T17:29:33.187004Z", "interface_id": "c9483535-0b2b-485b-a804-f8f33fe9940e"}], "position": 1, "created_at": "2026-03-26T17:29:33.187008Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:29:33.187008Z", "virtualization": null, "service_definition": "Home Assistant"}, {"id": "6e26fdb7-2273-4cae-b582-14eddaa7591a", "name": "SSH", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-03-26T17:29:36.201074579Z", "type": "Network", "daemon_id": "565678ea-6b27-45bf-940a-7cfbb3b598d5", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "856d9408-482a-457a-8e2e-ecc74e3d651f", "bindings": [{"id": "484ffaf0-a02c-46db-a550-0814446b0a46", "type": "Port", "port_id": "abd516d8-4efa-45ae-b21e-2cb4c43aaeb5", "created_at": "2026-03-26T17:29:36.201090Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "service_id": "6e26fdb7-2273-4cae-b582-14eddaa7591a", "updated_at": "2026-03-26T17:29:36.201090Z", "interface_id": "c9483535-0b2b-485b-a804-f8f33fe9940e"}], "position": 2, "created_at": "2026-03-26T17:29:36.201094Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:29:36.201094Z", "virtualization": null, "service_definition": "SSH"}]	[{"id": "83c743de-9862-4e42-858c-d235da3a15e4", "name": "", "tags": [], "color": "Yellow", "source": {"type": "Manual"}, "created_at": "2026-03-26T17:30:31.932472Z", "edge_style": "SmoothStep", "group_type": "RequestPath", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:30:31.932472Z", "binding_ids": [], "description": null}, {"id": "f158d8f8-e371-4469-9a92-7feaf198334e", "name": "Updated Group", "tags": [], "color": "Red", "source": {"type": "Manual"}, "created_at": "2026-03-26T17:30:53.730694Z", "edge_style": "Bezier", "group_type": "RequestPath", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "updated_at": "2026-03-26T17:30:53.730694Z", "binding_ids": [], "description": "Test description"}]	t	2026-03-26 17:23:34.458345+00	f	\N	\N	{64e8aad3-3caf-47eb-96d2-50d11c3570db,7e4d8805-a31d-4950-9ec4-9a5248766397,eedfd193-3686-4b53-b302-e9a2135b0ab4}	{ca72ecb5-124b-4b02-927f-34635e637c93}	{90aefcc3-9d25-4324-bf75-ddea93d7942a}	{f158d8f8-e371-4469-9a92-7feaf198334e}	\N	2026-03-26 17:23:34.445444+00	2026-03-26 17:23:34.445444+00	{}	[]	{}	[]	{}	[{"id": "3075b765-205e-4c0c-9712-e98d915d3584", "type": "Port", "port_id": "44cc17cb-c646-4334-9bbf-2d18274dd959", "created_at": "2026-03-26T17:23:47.092644Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "service_id": "1507e156-625d-4143-843d-f977bd927c6c", "updated_at": "2026-03-26T17:23:47.092644Z", "interface_id": "c7649d0d-563e-4a3b-a703-833923fe1857"}, {"id": "d03ad7b5-8aba-45c9-9891-91df77403bae", "type": "Port", "port_id": "44cc17cb-c646-4334-9bbf-2d18274dd959", "created_at": "2026-03-26T17:23:47.092646Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "service_id": "1507e156-625d-4143-843d-f977bd927c6c", "updated_at": "2026-03-26T17:23:47.092646Z", "interface_id": "b37c0584-5876-47a1-908d-39efe6f26c05"}, {"id": "0c6b94f1-1327-484d-aad3-280faa02b02d", "type": "Port", "port_id": "5fa1c26b-ec39-4ae8-8d51-97a77a931a6b", "created_at": "2026-03-26T17:24:19.889213Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "service_id": "ee7949bc-3fea-448d-9bff-c7132aeed08f", "updated_at": "2026-03-26T17:24:19.889213Z", "interface_id": "44fc5b3b-9a76-4d1a-8053-f5609883df26"}, {"id": "8b424ff1-1732-4bff-b667-508540da4785", "type": "Port", "port_id": "fe33291e-ab4e-4004-90fd-d815dbb4b8ea", "created_at": "2026-03-26T17:24:47.148573Z", "network_id": "278ebd82-9714-4374-9e9d-6f36fc737e13", "service_id": "a683a135-43cf-48ad-af69-d95fe9f4856e", "updated_at": "2026-03-26T17:24:47.148573Z", "interface_id": "dec693a5-6633-4ae9-bb8d-4a24b808589c"}]	{}	[]	{}	[]
\.


--
-- Data for Name: user_api_key_network_access; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_api_key_network_access (id, api_key_id, network_id, created_at) FROM stdin;
\.


--
-- Data for Name: user_api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_api_keys (id, key, user_id, organization_id, permissions, name, created_at, updated_at, last_used, expires_at, is_enabled) FROM stdin;
\.


--
-- Data for Name: user_network_access; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_network_access (id, user_id, network_id, created_at) FROM stdin;
\.


--
-- Data for Name: users; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.users (id, created_at, updated_at, password_hash, oidc_provider, oidc_subject, oidc_linked_at, email, organization_id, permissions, tags, terms_accepted_at, email_verified, email_verification_token, email_verification_expires, password_reset_token, password_reset_expires, pending_email) FROM stdin;
b203dc51-2299-4adb-bca0-ac40343ff5e3	2026-03-26 17:23:34.428671+00	2026-03-26 17:23:34.428671+00	$argon2id$v=19$m=19456,t=2,p=1$d8rxTq/q4kB5RCwKkyItow$3NmUH7lZM+l9WBUosKny9mtXhqC72fky0nGQp02pfMw	\N	\N	\N	user@gmail.com	ee302260-241e-4199-b849-849c6c7bb227	Owner	{}	\N	t	\N	\N	\N	\N	\N
57775825-f864-422e-b2b0-aedb0bfd54cb	2026-03-26 17:30:54.39506+00	2026-03-26 17:30:54.39506+00	\N	\N	\N	\N	user@example.com	ee302260-241e-4199-b849-849c6c7bb227	Owner	{}	\N	f	\N	\N	\N	\N	\N
\.


--
-- Data for Name: session; Type: TABLE DATA; Schema: tower_sessions; Owner: postgres
--

COPY tower_sessions.session (id, data, expiry_date) FROM stdin;
3Yyv6fEmJIEtkKEq4oWgvw	\\x93c410bfa085e22aa1902d812426f1e9af8cdd81a7757365725f6964d92462323033646335312d323239392d346164622d626361302d61633430333433666635653399cd07ea5c111722ce20de9f85000000	2026-04-02 17:23:34.55146+00
SaOsGLePAn6PzdNbRoWykA	\\x93c41090b285465bd3cd8f7e028fb718aca34982ad70656e64696e675f736574757082a76e6574776f726b83a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92434376436613561302d636434622d346433372d393035632d393965373732613138666263ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea7757365725f6964d92462323033646335312d323239392d346164622d626361302d61633430333433666635653399cd07ea5c111e20ce31a34064000000	2026-04-02 17:30:32.832782+00
8sEbhtoq2rRBvgItyfYqBg	\\x93c410062af6c92d02be41b4da2ada861bc1f282a7757365725f6964d92462323033646335312d323239392d346164622d626361302d616334303334336666356533ad70656e64696e675f736574757082a76e6574776f726b83a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92465393234336236632d356266382d343536302d613039652d383462623163613764653163ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6e99cd07ea5c111e2ece0065d777000000	2026-04-02 17:30:46.006674+00
5PT1Ko0msFmrkC-qjq4G4A	\\x93c410e006ae8eaa2f90ab59b0268d2af5f4e482a7757365725f6964d92462323033646335312d323239392d346164622d626361302d616334303334336666356533ad70656e64696e675f736574757082a76e6574776f726b83a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92434643664366530612d306461352d346537362d383336362d376335366637633136633330ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6e99cd07ea5c111e35ce23117479000000	2026-04-02 17:30:53.588346+00
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
-- Name: entity_tags entity_tags_entity_id_entity_type_tag_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_entity_id_entity_type_tag_id_key UNIQUE (entity_id, entity_type, tag_id);


--
-- Name: entity_tags entity_tags_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_pkey PRIMARY KEY (id);


--
-- Name: group_bindings group_bindings_group_id_binding_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_bindings
    ADD CONSTRAINT group_bindings_group_id_binding_id_key UNIQUE (group_id, binding_id);


--
-- Name: group_bindings group_bindings_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_bindings
    ADD CONSTRAINT group_bindings_pkey PRIMARY KEY (id);


--
-- Name: groups groups_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.groups
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
-- Name: if_entries if_entries_host_id_if_index_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_host_id_if_index_key UNIQUE (host_id, if_index);


--
-- Name: if_entries if_entries_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_pkey PRIMARY KEY (id);


--
-- Name: interfaces interfaces_host_id_subnet_id_ip_address_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_host_id_subnet_id_ip_address_key UNIQUE (host_id, subnet_id, ip_address);


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
-- Name: network_credentials network_credentials_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.network_credentials
    ADD CONSTRAINT network_credentials_pkey PRIMARY KEY (network_id, credential_id);


--
-- Name: networks networks_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.networks
    ADD CONSTRAINT networks_pkey PRIMARY KEY (id);


--
-- Name: organizations organizations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.organizations
    ADD CONSTRAINT organizations_pkey PRIMARY KEY (id);


--
-- Name: ports ports_host_id_port_number_protocol_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_host_id_port_number_protocol_key UNIQUE (host_id, port_number, protocol);


--
-- Name: ports ports_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_pkey PRIMARY KEY (id);


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
-- Name: subnets subnets_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_pkey PRIMARY KEY (id);


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
-- Name: user_api_key_network_access user_api_key_network_access_api_key_id_network_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_network_access
    ADD CONSTRAINT user_api_key_network_access_api_key_id_network_id_key UNIQUE (api_key_id, network_id);


--
-- Name: user_api_key_network_access user_api_key_network_access_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_network_access
    ADD CONSTRAINT user_api_key_network_access_pkey PRIMARY KEY (id);


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
-- Name: user_network_access user_network_access_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_network_access
    ADD CONSTRAINT user_network_access_pkey PRIMARY KEY (id);


--
-- Name: user_network_access user_network_access_user_id_network_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_network_access
    ADD CONSTRAINT user_network_access_user_id_network_id_key UNIQUE (user_id, network_id);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: session session_pkey; Type: CONSTRAINT; Schema: tower_sessions; Owner: postgres
--

ALTER TABLE ONLY tower_sessions.session
    ADD CONSTRAINT session_pkey PRIMARY KEY (id);


--
-- Name: idx_api_keys_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_api_keys_key ON public.api_keys USING btree (key);


--
-- Name: idx_api_keys_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_api_keys_network ON public.api_keys USING btree (network_id);


--
-- Name: idx_bindings_interface; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_interface ON public.bindings USING btree (interface_id);


--
-- Name: idx_bindings_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_network ON public.bindings USING btree (network_id);


--
-- Name: idx_bindings_port; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_port ON public.bindings USING btree (port_id);


--
-- Name: idx_bindings_service; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_service ON public.bindings USING btree (service_id);


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
-- Name: idx_daemons_api_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_daemons_api_key ON public.daemons USING btree (api_key_id) WHERE (api_key_id IS NOT NULL);


--
-- Name: idx_daemons_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_daemons_network ON public.daemons USING btree (network_id);


--
-- Name: idx_discovery_daemon; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_daemon ON public.discovery USING btree (daemon_id);


--
-- Name: idx_discovery_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_network ON public.discovery USING btree (network_id);


--
-- Name: idx_entity_tags_entity; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_entity ON public.entity_tags USING btree (entity_id, entity_type);


--
-- Name: idx_entity_tags_tag_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_tag_id ON public.entity_tags USING btree (tag_id);


--
-- Name: idx_group_bindings_binding; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_group_bindings_binding ON public.group_bindings USING btree (binding_id);


--
-- Name: idx_group_bindings_group; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_group_bindings_group ON public.group_bindings USING btree (group_id);


--
-- Name: idx_groups_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_groups_network ON public.groups USING btree (network_id);


--
-- Name: idx_hosts_chassis_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_chassis_id ON public.hosts USING btree (chassis_id);


--
-- Name: idx_hosts_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_network ON public.hosts USING btree (network_id);


--
-- Name: idx_if_entries_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_host ON public.if_entries USING btree (host_id);


--
-- Name: idx_if_entries_interface; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_interface ON public.if_entries USING btree (interface_id);


--
-- Name: idx_if_entries_mac_address; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_mac_address ON public.if_entries USING btree (mac_address);


--
-- Name: idx_if_entries_neighbor_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_neighbor_host ON public.if_entries USING btree (neighbor_host_id);


--
-- Name: idx_if_entries_neighbor_if_entry; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_neighbor_if_entry ON public.if_entries USING btree (neighbor_if_entry_id);


--
-- Name: idx_if_entries_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_network ON public.if_entries USING btree (network_id);


--
-- Name: idx_interfaces_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_host ON public.interfaces USING btree (host_id);


--
-- Name: idx_interfaces_host_mac; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_host_mac ON public.interfaces USING btree (host_id, mac_address) WHERE (mac_address IS NOT NULL);


--
-- Name: idx_interfaces_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_network ON public.interfaces USING btree (network_id);


--
-- Name: idx_interfaces_subnet; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_subnet ON public.interfaces USING btree (subnet_id);


--
-- Name: idx_invites_expires_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_invites_expires_at ON public.invites USING btree (expires_at);


--
-- Name: idx_invites_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_invites_organization ON public.invites USING btree (organization_id);


--
-- Name: idx_networks_owner_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_networks_owner_organization ON public.networks USING btree (organization_id);


--
-- Name: idx_organizations_stripe_customer; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_organizations_stripe_customer ON public.organizations USING btree (stripe_customer_id);


--
-- Name: idx_ports_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_host ON public.ports USING btree (host_id);


--
-- Name: idx_ports_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_network ON public.ports USING btree (network_id);


--
-- Name: idx_ports_number; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_number ON public.ports USING btree (port_number);


--
-- Name: idx_services_host_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_host_id ON public.services USING btree (host_id);


--
-- Name: idx_services_host_position; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_host_position ON public.services USING btree (host_id, "position");


--
-- Name: idx_services_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_network ON public.services USING btree (network_id);


--
-- Name: idx_shares_enabled; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_shares_enabled ON public.shares USING btree (is_enabled) WHERE (is_enabled = true);


--
-- Name: idx_shares_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_shares_network ON public.shares USING btree (network_id);


--
-- Name: idx_shares_topology; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_shares_topology ON public.shares USING btree (topology_id);


--
-- Name: idx_subnets_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnets_network ON public.subnets USING btree (network_id);


--
-- Name: idx_tags_org_name; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_tags_org_name ON public.tags USING btree (organization_id, name);


--
-- Name: idx_tags_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tags_organization ON public.tags USING btree (organization_id);


--
-- Name: idx_topologies_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_topologies_network ON public.topologies USING btree (network_id);


--
-- Name: idx_user_api_key_network_access_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_key_network_access_key ON public.user_api_key_network_access USING btree (api_key_id);


--
-- Name: idx_user_api_key_network_access_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_key_network_access_network ON public.user_api_key_network_access USING btree (network_id);


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
-- Name: idx_user_network_access_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_network_access_network ON public.user_network_access USING btree (network_id);


--
-- Name: idx_user_network_access_user; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_network_access_user ON public.user_network_access USING btree (user_id);


--
-- Name: idx_users_email_lower; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_users_email_lower ON public.users USING btree (lower(email));


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
-- Name: users reassign_daemons_before_user_delete; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER reassign_daemons_before_user_delete BEFORE DELETE ON public.users FOR EACH ROW EXECUTE FUNCTION public.reassign_daemons_on_user_delete();


--
-- Name: api_keys api_keys_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_interface_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_interface_id_fkey FOREIGN KEY (interface_id) REFERENCES public.interfaces(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


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
-- Name: credentials credentials_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.credentials
    ADD CONSTRAINT credentials_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: daemons daemons_api_key_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_api_key_id_fkey FOREIGN KEY (api_key_id) REFERENCES public.api_keys(id) ON DELETE SET NULL;


--
-- Name: daemons daemons_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: daemons daemons_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: discovery discovery_daemon_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery
    ADD CONSTRAINT discovery_daemon_id_fkey FOREIGN KEY (daemon_id) REFERENCES public.daemons(id) ON DELETE CASCADE;


--
-- Name: discovery discovery_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery
    ADD CONSTRAINT discovery_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: entity_tags entity_tags_tag_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_tag_id_fkey FOREIGN KEY (tag_id) REFERENCES public.tags(id) ON DELETE CASCADE;


--
-- Name: group_bindings group_bindings_binding_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_bindings
    ADD CONSTRAINT group_bindings_binding_id_fkey FOREIGN KEY (binding_id) REFERENCES public.bindings(id) ON DELETE CASCADE;


--
-- Name: group_bindings group_bindings_group_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_bindings
    ADD CONSTRAINT group_bindings_group_id_fkey FOREIGN KEY (group_id) REFERENCES public.groups(id) ON DELETE CASCADE;


--
-- Name: groups groups_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.groups
    ADD CONSTRAINT groups_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


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
-- Name: hosts hosts_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: if_entries if_entries_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: if_entries if_entries_interface_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_interface_id_fkey FOREIGN KEY (interface_id) REFERENCES public.interfaces(id) ON DELETE SET NULL;


--
-- Name: if_entries if_entries_neighbor_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_neighbor_host_id_fkey FOREIGN KEY (neighbor_host_id) REFERENCES public.hosts(id) ON DELETE SET NULL;


--
-- Name: if_entries if_entries_neighbor_if_entry_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_neighbor_if_entry_id_fkey FOREIGN KEY (neighbor_if_entry_id) REFERENCES public.if_entries(id) ON DELETE SET NULL;


--
-- Name: if_entries if_entries_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_subnet_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_subnet_id_fkey FOREIGN KEY (subnet_id) REFERENCES public.subnets(id) ON DELETE CASCADE;


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
-- Name: network_credentials network_credentials_credential_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.network_credentials
    ADD CONSTRAINT network_credentials_credential_id_fkey FOREIGN KEY (credential_id) REFERENCES public.credentials(id) ON DELETE CASCADE;


--
-- Name: network_credentials network_credentials_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.network_credentials
    ADD CONSTRAINT network_credentials_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: networks organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.networks
    ADD CONSTRAINT organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: ports ports_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: ports ports_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: services services_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: services services_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: shares shares_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: shares shares_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: shares shares_topology_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_topology_id_fkey FOREIGN KEY (topology_id) REFERENCES public.topologies(id) ON DELETE CASCADE;


--
-- Name: subnets subnets_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: tags tags_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: topologies topologies_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.topologies
    ADD CONSTRAINT topologies_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: user_api_key_network_access user_api_key_network_access_api_key_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_network_access
    ADD CONSTRAINT user_api_key_network_access_api_key_id_fkey FOREIGN KEY (api_key_id) REFERENCES public.user_api_keys(id) ON DELETE CASCADE;


--
-- Name: user_api_key_network_access user_api_key_network_access_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_network_access
    ADD CONSTRAINT user_api_key_network_access_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


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
-- Name: user_network_access user_network_access_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_network_access
    ADD CONSTRAINT user_network_access_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: user_network_access user_network_access_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_network_access
    ADD CONSTRAINT user_network_access_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: users users_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict PJ3btamkYVOtmTuYU7kCTMvkO0oaN26fjXu3b93b62QeEiAObF4G7oAE5uTbOAG

