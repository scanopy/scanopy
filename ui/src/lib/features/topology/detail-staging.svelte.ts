/**
 * Card detail, mounted a little at a time ahead of the zoom where it shows.
 *
 * At the detail threshold every card on screen used to go from an empty box to its full markup in
 * one frame: about a quarter of a second for a hundred switch cards, in the middle of a zoom. The
 * same happened to each card entering the canvas while detail was on.
 *
 * Here a node asks for its detail when it mounts and gets it when the scheduler reaches it. The
 * scheduler runs from a zoom a little under the threshold (`PREWARM_ZOOM`) and spends a fixed
 * budget per frame, so the mounts are spread over the approach and no frame carries more than its
 * share. A node that has its detail below the threshold keeps it under a veil that looks like the
 * box (`.lod-veil` in `topology-viewer.css`); crossing the threshold lifts the veil, which is a
 * repaint and not a mount.
 *
 * Only graphs large enough to be simplified are staged. On a small one `active` is false and a
 * node draws in full at once, as it always has.
 *
 * One instance per viewer, handed to its nodes through context: two canvases on one page (the
 * dependency tutorial over the topology) must not share a queue or a decision.
 */

import { flushSync, getContext, setContext, untrack } from 'svelte';
import { SvelteSet } from 'svelte/reactivity';

/** Time a frame may spend mounting detail, in milliseconds. A frame at 120 Hz is 8.3 ms. */
export const STAGING_BUDGET_MS = 5;

/**
 * Grant waiting ids until the budget is spent. Always grants one, so a card whose mount alone
 * exceeds the budget cannot stall the queue.
 *
 * @returns how many were granted.
 */
export function grantWithinBudget(
	queue: Set<string>,
	grant: (id: string) => void,
	now: () => number,
	budgetMs: number
): number {
	const startedAt = now();
	let granted = 0;
	for (const id of queue) {
		queue.delete(id);
		grant(id);
		granted++;
		if (now() - startedAt >= budgetMs) break;
	}
	return granted;
}

export interface DetailStaging {
	/**
	 * A node has mounted and wants its detail. Call `release` when it unmounts.
	 *
	 * @param element the node's own element, so nodes on screen can be served first. SvelteFlow
	 * keeps the endpoints of every visible link mounted, so most mounted nodes are off screen:
	 * 96 mounted for 5 visible on a 252-switch view at the detail zoom.
	 */
	request(id: string, element?: Element): void;
	release(id: string): void;
	/**
	 * Whether this node mounts its full markup.
	 *
	 * @param simplified the graph-wide decision (`detailSimplified`), which is all that decides
	 * it on a graph that is not staged.
	 */
	hasDetail(id: string, simplified: boolean): boolean;
	/**
	 * Set by the viewer as the graph and the zoom change.
	 *
	 * @param active the graph is one that gets simplified, and nothing needs every card in full
	 * right now (a measure pass, an export).
	 * @param prewarm the zoom is at or past the point where detail starts being prepared.
	 */
	set(active: boolean, prewarm: boolean): void;
}

export function createDetailStaging(): DetailStaging {
	/** Nodes that have their detail. Reactive per id: a grant re-renders that node only. */
	const ready = new SvelteSet<string>();
	/** Mounted nodes still waiting, in the order they mounted. */
	const waiting = new Set<string>();
	/** The waiting nodes that are on screen. Served before the rest. */
	const waitingOnScreen = new Set<string>();
	const onScreen = new Set<string>();
	const elementOf = new Map<string, Element>();
	const idOf = new WeakMap<Element, string>();
	const stage = $state({ active: false });
	let prewarming = false;
	let frame: number | null = null;

	const observer =
		typeof IntersectionObserver === 'undefined'
			? null
			: new IntersectionObserver((entries) => {
					for (const entry of entries) {
						const id = idOf.get(entry.target);
						if (id === undefined) continue;
						if (entry.isIntersecting) {
							onScreen.add(id);
							if (waiting.has(id)) waitingOnScreen.add(id);
						} else {
							onScreen.delete(id);
							waitingOnScreen.delete(id);
						}
					}
					schedule();
				});

	function wait(id: string): void {
		waiting.add(id);
		if (onScreen.has(id)) waitingOnScreen.add(id);
	}

	function runFrame(): void {
		frame = null;
		if (!prewarming || waiting.size === 0) return;
		const startedAt = performance.now();
		const grant = (id: string) => {
			waiting.delete(id);
			waitingOnScreen.delete(id);
			ready.add(id);
			// Mount it now, so the time it takes is counted against this frame's budget.
			flushSync();
		};
		grantWithinBudget(waitingOnScreen, grant, () => performance.now(), STAGING_BUDGET_MS);
		const left = STAGING_BUDGET_MS - (performance.now() - startedAt);
		if (left > 0 && waitingOnScreen.size === 0) {
			grantWithinBudget(waiting, grant, () => performance.now(), left);
		}
		schedule();
	}

	function schedule(): void {
		if (frame !== null || !prewarming || waiting.size === 0) return;
		if (typeof requestAnimationFrame === 'undefined') return;
		frame = requestAnimationFrame(runFrame);
	}

	return {
		request(id, element) {
			if (element && elementOf.get(id) !== element) {
				const previous = elementOf.get(id);
				if (previous) observer?.unobserve(previous);
				elementOf.set(id, element);
				idOf.set(element, id);
				observer?.observe(element);
			}
			// Untracked: a node calls this from an effect, and reading the reactive set there would
			// make being granted re-run the effect, whose cleanup releases the grant.
			if (untrack(() => ready.has(id))) return;
			wait(id);
			schedule();
		},
		release(id) {
			const element = elementOf.get(id);
			if (element) observer?.unobserve(element);
			elementOf.delete(id);
			onScreen.delete(id);
			waiting.delete(id);
			waitingOnScreen.delete(id);
			ready.delete(id);
		},
		hasDetail(id, simplified) {
			return stage.active ? ready.has(id) : !simplified;
		},
		set(active, prewarm) {
			stage.active = active;
			const wasPrewarming = prewarming;
			prewarming = active && prewarm;
			if (prewarming) {
				schedule();
			} else if (wasPrewarming || ready.size > 0) {
				// Back to plain boxes: every node that had its detail waits for it again.
				for (const id of ready) wait(id);
				ready.clear();
			}
		}
	};
}

const CONTEXT_KEY = Symbol('topology-detail-staging');

/** Called by the viewer, once, before its nodes mount. */
export function provideDetailStaging(): DetailStaging {
	const staging = createDetailStaging();
	setContext(CONTEXT_KEY, staging);
	return staging;
}

/** The staging of the viewer this node is drawn in, or undefined outside one (a test, a preview). */
export function useDetailStaging(): DetailStaging | undefined {
	return getContext<DetailStaging | undefined>(CONTEXT_KEY);
}
