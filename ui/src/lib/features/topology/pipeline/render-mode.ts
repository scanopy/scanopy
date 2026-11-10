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
 * Size on screen, in CSS pixels, below which a card's text is not legible.
 *
 * WCAG 2.2 sets no minimum text size: no success criterion gives one. The nearest published floor
 * is the APCA Readability Criterion, the contrast work drafted for WCAG 3 (not a ratified
 * standard), whose Silver level puts label text read at a glance, which is what a card holds, at
 * 10px and body text at 13 to 14px:
 * https://a11yreadtech.github.io/ARC/tests/visual-readability-contrast
 *
 * A CSS pixel is a visual angle (about 0.0213 degrees: 1/96 inch at 28 inches) that each device
 * maps to whole device pixels, so this holds across screen resolutions and pixel densities with
 * no correction. Browser zoom and viewing distance do change the real size, and a page can read
 * neither.
 */
export const LEGIBLE_TEXT_PX = 10;

/**
 * Font size of the text a card is made of.
 *
 * Measured on rendered cards in L2, L3 and Workloads (system font, x-height 0.51 of the size):
 * every line is 12px (names, subgroup rows, counts, addresses) except the summary line of a
 * collapsed container, which is 16px.
 */
export const CARD_TEXT_PX = 12;

/**
 * Zoom below which a node stops rendering its contents: where the text a card is made of drops
 * under the legible size. 10 / 12, about 0.83.
 *
 * This was 0.35, picked by eye as the point where titles were "still shaped like words" at about
 * 5px. Nothing on a card can be read there, and drawing it cost 125 ms a frame while zooming a
 * 252-switch view. Below this zoom the graph is boxes with each container's name drawn at a
 * legible size on its own layer (`label-placement.ts`), and the indicator in the viewer says so.
 *
 * It is the only zoom transition: a card goes from its state-coloured box straight to full detail,
 * its status dot and badges included.
 */
export const DETAIL_ZOOM = LEGIBLE_TEXT_PX / CARD_TEXT_PX;

/**
 * Zoom below which a graph that keeps full cards (one under `LOD_MIN_NODES`) also gets its
 * container names on the names layer. Its titles are 3px here. Set under the zoom a small site
 * fits at (0.33 to 0.57 across the demo sites' views, less in a small window), so its fit view
 * is its cards alone and the names come in only on zooming out past it.
 */
export const FULL_CARD_NAMES_ZOOM = 0.25;

/** Below this on-screen width a box is too small to be worth drawing structure inside. */
export const BOX_MIN_PX = 12;

/**
 * Graph size at or above which reduced detail is worth having at all.
 *
 * Simplifying is a real cost to the operator — the cards stop saying what they are — and it only
 * buys anything on a graph big enough for the DOM to hurt. Below this a graph keeps full cards at
 * every zoom.
 *
 * The same line as `CULLING_THRESHOLD_ELEMENTS`. It was 1,000, which left a view of a few hundred
 * containers drawing every card in full from a zoom of 0.04 up: 252 collapsed switches are 13,460
 * DOM elements, mounted in one 1.7 s frame on the way in and then repainted at 125 ms a frame
 * with text 1 to 5px tall. The demo sites this must leave alone top out at 102 nodes in their
 * largest fully expanded view.
 */
export const LOD_MIN_NODES = CULLING_THRESHOLD_ELEMENTS;

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
