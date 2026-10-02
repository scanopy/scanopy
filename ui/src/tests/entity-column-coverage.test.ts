import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

/**
 * Every field the API returns for an entity is either a column on that entity's tab or is
 * excluded here with the reason it is not shown.
 *
 * A field the backend adds reaches the frontend through `static/openapi.json` and the generated
 * types, and nothing else asks whether anyone can see it. This test does: the new property has no
 * column and no exclusion, so it fails until someone decides. Most new columns belong in the
 * table with `display: { hiddenByDefault: true }`, so they cost nothing until a user asks.
 *
 * Column keys are read from the tab source rather than listed here, so this file never restates a
 * tab's columns:
 * - the values of the tab's `*OrderField` enum, when the tab calls `defineFields<…, XOrderField>`
 * - every `key: '…'` literal in the tab
 * - `tags`, when the tab passes `getItemTags`
 */

const UI = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const FEATURES = path.join(UI, 'src/lib/features');

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type SchemaNode = any;
const SCHEMAS: Record<string, SchemaNode> = JSON.parse(
	fs.readFileSync(path.join(UI, 'static/openapi.json'), 'utf-8')
).components.schemas;

interface SchemaCoverage {
	schema: string;
	/** A property shown under a column with a different key: property → column key. */
	aliases?: Record<string, string>;
	/** A property with no column: property → why it is not shown. */
	excluded?: Record<string, string>;
}

interface TabCoverage {
	/** The `*OrderField` schema the tab passes to `defineFields`, if it uses `defineFields`. */
	orderField?: string;
	schemas: SchemaCoverage[];
}

const ID = 'Internal identifier; the row itself is the entity';
const ORGANIZATION = "Always the viewer's organization";

/** Bookkeeping on entities that keep version history; the tab lists the current version only. */
const HISTORY = {
	lineage_id: 'Links the versions of one entity; the tab lists the current version only',
	valid_from: 'Start of this version; the tab lists the current version only',
	valid_to: 'End of a superseded version; always empty on the current version this tab lists',
	first_discovery_id: 'Id of the scan that first saw it; Created already says when',
	last_discovery_id: 'Id of the scan that last saw it; Last seen already says when'
};

/** Host facts each carry a `*_source` recording which source wrote them. */
const hostAttributeSources = (fields: string[]) =>
	Object.fromEntries(
		fields.map((field) => [
			`${field}_source`,
			`Which source wrote ${field}; the host editor shows it next to the value`
		])
	);

/** Keyed by tab path relative to `src/lib/features`. */
const COVERAGE: Record<string, TabCoverage> = {
	'vlans/components/VlanTab.svelte': {
		orderField: 'VlanOrderField',
		schemas: [{ schema: 'Vlan', excluded: { id: ID, organization_id: ORGANIZATION, ...HISTORY } }]
	},
	'daemons/components/DaemonTab.svelte': {
		orderField: 'DaemonOrderField',
		schemas: [
			{
				schema: 'DaemonResponse',
				excluded: {
					id: ID,
					is_unreachable: 'Rendered by the Status column',
					standby: 'Rendered by the Status column',
					version_status: 'Rendered by the Status column, and drives the Update action',
					standby_cleared_at:
						'Server bookkeeping for leaving standby; Status shows the current state',
					api_key_id: "The daemon's key is managed from the daemon edit modal",
					user_id: 'The owning user; this tab loads no users to name them'
				}
			}
		]
	},
	'discovery/components/tabs/DiscoveryScheduledTab.svelte': {
		orderField: 'DiscoveryOrderField',
		schemas: [
			{
				schema: 'Discovery',
				excluded: {
					id: ID,
					integration_targets: 'Nested per-integration IP targeting; edited in the scan modal'
				}
			}
		]
	},
	'discovery/components/tabs/DiscoveryHistoryTab.svelte': {
		orderField: 'DiscoveryOrderField',
		schemas: [
			{
				schema: 'Discovery',
				excluded: {
					id: ID,
					run_type:
						'Always Historical here; its results render as Status, Started, Finished, Duration and Warnings',
					tags: 'Always empty: the server writes run records with no tags',
					scan_count: 'Always 0 on run records; the count lives on the scheduled scan',
					force_full_scan: 'Always false on run records; the flag lives on the scheduled scan',
					integration_targets: 'Always empty on run records; targeting lives on the scheduled scan'
				}
			}
		]
	},
	'daemon_api_keys/components/ApiKeyTab.svelte': {
		schemas: [
			{
				schema: 'DaemonApiKey',
				excluded: {
					id: ID,
					key: 'Secret',
					daemon_id: 'Always empty: the tab lists only keys not bound to a daemon'
				}
			}
		]
	},
	'tags/components/TagTab.svelte': {
		orderField: 'TagOrderField',
		schemas: [
			{
				schema: 'Tag',
				excluded: {
					id: ID,
					organization_id: ORGANIZATION,
					lineage_id: HISTORY.lineage_id,
					valid_from: HISTORY.valid_from,
					valid_to: HISTORY.valid_to
				}
			}
		]
	},
	'networks/components/NetworksTab.svelte': {
		schemas: [
			{
				schema: 'Network',
				aliases: { credential_ids: 'credentials' },
				excluded: {
					id: ID,
					organization_id: ORGANIZATION,
					stale_after_hours:
						'The override alone, empty when unset; Stale after shows the effective value'
				}
			}
		]
	},
	'users/components/UserTab.svelte': {
		schemas: [
			{
				schema: 'User',
				excluded: {
					id: ID,
					organization_id: ORGANIZATION,
					has_password: 'Auth method already shows how the user signs in',
					display_settings: "The user's own display preferences",
					email_settings: "The user's own email preferences"
				}
			},
			{
				schema: 'Invite',
				aliases: { send_to: 'email', created_by: 'invited_by', url: 'invite_url' },
				excluded: { id: ID, organization_id: ORGANIZATION }
			}
		]
	},
	'user_api_keys/components/UserApiKeyTab.svelte': {
		schemas: [
			{
				schema: 'UserApiKey',
				excluded: {
					id: ID,
					organization_id: ORGANIZATION,
					key: 'Secret',
					user_id: 'Always the viewer: the tab lists their own keys'
				}
			}
		]
	},
	'hosts/components/HostTab.svelte': {
		orderField: 'HostOrderField',
		schemas: [
			{
				schema: 'Host',
				aliases: {
					credential_assignments: 'credentials',
					virtualization_service_id: 'virtualized_by'
				},
				excluded: {
					id: ID,
					...HISTORY,
					virtualization_metadata: 'Provider details of a VM guest; Virtualized by names the host',
					...hostAttributeSources([
						'name',
						'hostname',
						'sys_descr',
						'sys_object_id',
						'sys_location',
						'sys_contact',
						'sys_name',
						'management_url',
						'chassis_id',
						'manufacturer',
						'model',
						'serial_number',
						'firmware_revision',
						'software_revision',
						'os'
					])
				}
			}
		]
	},
	'subnets/components/SubnetTab.svelte': {
		orderField: 'SubnetOrderField',
		schemas: [
			{
				schema: 'Subnet',
				excluded: {
					id: ID,
					...HISTORY,
					cidr_source: 'Rendered by the CIDR column as the provisional-range badge',
					virtualization_service_id:
						'The container runtime owning a bridge subnet; this tab loads no services to name it, and Subnet type already marks bridges'
				}
			}
		]
	},
	'services/components/ServiceTab.svelte': {
		orderField: 'ServiceOrderField',
		schemas: [
			{
				schema: 'Service',
				aliases: {
					bindings: 'port_bindings',
					host_id: 'host',
					virtualization_service_id: 'containerized_by'
				},
				excluded: {
					id: ID,
					...HISTORY,
					virtualization_metadata:
						'Runtime details of a container; Containerized by names the runtime'
				}
			}
		]
	},
	'credentials/components/CredentialsTab.svelte': {
		orderField: 'CredentialOrderField',
		schemas: [
			{
				schema: 'Credential',
				aliases: {
					assigned_network_ids: 'assigned_networks',
					host_assignments: 'assigned_hosts'
				},
				excluded: { id: ID, organization_id: ORGANIZATION }
			}
		]
	}
};

/** Property names of a schema, following `$ref` and `allOf` down to every `properties` block. */
function schemaProperties(name: string): Set<string> {
	const walk = (node: SchemaNode): string[] =>
		node.$ref
			? walk(SCHEMAS[node.$ref.split('/').pop()])
			: [...Object.keys(node.properties ?? {}), ...(node.allOf ?? []).flatMap(walk)];
	return new Set(walk(SCHEMAS[name]));
}

/** The `XOrderField` a tab passes to `defineFields<T, XOrderField>`, if any. */
function definedOrderField(source: string): string | undefined {
	return source.match(/defineFields<\s*\w+\s*,\s*(\w+)\s*>/)?.[1];
}

/** The column keys a tab declares, read from its source. */
function columnKeys(source: string): Set<string> {
	const keys = new Set([...source.matchAll(/\bkey:\s*'([^']+)'/g)].map((match) => match[1]));
	const orderField = definedOrderField(source);
	if (orderField) SCHEMAS[orderField]?.enum?.forEach((value: string) => keys.add(value));
	if (/getItemTags=/.test(source)) keys.add('tags');
	return keys;
}

/** Every `.svelte` file under `src/lib/features` that renders `<DataControls`. */
function dataControlsTabs(): string[] {
	return (fs.readdirSync(FEATURES, { recursive: true }) as string[])
		.filter((file) => file.endsWith('.svelte'))
		.filter((file) => fs.readFileSync(path.join(FEATURES, file), 'utf-8').includes('<DataControls'))
		.map((file) => file.split(path.sep).join('/'))
		.sort();
}

/** Everything wrong with one tab's coverage, as messages naming the tab, schema and property. */
function coverageProblems(tab: string, coverage: TabCoverage): string[] {
	const source = fs.readFileSync(path.join(FEATURES, tab), 'utf-8');
	const problems: string[] = [];

	const declaredOrderField = definedOrderField(source);
	if (declaredOrderField !== coverage.orderField) {
		problems.push(
			`${tab}: the tab calls defineFields with order field "${declaredOrderField ?? 'none'}" ` +
				`but COVERAGE says "${coverage.orderField ?? 'none'}". Set COVERAGE["${tab}"].orderField ` +
				`to match the tab.`
		);
	}
	if (coverage.orderField && !SCHEMAS[coverage.orderField]?.enum) {
		problems.push(`${tab}: "${coverage.orderField}" is not an enum schema in openapi.json.`);
	}

	const keys = columnKeys(source);
	for (const { schema, aliases = {}, excluded = {} } of coverage.schemas) {
		if (!SCHEMAS[schema]) {
			problems.push(`${tab}: schema "${schema}" does not exist in openapi.json.`);
			continue;
		}
		const where = `${tab} (${schema})`;
		const properties = schemaProperties(schema);

		for (const property of properties) {
			if (!keys.has(property) && !(property in aliases) && !(property in excluded)) {
				problems.push(
					`${where}: "${property}" has no column and no exclusion. Add a column with ` +
						`display: { hiddenByDefault: true } to the tab, or add "${property}" to ` +
						`COVERAGE["${tab}"] ${schema}.excluded with the reason it is not shown.`
				);
			}
		}
		for (const [property, reason] of Object.entries(excluded)) {
			if (!properties.has(property)) {
				problems.push(
					`${where}: excluded "${property}" is not a property of ${schema}. Remove the stale exclusion.`
				);
			}
			if (!reason.trim()) {
				problems.push(
					`${where}: excluded "${property}" has an empty reason. Say why it is not shown.`
				);
			}
			if (keys.has(property) || property in aliases) {
				problems.push(
					`${where}: "${property}" is excluded but is also a column. Remove the exclusion.`
				);
			}
		}
		for (const [property, column] of Object.entries(aliases)) {
			if (!properties.has(property)) {
				problems.push(
					`${where}: alias "${property}" is not a property of ${schema}. Remove the stale alias.`
				);
			}
			if (!keys.has(column)) {
				problems.push(
					`${where}: alias "${property}" points at column "${column}", which the tab does not ` +
						`declare. Point it at the column that renders "${property}", or exclude it instead.`
				);
			}
		}
	}
	return problems;
}

describe('entity column coverage', () => {
	it('has an entry for every tab that renders DataControls', () => {
		const tabs = dataControlsTabs();
		expect(tabs.length).toBeGreaterThan(10);

		const missing = tabs.filter((tab) => !(tab in COVERAGE));
		expect(
			missing,
			`These tabs render <DataControls> with no COVERAGE entry:\n\n  ${missing.join('\n  ')}\n\n` +
				`Add one per tab, naming the schema(s) its rows come from.`
		).toEqual([]);

		const stale = Object.keys(COVERAGE).filter((tab) => !tabs.includes(tab));
		expect(
			stale,
			`COVERAGE entries whose file is missing or no longer renders <DataControls>:\n\n  ` +
				`${stale.join('\n  ')}\n\nUpdate the path or remove the entry.`
		).toEqual([]);
	});

	for (const [tab, coverage] of Object.entries(COVERAGE)) {
		it(`${tab} shows or excludes every schema property`, () => {
			expect(fs.existsSync(path.join(FEATURES, tab)), `${tab} does not exist`).toBe(true);
			const problems = coverageProblems(tab, coverage);
			expect(problems, problems.join('\n')).toEqual([]);
		});
	}
});
