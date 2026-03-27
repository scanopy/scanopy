/**
 * Entity-related utilities and mappings
 */

import type { components } from './schema';

export type EntityDiscriminants = components['schemas']['EntityDiscriminants'];

/**
 * Map EntityDiscriminants to API path segments for CSV export.
 * Paths are relative to /api/v1/
 *
 * Note: Some entities don't support CSV export (Organization, Invite, Unknown)
 */
export const entityToExportPath: Record<EntityDiscriminants, string | null> = {
	// Standard entity paths
	Host: 'hosts',
	Service: 'services',
	Subnet: 'subnets',
	Interface: 'interfaces',
	Port: 'ports',
	Binding: 'bindings',
	Group: 'groups',
	Tag: 'tags',
	Daemon: 'daemons',
	Network: 'networks',
	Share: 'shares',
	Discovery: 'discoveries',
	Topology: 'topologies',
	User: 'users',
	IfEntry: 'if-entries',
	Credential: 'credentials',
	// API keys use auth paths
	UserApiKey: 'auth/keys',
	DaemonApiKey: 'auth/daemon',
	// No CSV export
	Organization: null,
	Invite: null,
	Unknown: null
};
