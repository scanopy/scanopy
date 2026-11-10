/**
 * When SvelteFlow may cull off-screen nodes and edges.
 *
 * At a few hundred hosts the L2 view mounts ~1250 nodes and >20,000 DOM
 * elements, and panning spends most frames over the 50ms jank threshold.
 * Viewport culling is the direct fix, but it cannot simply be switched on:
 *
 *  - **The measure pass needs every node in the DOM.** It mounts all nodes and
 *    reads their heights; culled nodes never mount, so it would time out and
 *    hand ELK fallback sizes.
 *  - **Export rasterises the whole flow element** (`html-to-image`), so a
 *    culled graph exports cropped.
 *
 * Note this helps *interaction*, not the cold load: SvelteFlow force-renders a
 * node until its `handleBounds` exist, and those only appear once a node has
 * mounted and been measured (`@xyflow/system`: `forceInitialRender =
 * !node.internals.handleBounds`). Since the pipeline rebuilds node objects
 * without a `measured` field on every run, everything renders at least once
 * regardless. Panning afterwards is where culling pays.
 */

/**
 * Exported so a diagnostic can report the gate's inputs *and* the line they were
 * measured against — "412 nodes, culling on" is only actionable next to the
 * threshold that made it so.
 */
export const CULLING_THRESHOLD_ELEMENTS = 150;

/**
 * Escape hatch for tooling that reads the graph out of the DOM.
 *
 * The layout-quality eval extracts node positions by querying
 * `.svelte-flow__node`; with culling on it would silently score only the
 * visible subset and report a healthy-looking result for a graph it never saw.
 */
export function cullingDisabledForTooling(): boolean {
	return (
		typeof window !== 'undefined' &&
		(window as unknown as { __topoNoCull?: boolean }).__topoNoCull === true
	);
}

export interface CullingConditions {
	/** Nodes currently handed to SvelteFlow. */
	renderedCount: number;
	/** A DOM measurement pass is in progress. */
	measuring: boolean;
	/** An export is capturing the flow element. */
	exporting: boolean;
}

/**
 * Culling is off below the threshold, so small topologies keep exactly the
 * behaviour they have today — the guardrail is that nothing changes at normal
 * scale, and a count-based gate is view-agnostic.
 */
export function shouldCull({ renderedCount, measuring, exporting }: CullingConditions): boolean {
	if (measuring || exporting || cullingDisabledForTooling()) return false;
	return renderedCount >= CULLING_THRESHOLD_ELEMENTS;
}

/**
 * Zoom below which a node stops rendering its contents.
 *
 * A card mixes four type sizes, so there is no single point where "the text" stops being readable.
 * Measured against a 6px floor — roughly where glyphs stop resolving at all — and an 8px
 * comfortable-reading mark:
 *
 *   text            used by                                6px at   8px at
 *   8.8px @ 70%     the MAC address line                     0.68     0.91
 *   12px            headers, port/speed, status, tags        0.50     0.67
 *   14px            subtitles, container titles              0.43     0.57
 *   16px            the largest container title              0.38     0.50
 *
 * Set below the 12px floor on purpose. Keeping detail to 0.50 would cut it off exactly as the bulk
 * text stopped resolving, which is the defensible reading of the arithmetic but the wrong reading
 * of how the view is used: an operator scanning at 0.4 wants to see which card is which and can
 * lean in, and having the whole graph turn to boxes at the moment they were still getting something
 * from it is worse than a few hundred milliseconds of frame time. At 0.35 the container titles are
 * ~5px and still shaped like words, while the smallest line — the MAC, which has been illegible
 * since 0.68 — is long past useful either way.
 *
 * The trade is stated rather than hidden: below this the graph is boxes, and the indicator in the
 * viewer says so, because a view that silently drops what it was showing reads as a bug.
 *
 * It is the only zoom transition: a card goes from its state-coloured box straight to full detail,
 * its status dot and badges included.
 */
export const DETAIL_ZOOM = 0.35;

/** Below this on-screen width a box is too small to be worth drawing structure inside. */
export const BOX_MIN_PX = 12;

/**
 * Graph size at or above which reduced detail is worth having at all.
 *
 * Simplifying is a real cost to the operator — the cards stop saying what they are — and it only
 * buys anything on a graph big enough for the DOM to hurt. On a few hundred nodes there is nothing
 * to reclaim and hiding detail is just worse. Set well above `CULLING_THRESHOLD_ELEMENTS`, since
 * culling is invisible and this is not, and at the scale the cost was actually measured: the
 * capture behind this work peaked at 1,501 mounted nodes before its tab ran out of memory.
 */
export const LOD_MIN_NODES = 1_000;

export interface SimplifyConditions {
	/** The viewport's current zoom. */
	zoom: number;
	/** Nodes in the store. Below `LOD_MIN_NODES` nothing is simplified at any zoom. */
	nodeCount: number;
	/**
	 * Layout width of the widest node drawn, in flow pixels. Once even that node is narrower than
	 * `BOX_MIN_PX` on screen, no card has anything legible on it, whatever the graph's size.
	 */
	widestNodeWidth: number;
	/** A DOM measurement pass is in progress. */
	measuring: boolean;
	/** An export is capturing the flow element. */
	exporting: boolean;
}

/**
 * Whether nodes should draw reduced, given the viewport and what else is happening.
 *
 * The same two suspensions as `shouldCull`, for the same two reasons and with the same
 * consequences if they are missed:
 *
 *  - **`measuring`** — the measure pass mounts every node to read its real card height. A reduced
 *    card measures its pinned height rather than its content's, so the pass would confirm whatever
 *    the last measurement happened to be and ELK would inherit it. Suspending here is also what
 *    makes the height pin correct: measured heights are always full-detail heights, so a reduced
 *    card pinned to `measured` occupies exactly the box the layout was built around and nothing
 *    reflows as the threshold is crossed.
 *  - **`exporting`** — `html-to-image` rasterises the flow element as it stands, so an export taken
 *    below the threshold would be a page of empty rectangles.
 *
 * Tooling shares `__topoNoCull` rather than getting a second flag: anything reading the graph out
 * of the DOM wants it whole and detailed, which is the same thing that switch already means.
 */
export function shouldSimplify(conditions: SimplifyConditions): boolean {
	if (reducedDetailSuspended(conditions)) return false;
	if (conditions.nodeCount >= LOD_MIN_NODES && conditions.zoom < DETAIL_ZOOM) return true;
	return everyNodeTooSmallToRead(conditions);
}

/** The suspensions every reduction shares: a measure pass, an export, or tooling. */
function reducedDetailSuspended({ measuring, exporting }: SimplifyConditions): boolean {
	return measuring || exporting || cullingDisabledForTooling();
}

/**
 * Every node is under `BOX_MIN_PX` wide on screen, so full cards show nothing and still cost a
 * repaint per pan frame. Measured on 252 collapsed switch cards (13,460 DOM elements) at the
 * zoom they fit at, 0.015: 11 fps drawn in full, 46 fps as boxes. Not gated on `LOD_MIN_NODES`,
 * because there is no detail left to lose. A small site reaches it only by zooming far out past
 * its fit zoom.
 */
function everyNodeTooSmallToRead({ zoom, widestNodeWidth }: SimplifyConditions): boolean {
	return widestNodeWidth > 0 && widestNodeWidth * zoom < BOX_MIN_PX;
}

/**
 * What a node draws once `shouldSimplify` says the graph is past full detail.
 *
 *  - `full` — contents as normal.
 *  - `boxed` — the box alone, which for an element card means its state colour.
 *  - `hidden` — nothing at all.
 *
 * Keyed on the node's *own* on-screen size rather than on zoom, which is what makes one rule work
 * across the hierarchy: at the zoom a large L2 graph fits at, a host container is 3.8px and the
 * element cards inside it are 2.7px, while zooming in moves them apart by two orders of magnitude.
 *
 * **Only subcontainers are ever `hidden`.** A grouping box a few pixels across is noise inside its
 * parent, and dropping it costs nothing because its children draw at their own absolute positions
 * regardless. An element card that small is still the graph's texture, and its colour is the one
 * thing that survives at that size, so it keeps its box.
 *
 * Nothing is labelled below full detail. A container's name cannot be drawn inside its own box —
 * SvelteFlow renders child nodes as siblings stacked above their parent, so a subcontainer's box
 * paints over anything the parent draws, at any z-index — and drawn outside the box it reads as
 * labelling whatever sits above it. Names return with the real header at `DETAIL_ZOOM`.
 */
export type NodeDetail = 'full' | 'boxed' | 'hidden';

export interface NodeDetailConditions {
	/** False once `shouldSimplify` is true. */
	detail: boolean;
	/** The node's own width in screen pixels — its layout width times the zoom. */
	screenWidth: number;
	/** A grouping container rather than one a user navigates by. Always false for elements. */
	isSubcontainer: boolean;
}

export function nodeDetail({
	detail,
	screenWidth,
	isSubcontainer
}: NodeDetailConditions): NodeDetail {
	if (detail) return 'full';
	if (isSubcontainer && screenWidth < BOX_MIN_PX) return 'hidden';
	return 'boxed';
}
