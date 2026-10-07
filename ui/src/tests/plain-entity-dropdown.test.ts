import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * Entity pickers use `RichSelect` with the entity's `*Display` component, so a picked site,
 * IP address or port shows its icon, tags and context like everywhere else in the app. A plain
 * `<select>` or `SelectInput` shows a bare name.
 *
 * Two rules:
 * - No plain dropdown is bound to an entity id (`site_id`, `selectedSiteId`, ...).
 * - Every plain dropdown lives in a file listed in ALLOWED_PLAIN_DROPDOWNS, with the reason it
 *   stays plain. A new one needs an entry, which is the moment to ask whether it picks an entity.
 */

/** Files allowed to keep plain dropdowns, keyed by path relative to `src/`. */
const ALLOWED_PLAIN_DROPDOWNS = new Map<string, string>([
	[
		'lib/features/daemons/components/DaemonEditModal.svelte',
		'Daemon mode: disabled, read-only, two fixed values.'
	],
	[
		'lib/features/daemons/components/OsSelector.svelte',
		'Mobile fallback for the OS button row; OS is an enum with no display component.'
	],
	[
		'lib/features/daemons/components/CreateDaemonModal/steps/AdvancedStep.svelte',
		'Generic renderer for daemon config fields; options come from the field definitions.'
	],
	[
		'lib/features/discovery/components/DiscoveryModal/DiscoveryDetailsForm.svelte',
		'Host naming fallback: two-value enum.'
	],
	[
		'lib/features/discovery/components/DiscoveryModal/DiscoveryScheduleForm.svelte',
		'Timezone: a long plain-text list.'
	],
	[
		'lib/features/hosts/components/HostEditModal/Ports/PortConfigPanel.svelte',
		'Protocol: TCP/UDP.'
	],
	[
		'lib/features/credentials/components/CredentialForm.svelte',
		'Credential select fields: options are backend-defined field values, not entities.'
	],
	[
		'lib/features/settings/SystemTab.svelte',
		'Display preferences: short static lists and timezone.'
	],
	['lib/features/home/components/ReferralSourcePrompt.svelte', 'Survey answer: short static list.'],
	['lib/features/home/components/ProfilePrompt.svelte', 'Survey answers: short static lists.'],
	[
		'lib/features/billing/CancelSubscriptionModal.svelte',
		'Cancellation reason: short static list.'
	],
	['lib/features/billing/PlanInquiryModal.svelte', 'Inquiry answers: short static lists.'],
	['lib/shared/components/data/controls/PaginationBar.svelte', 'Page size.']
]);

/** The dropdown primitives themselves. */
const PRIMITIVES = new Set([
	'lib/shared/components/forms/input/SelectInput.svelte',
	'lib/shared/components/forms/input/MultiSelect.svelte'
]);

/** An identifier ending in `_id(s)` or `Id(s)`: `site_id`, `selectedSiteId`, `tag_ids`. */
const ENTITY_ID = /(?:\w_ids?|[a-z]Ids?)\b/;

export interface PlainDropdown {
	line: number;
	/** The bound expression or form field name, '' when none was found. */
	binding: string;
}

/** Blank out a match but keep its newlines, so line numbers still line up. */
function blank(match: string): string {
	return match.replace(/[^\n]/g, ' ');
}

/** Index just past the brace group opening at `start`. */
function skipBraces(source: string, start: number): number {
	let depth = 0;
	for (let i = start; i < source.length; i++) {
		if (source[i] === '{') depth++;
		else if (source[i] === '}' && --depth === 0) return i + 1;
	}
	return source.length;
}

/** The text of a tag's attributes, from after its name to its closing `>`, skipping
 *  `{...}` expressions (which may contain `=>`) and quoted strings. */
function tagAttributes(source: string, start: number): string {
	let i = start;
	while (i < source.length && source[i] !== '>') {
		if (source[i] === '{') i = skipBraces(source, i);
		else if (source[i] === '"' || source[i] === "'")
			i = source.indexOf(source[i], i + 1) + 1 || source.length;
		else i++;
	}
	return source.slice(start, i);
}

/** The value of attribute `name` in `attrs`: the inside of `{...}` or of quotes. */
function attribute(attrs: string, name: string): string | null {
	const match = new RegExp(`(?:^|\\s)${name}=(["'{])`).exec(attrs);
	if (!match) return null;
	const open = match.index + match[0].length - 1;
	if (match[1] === '{') return attrs.slice(open + 1, skipBraces(attrs, open) - 1);
	return attrs.slice(open + 1, attrs.indexOf(match[1], open + 1));
}

/** Every `<select>` and `<SelectInput>` in a component's markup, with what it is bound to: a
 *  `<select>`'s `value`, or the `name` of the form field a `SelectInput` sits in. */
export function findPlainDropdowns(source: string): PlainDropdown[] {
	const markup = source
		.replace(/<script\b[\s\S]*?<\/script\b[^>]*>/gi, blank)
		.replace(/<!--[\s\S]*?-->/g, blank);

	const found: PlainDropdown[] = [];
	for (const match of markup.matchAll(/<(select|SelectInput)\b/g)) {
		const index = match.index!;
		const attrs = tagAttributes(markup, index + match[0].length);
		let binding: string | null;
		if (match[1] === 'select') {
			binding = attribute(attrs, 'bind:value') ?? attribute(attrs, 'value');
		} else {
			const fields = [...markup.slice(0, index).matchAll(/<[\w.]+\.Field\b/g)];
			const field = fields.at(-1);
			binding = field
				? attribute(tagAttributes(markup, field.index! + field[0].length), 'name')
				: null;
		}
		found.push({
			line: markup.slice(0, index).split('\n').length,
			binding: binding ?? ''
		});
	}
	return found;
}

function findSvelteFiles(dir: string): string[] {
	const files: string[] = [];
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const fullPath = path.join(dir, entry.name);
		if (entry.isDirectory() && !['node_modules', 'tests', 'paraglide'].includes(entry.name)) {
			files.push(...findSvelteFiles(fullPath));
		} else if (entry.isFile() && entry.name.endsWith('.svelte')) {
			files.push(fullPath);
		}
	}
	return files;
}

describe('findPlainDropdowns', () => {
	it('flags a <select> bound to an entity id', () => {
		const found = findPlainDropdowns(
			'<div>\n<select value={siteId} onchange={(e) => pick(e)}>\n</select>\n</div>'
		);
		expect(found).toEqual([{ line: 2, binding: 'siteId' }]);
		expect(ENTITY_ID.test(found[0].binding)).toBe(true);
	});

	it('takes a SelectInput binding from its enclosing form field', () => {
		const found = findPlainDropdowns(
			'<form.Field name="site_id">\n{#snippet children(field)}\n<SelectInput {field} options={o} />\n{/snippet}\n</form.Field>'
		);
		expect(found).toEqual([{ line: 3, binding: 'site_id' }]);
		expect(ENTITY_ID.test(found[0].binding)).toBe(true);
	});

	it('passes an enum binding and ignores script and comments', () => {
		const found = findPlainDropdowns(
			'<script>\n// a <select> value\n</script>\n<!-- <select value={hostId}> -->\n<form.Field name="subnet_type">\n<SelectInput {field} />\n</form.Field>\n<select bind:value={fieldValues[field.id]}></select>'
		);
		expect(found.map((d) => d.binding)).toEqual(['subnet_type', 'fieldValues[field.id]']);
		expect(found.some((d) => ENTITY_ID.test(d.binding))).toBe(false);
	});
});

describe('plain entity dropdowns', () => {
	const srcPath = path.resolve(__dirname, '..');
	const dropdowns = findSvelteFiles(srcPath)
		.map((file) => path.relative(srcPath, file))
		.filter((rel) => !PRIMITIVES.has(rel))
		.flatMap((rel) =>
			findPlainDropdowns(fs.readFileSync(path.join(srcPath, rel), 'utf8')).map((d) => ({
				rel,
				...d
			}))
		);

	it('are not bound to an entity id', () => {
		const violations = dropdowns
			.filter((d) => ENTITY_ID.test(d.binding))
			.map((d) => `  ${d.rel}:${d.line}  bound to ${d.binding}`);

		if (violations.length > 0) {
			expect.fail(
				`Found ${violations.length} plain dropdown(s) picking an entity:\n\n${violations.join('\n')}\n\n` +
					`Use RichSelect with the entity's Display component from ` +
					`lib/shared/components/forms/selection/display/ (see SelectSite.svelte).`
			);
		}
	});

	it('appear only in allowlisted files', () => {
		const violations = dropdowns
			.filter((d) => !ALLOWED_PLAIN_DROPDOWNS.has(d.rel))
			.map((d) => `  ${d.rel}:${d.line}  bound to ${d.binding || '(unknown)'}`);

		if (violations.length > 0) {
			expect.fail(
				`Found ${violations.length} plain dropdown(s) outside ALLOWED_PLAIN_DROPDOWNS:\n\n${violations.join('\n')}\n\n` +
					`If it picks an entity, use RichSelect with the entity's Display component. ` +
					`Otherwise add the file to ALLOWED_PLAIN_DROPDOWNS with the reason it stays plain.`
			);
		}
	});

	it('has no stale allowlist entries', () => {
		const stale = [...ALLOWED_PLAIN_DROPDOWNS.keys()]
			.filter((rel) => !dropdowns.some((d) => d.rel === rel))
			.map((rel) =>
				fs.existsSync(path.join(srcPath, rel))
					? `  ${rel} — has no plain dropdown any more, remove the entry`
					: `  ${rel} — file no longer exists`
			);

		if (stale.length > 0) {
			expect.fail(`Stale ALLOWED_PLAIN_DROPDOWNS entries:\n\n${stale.join('\n')}`);
		}
	});
});
