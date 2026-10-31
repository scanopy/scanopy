import { describe, expect, it } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import resourcesJson from '$lib/data/resources.json';
import schema from '../../static/openapi.json';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SRC = path.resolve(__dirname, '..');

function sourceFiles(dir: string): string[] {
	return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
		const full = path.join(dir, entry.name);
		if (entry.isDirectory()) {
			return ['paraglide', 'data', 'tests', 'node_modules'].includes(entry.name)
				? []
				: sourceFiles(full);
		}
		return /\.(svelte|ts)$/.test(entry.name) && entry.name !== 'schema.d.ts' ? [full] : [];
	});
}

const sources = sourceFiles(SRC).map((file) => ({
	file: path.relative(SRC, file),
	text: fs.readFileSync(file, 'utf8')
}));

/**
 * Every permission question goes through `can`, which reads what `/api/auth/me` sends. A role
 * compared by name would say nothing about a custom role, or about a role on one site.
 */
describe('permission checks', () => {
	it('compare no role by name', () => {
		const roleCompared = /\.permissions\s*[!=]==?\s*['"]|userPermissions/;
		expect(sources.filter(({ text }) => roleCompared.test(text)).map((s) => s.file)).toEqual([]);
	});

	it('read no retired role metadata', () => {
		const retired = /manage_org_entities|canListUsers|requiredPermissions/;
		expect(sources.filter(({ text }) => retired.test(text)).map((s) => s.file)).toEqual([]);
	});

	it('name only resources and actions the generated schema defines', () => {
		const { Resource, Action } = schema.components.schemas;
		const calls = sources.flatMap(({ text }) =>
			[...text.matchAll(/\bcan\('(\w+)', '(\w+)'/g)].map(([, resource, action]) => ({
				resource,
				action
			}))
		);
		expect(calls.length).toBeGreaterThan(0);
		for (const { resource, action } of calls) {
			expect(Resource.enum).toContain(resource);
			expect(Action.enum).toContain(action);
		}
	});

	it('take their types from the schema, with no second list of variants', () => {
		const access = sources.find(({ file }) => file.endsWith('features/auth/access.ts'));
		expect(access?.text).toContain("export type Resource = components['schemas']['Resource']");
		expect(access?.text).toContain("export type Action = components['schemas']['Action']");
		const handWritten = /type (Resource|Action) =\s*'/;
		expect(sources.filter(({ text }) => handWritten.test(text)).map((s) => s.file)).toEqual([]);
	});

	it('cover every resource in the fixture the comparison reads', () => {
		const { Resource } = schema.components.schemas;
		expect(resourcesJson.map((r) => r.id).sort()).toEqual([...Resource.enum].sort());
	});
});
