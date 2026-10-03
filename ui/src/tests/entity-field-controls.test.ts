import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import {
	isGroupableField,
	isSortableField,
	serverOrderViolations
} from '$lib/shared/components/data/controls/sorting';
import { serverFilterViolations } from '$lib/shared/components/data/controls/filtering';
import type { FieldConfig } from '$lib/shared/components/data/types';
import {
	COVERAGE,
	FEATURES,
	OPENAPI,
	SCHEMAS,
	UI,
	columnKeys,
	dataControlsTabs,
	definedOrderField,
	orderFieldSchema,
	readTab,
	type SchemaNode
} from './entity-tabs';

/**
 * Every column on every entity tab has a recorded sort, group and filter decision, and the tab
 * offers exactly what was decided.
 *
 * A new column fails here until someone decides whether it sorts, groups and filters, and writes
 * down why not when it doesn't. The tab's flags are read from its source and run through the same
 * `isSortableField` / `isGroupableField` rules the controls use, so a flag added or dropped
 * without updating the decision fails too.
 *
 * On a server-paginated tab (one that passes `serverPagination=`), only `*OrderField` variants
 * sort and group, and every filter is applied by the server through a query parameter of the
 * tab's list operation, named here as `{ param }` and checked against `static/openapi.json`.
 */

/** `true` means the column offers it; a string means it doesn't, and says why. */
type Choice = true | string;
/** A server-side filter names the list operation's query parameter that applies it. */
type FilterChoice = Choice | { param: string };

interface ColumnDecision {
	sort: Choice;
	group: Choice;
	filter: FilterChoice;
}

interface TabDecisions {
	/** Server-paginated tabs only: the path of the `GET` list operation that applies filters. */
	listPath?: string;
	/** Column key (as `columnKeys` derives it) → decision. */
	columns: Record<string, ColumnDecision>;
}

const CAPABILITIES = ['sort', 'group', 'filter'] as const;
type Capability = (typeof CAPABILITIES)[number];

const SEARCH = 'Search finds it';
const UNIQUE = 'Unique per row';
const NO_DATE_RANGE = 'No date-range control';
const ARRAY_SORT = 'An array has no single order';
const ARRAY_GROUP = 'An array has no single group';
const COUNT = 'A count; every value would be its own group';
const FREE_TEXT = 'Free text; search covers it';

/** Sorted, grouped and filtered on the client. */
const YES: ColumnDecision = { sort: true, group: true, filter: true };
/** The row's name: sorts, but every row would be its own group. */
const IDENTITY: ColumnDecision = { sort: true, group: UNIQUE, filter: SEARCH };
const DATE: ColumnDecision = { sort: true, group: 'Near-unique timestamp', filter: NO_DATE_RANGE };
const LAST_SEEN: ColumnDecision = {
	...DATE,
	filter: 'No date-range control; Stale covers last seen'
};
const BOOLEAN: ColumnDecision = {
	sort: 'Two values; group and filter cover it',
	group: true,
	filter: true
};
const TEXT: ColumnDecision = { sort: 'Free text', group: 'Free text', filter: FREE_TEXT };
/** An array whose members are shared across rows, so filtering by one narrows the list. */
const SHARED_ARRAY: ColumnDecision = { sort: ARRAY_SORT, group: ARRAY_GROUP, filter: true };
/** An array whose members each belong to one row. */
const ownedArray = (filter: string): ColumnDecision => ({
	sort: ARRAY_SORT,
	group: ARRAY_GROUP,
	filter
});
/** Not sorted, grouped or filtered, all for one reason. */
const none = (reason: string): ColumnDecision => ({ sort: reason, group: reason, filter: reason });
/** Sorted and grouped by the server's order field, filtered by the server through `param`. */
const server = (param: string): ColumnDecision => ({ sort: true, group: true, filter: { param } });
/** Sorted by the server's order field only. */
const serverIdentity = (group: string): ColumnDecision => ({ sort: true, group, filter: SEARCH });

const RUN_RESULT = none('Run result; a scan configuration has none');

/** Keyed by tab path relative to `src/lib/features`. */
const DECISIONS: Record<string, TabDecisions> = {
	'hosts/components/HostTab.svelte': {
		listPath: '/api/v1/hosts',
		columns: {
			name: serverIdentity('Unique per host'),
			hostname: serverIdentity('Near-unique per host'),
			virtualized_by: server('virtualization_service_ids'),
			interface_ip: serverIdentity('Unique per host'),
			mac_address: serverIdentity('Unique per host'),
			network_id: server('network_ids'),
			created_at: DATE,
			updated_at: DATE,
			last_seen_at: LAST_SEEN,
			source: server('sources'),
			manufacturer: server('manufacturers'),
			model: server('models'),
			sys_location: server('sys_locations'),
			os_family: server('os_families'),
			hidden: server('hidden'),
			description: TEXT,
			serial_number: none('Identifier, unique per device; search finds one'),
			chassis_id: none('Identifier, unique per device; search finds one'),
			sys_name: none('Identifier, unique per device; search finds one'),
			management_url: none('Identifier, unique per device; search finds one'),
			firmware_revision: {
				sort: 'Version strings do not compare across vendors',
				group: 'High cardinality',
				filter: 'High cardinality'
			},
			software_revision: {
				sort: 'Version strings do not compare across vendors',
				group: 'High cardinality',
				filter: 'High cardinality'
			},
			sys_object_id: {
				sort: 'Opaque OID',
				group: 'Model carries the readable form',
				filter: 'Model carries the readable form'
			},
			sys_contact: TEXT,
			sys_descr: TEXT,
			tags: { ...SHARED_ARRAY, filter: { param: 'tag_ids' } },
			credentials: { ...SHARED_ARRAY, filter: { param: 'credential_ids' } },
			interfaces: ownedArray('Interface names are unique per host; search finds them'),
			services: { ...SHARED_ARRAY, filter: { param: 'service_names' } }
		}
	},
	'services/components/ServiceTab.svelte': {
		listPath: '/api/v1/services',
		columns: {
			name: serverIdentity('Type groups the same values'),
			host: server('host_ids'),
			network_id: server('network_ids'),
			service_definition: server('service_definitions'),
			position: { sort: true, group: 'Per-host ordinal', filter: 'Per-host ordinal' },
			created_at: DATE,
			updated_at: DATE,
			last_seen_at: LAST_SEEN,
			containerized_by: server('virtualization_service_ids'),
			match_confidence: server('match_confidences'),
			source: server('sources'),
			port_bindings: ownedArray('The port filter covers it'),
			port: { sort: 'Filter-only', group: 'Filter-only', filter: { param: 'ports' } },
			ip_bindings: ownedArray('Near-unique; search finds it'),
			category: {
				sort: 'Derived in code; no SQL column',
				group: 'Derived in code; no SQL column',
				filter: { param: 'exclude_categories' }
			},
			tags: { ...SHARED_ARRAY, filter: { param: 'tag_ids' } }
		}
	},
	'discovery/components/tabs/DiscoveryHistoryTab.svelte': {
		listPath: '/api/v1/discovery',
		columns: {
			// Runs of one scan share its name, so grouping gathers a scan's history.
			name: { sort: true, group: true, filter: SEARCH },
			daemon_id: server('daemon_ids'),
			network_id: server('network_ids'),
			discovery_type: server('discovery_types'),
			created_at: DATE,
			updated_at: DATE,
			phase: server('phases'),
			started_at: DATE,
			finished_at: DATE,
			duration: { sort: true, group: 'Continuous value', filter: 'Continuous value' },
			warnings: {
				sort: true,
				group: COUNT,
				filter: 'Sorting brings runs with warnings to the top'
			}
		}
	},
	'discovery/components/tabs/DiscoveryScheduledTab.svelte': {
		columns: {
			name: IDENTITY,
			created_at: DATE,
			updated_at: DATE,
			daemon_id: YES,
			network_id: YES,
			discovery_type: YES,
			phase: RUN_RESULT,
			started_at: RUN_RESULT,
			finished_at: RUN_RESULT,
			duration: RUN_RESULT,
			warnings: RUN_RESULT,
			scan_count: { sort: true, group: COUNT, filter: COUNT },
			force_full_scan: BOOLEAN,
			legacy: YES,
			run_type: YES,
			schedule: none('Cron text'),
			last_run: DATE,
			progress: none('Live state of a running scan'),
			tags: SHARED_ARRAY
		}
	},
	'daemons/components/DaemonTab.svelte': {
		columns: {
			name: IDENTITY,
			network_id: YES,
			last_seen: DATE,
			created_at: DATE,
			updated_at: DATE,
			host_id: { sort: 'Near-unique', group: 'One daemon per host', filter: SEARCH },
			status: YES,
			os: YES,
			mode: YES,
			version: YES,
			url: none('Unique per daemon'),
			interfaced_subnet_ids: SHARED_ARRAY,
			tags: SHARED_ARRAY
		}
	},
	'daemon_api_keys/components/ApiKeyTab.svelte': {
		columns: {
			name: IDENTITY,
			network_id: YES,
			is_enabled: BOOLEAN,
			last_used: DATE,
			expires_at: DATE,
			created_at: DATE,
			updated_at: DATE,
			tags: SHARED_ARRAY
		}
	},
	'tags/components/TagTab.svelte': {
		columns: {
			name: IDENTITY,
			color: YES,
			// An orderable boolean: the order field sorts it as a side effect of grouping.
			is_application: YES,
			created_at: DATE,
			updated_at: DATE,
			description: TEXT
		}
	},
	'networks/components/NetworksTab.svelte': {
		columns: {
			name: IDENTITY,
			vlans: ownedArray('Each belongs to one network'),
			daemons: ownedArray('Each belongs to one network'),
			subnets: ownedArray('Each belongs to one network'),
			// Credentials are shared across networks.
			credentials: SHARED_ARRAY,
			tags: SHARED_ARRAY,
			created_at: DATE,
			updated_at: DATE,
			effective_stale_after_hours: YES
		}
	},
	'users/components/UserTab.svelte': {
		columns: {
			email: IDENTITY,
			invite_status: YES,
			permissions: YES,
			network_ids: SHARED_ARRAY,
			oidc_provider: YES,
			invited_by: YES,
			invite_url: none('Unique per invite'),
			created_at: DATE,
			expires_at: DATE,
			updated_at: DATE,
			oidc_linked_at: DATE,
			terms_accepted_at: DATE,
			email_verified: BOOLEAN
		}
	},
	'user_api_keys/components/UserApiKeyTab.svelte': {
		columns: {
			name: IDENTITY,
			permissions: YES,
			network_ids: SHARED_ARRAY,
			is_enabled: BOOLEAN,
			last_used: DATE,
			expires_at: DATE,
			created_at: DATE,
			updated_at: DATE,
			tags: SHARED_ARRAY
		}
	},
	'subnets/components/SubnetTab.svelte': {
		columns: {
			name: IDENTITY,
			cidr: IDENTITY,
			subnet_type: YES,
			network_id: YES,
			created_at: DATE,
			updated_at: DATE,
			last_seen_at: DATE,
			description: TEXT,
			source: YES,
			tags: SHARED_ARRAY
		}
	},
	'credentials/components/CredentialsTab.svelte': {
		columns: {
			name: IDENTITY,
			created_at: DATE,
			updated_at: DATE,
			daemon_os: YES,
			credential_type: YES,
			description: TEXT,
			assigned_networks: SHARED_ARRAY,
			assigned_hosts: ownedArray(SEARCH),
			target: SHARED_ARRAY,
			tags: SHARED_ARRAY
		}
	},
	'vlans/components/VlanTab.svelte': {
		columns: {
			vlan_number: { sort: true, group: 'Unique per network', filter: SEARCH },
			name: IDENTITY,
			created_at: DATE,
			updated_at: DATE,
			last_seen_at: DATE,
			description: TEXT,
			source: YES,
			network_id: YES,
			subnet_ids: SHARED_ARRAY
		}
	}
};

// ============================================================================
// Reading field flags from source
// ============================================================================

const OPENERS = '{([';
const CLOSERS = '})]';
const FLAGS = ['sortable', 'groupable', 'filterable', 'serverFiltered'] as const;
type Flag = (typeof FLAGS)[number];

/** A field as its tab declares it. */
interface ParsedField {
	orderable: boolean;
	type: FieldConfig<unknown>['type'];
	flags: Partial<Record<Flag, boolean>>;
}

/** Index just past the string, template literal or comment opening at `i`; `i` when none does. */
function skipLiteral(src: string, i: number): number {
	const c = src[i];
	if (c === '/' && src[i + 1] === '/') {
		const end = src.indexOf('\n', i);
		return end === -1 ? src.length : end;
	}
	if (c === '/' && src[i + 1] === '*') {
		const end = src.indexOf('*/', i + 2);
		if (end === -1) throw new Error(`unterminated comment at ${i}`);
		return end + 2;
	}
	if (c !== "'" && c !== '"' && c !== '`') return i;
	for (let j = i + 1; j < src.length;) {
		if (src[j] === '\\') j += 2;
		else if (src[j] === c) return j + 1;
		else if (c === '`' && src[j] === '$' && src[j + 1] === '{') j = closingBracket(src, j + 1) + 1;
		else j++;
	}
	throw new Error(`unterminated string at ${i}`);
}

/** Index of the bracket closing the one at `open`, skipping strings and comments. */
function closingBracket(src: string, open: number): number {
	let depth = 0;
	for (let i = open; i < src.length;) {
		const skipped = skipLiteral(src, i);
		if (skipped !== i) {
			i = skipped;
			continue;
		}
		if (OPENERS.includes(src[i])) depth++;
		else if (CLOSERS.includes(src[i]) && --depth === 0) return i;
		i++;
	}
	throw new Error(`unbalanced bracket at ${open}`);
}

/** Every `{ … }` pair in `src` as [open, close], skipping strings and comments. */
function bracePairs(src: string): [number, number][] {
	const pairs: [number, number][] = [];
	const stack: number[] = [];
	for (let i = 0; i < src.length;) {
		const skipped = skipLiteral(src, i);
		if (skipped !== i) {
			i = skipped;
			continue;
		}
		if (src[i] === '{') stack.push(i);
		else if (src[i] === '}') pairs.push([stack.pop()!, i]);
		i++;
	}
	return pairs;
}

/** One top-level entry of an object literal: `name: value`, or `...` for a spread. */
interface Entry {
	name: string;
	value: string;
	valueStart: number;
}

/** The top-level entries of the object literal whose `{` is at `open`. */
function objectEntries(src: string, open: number): Entry[] {
	const close = closingBracket(src, open);
	const entries: Entry[] = [];
	let start = open + 1;
	let depth = 0;
	for (let i = open + 1; i <= close;) {
		const skipped = skipLiteral(src, i);
		if (skipped !== i) {
			i = skipped;
			continue;
		}
		if (OPENERS.includes(src[i])) depth++;
		else if (CLOSERS.includes(src[i])) depth--;
		if ((src[i] === ',' && depth === 0) || i === close) {
			const entry = parseEntry(src, start, i);
			if (entry) entries.push(entry);
			start = i + 1;
		}
		i++;
	}
	return entries;
}

function parseEntry(src: string, start: number, end: number): Entry | null {
	let i = start;
	while (i < end) {
		if (/\s/.test(src[i])) i++;
		else if (src[i] === '/' && skipLiteral(src, i) !== i) i = skipLiteral(src, i);
		else break;
	}
	const head = /^(\.\.\.|[A-Za-z_$][\w$]*|'[^']*')\s*(:?)/.exec(src.slice(i, end));
	if (!head) return null;
	const name = head[1].replace(/'/g, '');
	if (name === '...' || head[2]) {
		let valueStart = i + head[0].length;
		while (valueStart < end && /\s/.test(src[valueStart])) valueStart++;
		return { name, value: src.slice(valueStart, end).trim(), valueStart };
	}
	return { name, value: name, valueStart: i };
}

/** The flags of the field object literal at `open`, or why they cannot be read. */
function parseField(src: string, open: number, orderable: boolean): ParsedField | string {
	const entries = objectEntries(src, open);
	if (entries.some((entry) => entry.name === '...')) {
		return 'spreads another object into the field, so its flags cannot be read here';
	}
	const byName = new Map(entries.map((entry) => [entry.name, entry.value]));
	const type = byName.get('type')?.match(/^'(string|boolean|date|array)'$/)?.[1];
	if (!type) return `has no literal \`type\` ('string', 'boolean', 'date' or 'array')`;
	const flags: ParsedField['flags'] = {};
	for (const flag of FLAGS) {
		const value = byName.get(flag);
		if (value === undefined) continue;
		if (value !== 'true' && value !== 'false') {
			return `sets \`${flag}: ${value}\`; write it as a literal true or false`;
		}
		flags[flag] = value === 'true';
	}
	return { orderable, type: type as ParsedField['type'], flags };
}

/** The `<script>` blocks of a Svelte file: the template's text would derail the brace matcher. */
function scriptOf(source: string): string {
	return [...source.matchAll(/<script\b[^>]*>([\s\S]*?)<\/script>/g)].map((m) => m[1]).join('\n');
}

/**
 * The source and `{` position of the orderable-fields object a tab passes to `defineFields`.
 *
 * Usually an object literal in the call. Otherwise the argument calls an imported function that
 * returns one (`discoveryFields` in `discovery/queries.ts`), read from that module. A local
 * wrapper around it (the scheduled tab's `withSharedDisplay`) may change `display` only.
 */
function orderableFieldsObject(
	tab: string,
	script: string
): { src: string; open: number } | string {
	const call = /defineFields<[^>]*>\s*\(\s*/.exec(script);
	if (!call) return 'calls no defineFields';
	const argStart = call.index + call[0].length;
	if (script[argStart] === '{') return { src: script, open: argStart };

	const paren = call.index + call[0].lastIndexOf('(');
	const argument = script.slice(argStart, closingBracket(script, paren));
	const imports = [...script.matchAll(/import\s*\{([^}]*)\}\s*from\s*'([^']+)'/g)];
	for (const [, name] of argument.matchAll(/\b(\w+)\(/g)) {
		const from = imports.find(([, names]) => names.split(',').some((n) => n.trim() === name))?.[2];
		if (!from || !/^(\$lib\/|\.)/.test(from)) continue;
		const module = from.startsWith('$lib/')
			? path.join(UI, 'src/lib', from.slice('$lib/'.length))
			: path.resolve(path.dirname(path.join(FEATURES, tab)), from);
		const src = fs.readFileSync(module.endsWith('.ts') ? module : `${module}.ts`, 'utf-8');
		const declaration = src.indexOf(`export const ${name} =`);
		const body = declaration === -1 ? -1 : src.indexOf('=> ({', declaration);
		if (body !== -1) return { src, open: body + '=> ('.length };
	}
	return `passes defineFields "${argument.trim()}", which this test cannot trace to an object literal`;
}

/** Every field a tab declares, by column key, plus anything that could not be read. */
function tabFields(
	tab: string,
	source: string
): { fields: Map<string, ParsedField>; problems: string[] } {
	const script = scriptOf(source);
	const fields = new Map<string, ParsedField>();
	const problems: string[] = [];

	if (definedOrderField(source)) {
		const object = orderableFieldsObject(tab, script);
		if (typeof object === 'string') problems.push(object);
		else {
			for (const entry of objectEntries(object.src, object.open)) {
				const field =
					object.src[entry.valueStart] === '{'
						? parseField(object.src, entry.valueStart, true)
						: 'is not an object literal';
				if (typeof field === 'string') problems.push(`orderable field "${entry.name}" ${field}`);
				else fields.set(entry.name, field);
			}
		}
	}

	const pairs = bracePairs(script);
	for (const match of script.matchAll(/\bkey:\s*'([^']+)'/g)) {
		const key = match[1];
		const at = match.index!;
		const enclosing = pairs
			.filter(([open, close]) => open < at && close > at)
			.reduce<[number, number] | undefined>(
				(inner, pair) => (!inner || pair[0] > inner[0] ? pair : inner),
				undefined
			);
		const declaresKey =
			enclosing &&
			objectEntries(script, enclosing[0]).some(
				(entry) => entry.name === 'key' && entry.value === `'${key}'`
			);
		if (!enclosing || !declaresKey) {
			problems.push(`cannot find the field object that declares key '${key}'`);
			continue;
		}
		if (fields.has(key)) {
			problems.push(`declares the column "${key}" twice`);
			continue;
		}
		const field = parseField(script, enclosing[0], false);
		if (typeof field === 'string') problems.push(`display field "${key}" ${field}`);
		else fields.set(key, field);
	}
	return { fields, problems };
}

/** The field as the runtime sees it, built from the parsed flags. */
function fieldConfig(column: string, field: ParsedField): FieldConfig<unknown> {
	const base = { label: column, type: field.type, ...field.flags };
	return (
		field.orderable ? { ...base, orderField: column } : { ...base, key: column }
	) as FieldConfig<unknown>;
}

/** The query parameter names of `GET path`, following `$ref` into `components.parameters`. */
function listQueryParams(listPath: string): Set<string> | undefined {
	const operation = OPENAPI.paths[listPath]?.get;
	if (!operation) return undefined;
	const resolve = (node: SchemaNode): SchemaNode =>
		node.$ref ? resolve(OPENAPI.components.parameters[node.$ref.split('/').pop()]) : node;
	return new Set(
		(operation.parameters ?? [])
			.map(resolve)
			.filter((param: SchemaNode) => param.in === 'query')
			.map((param: SchemaNode) => param.name)
	);
}

// ============================================================================
// Comparing decisions with the tab
// ============================================================================

const VERB: Record<Capability, string> = { sort: 'sorts', group: 'groups', filter: 'filters' };

/** Everything wrong with one tab's decisions, as messages naming the tab, column and fix. */
function decisionProblems(tab: string, decided: TabDecisions): string[] {
	const source = readTab(tab);
	const problems: string[] = [];
	const at = `DECISIONS["${tab}"].columns`;

	// A field this test cannot read would surface as a disagreement on every capability, so stop
	// at the cause.
	const { fields, problems: unreadable } = tabFields(tab, source);
	if (unreadable.length > 0) return unreadable.map((problem) => `${tab}: ${problem}.`);

	const serverPaginated = /serverPagination=/.test(source);
	const serverTags = /onTagFilterChange=/.test(source);
	const enumSchema = COVERAGE[tab] ? orderFieldSchema(COVERAGE[tab]) : definedOrderField(source);
	const variants = new Set<string>(SCHEMAS[enumSchema ?? '']?.enum ?? []);
	const columns = columnKeys(source, enumSchema);

	if (serverPaginated && !decided.listPath) {
		problems.push(
			`${tab}: the tab is server-paginated but DECISIONS["${tab}"] has no listPath. Set it to ` +
				`the path of the GET operation that lists these rows.`
		);
	}
	if (!serverPaginated && decided.listPath) {
		problems.push(`${tab}: listPath is set but the tab is not server-paginated. Remove listPath.`);
	}
	const params = decided.listPath ? listQueryParams(decided.listPath) : undefined;
	if (decided.listPath && !params) {
		problems.push(`${tab}: listPath "${decided.listPath}" has no GET operation in openapi.json.`);
	}

	for (const variant of variants) {
		if (!(variant in decided.columns)) {
			problems.push(
				`${tab}: ${enumSchema} variant "${variant}" has no decision. Add ${at}.${variant} ` +
					`with { sort, group, filter }, each true or the reason it is not offered.`
			);
		}
	}
	for (const column of columns) {
		if (!variants.has(column) && !(column in decided.columns)) {
			problems.push(
				`${tab}: column "${column}" has no decision. Add ${at}.${column} with ` +
					`{ sort, group, filter }, each true or the reason it is not offered.`
			);
		}
	}

	const configs = new Map(
		[...fields].map(([column, field]) => [column, fieldConfig(column, field)] as const)
	);
	if (serverPaginated) {
		for (const column of serverOrderViolations([...configs.values()], true)) {
			problems.push(
				`${tab}: display field "${column}" sets sortable or groupable on a server-paginated ` +
					`list, which would order only the loaded page. Drop the flag, or add a ` +
					`${enumSchema} variant for it.`
			);
		}
		for (const column of serverFilterViolations([...configs.values()], true, {
			tags: serverTags,
			fields: true
		})) {
			const fix =
				column === 'tags'
					? 'Pass onTagFilterChange to DataControls and send the ids as tag_ids'
					: 'Mark it serverFiltered and handle it in onFilterChange';
			problems.push(
				`${tab}: "${column}" is filterable on a server-paginated list but not filtered by the ` +
					`server, so it would narrow only the loaded page. ${fix}, or drop filterable.`
			);
		}
	}

	for (const [column, decision] of Object.entries(decided.columns)) {
		if (!columns.has(column)) {
			problems.push(
				`${tab}: ${at}.${column} names a column the tab no longer declares. Remove the stale decision.`
			);
			continue;
		}
		const config = configs.get(column);
		const offered: Record<Capability, boolean> = {
			sort: config ? isSortableField(config, serverPaginated) : false,
			group: config ? isGroupableField(config, serverPaginated) : false,
			filter: config?.filterable === true
		};

		for (const capability of CAPABILITIES) {
			const choice = decision[capability] as FilterChoice | undefined;
			const where = `${tab}: ${at}.${column}.${capability}`;
			if (choice === undefined) {
				problems.push(`${where} is missing. Set it to true or the reason it is not offered.`);
				continue;
			}
			if (typeof choice === 'string' && !choice.trim()) {
				problems.push(`${where} has an empty reason. Say why "${column}" has no ${capability}.`);
				continue;
			}
			if (typeof choice === 'object' && capability !== 'filter') {
				problems.push(`${where} is { param }, which only a filter takes. Use true or a reason.`);
				continue;
			}
			const yes = choice === true || typeof choice === 'object';

			if (serverPaginated && yes && capability !== 'filter' && !variants.has(column)) {
				problems.push(
					`${where} is true, but "${column}" is not a ${enumSchema} variant and a ` +
						`server-paginated list ${VERB[capability]} only by order fields. Add the variant ` +
						`to the backend enum, or record the reason it does not ${capability}.`
				);
			} else if (yes && !offered[capability]) {
				problems.push(
					`${where} is true, but the tab does not offer it. Set the field's flag, or record ` +
						`the reason "${column}" has no ${capability}.`
				);
			} else if (!yes && offered[capability]) {
				problems.push(
					`${tab}: the tab ${VERB[capability]} by "${column}", but ${at}.${column}.${capability} ` +
						`says no ("${choice}"). Drop the flag from the field, or set the decision to true.`
				);
			}

			if (capability !== 'filter' || !yes) continue;
			if (serverPaginated && choice === true) {
				problems.push(
					`${where} is true on a server-paginated tab. Name the query parameter that applies ` +
						`it: { param: '…' } from GET ${decided.listPath}.`
				);
			}
			if (typeof choice === 'object') {
				if (!serverPaginated) {
					problems.push(
						`${where} names param "${choice.param}", but the tab filters client-side. Set it to true.`
					);
				} else if (params && !params.has(choice.param)) {
					problems.push(
						`${where} names param "${choice.param}", which is not a query parameter of GET ` +
							`${decided.listPath}. Use one of: ${[...params].join(', ')}.`
					);
				}
			}
		}
	}
	return problems;
}

describe('entity field controls', () => {
	it('has decisions for every tab that renders DataControls', () => {
		const tabs = dataControlsTabs();
		expect(tabs.length).toBeGreaterThan(10);

		const missing = tabs.filter((tab) => !(tab in DECISIONS));
		expect(
			missing,
			`These tabs render <DataControls> with no DECISIONS entry:\n\n  ${missing.join('\n  ')}\n\n` +
				`Add one per tab with a sort, group and filter decision for every column.`
		).toEqual([]);

		const stale = Object.keys(DECISIONS).filter((tab) => !tabs.includes(tab));
		expect(
			stale,
			`DECISIONS entries whose file is missing or no longer renders <DataControls>:\n\n  ` +
				`${stale.join('\n  ')}\n\nUpdate the path or remove the entry.`
		).toEqual([]);
	});

	for (const [tab, decided] of Object.entries(DECISIONS)) {
		it(`${tab} offers the decided sort, group and filter on every column`, () => {
			expect(fs.existsSync(path.join(FEATURES, tab)), `${tab} does not exist`).toBe(true);
			const problems = decisionProblems(tab, decided);
			expect(problems, problems.join('\n')).toEqual([]);
		});
	}
});
