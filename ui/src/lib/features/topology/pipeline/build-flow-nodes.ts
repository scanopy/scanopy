import type { Node, Position } from '@xyflow/svelte';
import type { LayoutGraph } from '../layout/layout-graph';
import type { RenderableTopology, TopologyNode } from '../types/base';
import type { XY } from './types';
import { getTopologyIndex } from '../entity-index';
import { entities } from '$lib/shared/stores/metadata';

export interface BuildFlowNodesParams {
	visibleNodes: TopologyNode[];
	collapsed: Set<string>;
	layoutGraph: LayoutGraph | null;
	topology: RenderableTopology;
	isNewStructure: boolean;
	useGraph: boolean;
	liveNodes: Node[];
	infraRuleId: string | null;
	editMode: boolean;
	/**
	 * DOM-measured sizes for this view, by node id.
	 *
	 * The pipeline's own measurement is the only source of truth for how big a card actually
	 * renders. It used to be read back off the live nodes instead — see the note on `liveNodes`
	 * below for why that never worked.
	 */
	sizeHints?: Map<string, XY> | null;
	/**
	 * Handles each node's edges actually name, from `collectEdgeHandles`. Omitted, every node
	 * declares all eight as before.
	 */
	usedHandles?: Map<string, Set<string>> | null;
}

/**
 * Handle box size, in CSS px, for each node type.
 *
 * Exported because `synthesizeHandles` below and the components that render the handles must agree
 * exactly: SvelteFlow derives edge endpoints from handle geometry, so a synthesized box that
 * differs from the rendered one moves every edge endpoint on that node. `ElementNode.svelte` and
 * `ContainerNode.svelte` set their handle styles from these constants for that reason — the sizes
 * lived only in component CSS before, which is precisely the sort of drift this file now depends
 * on not happening.
 */
export const ELEMENT_HANDLE_SIZE_PX = 8;

/** Shared empty set for nodes no edge names, so each such node does not allocate its own. */
const EMPTY_HANDLE_SET: ReadonlySet<string> = new Set<string>();

/** Width of an element with no glyph, and the fallback for handle geometry. */
export const ELEMENT_WIDTH_PX = 250;

/** Width of an icon card. */
export const ICON_ELEMENT_WIDTH_PX = 120;

/** The icon card's large tile. The top service badge is centred on its bottom-right corner. */
export const ICON_TILE_PX = 48;

/** A service or OS badge on the tile's corner. */
export const ICON_BADGE_PX = 24;

/** Where the top service badge rests: from the tile's centre to the badge's left edge. */
export const ICON_BADGE_PILE_OFFSET_PX = ICON_TILE_PX / 2 - ICON_BADGE_PX / 2;

/**
 * Where service badge `index` of `count` sits once the stack fans into a row centred under the
 * tile (on hover, and in the diagram exports): from the tile's centre to the badge's left edge,
 * spaced to fit inside the card's `px-2` padding. A lone badge has nothing to fan out from and
 * stays on the corner.
 */
export function iconBadgeFanOffsetPx(index: number, count: number): number {
	if (count <= 1) return ICON_BADGE_PILE_OFFSET_PX;
	const inner = ICON_ELEMENT_WIDTH_PX - 16;
	const step = Math.min(ICON_BADGE_PX + 2, (inner - ICON_BADGE_PX) / (count - 1));
	return (index - (count - 1) / 2) * step - ICON_BADGE_PX / 2;
}

/**
 * Width of an icon card that lists a guest's containers in rows. Each row holds a glyph, a name and
 * a status dot, and at 120px a name kept six characters before truncating.
 */
export const ICON_GROUP_ELEMENT_WIDTH_PX = 168;

/**
 * The width an element card is built at: an icon card when its entity type has `glyph` metadata,
 * wider when it carries inline groups.
 */
export function elementWidthPx(node: TopologyNode): number {
	const glyph =
		node.node_type === 'Element'
			? (entities.getMetadata(node.element_type) as { glyph?: string } | null)?.glyph
			: null;
	if (!glyph) return ELEMENT_WIDTH_PX;
	const groups = (node as { inline_groups?: unknown[] }).inline_groups ?? [];
	return groups.length > 0 ? ICON_GROUP_ELEMENT_WIDTH_PX : ICON_ELEMENT_WIDTH_PX;
}

/** Horizontal padding on an element card's text, mirroring `px-2` in `ElementNode.svelte`. */
const ELEMENT_CARD_PADDING_X_PX = 8;

/**
 * Width a card's text actually gets, once its padding is taken off.
 *
 * Exported because it is the reference anything truncating a name has to match: a card ellipsizes
 * at this width, so anything wanting to truncate "like a card does" measures against this rather
 * than repeating the card's dimensions.
 */
export const ELEMENT_TEXT_WIDTH_PX = ELEMENT_WIDTH_PX - ELEMENT_CARD_PADDING_X_PX * 2;

/**
 * Stand-in height for handle geometry on a node with no measurement yet.
 *
 * Only ever wrong for the frame before a real size arrives, and only by however far the card
 * differs from it — the same placeholder `measure.ts` uses when it has nothing better.
 */
const ELEMENT_FALLBACK_HEIGHT_PX = 100;
export const CONTAINER_HANDLE_SIZE_PX = 5;

/** The four sides SvelteFlow positions handles on, as both source and target. */
const HANDLE_POSITIONS = ['top', 'right', 'bottom', 'left'] as const;

/**
 * Build the handle geometry SvelteFlow would have measured, without mounting the node.
 *
 * `getNodesInside` treats a node as unconditionally visible while `internals.handleBounds` is
 * undefined (`forceInitialRender`), and `adoptUserNodes` only carries handle bounds forward for a
 * node that has already mounted once. A node the user has never scrolled to therefore defeats
 * culling no matter what else is set on it — and expanding a collapsed container produces
 * thousands of exactly those at once. Supplying `handles` is the one input that makes such a node
 * cullable on its first build: `parseHandles` synthesizes the bounds from it instead of reaching
 * for a previous mount that never happened.
 *
 * Geometry mirrors `@xyflow/svelte`'s `base.css` (`.svelte-flow__handle-{top,right,bottom,left}`),
 * which centres each handle on its edge with a 50% translate, and the reading `getHandleBounds`
 * takes from it — the handle's top-left corner relative to the node's.
 */
export function synthesizeHandles(
	width: number,
	height: number,
	handleSize: number,
	used?: ReadonlySet<string>
): NonNullable<Node['handles']> {
	const half = handleSize / 2;
	const geometry = {
		top: { x: width / 2 - half, y: -half },
		right: { x: width - half, y: height / 2 - half },
		bottom: { x: width / 2 - half, y: height - half },
		left: { x: -half, y: height / 2 - half }
	};

	const handles: NonNullable<Node['handles']> = [];
	for (const position of HANDLE_POSITIONS) {
		const id = position.charAt(0).toUpperCase() + position.slice(1);
		for (const type of ['source', 'target'] as const) {
			// Only the handles an edge on this node actually names, when that is known.
			//
			// All eight were emitted unconditionally, and each costs twice: once here, and again in
			// `parseHandles`, which builds a `handleBounds` object per handle on every adoption of
			// every node. Most nodes carry one or two edges, so most of that was allocated and never
			// touched — measured at ~104,000 handle objects across a session against ~3,700 after.
			// `NodeHandles.svelte` already narrows the *DOM* this way from the same source; this
			// closes the gap between what is rendered and what is declared.
			//
			// An empty set is still a set: `parseHandles` yields `{source: [], target: []}`, which is
			// a defined `handleBounds`, so an edgeless node stays cullable rather than falling back
			// to `forceInitialRender`.
			if (used && !used.has(`${type}:${id}`)) continue;
			handles.push({
				id,
				type,
				position: position as Position,
				x: geometry[position].x,
				y: geometry[position].y,
				width: handleSize,
				height: handleSize
			});
		}
	}
	return handles;
}

/** Count elements recursively within a container from raw topology nodes. */
function countChildElements(containerId: string, nodes: TopologyNode[]): number {
	let count = 0;
	for (const n of nodes) {
		if (n.node_type === 'Element' && (n as Record<string, unknown>).container_id === containerId) {
			count++;
		}
		if (
			n.node_type === 'Container' &&
			(n as Record<string, unknown>).parent_container_id === containerId
		) {
			count += countChildElements(n.id, nodes);
		}
	}
	return count;
}

/** Build subgroup summaries from raw topology nodes (fallback when layoutGraph is unavailable). */
function fallbackSubgroupSummaries(
	containerId: string,
	nodes: TopologyNode[]
): { groupId: string; childCount: number }[] {
	const summaries: { groupId: string; childCount: number }[] = [];
	for (const n of nodes) {
		if (
			n.node_type === 'Container' &&
			(n as Record<string, unknown>).parent_container_id === containerId
		) {
			summaries.push({
				groupId: n.id,
				childCount: countChildElements(n.id, nodes)
			});
		}
	}
	return summaries;
}

export function buildFlowNodes(params: BuildFlowNodesParams): Node[] {
	const {
		visibleNodes,
		collapsed,
		layoutGraph,
		topology,
		isNewStructure,
		useGraph,
		liveNodes,
		infraRuleId,
		editMode,
		sizeHints,
		usedHandles
	} = params;

	const currentPositions = new Map(liveNodes.map((n) => [n.id, n.position]));

	// Sizes come from `sizeHints` — the pipeline's own measurement — not from the live nodes.
	//
	// This used to read `n.measured` off `liveNodes`, which is `getNodes()`, which is the plain
	// user array the app itself set. SvelteFlow writes `measured` into its internal `nodeLookup`
	// and never back onto the user node, so that read resolved to undefined for every node and
	// this map only ever carried the literal props: a hardcoded 250 for elements, undefined for
	// their heights. Fixing the earlier `computed` → `measured` rename corrected the field name
	// but not the object it was read from, so the path stayed dead. Do not reintroduce either
	// `computed` (the v0 name, which does not exist in v1) or the read-back off `getNodes()`.
	const sizeOf = (id: string): { width?: number; height?: number } => {
		const hint = sizeHints?.get(id);
		return hint ? { width: hint.x, height: hint.y } : {};
	};

	return visibleNodes.map((node) => {
		const isNodeCollapsed = collapsed.has(node.id);
		let position: { x: number; y: number };
		let width: number | undefined;
		let height: number | undefined;
		// Whether this node took the measurement placement below, which is the only branch the size
		// hint must not be applied to. Tracked rather than inferred from `isNewStructure`: that flag
		// is true for any collapse change, so using it as the proxy dropped the hint on every press
		// — see the gate further down.
		let measurementPlacement = false;

		const isElement = node.node_type === 'Element';
		const elementWidth = elementWidthPx(node);

		// Container size from layout graph (collapsed = metadata size, expanded = ELK size)
		const containerSize =
			!isElement && layoutGraph ? layoutGraph.getContainerSize(node.id) : undefined;

		if (useGraph && layoutGraph) {
			const graphPos = layoutGraph.getPosition(node.id);
			position = graphPos ?? { x: node.position.x, y: node.position.y };
			width = isNodeCollapsed
				? (containerSize?.width ?? undefined)
				: isElement
					? elementWidth
					: (containerSize?.width ?? undefined);
			height = isNodeCollapsed
				? (containerSize?.height ?? undefined)
				: isElement
					? undefined
					: (containerSize?.height ?? undefined);
		} else if (!isNewStructure) {
			const curPos = currentPositions.get(node.id);
			const curSize = sizeOf(node.id);
			position = curPos ?? { x: node.position.x, y: node.position.y };
			// A container's size comes from the layout graph, collapsed or not, with the measured
			// hint only as a fallback.
			//
			// An expanded container used to read the hint *first*, and the hint lives in
			// `viewSizeCache`, which `prepare` clears whenever the topology changes. So a run
			// triggered by a data refetch — no new structure, no ELK, nothing to repopulate the
			// cache — built every expanded container with no width or height at all and it
			// rendered as its borders with its contents outside. The graph knew the size the whole
			// time: the capture that found this shows `containersZeroSizedAfter: 0` on the very run
			// that produced fourteen zero-sized containers in the DOM.
			width = isElement ? elementWidth : (containerSize?.width ?? curSize?.width ?? undefined);
			height = isElement ? undefined : (containerSize?.height ?? curSize?.height ?? undefined);
		} else {
			// Measurement pass: place at origin, let content determine size
			measurementPlacement = true;
			position = { x: 0, y: 0 };
			width = isElement ? elementWidth : undefined;
			height = undefined;
		}

		// Sizes SvelteFlow can cull against before the node has ever mounted.
		//
		// Culling is defeated twice over on a freshly built node set. `getNodesInside` computes
		// `area = width * height` and treats `overlappingArea >= area` as visible, so a node with
		// no known height has `area === 0` and passes unconditionally; and a node whose object
		// carries no `measured` loses its handle bounds in `adoptUserNodes`, which sets
		// `forceInitialRender`. Both are true of every node this function used to return, so every
		// pipeline run mounted the entire graph at least once no matter what the culling gate said.
		// At a few hundred hosts that full mount survives and culling engages afterwards; at
		// 17,000 nodes it exhausts memory first, and because measurement never completes the graph
		// never becomes cullable at all.
		//
		// `measured` is the right vehicle and the `height` prop is not: `NodeWrapper` applies a
		// height style only when the `height` prop is set, so seeding `measured.height` alone
		// leaves element cards free to size to their content while still giving culling a real
		// area to test.
		//
		// The measurement branch above is deliberately excluded — it exists to let content
		// determine size, and seeding it would make the measure pass read back its own guess.
		//
		// Gated on that branch specifically, not on `isNewStructure`. `isNewStructure` includes the
		// collapsed set, so it is true on every collapse press, and using it here dropped the hint
		// for nodes that had been placed from the layout graph and were not being measured at all.
		// An element's `height` is deliberately `undefined` on that path, so with no hint
		// `measuredHeight` was undefined, `cullable` was false, and the node was emitted with no
		// `measured` field — which per the note above means `forceInitialRender`. Every collapse
		// press therefore rebuilt the whole graph as unculled and mounted all of it, recovering only
		// on the following run. It also churned the size of ~99.6% of nodes between runs, which is
		// what blocks reusing node objects at all.
		const hint = measurementPlacement ? undefined : sizeHints?.get(node.id);
		const measuredWidth = width ?? hint?.x;
		const measuredHeight = height ?? hint?.y;
		// No hint and no known size: emit neither field. The node force-renders once, gets
		// measured for real, and is cullable on the next build. Intentional degradation — a
		// guessed size here would stick, because a node carrying both fields reads as initialised
		// and `NodeWrapper` never attaches its ResizeObserver to correct it.
		// A zero is not a measurement. `LayoutContainer.expandedSize` starts at `{0, 0}` and
		// `getContainerSize` returns that rather than `undefined`, so a container ELK has not sized
		// yet arrives here as a real-looking `0`. Seeding that would publish a zero-area node as
		// measured and hand it synthesized handles at 0x0.
		const cullable = !!measuredWidth && !!measuredHeight;

		// Handles are supplied for *every* node, sized or not.
		//
		// The node components no longer render handle DOM — eight `<Handle>` elements per node was
		// half of all in-node DOM and most of its event listeners, and nothing could interact with
		// them because topology editing is disabled. That makes this the only source of handle
		// geometry, and an edge whose endpoint has none anchors at the node's origin instead of its
		// edge. So unlike `measured`, which is withheld until a real size is known, these fall back
		// to the node's own literal size: a guessed anchor on a node that has not been measured yet
		// is corrected on the next build, while a missing one is a visibly wrong edge.
		const handleWidth = measuredWidth ?? width ?? ELEMENT_WIDTH_PX;
		const handleHeight = measuredHeight ?? height ?? ELEMENT_FALLBACK_HEIGHT_PX;

		return {
			id: node.id,
			type: node.node_type,
			position,
			...(width !== undefined && { width }),
			...(height !== undefined && { height }),
			handles: synthesizeHandles(
				handleWidth,
				handleHeight,
				isElement ? ELEMENT_HANDLE_SIZE_PX : CONTAINER_HANDLE_SIZE_PX,
				usedHandles?.get(node.id) ?? (usedHandles ? EMPTY_HANDLE_SET : undefined)
			),
			...(cullable && {
				measured: { width: measuredWidth, height: measuredHeight }
			}),
			expandParent: true,
			deletable: false,
			selectable: node.node_type !== 'Container',
			// `container_id` is a non-optional Uuid on the backend's NodeType::Element,
			// so it is always present. This used to fall back to the resolved
			// `subnetId`, which is L3-specific (only the IPAddress resolver ever
			// returns one) — a view-specific lookup in a shared builder, and an O(n)
			// resolver call per node, that could never actually fire.
			parentId:
				node.node_type == 'Element'
					? node.container_id
					: node.node_type == 'Container' && node.parent_container_id
						? (node.parent_container_id as string)
						: undefined,
			extent:
				editMode && (node.node_type == 'Element' || node.parent_container_id)
					? 'parent'
					: undefined,
			data: isNodeCollapsed
				? (() => {
						const totalCount =
							layoutGraph?.getChildCount(node.id) ?? countChildElements(node.id, topology.nodes);
						const summaries =
							layoutGraph?.getSubgroupSummaries(node.id) ??
							fallbackSubgroupSummaries(node.id, topology.nodes);
						// Exclude infrastructure services subgroup from workload count
						let excludedCount = 0;
						if (infraRuleId) {
							const { nodesById } = getTopologyIndex(topology);
							for (const s of summaries) {
								const groupNode = nodesById.get(s.groupId);
								if (
									groupNode &&
									(groupNode as Record<string, unknown>).element_rule_id === infraRuleId
								) {
									excludedCount += s.childCount;
								}
							}
						}
						return {
							...node,
							isCollapsed: true,
							childCount: totalCount - excludedCount,
							subgroupSummaries: summaries
						};
					})()
				: node
		};
	});
}

/**
 * Drop the seeded `measured`/`handles` from a built node set.
 *
 * For the full measurement pass, which mounts every node to read its real size. A node carrying
 * both fields reads as initialised to `NodeWrapper`, so it never gets a ResizeObserver and would
 * be presented at the size the seed guessed — letting the pass confirm its own input. Culling is
 * suspended for that pass anyway, so nothing is lost by removing them.
 */
export function stripSizeSeed(flowNodes: Node[]): Node[] {
	return flowNodes.map((node) => {
		const stripped = { ...node };
		delete stripped.measured;
		delete stripped.handles;
		return stripped;
	});
}

/** Sort parents before children (SvelteFlow requirement). */
export function sortFlowNodes(flowNodes: Node[]): Node[] {
	const depthOf = (n: Node) => {
		if (!n.parentId) return 0;
		if (n.type === 'Container') return 1;
		return 2;
	};
	return flowNodes.sort((a, b) => depthOf(a) - depthOf(b));
}
