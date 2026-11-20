import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import { brokenDocsLinks, DOCS_ORIGIN, type Occurrence } from './docs-link-check';

/**
 * Live validation of every outbound `scanopy.net/docs` link rendered by the app.
 *
 * The link set is derived from source at test time (a broad, wrapper-agnostic scan of
 * `ui/src`, `messages/en.json` and the metadata fixtures in `ui/src/lib/data`), NOT a
 * hand-maintained array — so a newly added docs link is covered automatically and the test
 * can't silently drift out of date. Links reach the UI through several paths (the shared `DocsHint` component, a plain `<a>` in `Tag`, `window.open`
 * in the support cards, and `MessageWithLinks` link maps), so we scan for the URL
 * itself rather than any one component. Root-relative paths are covered too: literal
 * `docsUrl('/docs/…')` calls, and every `docs_path` the backend publishes in the metadata
 * fixtures (`ui/src/lib/data/*.json`). Links that warning actions open are checked by
 * `warning-action-docs-links.test.ts`, which runs the actions themselves.
 *
 * See `docs-link-check.ts` for what counts as resolving.
 *
 * Network note: this test hits the live site. CI (`.github/workflows/ui-ci.yml`) does not run the
 * vitest suite — it runs only locally via `npm test` — so the retry logic (not a CI env gate) is
 * the flakiness mitigation. Offline developers can skip it with `SCANOPY_SKIP_ONLINE_TESTS=1`.
 */

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SRC_DIR = path.resolve(__dirname, '..'); // ui/src
const REPO_ROOT = path.resolve(__dirname, '../../..');
const MESSAGES_FILE = path.resolve(REPO_ROOT, 'messages/en.json');
const DATA_DIR = path.resolve(SRC_DIR, 'lib/data');

// Matches a docs URL up to the first quote / whitespace / bracket / backtick that would end it.
const DOCS_URL_RE = /https?:\/\/scanopy\.net\/docs[^\s"'`)<>\\]*/g;
// A `docsUrl('/docs/…')` call with a literal path, which `docsUrl` composes onto the docs origin.
const DOCS_PATH_CALL_RE = /docsUrl\(\s*['"`](\/docs[^'"`\s]*)['"`]\s*\)/g;
// A `docs_path` the backend publishes in a metadata fixture.
const DOCS_PATH_FIELD_RE = /"docs_path":\s*"(\/docs[^"]*)"/g;
// `CredentialForm` links each credential type's guide at this section too.
const CREDENTIAL_GUIDE_SECTION = '#what-gets-discovered';

function findFilesRecursively(dir: string, extensions: string[]): string[] {
	const files: string[] = [];
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const fullPath = path.join(dir, entry.name);
		if (entry.isDirectory() && entry.name !== 'node_modules' && entry.name !== 'tests') {
			files.push(...findFilesRecursively(fullPath, extensions));
		} else if (entry.isFile() && extensions.some((ext) => entry.name.endsWith(ext))) {
			files.push(fullPath);
		}
	}
	return files;
}

function collectDocsLinks(): Occurrence[] {
	const files = findFilesRecursively(SRC_DIR, ['.svelte', '.ts'])
		// Exclude generated OpenAPI type declarations — their docs links are backend-owned.
		.filter((f) => !f.endsWith('.d.ts'))
		// The fixtures are scanned for their `docs_path` fields below.
		.filter((f) => !f.startsWith(DATA_DIR));
	files.push(MESSAGES_FILE);
	// Backend metadata carries docs links too (`ScanFailureFix` docs_url), shipped as UI fixtures.
	files.push(...findFilesRecursively(DATA_DIR, ['.json']));

	const occurrences: Occurrence[] = [];
	for (const file of files) {
		const content = fs.readFileSync(file, 'utf8');
		const where = (index: number | undefined) =>
			`${path.relative(REPO_ROOT, file)}:${content.slice(0, index).split('\n').length}`;
		for (const match of content.matchAll(DOCS_URL_RE)) {
			occurrences.push({ url: match[0], where: where(match.index) });
		}
		for (const match of content.matchAll(DOCS_PATH_CALL_RE)) {
			occurrences.push({ url: `${DOCS_ORIGIN}${match[1]}`, where: where(match.index) });
		}
	}

	for (const file of fs.readdirSync(DATA_DIR).filter((f) => f.endsWith('.json'))) {
		const content = fs.readFileSync(path.join(DATA_DIR, file), 'utf8');
		const where = path.relative(REPO_ROOT, path.join(DATA_DIR, file));
		for (const match of content.matchAll(DOCS_PATH_FIELD_RE)) {
			occurrences.push({ url: `${DOCS_ORIGIN}${match[1]}`, where });
			if (file === 'credential-types.json') {
				occurrences.push({ url: `${DOCS_ORIGIN}${match[1]}${CREDENTIAL_GUIDE_SECTION}`, where });
			}
		}
	}
	return occurrences;
}

describe.skipIf(!!process.env.SCANOPY_SKIP_ONLINE_TESTS)('docs links (live)', () => {
	it(
		'every scanopy.net/docs link resolves (2xx) and any #fragment anchor exists',
		{ timeout: 120_000 },
		async () => {
			const occurrences = collectDocsLinks();
			// Guards the source-derived approach: if the scanner ever finds nothing, the test would
			// pass vacuously — fail loudly instead so a regressed scan is visible.
			expect(occurrences.length).toBeGreaterThan(0);

			const broken = await brokenDocsLinks(occurrences);
			if (broken.length > 0) {
				const checked = new Set(occurrences.map((o) => o.url)).size;
				expect.fail(
					`Found ${broken.length} broken docs link(s) (checked ${checked}):\n\n${broken.join('\n\n')}`
				);
			}
		}
	);
});
