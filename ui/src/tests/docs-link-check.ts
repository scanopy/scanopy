/**
 * Live checks for `scanopy.net/docs` links, shared by the tests that collect them.
 *
 * Each unique URL is fetched and must return 2xx after redirects. If a URL carries a `#fragment`,
 * the final page must contain that anchor (`id="fragment"`): a valid page with a stale anchor is
 * still a broken link. Requests retry with backoff before a URL is declared broken, so a transient
 * network blip doesn't red the suite.
 */

export const DOCS_ORIGIN = 'https://scanopy.net';

const ATTEMPTS = 3;
const BACKOFF_MS = [500, 1000, 2000];
const REQUEST_TIMEOUT_MS = 10_000;
const CONCURRENCY = 6;
// HEAD is unreliable on some hosts; fall back to GET on "method not allowed"-style responses.
const METHOD_FALLBACK_STATUS = new Set([403, 405, 501]);
const USER_AGENT = 'scanopy-docs-link-check (vitest)';

/** Where a URL was found, for readable failure output. */
export interface Occurrence {
	url: string;
	/** Repo-relative file, or a description of the source. */
	where: string;
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

/**
 * Fetch every unique URL among `occurrences` and return one failure line per broken link, naming
 * where each was found. Empty when every link resolves.
 */
export async function brokenDocsLinks(occurrences: Occurrence[]): Promise<string[]> {
	const byUrl = new Map<string, Occurrence[]>();
	for (const occ of occurrences) {
		const list = byUrl.get(occ.url);
		if (list) list.push(occ);
		else byUrl.set(occ.url, [occ]);
	}

	const results = await mapPool([...byUrl.keys()], CONCURRENCY, async (url) => {
		try {
			await withRetry(() => checkOnce(url));
			return null;
		} catch (err) {
			const locs = (byUrl.get(url) ?? []).map((o) => o.where).join(', ');
			return `  ${url}\n    reason: ${(err as Error).message}\n    used at: ${locs}`;
		}
	});
	return results.filter((r): r is string => r !== null);
}
