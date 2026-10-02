import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * Guards the fix for the app-wide full-hosts fetch.
 *
 * `useHostsQuery({ limit: 0 })` returns every host in the organisation with all
 * its nested ip-addresses, ports, services and interfaces (~1.9MB on a 440-host
 * estate). Because TanStack dedupes by query key, a dozen consumers all shared one
 * such request — which meant removing any single caller changed nothing, and any
 * new caller silently resurrects the whole cost for every surface at once.
 *
 * That coupling is why this is a static test rather than a code-review habit: the
 * regression is invisible at the call site that causes it.
 *
 * The rule: a host list is either paginated, or scoped (`useHostsByIds`), or
 * children-free (`useHostSummariesQuery`). If you need an unbounded nested list,
 * add the call site to ALLOWED_UNBOUNDED below with the reason.
 */

/**
 * Call sites permitted to issue an unbounded nested host query, keyed by path
 * relative to `src/`. Each entry is a surface that reads one of the child caches
 * `useHostsQuery` populates as a side effect, and cannot move until those caches
 * have real queries of their own — see
 * `planned-work/child-cache-rearchitecture.md`. Each is lazy (it mounts inside
 * a modal tab), so none costs anything on page load.
 */
const ALLOWED_UNBOUNDED = new Map<string, string>([
	[
		'lib/features/hosts/components/HostEditModal/Virtualization/VirtualizationForm.svelte',
		'VM picker labels candidate hosts with their services, which come from the services cache this query populates.'
	]
]);

function findFilesRecursively(dir: string, extensions: string[]): string[] {
	const files: string[] = [];
	const entries = fs.readdirSync(dir, { withFileTypes: true });

	for (const entry of entries) {
		const fullPath = path.join(dir, entry.name);
		if (entry.isDirectory() && entry.name !== 'node_modules' && entry.name !== 'tests') {
			files.push(...findFilesRecursively(fullPath, extensions));
		} else if (entry.isFile() && extensions.some((ext) => entry.name.endsWith(ext))) {
			files.push(fullPath);
		}
	}

	return files;
}

describe('unbounded hosts query', () => {
	const srcPath = path.resolve(__dirname, '..');
	// The hook's own definition legitimately documents and implements `limit: 0`.
	const HOOK_DEFINITION = 'lib/features/hosts/queries.ts';

	it('is not introduced at new call sites', () => {
		const files = findFilesRecursively(srcPath, ['.svelte', '.ts']);
		const violations: string[] = [];

		for (const file of files) {
			const rel = path.relative(srcPath, file);
			if (rel === HOOK_DEFINITION) continue;

			const content = fs.readFileSync(file, 'utf8');
			const lines = content.split('\n');

			lines.forEach((line, idx) => {
				// Only actual calls — `useHostsQuery(` followed by an inline `limit: 0`.
				// Comments explaining the historical query are common and must not trip.
				const trimmed = line.trim();
				if (trimmed.startsWith('//') || trimmed.startsWith('*')) return;
				if (!/useHostsQuery\s*\(/.test(line)) return;
				if (!/limit:\s*0\b/.test(line)) return;
				if (ALLOWED_UNBOUNDED.has(rel)) return;
				violations.push(`  ${rel}:${idx + 1}  ${trimmed}`);
			});
		}

		if (violations.length > 0) {
			expect.fail(
				`Found ${violations.length} unbounded nested host query call site(s):\n\n${violations.join('\n')}\n\n` +
					`An unbounded useHostsQuery downloads every host with all nested children ` +
					`(~1.9MB on a 440-host estate) and is shared by query key with every other ` +
					`consumer, so it loads on surfaces that never asked for it.\n\n` +
					`Use instead:\n` +
					`  - useHostsByIds(...)          when you need specific hosts (names, lookups)\n` +
					`  - useHostSummariesQuery(...)  when you need a host list without children\n` +
					`  - useHostsQuery({ limit: n }) when you genuinely need the nested graph, paginated\n\n` +
					`If none of those work, add the file to ALLOWED_UNBOUNDED with the reason.`
			);
		}
	});

	it('has no stale allowlist entries', () => {
		// An allowlist entry that no longer corresponds to a real unbounded call site
		// is worse than none: it silently permits a future regression in that file.
		const stale: string[] = [];

		for (const [rel, reason] of ALLOWED_UNBOUNDED) {
			const full = path.join(srcPath, rel);
			if (!fs.existsSync(full)) {
				stale.push(`  ${rel} — file no longer exists (reason given: ${reason})`);
				continue;
			}
			const content = fs.readFileSync(full, 'utf8');
			const hasUnbounded = content
				.split('\n')
				.some(
					(line) =>
						!line.trim().startsWith('//') &&
						/useHostsQuery\s*\(/.test(line) &&
						/limit:\s*0\b/.test(line)
				);
			if (!hasUnbounded) {
				stale.push(`  ${rel} — no longer issues an unbounded query, remove the entry`);
			}
		}

		if (stale.length > 0) {
			expect.fail(`Stale ALLOWED_UNBOUNDED entries:\n\n${stale.join('\n')}`);
		}
	});
});
