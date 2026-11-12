import type { RenderableTopology, TopologyNode, TopologyEdge } from '../types/base';
import type { LayoutState, PrepareResult, XY } from './types';
import { LayoutGraph, type LayoutContainer } from '../layout/layout-graph';
import { ElkLayoutEngine, type LayoutResult } from '../layout/engine';
import { getViewLayout } from '../layout/view-layout';
import { computeForceLayout, type ForceNode, type ForceLink } from '../layout/force-layout';
import { containerTypes } from '#lib/shared/stores/metadata.ts';
import * as perf from '../perf';
import { noteRunDetail, noteElkResultDiscarded } from '../diagnostics';
import { effectiveElementSort } from '../element-sort';

const layoutEngine = new ElkLayoutEngine();

/**
 * Execute layout computation (ELK or force-directed) and apply auto-collapse.
 *
 * Mutates state.layoutGraph, state.sessionStructureKey, state.sessionBaseKey,
 * state.seenAutoCollapseIds, state.viewSizeCache, state.containerSizeCache.
 *
 * `stored` is a layout engine result kept from an earlier run with the same inputs (see
 * `layout-store.ts`). Given one, the ELK path applies it and does not run the engine.
 * `onFirstPassDone` is handed to the engine; see `LayoutInput`.
 *
 * @returns Updated visible nodes and, on the ELK path, the engine result that was applied; or
 * null if stale.
 */
export async function executeLayout(
	topology: RenderableTopology,
	state: LayoutState,
	prep: PrepareResult,
	elementNodeSizes: Map<string, XY>,
	isStale: () => boolean,
	stored?: LayoutResult,
	onFirstPassDone?: () => Promise<void>
): Promise<{ visibleNodes: TopologyNode[]; engineResult?: LayoutResult } | null> {
	const {
		layoutNodes,
		collapsed,
		elevatedEdges,
		deferCollapse,
		prevExpandedSizes,
		prevChildPositions,
		structureKey,
		baseKey,
		currentView,
		hiddenEdgeTypes
	} = prep;
	let { visibleNodes } = prep;
	let engineResult: LayoutResult | undefined;

	// Detect if all root containers are collapsed -> use force layout
	const rootContainerNodes = visibleNodes.filter(
		(n) => n.node_type === 'Container' && !n.parent_container_id
	);
	// Force-directed "overview mode" is for a handful of collapsed roots, where a
	// cloud reads better than a column. It is also affordable only there: it runs
	// 300 synchronous simulation ticks.
	const FORCE_LAYOUT_MAX_ROOTS = 50;
	const allRootCollapsed =
		rootContainerNodes.length > 0 &&
		rootContainerNodes.length <= FORCE_LAYOUT_MAX_ROOTS &&
		rootContainerNodes.every((n) => collapsed.has(n.id));

	// A view without the force overview always lays out with ELK, at every collapse level. L2's
	// identity is the structured column of hosts under a switch; swapping to a force simulation
	// when the last container collapses makes the fully-collapsed view a different diagram rather
	// than a denser one, and reads worse. Workloads opts out for the same reason.
	const useForceLayout = allRootCollapsed && getViewLayout(currentView).force_overview;

	// One container opened on a large site: lay out its inside and make room around it. A stored
	// layout for this exact state is the tidier result and wins. False means the local path could
	// not size the container, and the full layout below runs as it always did.
	if (prep.localExpand && !stored) {
		const applied = await executeLocalExpand(topology, state, prep, elementNodeSizes, isStale);
		if (applied === null) return null;
		if (applied) return { visibleNodes };
	}

	if (useForceLayout) {
		// Force layout for all-collapsed overview mode
		state.sessionCollapsed = null;
		visibleNodes = executeForceLayout(
			state,
			rootContainerNodes,
			elevatedEdges,
			elementNodeSizes,
			layoutNodes,
			collapsed,
			structureKey,
			baseKey,
			prevExpandedSizes
		);
	} else {
		// ELK layout path
		const elkCollapsed = deferCollapse ? new Set<string>() : collapsed;
		const elkNodes = deferCollapse ? layoutNodes : visibleNodes;

		// Last chance to abandon a superseded run before the expensive part.
		//
		// ELK is ~96% of pipeline time and the bulk of the allocation — runs measured at 1.8-3.7s
		// costing 250-340MB each. Collapse presses arrive faster than that, so the run in flight is
		// routinely known-stale before it even starts laying out, and the check after `compute()`
		// below paid for the layout in full before discarding it. Checking here turns a superseded
		// press into no ELK run at all.
		if (isStale()) {
			perf.count('elk-skipped-stale');
			noteRunDetail({ supersededBeforeElk: true });
			return null;
		}

		const elkComputeDone = perf.stage('layout.elk-compute');
		const elkResult =
			stored ??
			(await layoutEngine.compute({
				nodes: elkNodes,
				edges: elevatedEdges,
				topology: topology,
				view: currentView,
				parentIndex: prep.parentIndex,
				collapsedContainers: elkCollapsed,
				expandedContainerSizes: prevExpandedSizes,
				elementNodeSizes,
				hiddenEdgeTypes,
				// Read from the options these nodes were built with, so the flag always matches their
				// order.
				preserveChildOrder:
					effectiveElementSort(topology.options?.request, currentView) !== 'Automatic',
				onFirstPassDone
			}));
		elkComputeDone();
		engineResult = elkResult;
		// Superseded, but too late for the check above to have saved anything: this layout ran in
		// full and is now thrown away. Counted separately from `runsSuperseded`, which records the
		// signal rather than the saving — read together they say whether cancellation is firing
		// early enough to be worth having.
		if (isStale()) {
			perf.count('elk-result-discarded');
			noteRunDetail({ supersededAfterElk: true });
			noteElkResultDiscarded();
			return null;
		}

		state.sessionStructureKey = structureKey;
		state.sessionBaseKey = baseKey;
		state.sessionCollapsed = new Set(collapsed);

		// Rebuild the graph only when the node set changed; otherwise reuse and re-sync.
		//
		// Rebuilding on every layout discarded a graph that was structurally identical whenever only
		// collapse had moved, and each rebuild zeroes every `expandedSize` — recovered immediately
		// below, but only for containers `prevExpandedSizes` knows about. Reusing keeps the sizes
		// that are already correct and drops one full graph construction per press. `prepare` uses
		// the same `isNewBaseStructure` test, so the two stay in agreement about when a graph is
		// still valid.
		const graphBuildDone = perf.stage('layout.graph-build');
		if (!state.layoutGraph || prep.isNewBaseStructure) {
			state.layoutGraph = LayoutGraph.fromTopology(layoutNodes);
		}
		if (!deferCollapse) {
			state.layoutGraph.syncCollapseState(collapsed);
			if (prevExpandedSizes) {
				state.layoutGraph.restoreExpandedSizes(prevExpandedSizes);
			}
			if (prevChildPositions) {
				state.layoutGraph.restoreContainerChildPositions(prevChildPositions);
			}
		}
		graphBuildDone();
		const applyDone = perf.stage('layout.apply-elk');
		state.layoutGraph.applyElkResult(
			elkResult.nodePositions,
			elkResult.containerSizes,
			elkResult.elementNodeSizes
		);
		applyDone();
		noteRunDetail({ elkSizedContainers: elkResult.containerSizes.size });

		// When collapse was deferred, apply it AFTER ELK result
		if (deferCollapse) {
			const visibleDone = perf.stage('layout.visible-nodes');
			state.layoutGraph.syncCollapseState(collapsed);
			visibleNodes = state.layoutGraph.getVisibleNodes(layoutNodes);
			visibleDone();
		}

		// Cache container sizes from ELK result
		const cacheDone = perf.stage('layout.cache-sizes');
		for (const [id, size] of elkResult.containerSizes) {
			if (state.layoutGraph?.containers.has(id)) {
				const entry = state.containerSizeCache.get(id) ?? {};
				if (elkCollapsed.has(id)) {
					entry.collapsed = { x: size.width, y: size.height };
				} else {
					entry.expanded = { x: size.width, y: size.height };
				}
				state.containerSizeCache.set(id, entry);
			}
		}
		cacheDone();
	}

	return { visibleNodes, engineResult };
}

/**
 * The container a local expand applies to: the one container that is collapsed in the layout on
 * screen (`laidOut`) and open in `collapsed`, when it is visible and nothing else differs but
 * containers inside it opening with it.
 *
 * Undefined when another container's collapse flag in the graph has moved away from the layout on
 * screen, which a superseded run leaves behind: the graph's positions are then no longer the ones
 * on screen.
 */
export function localExpandTarget(
	graph: LayoutGraph,
	laidOut: Set<string> | null,
	collapsed: Set<string>
): string | undefined {
	if (!laidOut) return undefined;
	const opened = new Set<string>();
	for (const id of laidOut) if (!collapsed.has(id)) opened.add(id);
	// Something opened, and nothing closed.
	if (opened.size === 0 || laidOut.size - opened.size !== collapsed.size) return undefined;

	// Opening a container opens its subcontainers with it (`toggleCollapse`), so the opened set is
	// one container and any number of containers inside it.
	let top: LayoutContainer | undefined;
	for (const id of opened) {
		const container = graph.containers.get(id);
		if (!container) return undefined;
		let inside = false;
		for (let parent = container.parent; parent && !inside; parent = parent.parent) {
			inside = opened.has(parent.id);
		}
		if (inside) continue;
		if (top) return undefined;
		top = container;
	}
	if (!top) return undefined;
	for (let parent = top.parent; parent; parent = parent.parent) {
		if (collapsed.has(parent.id)) return undefined;
	}
	// The opened containers themselves may already be flagged open: an earlier run for this same
	// expand that was superseded before it laid anything out.
	for (const other of graph.containers.values()) {
		if (!opened.has(other.id) && other.collapsed !== laidOut.has(other.id)) return undefined;
	}
	return top.id;
}

/**
 * Lay out one newly opened container without running the layout engine on the whole graph.
 *
 * A full run lays out every root container and the inside of every open one, which on a site of a
 * few thousand hosts takes seconds, to open one switch. Here the engine runs on the opened
 * container and its descendants alone, the container keeps its position, and
 * `LayoutGraph.makeRoomFor` moves what is right of it and below it out of the way.
 *
 * The result is not what a full run would draw: the rest of the graph is not re-packed around the
 * larger container, and its children are not ordered by the side their links leave from. It is
 * never written to the layout store, so a reload of the same state gets the full layout.
 *
 * @returns true when applied, false when the engine returned no size for the container, null when
 * the run went stale.
 */
async function executeLocalExpand(
	topology: RenderableTopology,
	state: LayoutState,
	prep: PrepareResult,
	elementNodeSizes: Map<string, XY>,
	isStale: () => boolean
): Promise<boolean | null> {
	const graph = state.layoutGraph;
	const { containerId, previousSize } = prep.localExpand!;
	const container = graph?.containers.get(containerId);
	if (!graph || !container) return false;

	// A container of cards only, opened earlier in the session, still holds that layout.
	const insideKnown =
		container.childContainers.length === 0 &&
		container.expandedSize.width > 0 &&
		!container.childElements.some((element) => element.size.y === 0);

	if (!insideKnown) {
		const inside = graph.descendantIds(containerId);
		inside.add(containerId);
		const nodes = prep.visibleNodes
			.filter((node) => inside.has(node.id))
			// The engine lays out root containers; inside a parent, the opened container stands as one.
			.map((node) =>
				node.id === containerId
					? ({ ...node, parent_container_id: undefined } as unknown as TopologyNode)
					: node
			);
		const edges = prep.elevatedEdges.filter(
			(edge) => inside.has(edge.source) && inside.has(edge.target)
		);

		if (isStale()) return null;
		const computeDone = perf.stage('layout.local-compute');
		const result = await layoutEngine.compute({
			nodes,
			edges,
			topology,
			view: prep.currentView,
			collapsedContainers: prep.collapsed,
			expandedContainerSizes: prep.prevExpandedSizes,
			elementNodeSizes,
			hiddenEdgeTypes: prep.hiddenEdgeTypes,
			preserveChildOrder:
				effectiveElementSort(topology.options?.request, prep.currentView) !== 'Automatic'
		});
		computeDone();
		if (isStale()) return null;
		if (!result.containerSizes.get(containerId)?.width) return false;

		// The container stays where it is; only its inside and its size come from this run.
		const positions = new Map(result.nodePositions);
		positions.set(containerId, { ...container.position });
		graph.applyElkResult(positions, result.containerSizes, result.elementNodeSizes);
		for (const [id, size] of result.containerSizes) {
			const entry = state.containerSizeCache.get(id) ?? {};
			if (prep.collapsed.has(id)) entry.collapsed = { x: size.width, y: size.height };
			else entry.expanded = { x: size.width, y: size.height };
			state.containerSizeCache.set(id, entry);
		}
	}

	graph.makeRoomFor(containerId, previousSize);
	state.sessionStructureKey = prep.structureKey;
	state.sessionBaseKey = prep.baseKey;
	state.sessionCollapsed = new Set(prep.collapsed);
	perf.count('layout:local-expand');
	return true;
}

function executeForceLayout(
	state: LayoutState,
	rootContainerNodes: TopologyNode[],
	elevatedEdges: TopologyEdge[],
	elementNodeSizes: Map<string, XY>,
	layoutNodes: TopologyNode[],
	collapsed: Set<string>,
	structureKey: string,
	baseKey: string,
	prevExpandedSizes: Map<string, { width: number; height: number }> | undefined
): TopologyNode[] {
	const forceNodes: ForceNode[] = rootContainerNodes.map((n) => {
		const measured = elementNodeSizes.get(n.id);
		const meta = containerTypes.getMetadata(
			((n as Record<string, unknown>).container_type as string) ?? 'Subnet'
		);
		return {
			id: n.id,
			width: measured?.x ?? meta.collapsed_size.width,
			height: measured?.y ?? meta.collapsed_size.height
		};
	});

	// Build deduplicated links from elevated edges between root containers
	const rootIds = new Set(rootContainerNodes.map((n) => n.id));
	const forceLinks: ForceLink[] = [];
	const seenLinks = new Set<string>();
	for (const edge of elevatedEdges) {
		const src = edge.source as string;
		const tgt = edge.target as string;
		if (rootIds.has(src) && rootIds.has(tgt) && src !== tgt) {
			const key = `${src}->${tgt}`;
			if (!seenLinks.has(key)) {
				seenLinks.add(key);
				forceLinks.push({ source: src, target: tgt });
			}
		}
	}

	const forceResult = computeForceLayout(forceNodes, forceLinks);

	state.sessionStructureKey = structureKey;
	state.sessionBaseKey = baseKey;
	state.layoutGraph = LayoutGraph.fromTopology(layoutNodes);
	state.layoutGraph.syncCollapseState(collapsed);
	// Same restore as the ELK path above, and for the same reason: rebuilding recreates every
	// container with `expandedSize` at {0, 0}, and the force layout only sizes the root containers
	// it was given. Without this, anything it does not touch is left at zero, `getContainerSize`
	// returns that rather than undefined, and the container renders as its borders with its
	// contents outside — persistently, until something forces a fresh ELK run.
	if (prevExpandedSizes) {
		state.layoutGraph.restoreExpandedSizes(prevExpandedSizes);
	}
	state.layoutGraph.applyForceResult(forceResult.nodePositions, elementNodeSizes);

	// Recompute visible nodes after force layout rebuilds the graph
	return state.layoutGraph.getVisibleNodes(layoutNodes);
}

/**
 * Ids of the cards whose content keys differ between `previous` and `current`. A key is a card's
 * node id, optionally followed by `|` and more (`inlineGroupKey`).
 */
export function changedCardIds(previous: Set<string>, current: Set<string>): Set<string> {
	const cardOf = (key: string) => key.split('|')[0];
	const changed = new Set<string>();
	for (const key of current) if (!previous.has(key)) changed.add(cardOf(key));
	for (const key of previous) if (!current.has(key)) changed.add(cardOf(key));
	return changed;
}
