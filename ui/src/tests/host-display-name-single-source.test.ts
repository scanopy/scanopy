import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SRC = path.resolve(__dirname, '..');

/**
 * A host is titled one way, by one function.
 *
 * `Host::display_name` is the ladder — `name`, else hostname, else sysName, else chassis id, else
 * the first address — and it has been on the API as `display_name` since it shipped. The frontend
 * ignored it and read `host.name` instead, so every host that had never been named (the entire
 * `Unnamed` backfill, controller-imported devices, every LLDP far end) rendered as the empty string
 * in the table, in every picker and in every confirm dialog, while the same host was
 * titled correctly on the topology canvas.
 *
 * Nothing about that was hard to fix and nothing stops it recurring: `host.name` is right there on
 * the type, it is a `string`, and it reads like the answer. So the rule is mechanical — reading it
 * for display is what a reviewer has to catch, and this catches it instead.
 *
 * Chosen over the alternatives, and this is the reason each was rejected:
 *  - A custom ESLint rule would need plugin infrastructure this flat config doesn't have, for no
 *    more precision than the patterns below.
 *  - Removing `name` from the type is impossible: the editor writes it back.
 *  - Branding `Host['name']` nominally buys nothing — a branded string is still a `string`, so
 *    `{host.name}` in a template and any `title={…}` prop still compile.
 */

/** The helper, and the type that documents it. Both name `host.name` legitimately. */
const HELPER = path.join(SRC, 'lib/features/hosts/host-display-name.ts');
const HOST_TYPE = path.join(SRC, 'lib/features/hosts/types/base.ts');

/**
 * Files allowed to read `host.name`, each because it is not a title.
 *
 * `name` is what a person typed and what the editor writes back, so the create/update payloads and
 * the form bindings must round-trip it verbatim — titling those with the ladder would persist a
 * chassis id as a name the next save keeps. Add to this list only for that reason; a display that
 * "needs the raw name" is the bug.
 */
const ALLOWED = new Set([
	HELPER,
	HOST_TYPE,
	// Create/update request bodies and the host edit form's own field state.
	path.join(SRC, 'lib/features/hosts/queries.ts'),
	path.join(SRC, 'lib/features/hosts/components/HostEditModal/HostEditor.svelte'),
	path.join(SRC, 'lib/features/hosts/components/HostEditModal/Details/HostDetailsForm.svelte'),
	// Fixture data for the dependency tutorial: invented hosts, not API rows.
	path.join(SRC, 'lib/features/topology/components/dependency-tutorial-data.ts')
]);

/**
 * Reads of `.name` off something host-shaped.
 *
 * Deliberately shape-matched rather than type-aware: a regex cannot know what `h` is, but every
 * occurrence this change fixed was spelled one of these ways, and a new surface will spell it the
 * same. Comments and message-key names are stripped first so prose about "host names" doesn't trip
 * it.
 */
const HOST_NAME_READS = [
	// `host.name`, `otherHost.name`, `neighborHost?.name`, `hypervisorHost.name`, `h.name`
	/\b(?:[A-Za-z_$][\w$]*)?[Hh]ost\??\.name\b/,
	/\bh\??\.name\b/,
	// `hosts[0].name`, `hosts.find(...)?.name`, `hostsData.find(...)?.name`
	/\bhosts(?:Data)?\b[^\n;]{0,120}?\??\.name\b/
];

function findFilesRecursively(dir: string, extensions: string[]): string[] {
	const files: string[] = [];
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const fullPath = path.join(dir, entry.name);
		if (entry.isDirectory() && entry.name !== 'node_modules' && entry.name !== 'paraglide') {
			files.push(...findFilesRecursively(fullPath, extensions));
		} else if (entry.isFile() && extensions.some((ext) => entry.name.endsWith(ext))) {
			files.push(fullPath);
		}
	}
	return files;
}

/** Blank out comments so prose ("needs one host name per daemon chip") isn't scanned. */
function stripComments(source: string): string {
	return source.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|[^:])\/\/.*$/gm, '$1');
}

describe('host display name is the single source of a host title', () => {
	const files = findFilesRecursively(SRC, ['.svelte', '.ts']).filter(
		(file) => !file.startsWith(path.join(SRC, 'tests'))
	);

	it('is the only thing any surface reads to title a host', () => {
		const offenders: string[] = [];

		for (const file of files) {
			if (ALLOWED.has(file)) continue;
			const source = stripComments(fs.readFileSync(file, 'utf-8'));

			source.split('\n').forEach((line, index) => {
				if (!HOST_NAME_READS.some((pattern) => pattern.test(line))) return;
				offenders.push(`${path.relative(SRC, file)}:${index + 1}  ${line.trim()}`);
			});
		}

		expect(
			offenders,
			`These read a host's raw \`name\` for display. A host that has never been named stores it ` +
				`as the empty string, so each of these renders nothing at all for exactly the hosts ` +
				`that most need a label. Call \`hostDisplayName(host)\` instead — it reads the ` +
				`server-resolved \`display_name\` and falls back once, for everyone.\n\n` +
				offenders.join('\n')
		).toEqual([]);
	});

	it('is what the two seams that carry most surfaces actually call', () => {
		// The negative check above cannot see a refactor that quietly stops calling the helper —
		// deleting a call is not an offending pattern. These two carry the bulk of the surfaces
		// between them, so they are asserted positively: `HostDisplay.getLabel` is the label for
		// every picker row, inspector card and entity chip, and the `name` field's `getValue` is
		// the host table cell, the row header and the row checkbox's accessible name at once.
		const seams = [
			path.join(SRC, 'lib/shared/components/forms/selection/display/HostDisplay.svelte'),
			path.join(SRC, 'lib/features/hosts/components/HostTab.svelte')
		];

		for (const seam of seams) {
			expect(
				fs.readFileSync(seam, 'utf-8'),
				`${path.relative(SRC, seam)} must title hosts`
			).toMatch(/hostDisplayName\(/);
		}
	});
});
