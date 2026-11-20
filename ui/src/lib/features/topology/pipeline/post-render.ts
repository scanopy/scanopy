import type { LayoutGraph } from '../layout/layout-graph';
import type { XY } from './types';

/** Height difference, in px, below which a card is considered unchanged. */
const SIZE_DRIFT_TOLERANCE_PX = 2;

/**
 * Correct cached element sizes against what the mounted cards actually measure.
 *
 * Nodes are built carrying `measured` and `handles` so SvelteFlow can cull them before they have
 * ever rendered. The cost of that is `NodeWrapper` treating them as initialised and never
 * attaching a ResizeObserver — so nothing else notices if a card's real height drifts from the
 * cached value. A pipeline run re-measures after any topology change, and port expansion
 * re-measures explicitly, but a change with no pipeline run behind it (a font finally loading, a
 * theme switch) would otherwise leave a wrong height cached indefinitely, laying the graph out
 * around a card size that no longer exists.
 *
 * **Containers are excluded.** `viewSizeCache` holds a container's *natural* size, measured with
 * width and height unconstrained, while the container renders at the expanded size ELK computed
 * for it — two different numbers by design. Comparing them reports drift on every container on
 * every pass, and when the caller answers drift with a re-layout that is an endless loop: the
 * seeded reproduction turned over two corrective re-layouts every five seconds indefinitely, which
 * presents exactly as the hang this work set out to fix.
 *
 * `alreadyCorrected` bounds it further, to one correction per node per structure. Even a card that
 * genuinely cannot settle — one whose rendered height never matches what was measured for it —
 * costs one re-layout rather than a permanent cycle. The caller clears the set when the structure
 * changes.
 *
 * Bounded by the mounted set: a few hundred nodes with culling on, not the whole graph.
 *
 * @returns How many nodes drifted for the first time under this structure.
 */
export function reconcileMeasuredSizes(
	containerElement: HTMLDivElement,
	viewSizeCache: Map<string, XY>,
	layoutGraph: LayoutGraph,
	alreadyCorrected: Set<string>
): number {
	let newlyCorrected = 0;

	for (const el of containerElement.querySelectorAll('.svelte-flow__node')) {
		const htmlEl = el as HTMLElement;
		const id = htmlEl.dataset.id;
		if (!id || layoutGraph.containers.has(id)) continue;

		const cached = viewSizeCache.get(id);
		if (!cached) continue;

		// A card drawn as a box has no content, so its height says nothing about the card. Until
		// SvelteFlow has measured it, the box is 2px of border, and recording that as the card's
		// height laid a container out around a card a fiftieth of its rendered size.
		if (htmlEl.querySelector('[data-lod-simplified]')) continue;

		const height = htmlEl.offsetHeight;
		// A zero height is a node mid-mount, not a card that shrank to nothing.
		if (!height) continue;

		if (Math.abs(height - cached.y) > SIZE_DRIFT_TOLERANCE_PX) {
			viewSizeCache.set(id, { x: cached.x, y: height });
			if (!alreadyCorrected.has(id)) {
				alreadyCorrected.add(id);
				newlyCorrected += 1;
			}
		}
	}

	return newlyCorrected;
}

/**
 * Cache collapsed container sizes after render.
 * Unconstrain width to read natural content size, then restore.
 * Synchronous — no paint between write-read-restore.
 *
 * @returns Number of new collapsed cache entries added.
 */
export function cacheCollapsedSizes(
	containerElement: HTMLDivElement,
	layoutGraph: LayoutGraph,
	collapsed: Set<string>,
	containerSizeCache: Map<string, { collapsed?: XY; expanded?: XY }>
): number {
	let newCollapsedCacheEntries = 0;

	const saved = new Map<HTMLElement, { w: string; h: string }>();
	const nodeEls = containerElement.querySelectorAll('.svelte-flow__node');

	for (const el of nodeEls) {
		const htmlEl = el as HTMLElement;
		const id = htmlEl.dataset.id;
		if (id && layoutGraph.containers.has(id) && collapsed.has(id)) {
			if (!containerSizeCache.get(id)?.collapsed) {
				saved.set(htmlEl, { w: htmlEl.style.width, h: htmlEl.style.height });
				htmlEl.style.width = 'auto';
				htmlEl.style.height = 'auto';
				const inner = htmlEl.querySelector(':scope > .relative') as HTMLElement;
				if (inner) {
					saved.set(inner, { w: inner.style.width, h: inner.style.height });
					inner.style.width = 'auto';
					inner.style.height = 'auto';
				}
			}
		}
	}

	if (saved.size > 0) {
		for (const el of nodeEls) {
			const htmlEl = el as HTMLElement;
			const id = htmlEl.dataset.id;
			if (id && saved.has(htmlEl)) {
				const w = htmlEl.offsetWidth || 250;
				const h = htmlEl.offsetHeight || 100;
				const entry = containerSizeCache.get(id) ?? {};
				entry.collapsed = { x: w, y: h };
				containerSizeCache.set(id, entry);
				newCollapsedCacheEntries++;
			}
		}
		for (const [el, { w, h }] of saved) {
			el.style.width = w;
			el.style.height = h;
		}
	}

	return newCollapsedCacheEntries;
}
