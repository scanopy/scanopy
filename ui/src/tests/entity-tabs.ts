import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { components } from '$lib/api/schema';

/**
 * What the entity-tab tests share: the committed OpenAPI spec, the tabs that render
 * `<DataControls>`, the column keys each one declares, and the schemas its rows come from.
 *
 * Column keys are read from the tab source rather than listed anywhere, so no test restates a
 * tab's columns:
 * - the values of the tab's `*OrderField` enum, when the tab calls `defineFields<…, XOrderField>`
 * - every `key: '…'` literal in the tab
 * - `tags`, when the tab passes `getItemTags`
 */

export const UI = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
export const FEATURES = path.join(UI, 'src/lib/features');

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type SchemaNode = any;
export const OPENAPI: SchemaNode = JSON.parse(
	fs.readFileSync(path.join(UI, 'static/openapi.json'), 'utf-8')
);
export const SCHEMAS: Record<string, SchemaNode> = OPENAPI.components.schemas;

type Schemas = components['schemas'];
type SchemaName = keyof Schemas;
/** Every field of a schema, optional ones included. */
type FieldOf<S extends SchemaName> = keyof Required<Schemas[S]>;

/** A field with no column, and why. */
export interface Excluded<Reason extends string = string> {
	excluded: Reason;
}

/** A field with no column. The reason has to meet the bar in `entity-column-coverage.test.ts`. */
export function excluded<const Reason extends string>(reason: Reason): Excluded<Reason> {
	return { excluded: reason };
}

/** Per field: the key of the column that shows it, or why no column does. */
export type FieldDecision = string | Excluded;

/** One decision for every field of `S`. A missing field fails to compile. */
type FieldDecisions<S extends SchemaName> = { [K in FieldOf<S>]-?: FieldDecision };

/**
 * Turns into `never` (so fails to compile) a decision for a field `S` does not have, and an
 * empty column key or reason.
 */
type Checked<S extends SchemaName, D> = {
	[K in keyof D]: K extends FieldOf<S> ? (D[K] extends '' | Excluded<''> ? never : D[K]) : never;
};

export interface SchemaCoverage {
	schema: SchemaName;
	/** Field → the key of the column that shows it, or why no column does. */
	fields: Record<string, FieldDecision>;
}

/**
 * Where every field of one schema is shown. Checked by `npm run check`: a field added to the
 * schema with no decision, a decision for a field the schema no longer has, and an empty reason
 * each fail to compile. The test checks what types cannot: that each column key is one the tab
 * declares.
 */
function decide<S extends SchemaName, const D extends FieldDecisions<S>>(
	schema: S,
	fields: D & Checked<S, D>
): SchemaCoverage {
	return { schema, fields };
}

export interface TabCoverage {
	/** The `*OrderField` schema the tab passes to `defineFields`, if it uses `defineFields`. */
	orderField?: string;
	/**
	 * The openapi enum `orderField` narrows, when the tab passes a frontend subset of an
	 * `*OrderField` (`Exclude<…>`) rather than the schema itself. Its variants count as columns.
	 */
	orderFieldSchema?: string;
	schemas: SchemaCoverage[];
}

const ID = excluded('Internal identifier; the row itself is the entity');
const ORGANIZATION = excluded("Always the viewer's organization");

/** Bookkeeping on entities that keep version history; the tab lists the current version only. */
const HISTORY = {
	lineage_id: excluded('Links the versions of one entity; the tab lists the current version only'),
	valid_from: excluded('Start of this version; the tab lists the current version only'),
	valid_to: excluded(
		'End of a superseded version; always empty on the current version this tab lists'
	)
};

/** The discovery runs that first and last found an entity. */
const FOUND_BY = {
	first_discovery_id: 'first_found_by',
	last_discovery_id: 'last_found_by'
} as const;

/** Discovery fields on a scan configuration and on a run record alike. */
const DISCOVERY = {
	created_at: 'created_at',
	daemon_id: 'daemon_id',
	discovery_type: 'discovery_type',
	name: 'name',
	site_id: 'site_id',
	updated_at: 'updated_at',
	id: ID,
	// Decided with Maya, 2026-10-04.
	integration_targets: excluded(
		'Transient one-shot targeting, dropped after the next successful scan'
	)
} as const;

/**
 * Which schemas each tab's rows come from, and for every field the column that shows it or why
 * none does. Keyed by tab path relative to `src/lib/features`.
 */
export const COVERAGE: Record<string, TabCoverage> = {
	'vlans/components/VlanTab.svelte': {
		orderField: 'VlanOrderField',
		schemas: [
			decide('Vlan', {
				created_at: 'created_at',
				description: 'description',
				last_seen_at: 'last_seen_at',
				name: 'name',
				site_id: 'site_id',
				source: 'source',
				subnet_ids: 'subnet_ids',
				updated_at: 'updated_at',
				vlan_number: 'vlan_number',
				...FOUND_BY,
				id: ID,
				organization_id: ORGANIZATION,
				...HISTORY
			})
		]
	},
	'daemons/components/DaemonTab.svelte': {
		orderField: 'DaemonOrderField',
		schemas: [
			decide('DaemonResponse', {
				created_at: 'created_at',
				host_id: 'host_id',
				interfaced_subnet_ids: 'interfaced_subnet_ids',
				last_seen: 'last_seen',
				mode: 'mode',
				name: 'name',
				site_id: 'site_id',
				os: 'os',
				tags: 'tags',
				updated_at: 'updated_at',
				url: 'url',
				version: 'version',
				user_id: 'maintainer',
				// Status renders one state per daemon, from `getDaemonStatusTag`: Unsupported, then
				// Unreachable, then Standby, then the version lifecycle.
				is_unreachable: 'status',
				standby: 'status',
				version_status: 'status',
				id: ID,
				// Decided with Maya, 2026-10-04.
				api_key_id: excluded("The daemon's key is not a meaningful entity of its own to users"),
				standby_cleared_at: excluded(
					'Internal: starts the grace window of the nightly inactivity check, its only reader'
				)
			})
		]
	},
	'discovery/components/tabs/DiscoveryScheduledTab.svelte': {
		// Scan configurations carry no run results, so the tab leaves out the run-result order fields.
		orderField: 'DiscoveryConfigOrderField',
		orderFieldSchema: 'DiscoveryOrderField',
		schemas: [
			decide('Discovery', {
				...DISCOVERY,
				force_full_scan: 'force_full_scan',
				run_type: 'run_type',
				scan_count: 'scan_count',
				tags: 'tags'
			})
		]
	},
	'discovery/components/tabs/DiscoveryHistoryTab.svelte': {
		orderField: 'DiscoveryOrderField',
		schemas: [
			decide('Discovery', {
				...DISCOVERY,
				tags: 'tags',
				// The tab lists `historical: true` rows only (`useDiscoveryHistoryQuery`).
				run_type: excluded(
					'Always Historical here; its results render as Status, Started, Finished, Duration and Warnings'
				),
				// The server writes run records in `dispatch.rs` and `cleanup.rs` with these at their
				// defaults, and the create and update routes refuse Historical payloads.
				scan_count: excluded('Always 0 on run records; the count lives on the scheduled scan'),
				force_full_scan: excluded(
					'Always false on run records; the flag lives on the scheduled scan'
				)
			})
		]
	},
	'daemon_api_keys/components/ApiKeyTab.svelte': {
		schemas: [
			decide('DaemonApiKey', {
				created_at: 'created_at',
				expires_at: 'expires_at',
				is_enabled: 'is_enabled',
				last_used: 'last_used',
				name: 'name',
				site_id: 'site_id',
				tags: 'tags',
				updated_at: 'updated_at',
				id: ID,
				key: excluded('Secret'),
				daemon_id: excluded(
					'Always empty: the tab lists only keys not bound to a daemon (`daemon_id == null`)'
				)
			})
		]
	},
	'tags/components/TagTab.svelte': {
		orderField: 'TagOrderField',
		schemas: [
			decide('Tag', {
				color: 'color',
				created_at: 'created_at',
				description: 'description',
				is_application: 'is_application',
				name: 'name',
				updated_at: 'updated_at',
				id: ID,
				organization_id: ORGANIZATION,
				...HISTORY
			})
		]
	},
	'sites/components/SitesTab.svelte': {
		schemas: [
			decide('Site', {
				created_at: 'created_at',
				credential_ids: 'credentials',
				effective_stale_after_hours: 'effective_stale_after_hours',
				name: 'name',
				tags: 'tags',
				updated_at: 'updated_at',
				// Stale after marks the value "(default)" when no override is set.
				stale_after_hours: 'effective_stale_after_hours',
				id: ID,
				organization_id: ORGANIZATION
			})
		]
	},
	'users/components/UserTab.svelte': {
		schemas: [
			decide('User', {
				created_at: 'created_at',
				email: 'email',
				email_verified: 'email_verified',
				site_ids: 'site_ids',
				oidc_linked_at: 'oidc_linked_at',
				oidc_provider: 'oidc_provider',
				permissions: 'permissions',
				terms_accepted_at: 'terms_accepted_at',
				updated_at: 'updated_at',
				// Auth method names the provider and/or password sign-in.
				has_password: 'oidc_provider',
				id: ID,
				organization_id: ORGANIZATION,
				// Decided with Maya, 2026-10-04.
				display_settings: excluded("The user's own display preferences, not an account fact"),
				email_settings: excluded("The user's own email preferences, not an account fact")
			}),
			decide('Invite', {
				created_at: 'created_at',
				created_by: 'invited_by',
				expires_at: 'expires_at',
				site_ids: 'site_ids',
				permissions: 'permissions',
				send_to: 'email',
				updated_at: 'updated_at',
				url: 'invite_url',
				id: ID,
				organization_id: ORGANIZATION
			})
		]
	},
	'user_api_keys/components/UserApiKeyTab.svelte': {
		schemas: [
			decide('UserApiKey', {
				created_at: 'created_at',
				expires_at: 'expires_at',
				is_enabled: 'is_enabled',
				last_used: 'last_used',
				name: 'name',
				site_ids: 'site_ids',
				permissions: 'permissions',
				tags: 'tags',
				updated_at: 'updated_at',
				id: ID,
				organization_id: ORGANIZATION,
				key: excluded('Secret'),
				user_id: excluded("Always the viewer: the list handler filters to the caller's own keys")
			})
		]
	},
	'hosts/components/HostTab.svelte': {
		orderField: 'HostOrderField',
		schemas: [
			// The response the tab lists, not the stored `Host`: they differ, and a field only the
			// stored type has never reaches the table (the discovery ids once didn't).
			decide('HostResponse', {
				chassis_id: 'chassis_id',
				created_at: 'created_at',
				credential_assignments: 'credentials',
				description: 'description',
				firmware_revision: 'firmware_revision',
				hidden: 'hidden',
				hostname: 'hostname',
				last_seen_at: 'last_seen_at',
				management_url: 'management_url',
				manufacturer: 'manufacturer',
				model: 'model',
				name: 'name',
				site_id: 'site_id',
				os: 'os_family',
				serial_number: 'serial_number',
				asset_tag: 'asset_tag',
				software_revision: 'software_revision',
				source: 'source',
				sys_contact: 'sys_contact',
				sys_descr: 'sys_descr',
				sys_location: 'sys_location',
				sys_name: 'sys_name',
				sys_object_id: 'sys_object_id',
				tags: 'tags',
				updated_at: 'updated_at',
				virtualization_interface_id: 'presented_by',
				virtualization_service_id: 'virtualized_by',
				// Where the host sits in its virtualization tree: how Virtualized By groups.
				virtualization_parent_host_id: 'virtualized_by',
				virtualization_root_host_id: 'virtualized_by',
				virtualization_depth: 'virtualized_by',
				...FOUND_BY,
				// Each source is explained in its value's tooltip (`getSource`, or `OsTag` for OS).
				chassis_id_source: 'chassis_id',
				firmware_revision_source: 'firmware_revision',
				hostname_source: 'hostname',
				management_url_source: 'management_url',
				manufacturer_source: 'manufacturer',
				model_source: 'model',
				name_source: 'name',
				os_source: 'os_family',
				serial_number_source: 'serial_number',
				asset_tag_source: 'asset_tag',
				software_revision_source: 'software_revision',
				sys_contact_source: 'sys_contact',
				sys_descr_source: 'sys_descr',
				sys_location_source: 'sys_location',
				sys_name_source: 'sys_name',
				sys_object_id_source: 'sys_object_id',
				// Name renders `display_name`, the winning rung of the ladder; every other rung is
				// its own column (Hostname, sysName, Chassis ID, IP Addresses), and the rung picks
				// whether Name's tooltip explains its source.
				display_name: 'name',
				display_name_rung: 'name',
				name_ladder: 'name',
				// The children the response nests, each its own column.
				interfaces: 'interfaces',
				ip_addresses: 'interface_ip',
				ports: 'ports',
				services: 'services',
				id: ID,
				// Decided with Maya, 2026-10-04.
				virtualization_metadata: excluded(
					"Nested provider details of a guest; they don't read as one column value"
				)
			})
		]
	},
	'subnets/components/SubnetTab.svelte': {
		orderField: 'SubnetOrderField',
		schemas: [
			decide('Subnet', {
				cidr: 'cidr',
				created_at: 'created_at',
				description: 'description',
				last_seen_at: 'last_seen_at',
				name: 'name',
				site_id: 'site_id',
				source: 'source',
				subnet_type: 'subnet_type',
				tags: 'tags',
				updated_at: 'updated_at',
				virtualization_service_id: 'managed_by',
				// CIDR renders an assumed range as the "Range assumed" chip.
				cidr_source: 'cidr',
				...FOUND_BY,
				id: ID,
				...HISTORY
			})
		]
	},
	'services/components/ServiceTab.svelte': {
		orderField: 'ServiceOrderField',
		schemas: [
			decide('Service', {
				bindings: 'port_bindings',
				created_at: 'created_at',
				host_id: 'host',
				last_seen_at: 'last_seen_at',
				name: 'name',
				site_id: 'site_id',
				position: 'position',
				service_definition: 'service_definition',
				source: 'source',
				tags: 'tags',
				updated_at: 'updated_at',
				virtualization_service_id: 'containerized_by',
				...FOUND_BY,
				id: ID,
				...HISTORY,
				// Decided with Maya, 2026-10-04.
				virtualization_metadata: excluded(
					"Nested runtime details of a container; they don't read as one column value"
				)
			})
		]
	},
	'credentials/components/CredentialsTab.svelte': {
		orderField: 'CredentialOrderField',
		schemas: [
			decide('Credential', {
				assigned_site_ids: 'assigned_sites',
				created_at: 'created_at',
				credential_type: 'credential_type',
				daemon_os: 'daemon_os',
				description: 'description',
				host_assignments: 'assigned_hosts',
				name: 'name',
				tags: 'tags',
				updated_at: 'updated_at',
				id: ID,
				organization_id: ORGANIZATION
			})
		]
	}
};

/** A tab's source, by its path relative to `src/lib/features`. */
export function readTab(tab: string): string {
	return fs.readFileSync(path.join(FEATURES, tab), 'utf-8');
}

/** The `XOrderField` a tab passes to `defineFields<T, XOrderField>`, if any. */
export function definedOrderField(source: string): string | undefined {
	return source.match(/defineFields<\s*\w+\s*,\s*(\w+)\s*>/)?.[1];
}

/** The openapi enum whose variants are the tab's orderable columns, if it has one. */
export function orderFieldSchema(coverage: TabCoverage): string | undefined {
	return coverage.orderFieldSchema ?? coverage.orderField;
}

/**
 * The column keys a tab declares, read from its source. `orderFieldSchema` is the openapi enum
 * whose variants are the tab's orderable columns; it defaults to the tab's own order field.
 */
export function columnKeys(
	source: string,
	orderFieldSchema = definedOrderField(source)
): Set<string> {
	const keys = new Set([...source.matchAll(/\bkey:\s*'([^']+)'/g)].map((match) => match[1]));
	if (orderFieldSchema)
		SCHEMAS[orderFieldSchema]?.enum?.forEach((value: string) => keys.add(value));
	if (/getItemTags=/.test(source)) keys.add('tags');
	return keys;
}

/** Every `.svelte` file under `src/lib/features` that renders `<DataControls`. */
export function dataControlsTabs(): string[] {
	return (fs.readdirSync(FEATURES, { recursive: true }) as string[])
		.filter((file) => file.endsWith('.svelte'))
		.filter((file) => fs.readFileSync(path.join(FEATURES, file), 'utf-8').includes('<DataControls'))
		.map((file) => file.split(path.sep).join('/'))
		.sort();
}
