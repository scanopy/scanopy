-- Universal Credentials: migrate from snmp_credentials to a generic credentials table
-- with JSONB credential_type and junction tables for multi-credential support.

-- Step 1: Create new credentials table
CREATE TABLE credentials (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    credential_type JSONB NOT NULL,
    target_ips INET[] DEFAULT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_credentials_org ON credentials(organization_id);
CREATE INDEX idx_credentials_type ON credentials((credential_type->>'type'));

-- Step 2: Migrate existing SNMP credentials (preserve UUIDs for FK continuity)
INSERT INTO credentials (id, organization_id, name, credential_type, created_at, updated_at)
SELECT
    id,
    organization_id,
    name,
    jsonb_build_object(
        'type', 'Snmp',
        'version', version,
        'community', jsonb_build_object('mode', 'Inline', 'value', community)
    ),
    created_at,
    updated_at
FROM snmp_credentials;

-- Step 3: Create junction tables for multi-credential support
CREATE TABLE network_credentials (
    network_id UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    credential_id UUID NOT NULL REFERENCES credentials(id) ON DELETE CASCADE,
    PRIMARY KEY (network_id, credential_id)
);

CREATE TABLE host_credentials (
    host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    credential_id UUID NOT NULL REFERENCES credentials(id) ON DELETE CASCADE,
    interface_ids UUID[] DEFAULT NULL,
    PRIMARY KEY (host_id, credential_id)
);

-- Step 4: Migrate existing single-FK data into junction tables
INSERT INTO network_credentials (network_id, credential_id)
SELECT id, snmp_credential_id FROM networks WHERE snmp_credential_id IS NOT NULL;

INSERT INTO host_credentials (host_id, credential_id)
SELECT id, snmp_credential_id FROM hosts WHERE snmp_credential_id IS NOT NULL;

-- Step 5: Drop old FK columns from networks and hosts
ALTER TABLE networks DROP COLUMN snmp_credential_id;
ALTER TABLE hosts DROP COLUMN snmp_credential_id;

-- Step 6: Drop old table
DROP TABLE snmp_credentials;

-- Step 7: Migrate DockerProxyLocal/DockerProxyRemote to unified DockerProxy
UPDATE credentials SET credential_type =
    jsonb_set(credential_type, '{type}', '"DockerProxy"')
WHERE credential_type->>'type' = 'DockerProxyLocal';

UPDATE credentials SET credential_type =
    jsonb_set(credential_type, '{type}', '"DockerProxy"')
WHERE credential_type->>'type' = 'DockerProxyRemote';
