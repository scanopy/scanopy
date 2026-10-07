/**
 * How low the zoom floor has to be for a fit to actually fit.
 *
 * A customer report of an empty L3 Logical canvas turned out to contain its own answer in a field
 * nobody was reading against its limit: every sample in the ring sat at `scale(0.1)` — both views,
 * all four collapse levels, after eight separate fits on eight differently sized graphs. That is
 * not eight coincidences. `minZoom` was a hard-coded `0.1` and `getViewportForBounds` clamps to it,
 * so each fit computed the zoom it wanted, had it clamped away, and centred on a graph several
 * times wider than it could show. An operator sees a scatter of 25px cards with every container
 * label off-screen and reports a blank view; pressing `F` recomputes the identical clamped
 * transform, so it looks inert.
 *
 * Deriving the floor from the graph rather than fixing it at `0.1` is the fix. Kept out of
 * `BaseTopologyViewer` so the arithmetic can be tested without a browser.
 *
 * This does not make a 3,000-node fully expanded view *readable* — nothing at this layer can. It
 * makes it visible, and makes `F` do something.
 */
import type { Rect } from '@xyflow/system';

/**
 * The floor as it was, and still the floor for any graph that already fits.
 *
 * Derived downwards only. Raising the floor for a small graph would change behaviour nobody has
 * reported as wrong.
 */
export const DEFAULT_MIN_ZOOM = 0.1;

/**
 * How far the derived floor may fall.
 *
 * Below roughly this a 250px element card is under three pixels, and the viewport stops being a
 * view of anything. A graph needing less than this is past the point where fitting it helps, so
 * the floor holds and the fit still clamps — but now it clamps at a documented limit rather than
 * at a number chosen for a much smaller estate, and `fitZoom.clampedAtFloor` says so in the export.
 */
export const ABSOLUTE_MIN_ZOOM = 0.01;

/**
 * Headroom between the derived floor and the zoom a fit asks for.
 *
 * Both come from the same bounds, but the required zoom is measured once and the floor is then fed
 * back into `getViewportForBounds`, which re-derives it. Landing them on exactly the same value
 * would leave a fit clamping or not on a rounding difference. A few per cent of slack costs nothing
 * visible and makes the outcome deterministic.
 */
const FIT_FLOOR_MARGIN = 0.95;

/**
 * The zoom floor to hand SvelteFlow, given the zoom a fit needs for the current graph.
 *
 * Never above `DEFAULT_MIN_ZOOM`, so a graph that already fits keeps exactly the behaviour it has;
 * never below `ABSOLUTE_MIN_ZOOM`, so a pathological layout cannot zoom the canvas into nothing.
 * Between those, low enough that the fit is not clamped.
 *
 * Takes the required zoom rather than computing it, so the caller can measure it with
 * `getViewportForBounds` itself — the same function that will later apply the clamp, resolving
 * padding the same way. Re-deriving that here is how the two would drift apart.
 *
 * The floor feeds both `getViewportForBounds` and d3's `scaleExtent`, so deriving the prop keeps
 * the fit and the interactive floor in agreement. Computing a viewport below the prop and pushing
 * it in would not — d3 would pull it back on the next interaction.
 */
export function zoomFloorFor(requiredZoom: number): number {
	if (!Number.isFinite(requiredZoom) || requiredZoom <= 0) return DEFAULT_MIN_ZOOM;
	// Tested against the floor before the headroom is applied, not after: a graph that fits at
	// exactly `DEFAULT_MIN_ZOOM` needs no derivation, and taking 5% off it there would lower the
	// floor for a graph that never asked.
	if (requiredZoom >= DEFAULT_MIN_ZOOM) return DEFAULT_MIN_ZOOM;
	return Math.max(requiredZoom * FIT_FLOOR_MARGIN, ABSOLUTE_MIN_ZOOM);
}

/** The parts of a flow node these bounds need. Structural, so a test needs no SvelteFlow. */
export interface BoundableNode {
	position: { x: number; y: number };
	measured?: { width?: number; height?: number };
	width?: number;
	height?: number;
	/** Set on a child; its `position` is then relative to that parent, not to the graph. */
	parentId?: string;
}

/**
 * Bounding box of a laid-out graph, in graph coordinates.
 *
 * Top-level nodes only, and that is the whole subtlety: a child's `position` is relative to its
 * parent, so folding children in mixes two coordinate spaces and drags the box towards the origin.
 * Nothing is lost by skipping them — every node is either a root container or inside one, and
 * containers carry `expandParent`, so the roots already enclose their descendants.
 *
 * Not `getNodesBounds` from `@xyflow/system`: that reads SvelteFlow's adopted internal nodes, and
 * on a view switch none of them have been adopted yet — precisely when the bounds are wanted. This
 * reads the node objects the pipeline just wrote.
 *
 * Returns `null` for a graph with nothing in it, so a caller can tell "no graph" from "a graph at
 * the origin".
 */
export function boundsOfNodes(nodes: readonly BoundableNode[]): Rect | null {
	let minX = Infinity;
	let minY = Infinity;
	let maxX = -Infinity;
	let maxY = -Infinity;
	let found = false;

	for (const node of nodes) {
		if (node.parentId) continue;
		const width = node.measured?.width ?? node.width ?? 0;
		const height = node.measured?.height ?? node.height ?? 0;
		const { x, y } = node.position;
		if (x < minX) minX = x;
		if (y < minY) minY = y;
		if (x + width > maxX) maxX = x + width;
		if (y + height > maxY) maxY = y + height;
		found = true;
	}

	if (!found) return null;
	return { x: minX, y: minY, width: maxX - minX, height: maxY - minY };
}

/** The parts of an adopted SvelteFlow node `boundsOfAdoptedNodes` reads. */
export interface AdoptedNode {
	internals: { positionAbsolute: { x: number; y: number } };
	measured?: { width?: number; height?: number };
	width?: number;
	height?: number;
}

/**
 * Bounding box of specific nodes, in graph coordinates, from SvelteFlow's adopted internal nodes.
 *
 * For moving the camera onto a node: search matches and the inspector's focus button. Unlike
 * `boundsOfNodes` it takes children, so it reads `positionAbsolute` rather than the parent-relative
 * `position`. Pair it with `fitBounds`, not `fitView({ nodes })`: `fitView` only queues a fit until
 * every node is measured, and the canvas's own whole-graph fit wins that race, so the camera never
 * reached the node.
 *
 * Returns `null` when none of the ids is adopted.
 */
export function boundsOfAdoptedNodes(
	ids: readonly string[],
	lookup: (id: string) => AdoptedNode | undefined
): Rect | null {
	const nodes = ids
		.map((id) => lookup(id))
		.filter((node): node is AdoptedNode => !!node)
		.map((node) => ({
			position: node.internals.positionAbsolute,
			measured: node.measured,
			width: node.width,
			height: node.height
		}));
	return boundsOfNodes(nodes);
}

/** The SvelteFlow calls `focusNodes` needs, from `useSvelteFlow()`. */
export interface FocusableFlow {
	fitBounds: (bounds: Rect, options?: { padding?: number; duration?: number }) => Promise<boolean>;
	getInternalNode: (id: string) => AdoptedNode | undefined;
}

/** Move the camera onto the given nodes: search matches, Focus buttons, the F shortcut. */
export function focusNodes(flow: FocusableFlow, ids: readonly string[]): void {
	const bounds = boundsOfAdoptedNodes(ids, flow.getInternalNode);
	if (bounds) void flow.fitBounds(bounds, { padding: 0.5, duration: 300 });
}
