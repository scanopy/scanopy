import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import { isUserManagedSubnet } from '$lib/features/subnets/queries';
import type { Subnet } from '$lib/features/subnets/types/base';
import type { components } from '$lib/api/schema';

type EntitySource = components['schemas']['EntitySource'];
type SubnetType = Subnet['subnet_type'];

function subnet(subnet_type: SubnetType, source: EntitySource): Subnet {
	return { id: 's1', name: 'n', cidr: '10.8.0.0/24', subnet_type, source } as Subnet;
}

/**
 * GH #677: assigning `Remote` to a subnet the user had created hid it from every
 * management view, leaving no way to edit, delete or recreate that CIDR.
 *
 * The rule is provenance, not category — it mirrors `Subnet::is_user_managed` in
 * `backend/src/server/subnets/impl/base.rs`, which the dashboard's subnet count
 * uses. If these two disagree the totals disagree with the pages, which is the
 * other half of what the reporter saw.
 */
describe('isUserManagedSubnet', () => {
	it('keeps a subnet the user created, whatever category they gave it', () => {
		for (const type of ['Remote', 'Internet', 'Loopback', 'Lan'] as SubnetType[]) {
			expect(isUserManagedSubnet(subnet(type, { type: 'Manual' })), type).toBe(true);
		}
	});

	it('omits the rows Scanopy fabricates for itself', () => {
		// The per-site 0.0.0.0/0 supernets seeded by `seed_data`...
		expect(isUserManagedSubnet(subnet('Internet', { type: 'System' }))).toBe(false);
		expect(isUserManagedSubnet(subnet('Remote', { type: 'System' }))).toBe(false);
		// ...and the loopback row seeded per daemon host.
		expect(isUserManagedSubnet(subnet('Loopback', { type: 'Discovery' }))).toBe(false);
	});

	it('keeps discovered subnets that are real inventory', () => {
		expect(isUserManagedSubnet(subnet('Lan', { type: 'Discovery' }))).toBe(true);
		expect(isUserManagedSubnet(subnet('DockerBridge', { type: 'Discovery' }))).toBe(true);
	});

	it('fails open for a subnet type this build does not know', () => {
		// A newer server may emit a type absent from our fixture. Showing an
		// unrecognised subnet is recoverable; hiding it reproduces #677.
		const future = subnet('SomeFutureType' as SubnetType, { type: 'Discovery' });
		expect(isUserManagedSubnet(future)).toBe(true);
	});
});

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FEATURES = path.resolve(__dirname, '../lib/features');

function svelteFiles(dir: string): string[] {
	const files: string[] = [];
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const full = path.join(dir, entry.name);
		if (entry.isDirectory() && entry.name !== 'node_modules') files.push(...svelteFiles(full));
		else if (entry.isFile() && entry.name.endsWith('.svelte')) files.push(full);
	}
	return files;
}

/**
 * The trap reported alongside #677: checking "Stale only" emptied the Subnets tab,
 * which swapped in the *never-configured* empty state — and that state replaces
 * `DataControls`, taking the filter controls with it, so the filter that caused it
 * could no longer be cleared.
 *
 * The narrowing is server-side, so the tab cannot tell "no rows" from "no matches"
 * by length alone: it has to consult the filter state it owns. A tab that hands
 * `DataControls` a server-side narrowing callback and still gates it on a bare
 * `length === 0` is one filter away from the same dead end.
 */
describe('server-filtered tabs keep their filter controls reachable', () => {
	const SERVER_NARROWING = /on(StaleFilterChange|SearchChange|TagFilterChange|FilterChange)=/;
	/** `items.length === 0` with nothing else in the condition. */
	const BARE_LENGTH_GATE = /\{:else if\s+\w+\.length === 0\s*\}/;

	const tabs = svelteFiles(FEATURES).filter((file) =>
		/from\s+['"][^'"]*DataControls\.svelte['"]/.test(fs.readFileSync(file, 'utf-8'))
	);

	it('finds the tabs to check', () => {
		// Guards against the filter above silently matching nothing.
		expect(tabs.length).toBeGreaterThan(8);
	});

	it.each(tabs.map((file) => [path.relative(FEATURES, file), file]))(
		'%s',
		(_name, file: string) => {
			const source = fs.readFileSync(file, 'utf-8');
			if (!SERVER_NARROWING.test(source)) return;
			expect(
				BARE_LENGTH_GATE.test(source),
				`gates DataControls on a bare length === 0 while passing a server-side narrowing ` +
					`callback, so an empty filtered result hides the control that would clear it`
			).toBe(false);
		}
	);
});
