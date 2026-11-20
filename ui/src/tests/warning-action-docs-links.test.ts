import { describe, it, expect, vi } from 'vitest';

vi.mock('#lib/shared/stores/modal-registry.ts', () => ({
	openModal: vi.fn(),
	closeModal: vi.fn(),
	navigateToEntity: vi.fn()
}));
vi.mock('#lib/features/credentials/utils/prefill.ts', () => ({
	createIntegrationCredentialLabel: () => 'Create credential',
	openCredentialCreate: vi.fn()
}));

import warningCodes from '#lib/data/warning-codes.json';
import { actionFor } from '#lib/features/discovery/utils/warningActions.ts';
import type { WarningEntry } from '#lib/features/discovery/utils/warnings.ts';
import { brokenDocsLinks, DOCS_ORIGIN, type Occurrence } from './docs-link-check';

/**
 * Live validation of every docs page a warning row's action button opens.
 *
 * The URLs come from running the actions, not from reading source: each warning code gets a row,
 * its action runs with `window.open` recorded, and every `scanopy.net/docs` URL opened is fetched.
 * So a guide composed at runtime is covered, and a code added later is covered without editing
 * this test. See `docs-link-check.ts` for what counts as resolving.
 *
 * Hits the live site like `docs-links.test.ts`. Skip offline with `SCANOPY_SKIP_ONLINE_TESTS=1`.
 */

const run = {
	daemonId: 'daemon',
	siteId: 'site',
	scanId: 'scan',
	credentialOf: () => undefined
};

/** A row of one warning of `code`, carrying the ids an action reads, so every branch can run. */
function rowOf(code: string): WarningEntry {
	return {
		code,
		warnings: [
			{
				code,
				credential_id: 'credential',
				integration: 'Snmp',
				host_id: 'host',
				subnet_id: 'subnet',
				candidate_host_ids: ['host-a', 'host-b']
			}
		]
	} as unknown as WarningEntry;
}

/** Every docs URL the code's action opens, with the code it came from. */
function docsLinksOpenedBy(code: string): Occurrence[] {
	const opened: string[] = [];
	vi.stubGlobal('window', {
		location: { hash: '' },
		open: (url: string) => {
			opened.push(url);
			return null;
		}
	});
	actionFor(rowOf(code), run)?.run();
	vi.unstubAllGlobals();
	return opened
		.filter((url) => url.startsWith(`${DOCS_ORIGIN}/docs`))
		.map((url) => ({ url, where: `the ${code} warning's action` }));
}

describe.skipIf(!!process.env.SCANOPY_SKIP_ONLINE_TESTS)('warning action docs links (live)', () => {
	it(
		'every docs page a warning action opens resolves, with its #fragment anchor',
		{ timeout: 120_000 },
		async () => {
			const occurrences = warningCodes.flatMap((c) => docsLinksOpenedBy(c.id));
			// Guards against a vacuous pass: some actions open guides, so finding none means the
			// rows no longer reach them.
			expect(occurrences.length).toBeGreaterThan(0);

			const broken = await brokenDocsLinks(occurrences);
			if (broken.length > 0) {
				expect.fail(`Found ${broken.length} broken action docs link(s):\n\n${broken.join('\n\n')}`);
			}
		}
	);
});
