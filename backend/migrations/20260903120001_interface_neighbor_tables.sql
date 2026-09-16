-- DOWNTIME MIGRATION (GH #701)
--
-- Replaces `interfaces`' single-neighbour columns with a proper 1:N shape. Two
-- independent 1:1 constraints were the root cause of "only 2 of 3 edges render on
-- a shared L2 segment" and "the stored neighbour flips between identical scans":
-- collection kept only the first LLDP/CDP entry per local port, and resolution let
-- a far-end interface be the endpoint of at most one link.
--
-- Three new tables:
--   interface_neighbor_candidates — resolution's input. Disposable: replaced
--     wholesale per interface on every complete scan, no SCD2, no identity across
--     scans. One row per distinct raw LLDP/CDP identity heard on a port.
--   interface_neighbor_interfaces / interface_neighbor_hosts — the actual
--     adjacency state, and the only tables topology reads. Real NOT NULL FKs, SCD2
--     (same substrate as ports/interfaces), upserted by natural key
--     (interface_id, neighbor_interface_id) / (interface_id, neighbor_host_id).
--     An adjacency lives in exactly one of the two by construction (real FK,
--     not a polymorphic column), not by convention.
--
-- Squawk will flag the DROP COLUMNs as unsafe (correctness in zero-downtime
-- deploys requires expand-and-contract). This migration runs during a coordinated
-- downtime window — see CHANGELOG / release notes for the deploy sequence, and the
-- 20260502120004_drop_legacy_topology_columns.sql precedent for the same pattern
-- — so we accept the unsafety here. `fdb_macs`, `native_vlan_id`, `vlan_ids` stay
-- on `interfaces`: properties of the local port, not of a specific neighbour.

SET lock_timeout = '5s';
SET statement_timeout = '0';

-- ============================================================================
-- interface_neighbor_candidates
-- ============================================================================

CREATE TABLE interface_neighbor_candidates (
    id             UUID PRIMARY KEY,
    network_id     UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    interface_id   UUID NOT NULL REFERENCES interfaces(id) ON DELETE CASCADE,
    lldp_chassis_id JSONB,
    lldp_port_id    JSONB,
    lldp_sys_name   TEXT,
    lldp_port_desc  TEXT,
    lldp_mgmt_addr  INET,
    lldp_sys_desc   TEXT,
    cdp_device_id   TEXT,
    cdp_port_id     TEXT,
    cdp_platform    TEXT,
    cdp_address     INET,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_interface_neighbor_candidates_interface
    ON interface_neighbor_candidates (interface_id);
CREATE INDEX idx_interface_neighbor_candidates_network
    ON interface_neighbor_candidates (network_id);

-- ============================================================================
-- interface_neighbor_interfaces — full resolution (specific remote port known)
-- ============================================================================

CREATE TABLE interface_neighbor_interfaces (
    id                    UUID PRIMARY KEY,
    network_id            UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    interface_id          UUID NOT NULL REFERENCES interfaces(id) ON DELETE CASCADE,
    neighbor_interface_id UUID NOT NULL REFERENCES interfaces(id) ON DELETE CASCADE,
    neighbor_seen_at      TIMESTAMPTZ,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    valid_from            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    valid_to              TIMESTAMPTZ NULL,
    lineage_id            UUID NULL,
    last_seen_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_discovery_id     UUID NULL,
    first_discovery_id    UUID NULL,
    snapshot_id           UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE
);

-- Natural key: one live row per (interface, neighbour interface) pair. Several
-- distinct neighbour interfaces per local interface are the whole point of this
-- table existing; the partial-unique index is what makes an upsert well-defined.
CREATE UNIQUE INDEX idx_interface_neighbor_interfaces_natural_key
    ON interface_neighbor_interfaces (interface_id, neighbor_interface_id)
    WHERE valid_to IS NULL;
CREATE INDEX idx_interface_neighbor_interfaces_neighbor
    ON interface_neighbor_interfaces (neighbor_interface_id);
CREATE INDEX idx_interface_neighbor_interfaces_live
    ON interface_neighbor_interfaces (network_id) WHERE valid_to IS NULL;
CREATE INDEX idx_interface_neighbor_interfaces_as_of
    ON interface_neighbor_interfaces (network_id, valid_from, valid_to);
CREATE INDEX idx_interface_neighbor_interfaces_lineage
    ON interface_neighbor_interfaces (lineage_id) WHERE valid_to IS NOT NULL;
CREATE INDEX idx_interface_neighbor_interfaces_snapshot_id
    ON interface_neighbor_interfaces (snapshot_id) WHERE snapshot_id IS NOT NULL;

-- ============================================================================
-- interface_neighbor_hosts — partial resolution (remote device known, port not)
-- ============================================================================

CREATE TABLE interface_neighbor_hosts (
    id                 UUID PRIMARY KEY,
    network_id         UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    interface_id       UUID NOT NULL REFERENCES interfaces(id) ON DELETE CASCADE,
    neighbor_host_id   UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    neighbor_seen_at   TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    valid_from         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    valid_to           TIMESTAMPTZ NULL,
    lineage_id         UUID NULL,
    last_seen_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_discovery_id  UUID NULL,
    first_discovery_id UUID NULL,
    snapshot_id        UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_interface_neighbor_hosts_natural_key
    ON interface_neighbor_hosts (interface_id, neighbor_host_id)
    WHERE valid_to IS NULL;
CREATE INDEX idx_interface_neighbor_hosts_neighbor
    ON interface_neighbor_hosts (neighbor_host_id);
CREATE INDEX idx_interface_neighbor_hosts_live
    ON interface_neighbor_hosts (network_id) WHERE valid_to IS NULL;
CREATE INDEX idx_interface_neighbor_hosts_as_of
    ON interface_neighbor_hosts (network_id, valid_from, valid_to);
CREATE INDEX idx_interface_neighbor_hosts_lineage
    ON interface_neighbor_hosts (lineage_id) WHERE valid_to IS NOT NULL;
CREATE INDEX idx_interface_neighbor_hosts_snapshot_id
    ON interface_neighbor_hosts (snapshot_id) WHERE snapshot_id IS NOT NULL;

-- ============================================================================
-- Drop the twelve single-neighbour columns from `interfaces`.
-- ============================================================================

ALTER TABLE interfaces
    DROP COLUMN neighbor_interface_id,
    DROP COLUMN neighbor_host_id,
    DROP COLUMN neighbor_seen_at,
    DROP COLUMN lldp_chassis_id,
    DROP COLUMN lldp_port_id,
    DROP COLUMN lldp_sys_name,
    DROP COLUMN lldp_port_desc,
    DROP COLUMN lldp_mgmt_addr,
    DROP COLUMN lldp_sys_desc,
    DROP COLUMN cdp_device_id,
    DROP COLUMN cdp_port_id,
    DROP COLUMN cdp_platform,
    DROP COLUMN cdp_address;
