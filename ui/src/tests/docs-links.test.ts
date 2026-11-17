import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

/**
 * Live validation of every outbound `scanopy.net/docs` link rendered by the app.
 *
 * The link set is derived from source at test time (a broad, wrapper-agnostic scan of
 * `ui/src` + `messages/en.json`), NOT a hand-maintained array — so a newly added docs link
 * is covered automatically and the test can't silently drift out of date. Links reach the UI
 * through several paths (the shared `DocsHint` component, a plain `<a>` in `Tag`, `window.open`
 * in the support cards, and raw `<a>` anchors embedded in i18n strings), so we scan for the URL
 * itself rather than any one component. Root-relative paths are covered too: literal
 * `docsUrl('/docs/…')` calls, and every warning code's `docs_path` in `warning-codes.json`.
 *
 * Each unique URL is fetched live and must return 2xx. If a URL carries a `#fragment`, the page
 * body must actually contain that anchor (`id="fragment"`) — a valid page with a stale anchor is
 * still a broken link. Requests retry with backoff before a URL is declared broken, so a transient
 * network blip doesn't red the suite.
 *
 * Network note: this test hits the live site. CI (`.github/workflows/ui-ci.yml`) does not run the
 * vitest suite — it runs only locally via `npm test` — so the retry logic (not a CI env gate) is
 * the flakiness mitigation. Offline developers can skip it with `SCANOPY_SKIP_ONLINE_TESTS=1`.
 */

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SRC_DIR = path.resolve(__dirname, '..'); // ui/src
const REPO_ROOT = path.resolve(__dirname, '../../..');
const MESSAGES_FILE = path.resolve(REPO_ROOT, 'messages/en.json');

const WARNING_CODES_FILE = path.resolve(SRC_DIR, 'lib/data/warning-codes.json');
const DOCS_ORIGIN = 'https://scanopy.net';

// Matches a docs URL up to the first quote / whitespace / bracket / backtick that would end it.
const DOCS_URL_RE = /https?:\/\/scanopy\.net\/docs[^\s"'`)<>\\]*/g;
// A `docsUrl('/docs/…')` call with a literal path, which `docsUrl` composes onto the docs origin.
const DOCS_PATH_CALL_RE = /docsUrl\(\s*['"`](\/docs[^'"`\s]*)['"`]\s*\)/g;

const ATTEMPTS = 3;
const BACKOFF_MS = [500, 1000, 2000];
const REQUEST_TIMEOUT_MS = 10_000;
const CONCURRENCY = 6;
// HEAD is unreliable on some hosts; fall back to GET on "method not allowed"-style responses.
const METHOD_FALLBACK_STATUS = new Set([403, 405, 501]);
const USER_AGENT = 'scanopy-docs-link-check (vitest)';

interface Occurrence {
	url: string;
	file: string; // repo-relative, for readable failure output
	line: number;
}

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
		.filter((f) => !f.endsWith('.d.ts'));
	files.push(MESSAGES_FILE);

	const occurrences: Occurrence[] = [];
	for (const file of files) {
		const content = fs.readFileSync(file, 'utf8');
		const lineOf = (index: number | undefined) => content.slice(0, index).split('\n').length;
		for (const match of content.matchAll(DOCS_URL_RE)) {
			occurrences.push({
				url: match[0],
				file: path.relative(REPO_ROOT, file),
				line: lineOf(match.index)
			});
		}
		for (const match of content.matchAll(DOCS_PATH_CALL_RE)) {
			occurrences.push({
				url: `${DOCS_ORIGIN}${match[1]}`,
				file: path.relative(REPO_ROOT, file),
				line: lineOf(match.index)
			});
		}
	}

	// Warning codes carry their docs page as a root-relative path the UI composes.
	const warningCodes: { id: string; metadata?: { docs_path?: string | null } }[] = JSON.parse(
		fs.readFileSync(WARNING_CODES_FILE, 'utf8')
	);
	for (const code of warningCodes) {
		const docsPath = code.metadata?.docs_path;
		if (docsPath) {
			occurrences.push({
				url: `${DOCS_ORIGIN}${docsPath}`,
				file: `${path.relative(REPO_ROOT, WARNING_CODES_FILE)} (${code.id})`,
				line: 0
			});
		}
	}
	return occurrences;
}

function splitFragment(url: string): { base: string; fragment: string | null } {
	const hashIdx = url.indexOf('#');
	if (hashIdx < 0) return { base: url, fragment: null };
	return { base: url.slice(0, hashIdx), fragment: url.slice(hashIdx + 1) };
}

function anchorPresent(html: string, fragment: string): boolean {
	const frag = decodeURIComponent(fragment).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	// Statically-rendered docs headings emit `id="fragment"` (or `name="fragment"`).
	return new RegExp(`(?:id|name)\\s*=\\s*["']${frag}["']`).test(html);
}

async function fetchWithTimeout(url: string, method: 'HEAD' | 'GET'): Promise<Response> {
	const controller = new AbortController();
	const timer = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
	try {
		return await fetch(url, {
			method,
			redirect: 'follow',
			signal: controller.signal,
			headers: { 'User-Agent': USER_AGENT }
		});
	} finally {
		clearTimeout(timer);
	}
}

/** One validation attempt. Throws with a human-readable reason if the link is broken. */
async function checkOnce(url: string): Promise<void> {
	const { base, fragment } = splitFragment(url);
	// A fragment check needs the body, so it must GET; otherwise prefer HEAD.
	let res = await fetchWithTimeout(base, fragment ? 'GET' : 'HEAD');
	if (!res.ok && !fragment && METHOD_FALLBACK_STATUS.has(res.status)) {
		res = await fetchWithTimeout(base, 'GET');
	}
	if (!res.ok) throw new Error(`status ${res.status}`);
	if (fragment) {
		const html = await res.text();
		if (!anchorPresent(html, fragment)) throw new Error(`missing anchor #${fragment}`);
	}
}

async function withRetry(fn: () => Promise<void>): Promise<void> {
	let lastErr: unknown;
	for (let i = 0; i < ATTEMPTS; i++) {
		try {
			return await fn();
		} catch (err) {
			lastErr = err;
			if (i < ATTEMPTS - 1) {
				await new Promise((resolve) => setTimeout(resolve, BACKOFF_MS[i] ?? 2000));
			}
		}
	}
	throw lastErr;
}

async function mapPool<T, R>(items: T[], limit: number, fn: (item: T) => Promise<R>): Promise<R[]> {
	const results = new Array<R>(items.length);
	let next = 0;
	const workers = Array.from({ length: Math.min(limit, items.length) }, async () => {
		while (true) {
			const i = next++;
			if (i >= items.length) break;
			results[i] = await fn(items[i]);
		}
	});
	await Promise.all(workers);
	return results;
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

			const byUrl = new Map<string, Occurrence[]>();
			for (const occ of occurrences) {
				const list = byUrl.get(occ.url);
				if (list) list.push(occ);
				else byUrl.set(occ.url, [occ]);
			}

			const urls = [...byUrl.keys()];
			const results = await mapPool(urls, CONCURRENCY, async (url) => {
				try {
					await withRetry(() => checkOnce(url));
					return { url, ok: true as const };
				} catch (err) {
					return { url, ok: false as const, reason: (err as Error).message };
				}
			});

			const broken = results.filter((r) => !r.ok);
			if (broken.length > 0) {
				const lines = broken.map((b) => {
					const locs = (byUrl.get(b.url) ?? []).map((o) => `${o.file}:${o.line}`).join(', ');
					return `  ${b.url}\n    reason: ${b.reason}\n    used at: ${locs}`;
				});
				expect.fail(
					`Found ${broken.length} broken docs link(s) (checked ${urls.length}):\n\n${lines.join('\n\n')}`
				);
			}
		}
	);
});
