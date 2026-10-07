/**
 * Entity-related utilities and mappings
 */

import type { components } from './schema';

export type EntityDiscriminants = components['schemas']['EntityDiscriminants'];

/**
 * Map EntityDiscriminants to API path segments for CSV export.
 * Paths are relative to /api/v1/
 *
 * Note: Some entities don't support CSV export (Organization, Invite)
 */
export const entityToExportPath: Record<EntityDiscriminants, string | null> = {
	// Standard entity paths
	Host: 'hosts',
	Service: 'services',
	Subnet: 'subnets',
	IPAddress: 'ip-addresses',
	Interface: 'interfaces',
	Vlan: 'vlans',
	Port: 'ports',
	Binding: 'bindings',
	Dependency: 'dependencies',
	Tag: 'tags',
	Daemon: 'daemons',
	Site: 'sites',
	Share: 'shares',
	Discovery: 'discoveries',
	Topology: 'topologies',
	Snapshot: 'snapshots',
	User: 'users',
	Credential: 'credentials',
	// API keys use auth paths
	UserApiKey: 'auth/keys',
	DaemonApiKey: 'auth/daemon',
	// No CSV export
	Organization: null,
	Invite: null
};
