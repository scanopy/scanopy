/**
 * Finished layouts, kept by what they were computed from.
 *
 * A layout costs a measure pass that mounts every node to read its size, then two ELK passes. On
 * a site of a few thousand hosts that is several seconds of pipeline and a mount and teardown of
 * tens of thousands of DOM elements, and it repeats on every reload, every return to the tab and
 * every return to a collapse state already seen, although nothing it depends on has changed.
 *
 * This keeps the measured sizes and the layout engine's result under a key built from every
 * input of both, in memory for the session and in IndexedDB across reloads. A hit hands the
 * pipeline the same sizes and positions the measure pass and ELK would produce, so neither runs.
 *
 * A stored entry is trusted only as far as its key. What the key cannot see is card content that
 * is not on a node row (a freshness pill that appears as time passes, for one). The post-render
 * size check is what catches that: when it finds a rendered card disagreeing with its stored
 * size, the viewer deletes the entry and lays out for real.
 */

import { browser } from '$app/environment';
import { getLocale } from '$lib/paraglide/runtime';
import type { TopologyEdge, TopologyNode } from '../types/base';
import type { LayoutResult } from '../layout/engine';
import { affectsLayout, affectsLayoutWithinContainer } from '../layout/edge-classification';
import type { LayoutState, XY } from './types';

/**
 * Part of every key. Bump it when a change makes stored layouts wrong without changing any input
 * the key covers: ELK options, card markup or styling that alters a card's size.
 */
export const LAYOUT_STORE_VERSION = 1;

/** Entries kept, most recently used first. A fully expanded large view is about a megabyte. */
const MAX_ENTRIES = 8;

const DB_NAME = 'scanopy-topology-layouts';
const STORE_NAME = 'layouts';

export interface StoredLayout {
	/** The size of every node the layout was computed with, as the measure stage returned them. */
	sizes: Map<string, XY>;
	/** Collapsed and expanded container sizes known when the layout finished. */
	containerSizes: Map<string, { collapsed?: XY; expanded?: XY }>;
	/** What the layout engine returned. */
	result: LayoutResult;
}

interface StoredRecord {
	key: string;
	usedAt: number;
	layout: StoredLayout;
}

export interface LayoutKeyInputs {
	/** View, node ids and parents, inline content, hide state and the collapsed set. */
	structureKey: string;
	/** Whether ELK lays out with everything expanded and collapse is applied afterwards. */
	deferCollapse: boolean;
	hiddenEdgeTypes: string[];
	/** The edges handed to the layout engine. */
	edges: TopologyEdge[];
	/** The node rows handed to the layout engine. */
	nodes: TopologyNode[];
}

/** cyrb53: a 53-bit string hash. Not cryptographic; `crypto.subtle` is HTTPS-only. */
function hashString(text: string, seed = 0): string {
	let h1 = 0xdeadbeef ^ seed;
	let h2 = 0x41c6ce57 ^ seed;
	for (let i = 0; i < text.length; i++) {
		const code = text.charCodeAt(i);
		h1 = Math.imul(h1 ^ code, 2654435761);
		h2 = Math.imul(h2 ^ code, 1597334677);
	}
	h1 = Math.imul(h1 ^ (h1 >>> 16), 2246822507) ^ Math.imul(h2 ^ (h2 >>> 13), 3266489909);
	h2 = Math.imul(h2 ^ (h2 >>> 16), 2246822507) ^ Math.imul(h1 ^ (h1 >>> 13), 3266489909);
	return (4294967296 * (2097151 & h2) + (h1 >>> 0)).toString(36);
}

/**
 * The key a layout is stored under: one hash per input, so a miss can be traced to the part
 * that changed.
 *
 * - **Structure**: the pipeline's own key for "this needs laying out again".
 * - **Edges**: the endpoints of the edges that steer layout. The structure key only counts edges.
 *   Sorted, because the backend mints edge ids per request and does not order them.
 * - **Nodes**: the node rows themselves, which carry the header text and everything else the
 *   backend decides a card shows. Sorted by id for the same reason.
 * - **Environment**: locale and root font size, which change text widths.
 */
export function layoutStoreKey(inputs: LayoutKeyInputs): string {
	const edges = inputs.edges
		.filter((edge) => affectsLayout(edge) || affectsLayoutWithinContainer(edge))
		.map((edge) => `${edge.source}>${edge.target}:${edge.edge_type}`)
		.sort()
		.join(',');
	const nodes = [...inputs.nodes]
		.sort((a, b) => a.id.localeCompare(b.id))
		.map((node) => JSON.stringify(node))
		.join('\n');
	const rootFontSize = browser ? getComputedStyle(document.documentElement).fontSize : '';
	return [
		`v${LAYOUT_STORE_VERSION}`,
		hashString(inputs.structureKey),
		hashString(edges),
		hashString(nodes),
		hashString(
			`${inputs.deferCollapse}|${[...inputs.hiddenEdgeTypes].sort().join(',')}|${getLocale()}|${rootFontSize}`
		)
	].join('-');
}

// Session copy. Also the only copy where IndexedDB is unavailable (private windows, blocked
// site data), which is why every database call below fails soft.
const memory = new Map<string, StoredLayout>();

function remember(key: string, layout: StoredLayout): void {
	memory.delete(key);
	memory.set(key, layout);
	while (memory.size > MAX_ENTRIES) {
		const oldest = memory.keys().next().value;
		if (oldest === undefined) break;
		memory.delete(oldest);
	}
}

let dbPromise: Promise<IDBDatabase | null> | null = null;

function openDb(): Promise<IDBDatabase | null> {
	if (!browser || typeof indexedDB === 'undefined') return Promise.resolve(null);
	dbPromise ??= new Promise((resolve) => {
		try {
			const request = indexedDB.open(DB_NAME, 1);
			request.onupgradeneeded = () => {
				request.result.createObjectStore(STORE_NAME, { keyPath: 'key' });
			};
			request.onsuccess = () => resolve(request.result);
			request.onerror = () => resolve(null);
			request.onblocked = () => resolve(null);
		} catch {
			resolve(null);
		}
	});
	return dbPromise;
}

function requestDone<T>(request: IDBRequest<T>): Promise<T | undefined> {
	return new Promise((resolve) => {
		request.onsuccess = () => resolve(request.result);
		request.onerror = () => resolve(undefined);
	});
}

/** The layout stored under `key`, or undefined. Never throws. */
export async function readStoredLayout(key: string): Promise<StoredLayout | undefined> {
	const held = memory.get(key);
	if (held) {
		remember(key, held);
		return held;
	}
	const db = await openDb();
	if (!db) return undefined;
	try {
		const store = db.transaction(STORE_NAME, 'readwrite').objectStore(STORE_NAME);
		const record = await requestDone<StoredRecord | undefined>(store.get(key));
		if (!record) return undefined;
		store.put({ ...record, usedAt: Date.now() });
		remember(key, record.layout);
		return record.layout;
	} catch {
		return undefined;
	}
}

/** Keep `layout` under `key`, dropping the least recently used entries past the limit. */
export function writeStoredLayout(key: string, layout: StoredLayout): void {
	remember(key, layout);
	void openDb().then(async (db) => {
		if (!db) return;
		try {
			const store = db.transaction(STORE_NAME, 'readwrite').objectStore(STORE_NAME);
			store.put({ key, usedAt: Date.now(), layout } satisfies StoredRecord);
			const records = (await requestDone<StoredRecord[]>(store.getAll())) ?? [];
			records
				.sort((a, b) => b.usedAt - a.usedAt)
				.slice(MAX_ENTRIES)
				.forEach((record) => store.delete(record.key));
		} catch {
			// Storage is full or was taken away mid-write. The session copy still serves.
		}
	});
}

/**
 * Put a stored layout's sizes where the pipeline keeps measured ones, and return the size map the
 * measure stage would have returned. After this the run needs no measure pass, and the
 * post-render checks see every container size they look for.
 */
export function adoptStoredSizes(
	state: LayoutState,
	viewCacheKey: string,
	stored: StoredLayout
): Map<string, XY> {
	state.viewSizeCache.set(viewCacheKey, new Map(stored.sizes));
	for (const [id, sizes] of stored.containerSizes) {
		state.containerSizeCache.set(id, { ...state.containerSizeCache.get(id), ...sizes });
	}
	return new Map(stored.sizes);
}

/** The container sizes worth keeping with a layout: those of the nodes it was computed for. */
export function containerSizesFor(
	state: LayoutState,
	sizes: Map<string, XY>
): StoredLayout['containerSizes'] {
	const kept: StoredLayout['containerSizes'] = new Map();
	for (const id of sizes.keys()) {
		const entry = state.containerSizeCache.get(id);
		if (entry) kept.set(id, { ...entry });
	}
	return kept;
}

/** Forget the layout under `key`: its sizes disagreed with what rendered. */
export function deleteStoredLayout(key: string): void {
	memory.delete(key);
	void openDb().then((db) => {
		if (!db) return;
		try {
			db.transaction(STORE_NAME, 'readwrite').objectStore(STORE_NAME).delete(key);
		} catch {
			// Nothing to undo: the session copy is already gone.
		}
	});
}
