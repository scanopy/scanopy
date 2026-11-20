/**
 * The one writer of the main page's hash and of the query params the app owns.
 *
 * Each group of params belongs to a scope, and each scope is shown only where it applies: the
 * modal params on every tab, the topology params on the topology tab, the global search while the
 * palette is open, a list's view on the list on screen. An owner never writes the URL itself. It
 * registers what its params are and calls `syncUrl`, which rebuilds the whole set from every owner
 * at once. Two writers each editing "their" keys left the other's stale keys behind: topology
 * params on `#hosts`, a closed list's sort on the next tab.
 *
 * Where a scope mirrors an API request, its params carry the API's names and values, so the same
 * query string sent to the endpoint returns what the page shows.
 */

export type UrlScope = 'modal' | 'topology' | 'search' | 'list';

interface ScopeRule {
	/** The scope's fixed keys. A list adds the keys its fields declare, through `UrlOwner.keys`. */
	keys: readonly string[];
	/** Shown only on this tab. */
	tab?: string;
	/** While shown, these scopes are left out of the URL. */
	hides?: readonly UrlScope[];
	/**
	 * A key the scope always writes while shown. A loaded URL without it doesn't carry the scope,
	 * and the scope's other keys there belong to the list.
	 */
	marker?: string;
}

export const URL_SCOPES: Record<UrlScope, ScopeRule> = {
	// `integration`, `hosts` and `sites` are the modal's prefill params (`PREFILL_PARAMS`).
	modal: { keys: ['modal', 'id', 'tab', 'subEntityId', 'integration', 'hosts', 'sites'] },
	topology: { keys: ['topologyId', 'view'], tab: 'topology' },
	// The palette's params are the search endpoint's, and a list may use the same names
	// (`tag_ids`), so the list steps out while the palette is open and comes back on close. `q` is
	// always written while it's open, even empty, so a loaded URL says which of the two it carries.
	search: { keys: ['q', 'tag_ids'], hides: ['list'], marker: 'q' },
	list: { keys: ['page'] }
};

/**
 * Params the shell reads once and clears itself (an OAuth error, a billing redirect). Never
 * stripped, and never handed to a list as one of its own.
 */
export const SHELL_KEYS: readonly string[] = ['error', 'login_method', 'token', 'billing_flow'];

const STATIC_KEYS = new Set(Object.values(URL_SCOPES).flatMap((scope) => scope.keys));

export interface UrlOwner {
	scope: UrlScope;
	/** False while the owner isn't the one on screen, such as a list on a hidden tab. */
	isActive: () => boolean;
	/** The scope's params at their current values. */
	params: () => URLSearchParams;
	/** Keys this owner may write beyond its scope's fixed ones. */
	keys?: () => readonly string[];
	/** Apply the params of a history entry the user stepped back or forward to. */
	restore?: (params: URLSearchParams) => void;
}

function tabOf(url: URL): string {
	return url.hash.replace(/^#/, '');
}

function ownedKeys(owner: UrlOwner): readonly string[] {
	return [...URL_SCOPES[owner.scope].keys, ...(owner.keys?.() ?? [])];
}

/** The owners whose params the URL shows on `tab`. */
function shownOwners(owners: Iterable<UrlOwner>, tab: string): UrlOwner[] {
	const active = [...owners].filter((owner) => {
		const scopeTab = URL_SCOPES[owner.scope].tab;
		return (scopeTab === undefined || scopeTab === tab) && owner.isActive();
	});
	const hidden = new Set(active.flatMap((owner) => URL_SCOPES[owner.scope].hides ?? []));
	return active.filter((owner) => !hidden.has(owner.scope));
}

/**
 * The URL with every managed param rebuilt from the owners on screen.
 *
 * `tab`, when given, becomes the hash. Otherwise the hash stays as it is, since a caller may have
 * just set `location.hash` to move tabs and the main page hasn't heard about it yet. `stale` names
 * keys to strip that no owner declares any more, such as a link's param for a removed field.
 */
export function buildUrl(
	current: URL,
	owners: Iterable<UrlOwner>,
	tab: string | null,
	stale: Iterable<string> = []
): URL {
	const url = new URL(current.href);
	if (tab !== null) url.hash = tab;

	const all = [...owners];
	const managed = new Set([...STATIC_KEYS, ...stale, ...all.flatMap(ownedKeys)]);
	for (const key of managed) url.searchParams.delete(key);

	for (const owner of shownOwners(all, tabOf(url))) {
		const allowed = new Set(ownedKeys(owner));
		for (const [key, value] of owner.params()) {
			if (allowed.has(key)) url.searchParams.append(key, value);
		}
	}
	return url;
}

const owners = new Set<UrlOwner>();

/** Register an owner. Returns the unregister function, for use as an effect teardown. */
export function registerUrlOwner(owner: UrlOwner): () => void {
	owners.add(owner);
	return () => owners.delete(owner);
}

let attached = false;
let initialParams: URLSearchParams | null = null;
/** Keys handed out at hydration. Stripped on each write unless an owner writes them again. */
const handedOut = new Set<string>();
let pendingTab: string | null = null;
let pendingPush = false;
let flushQueued = false;

/** The keys in `source` that belong to `scope`. A list's are every key no other scope claims. */
export function scopeKeysIn(scope: UrlScope, source: URLSearchParams): string[] {
	const keys = [...new Set(source.keys())];
	const carried = (other: UrlScope) => {
		const marker = URL_SCOPES[other].marker;
		return marker === undefined || source.has(marker);
	};
	if (scope !== 'list') {
		return carried(scope) ? keys.filter((key) => URL_SCOPES[scope].keys.includes(key)) : [];
	}
	const claimed = new Set([
		...SHELL_KEYS,
		...(Object.keys(URL_SCOPES) as UrlScope[])
			.filter((other) => other !== 'list' && carried(other))
			.flatMap((other) => URL_SCOPES[other].keys)
	]);
	return keys.filter((key) => !claimed.has(key));
}

/**
 * A scope's params from the URL the page loaded with, handed out once.
 *
 * Owners hydrate from this rather than from `location`, because an earlier owner's sync can strip
 * the keys of an owner that hasn't registered yet. Once taken they're gone, so a list that mounts
 * again later doesn't re-apply a link the user has since moved on from. Before the main page
 * attaches, it reads the live URL.
 */
export function takeInitialParams(scope: UrlScope): URLSearchParams {
	const source =
		initialParams ??
		new URLSearchParams(typeof window === 'undefined' ? '' : window.location.search);
	const taken = new URLSearchParams();
	for (const key of scopeKeysIn(scope, source)) {
		for (const value of source.getAll(key)) taken.append(key, value);
		initialParams?.delete(key);
		handedOut.add(key);
	}
	return taken;
}

function flush() {
	flushQueued = false;
	if (!attached) return;

	// Only once the main page has switched tabs: a sync before then (a modal closing as the link
	// opens) still has the old tab's list on screen, and it would take the link.
	if (pendingTab !== null) applyPendingOpen(pendingTab);
	const url = buildUrl(new URL(window.location.href), owners, pendingTab, handedOut);
	const push = pendingPush;
	pendingTab = null;
	pendingPush = false;

	if (push) {
		window.history.pushState({}, '', url.href);
	} else if (url.href !== window.location.href) {
		window.history.replaceState(window.history.state, '', url.href);
	}
}

/**
 * Rebuild the URL from the owners, once per microtask however many owners asked.
 *
 * `push` adds a history entry instead of replacing the current one, for a change the user should be
 * able to step back from (switching topology view).
 */
export function syncUrl(opts?: { push?: boolean }): void {
	if (opts?.push) pendingPush = true;
	if (flushQueued) return;
	flushQueued = true;
	queueMicrotask(flush);
}

/** The main page's tab changed. It becomes the hash, and decides which scopes show. */
export function setActiveTab(tab: string): void {
	pendingTab = tab;
	syncUrl();
}

function handlePopState() {
	const params = new URLSearchParams(window.location.search);
	for (const owner of shownOwners(owners, tabOf(new URL(window.location.href)))) {
		owner.restore?.(params);
	}
}

/** A tab an in-app link opened, with the list view it carries, until that tab's list takes it. */
let pendingOpen: { tab: string; params: URLSearchParams } | null = null;

/**
 * Open `tab` with `params` as its list's view, the same as following a link that carries them:
 * a nudge's "Review hosts" opens `?sources=Inferred#hosts`, and Back returns to where it was.
 *
 * Every list tab stays mounted, so the list on `tab` takes the params as the tab comes on screen
 * (`flush`). A list that isn't mounted yet takes them as it hydrates, like a pasted link.
 */
export function openTab(tab: string, params: URLSearchParams): void {
	pendingOpen = { tab, params };
	if (!initialParams) initialParams = new URLSearchParams();
	for (const key of scopeKeysIn('list', initialParams)) initialParams.delete(key);
	for (const [key, value] of params) {
		initialParams.append(key, value);
		handedOut.delete(key);
	}
	// Already on the tab: the main page hears no hash change, so hand the view over now.
	if (tabOf(new URL(window.location.href)) === tab) {
		setActiveTab(tab);
		return;
	}
	// Moving the hash adds the history entry; the flush that follows writes the params into it.
	window.location.hash = tab;
}

/** Hand a pending link's view to the list now on its tab. */
function applyPendingOpen(tab: string) {
	if (!pendingOpen || pendingOpen.tab !== tab) return;
	const list = shownOwners(owners, tab).find((owner) => owner.scope === 'list' && owner.restore);
	if (!list) return;
	list.restore?.(pendingOpen.params);
	for (const key of pendingOpen.params.keys()) initialParams?.delete(key);
	pendingOpen = null;
}

/**
 * Start managing the URL. The main page calls this as it initialises, before its tabs mount, and
 * calls the returned function as it is destroyed. Other routes are never written to.
 */
export function attachUrlState(): () => void {
	initialParams = new URLSearchParams(window.location.search);
	handedOut.clear();
	attached = true;
	window.addEventListener('popstate', handlePopState);
	return () => {
		attached = false;
		initialParams = null;
		window.removeEventListener('popstate', handlePopState);
	};
}
