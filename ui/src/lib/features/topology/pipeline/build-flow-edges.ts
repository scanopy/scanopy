import { get } from 'svelte/store';
import type { Edge, EdgeMarkerType } from '@xyflow/svelte';
import type { TopologyEdge } from '../types/base';
import type { TopologyView } from '../queries';
import type { EdgeHandles } from '../layout/elk-layout';
import { computeOptimalHandles, ObstacleGrid } from '../layout/handle-picking';
import { getViewLayout } from '../layout/view-layout';
import type { SelectionStores } from '../selection';
import type { AggregatedEdge } from '../collapse';
import type { LayoutGraph } from '../layout/layout-graph';
import { edgeTypes } from '#lib/shared/stores/metadata.ts';
import { getEdgeDisplayState, searchHiddenNodeIds, tagHiddenNodeIds } from '../interactions';
import { isDisabledEdge, isOverlayEdge } from '../layout/edge-classification';
import { bundleEdges } from '../layout/edge-bundling';

export interface CreateFlowEdgeParams {
	edge: TopologyEdge;
	index: number;
	edgeHandles: Map<string, EdgeHandles>;
	selectionStores: SelectionStores;
	extraData?: Record<string, unknown>;
}

export function createFlowEdge(params: CreateFlowEdgeParams): Edge {
	const { edge, index, edgeHandles: handles, selectionStores, extraData } = params;

	const edgeType = edge.edge_type as string;
	const edgeMetadata = edgeTypes.getMetadata(edgeType);
	const edgeColorHelper = edgeTypes.getColorHelper(edgeType);

	const markerStart = !edgeMetadata.has_start_marker
		? undefined
		: ({
				type: 'arrow',
				color: edgeColorHelper.rgb
			} as EdgeMarkerType);
	const markerEnd = !edgeMetadata.has_end_marker
		? undefined
		: ({
				type: 'arrow',
				color: edgeColorHelper.rgb
			} as EdgeMarkerType);

	const edgeId = `edge-${index}`;
	const flowEdge: Edge = {
		id: edgeId,
		source: edge.source,
		target: edge.target,
		markerEnd,
		markerStart,
		sourceHandle: (
			handles.get(`${edge.source}->${edge.target}`)?.sourceHandle ?? edge.source_handle
		).toString(),
		targetHandle: (
			handles.get(`${edge.source}->${edge.target}`)?.targetHandle ?? edge.target_handle
		).toString(),
		type: 'custom',
		label: edge.label ?? undefined,
		data: { ...edge, edgeIndex: index, ...extraData },
		animated: false,
		interactionWidth: 50
	};

	// Compute display state from current selection
	const curNode = get(selectionStores.selectedNode);
	const curEdge = get(selectionStores.selectedEdge);
	const searchHidden = get(searchHiddenNodeIds);
	const tagHidden = get(tagHiddenNodeIds);
	const { shouldAnimate, shouldShowFull, isEndpointSearchHidden, isEndpointTagHidden } =
		getEdgeDisplayState(flowEdge, curNode, curEdge, searchHidden, tagHidden);
	flowEdge.data = {
		...flowEdge.data,
		shouldShowFull,
		shouldAnimate,
		isSelected: curEdge?.id === flowEdge.id,
		hasActiveSelection: !!(curNode || curEdge),
		isEndpointSearchHidden,
		isEndpointTagHidden
	};

	return flowEdge;
}

export interface BuildFlowEdgesParams {
	elevatedEdges: TopologyEdge[];
	collapsed: Set<string>;
	elementToContainer: Map<string, string>;
	aggregatedEdges: AggregatedEdge[];
	hiddenEdgeTypes: string[];
	layoutNodes: import('../types/base').TopologyNode[];
	/** Active view — drives the per-view edge-handle rule (L2 left/right). */
	view: TopologyView;
	layoutGraph: LayoutGraph | null;
	bundleEnabled: boolean;
	currentExpandedBundles: Set<string>;
	selectionStores: SelectionStores;
}

export interface BuildFlowEdgesResult {
	flowEdges: Edge[];
	originalsMap: Map<string, TopologyEdge[]>;
}

export function buildFlowEdges(params: BuildFlowEdgesParams): BuildFlowEdgesResult {
	const {
		elevatedEdges,
		collapsed,
		elementToContainer,
		aggregatedEdges,
		hiddenEdgeTypes,
		layoutNodes,
		view,
		layoutGraph,
		bundleEnabled,
		currentExpandedBundles,
		selectionStores
	} = params;

	// Compute edge handles against final post-layout positions. Running it
	// here (rather than inside the layout engines) ensures handles always
	// match the coordinates xyflow will render, after snap / collapse / any
	// other final adjustments. Handle selection is purely cosmetic — no
	// layout algorithm consumes it — so deferring is safe.
	const edgeHandlesMap = computeHandlesFromLayout(
		elevatedEdges,
		aggregatedEdges,
		layoutGraph,
		view
	);

	let baseEdges: TopologyEdge[];
	const extraFlowEdges: Edge[] = [];
	const originalsMap = new Map<string, TopologyEdge[]>();

	if (collapsed.size > 0 && aggregatedEdges.length > 0) {
		// Filter out edges where source or target is inside a collapsed container
		baseEdges = elevatedEdges.filter((edge) => {
			const srcContainer = elementToContainer.get(edge.source as string);
			const tgtContainer = elementToContainer.get(edge.target as string);
			const srcCollapsed = srcContainer && collapsed.has(srcContainer);
			const tgtCollapsed = tgtContainer && collapsed.has(tgtContainer);
			return !srcCollapsed && !tgtCollapsed;
		});

		// Create aggregated flow edges for collapsed containers
		for (let index = 0; index < aggregatedEdges.length; index++) {
			const agg = aggregatedEdges[index];
			originalsMap.set(agg.id, agg.originalEdges);
			const edgeKey = `${agg.source}->${agg.target}`;
			const handles = edgeHandlesMap.get(edgeKey);

			const aggFlowEdge: Edge = {
				id: agg.id,
				source: agg.source,
				target: agg.target,
				sourceHandle: (handles?.sourceHandle ?? 'Bottom').toString(),
				targetHandle: (handles?.targetHandle ?? 'Top').toString(),
				type: 'custom',
				label: undefined,
				data: {
					...agg.originalEdges[0],
					source: agg.source,
					target: agg.target,
					isAggregated: true,
					aggregatedCount: agg.count,
					edgeIndex: 1000 + index
				},
				animated: false,
				interactionWidth: 50
			};

			// Compute display state (same pattern as createFlowEdge)
			const curNode = get(selectionStores.selectedNode);
			const curEdge = get(selectionStores.selectedEdge);
			const searchHidden = get(searchHiddenNodeIds);
			const tagHidden = get(tagHiddenNodeIds);
			const { shouldAnimate, shouldShowFull, isEndpointSearchHidden, isEndpointTagHidden } =
				getEdgeDisplayState(aggFlowEdge, curNode, curEdge, searchHidden, tagHidden);
			aggFlowEdge.data = {
				...aggFlowEdge.data,
				shouldShowFull,
				shouldAnimate,
				isSelected: curEdge?.id === aggFlowEdge.id,
				hasActiveSelection: !!(curNode || curEdge),
				isEndpointSearchHidden,
				isEndpointTagHidden
			};

			extraFlowEdges.push(aggFlowEdge);
		}
	} else {
		baseEdges = elevatedEdges;
	}

	// Filter visible edges
	const nonDisabledEdges = baseEdges.filter((e) => !isDisabledEdge(e));
	const visibleEdgesRaw = nonDisabledEdges.filter((e) => !hiddenEdgeTypes.includes(e.edge_type));

	// Build root container lookup
	const containerParent = new Map<string, string>();
	for (const node of layoutNodes) {
		if (node.node_type === 'Container' && node.parent_container_id) {
			containerParent.set(node.id, node.parent_container_id as string);
		}
	}
	function getRootContainer(id: string): string {
		let current = elementToContainer.get(id) ?? id;
		while (containerParent.has(current)) {
			current = containerParent.get(current)!;
		}
		return current;
	}

	// Remove self-edges and strip labels for intra-container edges
	const visibleEdges: TopologyEdge[] = visibleEdgesRaw
		.filter((e) => e.source !== e.target)
		.map((e) => {
			const srcRoot = getRootContainer(e.source as string);
			const tgtRoot = getRootContainer(e.target as string);
			if (srcRoot === tgtRoot && e.label) {
				return { ...e, label: null } as TopologyEdge;
			}
			return e;
		});

	let flowEdges: Edge[];
	const createParams = { edgeHandles: edgeHandlesMap, selectionStores };

	if (bundleEnabled) {
		const { bundles, unbundled } = bundleEdges(visibleEdges, elementToContainer);
		flowEdges = [];
		let edgeIndex = 0;

		for (const edge of unbundled) {
			flowEdges.push(createFlowEdge({ ...createParams, edge, index: edgeIndex++ }));
		}

		for (const bundle of bundles) {
			if (currentExpandedBundles.has(bundle.id)) {
				const fanTotal = bundle.edges.length;
				for (let i = 0; i < fanTotal; i++) {
					flowEdges.push(
						createFlowEdge({
							...createParams,
							edge: bundle.edges[i],
							index: edgeIndex++,
							extraData: {
								bundleId: bundle.id,
								bundleFanIndex: i,
								bundleFanTotal: fanTotal
							}
						})
					);
				}
			} else {
				const representative = bundle.edges[0];
				const bundleStrokeWidth = Math.min(2 + 0.5 * (bundle.count - 1), 6);
				flowEdges.push(
					createFlowEdge({
						...createParams,
						edge: representative,
						index: edgeIndex++,
						extraData: {
							isBundle: true,
							bundleId: bundle.id,
							bundleCount: bundle.count,
							bundleEdges: bundle.edges,
							bundleStrokeWidth,
							bundleIsOverlay: isOverlayEdge(representative)
						}
					})
				);
			}
		}
	} else {
		flowEdges = visibleEdges.map((edge, index) => createFlowEdge({ ...createParams, edge, index }));
	}

	// Add hidden edges (get filtered by CustomEdge's hideEdge logic)
	const hiddenEdges = nonDisabledEdges.filter((e) => hiddenEdgeTypes.includes(e.edge_type));
	for (const edge of hiddenEdges) {
		flowEdges.push(createFlowEdge({ ...createParams, edge, index: flowEdges.length }));
	}

	// Add aggregated collapse edges
	flowEdges.push(...extraFlowEdges);

	return { flowEdges, originalsMap };
}

/**
 * Compute edge handles against final post-layout positions from LayoutGraph.
 * Runs over both elementary (elevated) edges and aggregated collapse edges.
 * A view with the `horizontal` handle strategy uses a simple left/right rule; the
 * `optimal` strategy uses the generic `computeOptimalHandles` picker.
 */
function computeHandlesFromLayout(
	elevatedEdges: TopologyEdge[],
	aggregatedEdges: AggregatedEdge[],
	layoutGraph: LayoutGraph | null,
	view: TopologyView
): Map<string, EdgeHandles> {
	const out = new Map<string, EdgeHandles>();
	if (!layoutGraph) return out;

	const horizontalHandles = getViewLayout(view).handle_strategy === 'horizontal';

	const sizeFor = (id: string): { w: number; h: number } | undefined => {
		const c = layoutGraph.getContainerSize(id);
		if (c) return { w: c.width, h: c.height };
		const e = layoutGraph.getElementSize(id);
		if (e) return { w: e.x, h: e.y };
		return undefined;
	};

	// Absolute-positioned rects for every container + element, built once per
	// handle-resolution pass. Per-edge we filter out the endpoints and their
	// ancestor containers (a line from an element to its own parent naturally
	// crosses the parent's boundary — don't penalise that).
	const obstacles = new ObstacleGrid(layoutGraph.getAllNodeRects());

	const pick = (src: string, tgt: string): EdgeHandles | undefined => {
		const srcPos = layoutGraph.getAbsolutePosition(src);
		const tgtPos = layoutGraph.getAbsolutePosition(tgt);
		const srcSize = sizeFor(src);
		const tgtSize = sizeFor(tgt);
		if (!srcPos || !tgtPos || !srcSize || !tgtSize) return undefined;

		if (horizontalHandles) {
			const srcCx = srcPos.x + srcSize.w / 2;
			const tgtCx = tgtPos.x + tgtSize.w / 2;
			return {
				sourceHandle: srcCx < tgtCx ? 'Right' : 'Left',
				targetHandle: srcCx < tgtCx ? 'Left' : 'Right'
			};
		}

		const exclude = new Set<string>([src, tgt]);
		for (const a of layoutGraph.ancestorIdsOf(src)) exclude.add(a);
		for (const a of layoutGraph.ancestorIdsOf(tgt)) exclude.add(a);
		return computeOptimalHandles(srcPos, srcSize, tgtPos, tgtSize, (x1, y1, x2, y2) =>
			obstacles.countCrossings(x1, y1, x2, y2, exclude)
		);
	};

	for (const edge of elevatedEdges) {
		const h = pick(edge.source as string, edge.target as string);
		if (h) out.set(`${edge.source}->${edge.target}`, h);
	}
	for (const agg of aggregatedEdges) {
		const h = pick(agg.source, agg.target);
		if (h) out.set(`${agg.source}->${agg.target}`, h);
	}
	return out;
}
