import { get } from 'svelte/store';
import type { RenderableTopology } from '../types/base';
import type { LayoutState, PrepareResult } from './types';
import { LayoutGraph } from '../layout/layout-graph';
import {
	collapsedContainers,
	collapseLevel,
	hadStoredLevelOnLoad,
	inferCurrentLevel,
	computeCollapsedForLevel,
	buildElementToContainer,
	computeCollapsedEdges,
	isScaleCollapsed,
	scaleCollapseCandidates
} from '../collapse';
import { elevateEdgesToContainers } from '../layout/edge-elevation';
import { containerTypes, views } from '$lib/shared/stores/metadata';
import { activeView, topologyOptions } from '../queries';
import { tagHiddenNodeIds, hiddenEntityIdsByType } from '../interactions';
import { buildTopologyParentIndex } from '../topology-parent-index';
import { ENTITY_COLLECTIONS } from '../resolvers';
import { noteRunDetail } from '../diagnostics';
import { effectiveElementSort } from '../element-sort';

/**
 * Build a stable signature of everything the active view inlines on its
 * element cards. Returns '' when no view-declared inline entity types are in
 * play, so L2 / Workloads/Service / Application pay nothing here.
 *
 * Any field change on any inlined entity bumps the signature — that's the
 * design trade-off: view-agnostic, fully deterministic, bounded over-trigger
 * (a service name change that doesn't affect card height still re-layouts).
 */
function getInlineContentKey(topo: RenderableTopology, view: string): string {
	const meta = views.getMetadata(view) as {
		element_config?: {
			element_entities?: Array<{ entity_type: string; inline_entities: string[] }>;
		};
	} | null;
	const entries = meta?.element_config?.element_entities ?? [];
	const inlineTypes = new Set<string>();
	for (const ee of entries) {
		for (const t of ee.inline_entities) inlineTypes.add(t);
	}
	if (inlineTypes.size === 0) return '';

	const sigs: string[] = [];
	for (const type of inlineTypes) {
		const collectionKey = ENTITY_COLLECTIONS[type];
		if (!collectionKey) continue;
		const collection = topo[collectionKey] as unknown;
		if (!Array.isArray(collection)) continue;
		for (const entity of collection) {
			sigs.push(JSON.stringify(entity));
		}
	}
	sigs.sort();
	return sigs.join(';');
}

// Tab-scoped guard: only apply the "fresh session" default-level seeding on the
// very first pipeline run of this tab. Subsequent runs (topology switches,
// re-renders) always use the inferrer so a user's explicit level (e.g. after
// stepExpand to 4) is respected on later navigations.
let defaultsAppliedThisSession = false;

/**
 * Everything currently filtered out, in a form the structure key can compare.
 *
 * Hiding a node is only one of three ways a filter changes what has to be laid out, and the
 * other two do not touch `tagHiddenNodeIds` at all:
 *
 *  - An **entity shown inline** on another node's card (a service, a port) resizes that card
 *    when hidden or shown, while every node id stays the same.
 *  - A **metadata-value filter** (e.g. the OpenPorts service category) is applied at render
 *    time straight from the options and never reaches either hidden-id store.
 *
 * Both still resize cards, so both belong in the structure key: it is what clears
 * `viewSizeCache`/`containerSizeCache`, and without that ELK re-runs against the sizes the
 * cards used to have and the new ones overlap.
 */
function getHideStateKey(view: string): { resizing: string; structural: string } {
	const resizing: string[] = [];
	const structural: string[] = [];

	// Hiding a node removes it. Nothing that survives renders differently, so measured sizes stay
	// valid — this belongs on the structural side.
	const hiddenNodes = get(tagHiddenNodeIds);
	if (hiddenNodes.size > 0) structural.push(`n:${[...hiddenNodes].sort().join(',')}`);

	// A hidden entity belongs on whichever side its type does. Drawn inside another node's card, it
	// changes that card's height; drawn as a node of its own, it just disappears and every survivor
	// renders identically. The flat `hiddenEntityIds` cannot tell them apart, which is why the typed
	// store exists — reading the flat one here put a link-state toggle on the resizing side and
	// re-measured all 19,095 nodes on every press.
	const inlineTypes = inlineEntityTypes(view);
	for (const [entityType, ids] of get(hiddenEntityIdsByType)) {
		if (ids.size === 0) continue;
		const part = `e:${entityType}:${[...ids].sort().join(',')}`;
		(inlineTypes.has(entityType) ? resizing : structural).push(part);
	}

	// A metadata filter is whichever of the two its entity is in this view. Hiding unlinked ports in
	// L2 removes Interface *element nodes* — no card resizes — while hiding a service category in L3
	// removes services drawn inside host cards and every one of them shrinks. Splitting them is what
	// stops a filter toggle from discarding 19,095 measured sizes it did not invalidate.
	for (const [entityType, key] of hiddenMetadataKeysByEntity(view)) {
		(inlineTypes.has(entityType) ? resizing : structural).push(`m:${key}`);
	}

	return { resizing: resizing.join('|'), structural: structural.join('|') };
}

/** Entity types this view draws inside other nodes' cards. */
function inlineEntityTypes(view: string): Set<string> {
	const meta = views.getMetadata(view) as {
		element_config?: { element_entities?: Array<{ inline_entities: string[] }> };
	} | null;
	const out = new Set<string>();
	for (const ee of meta?.element_config?.element_entities ?? []) {
		for (const t of ee.inline_entities) out.add(t);
	}
	return out;
}

/** The active view's metadata hide-set, serialized per entity type so each can be classified. */
function hiddenMetadataKeysByEntity(view: string): [string, string][] {
	const byView = (get(topologyOptions).request.hide_metadata_values ?? {}) as Record<
		string,
		Record<string, Record<string, string[]>>
	>;
	const forView = byView[view];
	if (!forView) return [];
	return Object.entries(forView)
		.sort(([a], [b]) => a.localeCompare(b))
		.map(([entityType, fields]) => [
			entityType,
			`${entityType}.` +
				Object.entries(fields ?? {})
					.sort(([a], [b]) => a.localeCompare(b))
					.map(([field, values]) => `${field}=${[...(values ?? [])].sort().join('+')}`)
					.join(',')
		]);
}

/**
 * The active view's metadata-value filters, serialized deterministically.
 *
 * Sorted at every level rather than `JSON.stringify`d, because object key order follows
 * insertion and would make an unchanged filter set produce a different string — a spurious
 * full re-layout every time the options object is rebuilt.
 */
export function hiddenMetadataKey(view: string): string {
	const byView = (get(topologyOptions).request.hide_metadata_values ?? {}) as Record<
		string,
		Record<string, Record<string, string[]>>
	>;
	const forView = byView[view];
	if (!forView) return '';
	return Object.entries(forView)
		.sort(([a], [b]) => a.localeCompare(b))
		.flatMap(([entityType, fields]) =>
			Object.entries(fields ?? {})
				.sort(([a], [b]) => a.localeCompare(b))
				.map(([field, values]) => `${entityType}.${field}=${[...(values ?? [])].sort().join('+')}`)
		)
		.join(',');
}

/** The slice of the container-type metadata store this module needs. Keeps the helper testable. */
interface ContainerTypesAccessor {
	getMetadata: (containerType: string) => { is_subcontainer?: boolean };
}

/**
 * Drop subcontainers left with no element children once filters have removed nodes.
 *
 * A filter removes elements, not the boxes that grouped them, so without this a hidden set leaves
 * empty labelled containers behind.
 *
 * **Collapsed subcontainers are pruned too.** They were exempted for a while — the exemption
 * arrived with the commit that split this file out of the viewer, described as a pure refactor
 * with no behaviour changes — and it inverted the rule for precisely the containers it matters
 * most for: `PortOpStatus` is the only type marked `collapsed_by_default`, so it was always
 * exempt. Hiding unlinked ports then left a collapsed "Down" box on every host with nothing in it
 * — 716 of them on the seeded reproduction, a third of the rendered nodes at full expansion. The
 * exemption was never needed: collapse is applied later, by `getVisibleNodes`, so a collapsed
 * container's children are still present here and still counted.
 *
 * Counting only `Element` children is safe because subcontainers never nest: `apply_element_rules`
 * computes `elements_by_container` once, before running any rule, so every subcontainer a rule
 * creates is parented to a root container rather than to another rule's subcontainer.
 */
export function pruneEmptySubcontainers(
	layoutNodes: RenderableTopology['nodes'],
	containerTypes: ContainerTypesAccessor
): RenderableTopology['nodes'] {
	const subcontainerIds = new Set(
		layoutNodes
			.filter(
				(n) =>
					n.node_type === 'Container' &&
					containerTypes.getMetadata(
						((n as Record<string, unknown>).container_type as string) ?? 'Subnet'
					).is_subcontainer
			)
			.map((n) => n.id)
	);
	if (subcontainerIds.size === 0) return layoutNodes;

	const occupied = new Set<string>();
	for (const n of layoutNodes) {
		if (n.node_type !== 'Element') continue;
		const cid = (n as Record<string, unknown>).container_id as string;
		if (subcontainerIds.has(cid)) occupied.add(cid);
	}

	return layoutNodes.filter(
		(n) => !(n.node_type === 'Container' && subcontainerIds.has(n.id) && !occupied.has(n.id))
	);
}

/**
 * Collapse containers marked `collapsed_by_default`, plus the infrastructure
 * subcontainer, and return the resulting collapsed set.
 *
 * Returns the set rather than relying on the store write alone: the caller must
 * use this value for the rest of the run, so that the run it belongs to already
 * reflects it and no corrective re-layout is needed.
 *
 * `seenAutoCollapseIds` makes this one-shot per container — a container the user
 * has since expanded is never re-collapsed behind them.
 */
function applyAutoCollapse(
	topology: RenderableTopology,
	state: LayoutState,
	collapsed: Set<string>,
	getInfrastructureRuleId: () => string | null
): Set<string> {
	const currentLevel = get(collapseLevel);
	const infraRuleId = getInfrastructureRuleId();

	// Above a size threshold, containers also start collapsed regardless of type.
	// Collapsing is the only lever that reduces the node *count* rather than the
	// cost per node. The candidate set comes from `collapse.ts` so that the
	// manual ladder computes exactly the same thing — see
	// `scaleCollapseCandidates`.
	//
	// Routed through the same candidate/seen machinery as `collapsed_by_default`
	// so a container the user expands is never re-collapsed behind them.
	const scaleIds = scaleCollapseCandidates(topology.nodes, containerTypes);

	const allCandidates = topology.nodes.filter((n) => {
		if (n.node_type !== 'Container') return false;
		const data = n as Record<string, unknown>;
		const ct = data.container_type as string | undefined;
		if (scaleIds.has(n.id)) return true;
		return (
			(ct && containerTypes.getMetadata(ct).collapsed_by_default === true) ||
			(infraRuleId && data.element_rule_id === infraRuleId)
		);
	});

	const userExplicitlyExpandedAll = currentLevel === 4 && state.collapseLevelInferred;
	const autoCollapseIds = userExplicitlyExpandedAll
		? []
		: allCandidates
				.filter((n) => !collapsed.has(n.id) && !state.seenAutoCollapseIds.has(n.id))
				.map((n) => n.id);

	let next = collapsed;
	if (autoCollapseIds.length > 0) {
		for (const id of autoCollapseIds) state.seenAutoCollapseIds.add(id);
		next = new Set(collapsed);
		for (const id of autoCollapseIds) next.add(id);
		collapsedContainers.set(next);
	}

	// Re-infer the level from what auto-collapse actually produced.
	//
	// Gated on its own flag, not `collapseLevelInferred`. The seeding step upstream consumes
	// that one in the same run and sets the level from the stored default — *before* this
	// function scale-collapses the graph — so this correction never ran: the view opened fully
	// collapsed while the indicator read 3, and every subsequent step walked from a number that
	// described a graph nobody had drawn.
	if (!state.collapseLevelReconciled) {
		state.collapseLevelReconciled = true;
		const inferred = inferCurrentLevel(
			next,
			topology.nodes,
			containerTypes,
			getInfrastructureRuleId()
		);
		collapseLevel.set(inferred);
	}

	return next;
}

function getStructureKey(topo: RenderableTopology, view: string): string {
	const nodeKeys = topo.nodes.map((n) => {
		const parentId = n.node_type === 'Element' ? n.container_id : n.parent_container_id;
		return `${n.id}@${parentId ?? ''}`;
	});
	// Under a server-side sort the node order is the layout's input, so a reorder has to relayout.
	// Otherwise the order is incidental (the builders iterate hash maps) and must not.
	const elementSort = effectiveElementSort(topo.options?.request, view);
	const orderKey =
		elementSort === 'Automatic'
			? nodeKeys.sort().join(',')
			: `${elementSort}:${nodeKeys.join(',')}`;
	const inlineKey = getInlineContentKey(topo, view);
	const hide = getHideStateKey(view);
	// Segments: nodes | inline | resizing-hide | structural-hide. `prepare` compares the middle two
	// to decide whether measured sizes survive — see the cache handling in `prepareTopologyData`.
	return `${topo.nodes.length}:${topo.edges.length}:${orderKey}|${inlineKey}|${hide.resizing}|${hide.structural}`;
}

/**
 * Prepare topology data for layout: validate inputs, manage collapse state,
 * filter nodes, elevate edges, compute structure keys.
 *
 * @returns null to signal "skip this run" (view mismatch, stale data)
 */
export function prepareTopologyData(
	topology: RenderableTopology,
	state: LayoutState,
	getInfrastructureRuleId: () => string | null
): PrepareResult | null {
	const currentView = get(activeView);
	const topoKey = getStructureKey(topology, currentView);
	const viewChanged = state.lastRenderedView !== '' && currentView !== state.lastRenderedView;
	const topologyChanged = topoKey !== state.lastRenderedTopoKey;

	if (topologyChanged) {
		// Discard sizes only when the thing that changed can change a card's size.
		//
		// `topoKey` is `nodes|inline|hide` (see `getStructureKey`), and the three differ in what
		// they invalidate. A change to the inline content or the hide state resizes cards while
		// every node id stays the same — that is why this cleared the caches, and it must keep
		// doing so or ELK lays out against sizes the cards no longer have and they overlap.
		//
		// A change to the *node set* is different: the nodes that survived render exactly as before,
		// so their measured sizes are still correct. Clearing for that case threw away all 19,095
		// sizes on every data refresh and forced the full measurement pass — the whole graph
		// mounted, ~665MB and 5.5s. One capture showed eight of those across twelve runs, six of
		// them following a topology refetch.
		const [, prevInline = '', prevHide = ''] = state.lastRenderedTopoKey.split('|');
		const [, nextInline = '', nextHide = ''] = topoKey.split('|');
		// Segment 2 is the resizing hide-state only; the structural one (segment 3) removes nodes
		// without changing how anything that remains is drawn.
		const cardsMayHaveResized =
			state.lastRenderedTopoKey === '' || prevInline !== nextInline || prevHide !== nextHide;

		if (cardsMayHaveResized) {
			state.viewSizeCache.clear();
			state.containerSizeCache.clear();
		} else {
			const survivingIds = new Set(topology.nodes.map((n) => n.id));
			for (const sizes of state.viewSizeCache.values()) {
				for (const id of sizes.keys()) {
					if (!survivingIds.has(id)) sizes.delete(id);
				}
			}
			for (const id of state.containerSizeCache.keys()) {
				if (!survivingIds.has(id)) state.containerSizeCache.delete(id);
			}
		}
		// Sizes are being re-learned from scratch, so the one-correction-per-node budget resets.
		state.driftCorrectedIds.clear();
		// Remove seenAutoCollapseIds entries that don't exist in the new topology
		const newContainerIds = new Set(
			topology.nodes.filter((n) => n.node_type === 'Container').map((n) => n.id)
		);
		for (const id of state.seenAutoCollapseIds) {
			if (!newContainerIds.has(id)) state.seenAutoCollapseIds.delete(id);
		}
	}

	// Skip if view changed but the enriched topology hasn't re-sliced yet.
	// The topology now carries one node/edge set per view and the active
	// view's slice is selected upstream (toRenderableTopology), so a view switch
	// always changes the structure key — no per-view data-readiness guard
	// is needed here.
	if (viewChanged && !topologyChanged) {
		return null;
	}

	let collapsed = get(collapsedContainers);

	// Drop stale IDs from the persisted collapsed set before any level logic.
	// A set carried over from a different topology (e.g. auth app → share) can
	// contain IDs not present here; the inferrer's "all current containers are
	// collapsed → level 1" fallback then triggers spuriously whenever the stale
	// superset happens to cover every current container.
	if (collapsed.size > 0) {
		const currentContainerIds = new Set(
			topology.nodes.filter((n) => n.node_type === 'Container').map((n) => n.id)
		);
		const stripped = new Set([...collapsed].filter((id) => currentContainerIds.has(id)));
		if (stripped.size !== collapsed.size) {
			collapsedContainers.set(stripped);
			collapsed = stripped;
		}
	}

	// Seed collapse state on first topology render.
	//
	// Truly fresh session (no stored level, first pipeline run this tab):
	// apply the default level's collapsed set so the initial view matches
	// the intended default instead of being inferred as level 4 from an
	// empty collapsed set.
	//
	// Otherwise: infer the level from the persisted collapsed set so any
	// prior user choice (including an explicit stepExpand to 4) is respected.
	if (!state.collapseLevelInferred) {
		state.collapseLevelInferred = true;
		if (!defaultsAppliedThisSession && !hadStoredLevelOnLoad) {
			const defaultLevel = get(collapseLevel);
			const defaultCollapsed = computeCollapsedForLevel(
				defaultLevel,
				topology.nodes,
				containerTypes,
				getInfrastructureRuleId()
			);
			collapsedContainers.set(defaultCollapsed);
			collapsed = defaultCollapsed;
			collapseLevel.set(defaultLevel);
		} else {
			const inferred = inferCurrentLevel(
				collapsed,
				topology.nodes,
				containerTypes,
				getInfrastructureRuleId()
			);
			collapseLevel.set(inferred);
		}
		defaultsAppliedThisSession = true;
	}

	// On view switch, apply the current collapse level to the new view's containers
	if (viewChanged && topologyChanged && state.collapseLevelInferred) {
		// ...unless the view being entered is large enough to scale-collapse, in which case open
		// it collapsed instead of carrying the level across.
		//
		// Views do not hold comparable numbers of nodes. L3 draws one element per IP address and
		// inlines ports and services onto the card; L2 draws one per interface, so the same
		// network can be ~1,200 nodes in one and ~17,000 in the other. Carrying level 4 across
		// that boundary hands the whole of the larger view to the renderer fully expanded — which
		// is how a customer reached an out-of-memory failure: level 4 in L3, then switch to L2.
		//
		// Level 1 rather than a clamp to 2 or 3, because level 1 *is* the scale-collapsed state by
		// construction (`computeCollapsedForLevel` case 1 returns exactly
		// `scaleCollapseCandidates`), so `inferCurrentLevel` reads it back as 1 and the indicator
		// agrees with the diagram. Levels 2 and 3 leave every host container open, which at this
		// scale is most of the node count.
		//
		// This does not break the promise that a container the user expanded is never re-collapsed
		// behind them: container ids are per-view, so nothing here has been expanded by the user in
		// the view being entered.
		const enteringAtScale = isScaleCollapsed(topology.nodes);
		const currentLevel = get(collapseLevel);
		const effectiveLevel = enteringAtScale ? 1 : currentLevel;
		const levelCollapsed = computeCollapsedForLevel(
			effectiveLevel,
			topology.nodes,
			containerTypes,
			getInfrastructureRuleId()
		);
		collapsedContainers.set(levelCollapsed);
		collapsed = levelCollapsed;
		if (effectiveLevel !== currentLevel) collapseLevel.set(effectiveLevel);
	}

	// When topology identity changes, reset tracking and strip stale collapsed IDs
	const topologyId = topology.id ?? '';
	if (topologyId !== state.lastSeenTopologyId && state.lastSeenTopologyId !== '') {
		state.seenAutoCollapseIds = new Set<string>();
		state.containerSizeCache.clear();
		state.collapseLevelInferred = false;
		// A new topology re-runs auto-collapse, so its level needs reconciling again too.
		state.collapseLevelReconciled = false;

		if (collapsed.size > 0) {
			const newContainerIds = new Set(
				topology.nodes.filter((n) => n.node_type === 'Container').map((n) => n.id)
			);
			const validCollapsed = new Set([...collapsed].filter((id) => newContainerIds.has(id)));
			const staleCount = collapsed.size - validCollapsed.size;

			// If ALL old root containers were collapsed, preserve "overview mode"
			if (state.layoutGraph) {
				const oldRootIds = [...state.layoutGraph.containers.values()]
					.filter((c) => !c.parent)
					.map((c) => c.id);
				const wasFullyCollapsed =
					oldRootIds.length > 0 && oldRootIds.every((id) => collapsed.has(id));
				if (wasFullyCollapsed) {
					const allContainerIds = topology.nodes
						.filter((n) => n.node_type === 'Container')
						.map((n) => n.id);
					const allCollapsed = new Set(allContainerIds);
					collapsedContainers.set(allCollapsed);
					collapseLevel.set(1);
					collapsed = allCollapsed;
					state.fitViewPending = true;
				} else if (staleCount > 0) {
					collapsedContainers.set(validCollapsed);
					collapsed = validCollapsed;
				}
			} else if (staleCount > 0) {
				collapsedContainers.set(validCollapsed);
				collapsed = validCollapsed;
			}
		}
	}
	state.lastSeenTopologyId = topologyId;

	// Collapse containers whose type is marked collapsed_by_default (plus the
	// infrastructure subcontainer).
	//
	// This runs here, before layout, rather than after it. It depends only on
	// node metadata and collapse state — never on layout output — and doing it
	// afterwards meant writing `collapsedContainers` once ELK had already laid
	// the graph out expanded, which the viewer saw as an external change and
	// answered with a second complete pipeline run: another full DOM measure
	// pass and two more elk.layout() calls, on every cold load.
	collapsed = applyAutoCollapse(topology, state, collapsed, getInfrastructureRuleId);

	// Filter out nodes hidden by any filter source (tag, category/metadata,
	// entity-hide). Filter = structural remove, uniformly across sources —
	// the node is absent from ELK input, DOM, and edge graph. Fade is now
	// reserved for focus operations (search, selection).
	const hiddenByFilter = get(tagHiddenNodeIds);
	let layoutNodes =
		hiddenByFilter.size > 0
			? topology.nodes.filter((n) => !hiddenByFilter.has(n.id))
			: topology.nodes;

	layoutNodes = pruneEmptySubcontainers(layoutNodes, containerTypes);

	const elementToContainer = buildElementToContainer(layoutNodes);
	const parentIndex = buildTopologyParentIndex(topology.nodes);
	const hiddenEdgeTypes = get(topologyOptions).local.hide_edge_types ?? [];

	// Elevate edges targeting elements inside absorbing containers.
	// Then drop edges whose endpoints were filtered out so ELK doesn't
	// see orphaned references and the renderer doesn't draw ghost lines.
	const elevatedEdgesRaw = elevateEdgesToContainers(topology.edges, layoutNodes);
	const elevatedEdges =
		hiddenByFilter.size > 0
			? elevatedEdgesRaw.filter(
					(e) => !hiddenByFilter.has(e.source) && !hiddenByFilter.has(e.target)
				)
			: elevatedEdgesRaw;

	// Map containers to themselves for bundling
	for (const node of layoutNodes) {
		if (node.node_type === 'Container' && !elementToContainer.has(node.id)) {
			elementToContainer.set(node.id, node.id);
		}
	}

	// Compute structure and base keys
	// Edge visibility intentionally excluded — layout-affecting edges are always
	// fed to ELK regardless of visibility, so toggling shouldn't trigger rebuild.
	const baseKey = currentView + ':' + topoKey;
	const structureKey = baseKey + ':' + Array.from(collapsed).sort().join(',');
	const isNewStructure = state.sessionStructureKey !== structureKey;
	const isNewBaseStructure = state.sessionBaseKey !== baseKey;

	// Capture expanded sizes/positions before rebuilding the graph — but NOT
	// across a view switch. The existing layoutGraph belongs to the previous
	// view, whose nodes/containers differ from this view's slice; restoring its
	// sizes/positions onto the new view's graph piles children at the origin on
	// the first expand. On a view switch we start fresh (like a reload) and let
	// ELK lay out; each view's persisted positions come from its own backend
	// slice. Same-view re-renders (e.g. expanding a container) still reuse them.
	const prevExpandedSizes = viewChanged
		? undefined
		: state.layoutGraph?.getExpandedContainerSizes();
	const prevChildPositions = viewChanged
		? undefined
		: state.layoutGraph?.getContainerChildPositions();

	// Build/rebuild layout graph when the *node set* changes — not when collapse does.
	//
	// `baseKey` covers everything the graph is built from: node ids, their parent links, inline
	// content, and hide state. `structureKey` is that plus the collapsed set, so keying the rebuild
	// on it threw the graph away every time a container was collapsed, even though collapse is
	// visibility applied on top of an unchanged node set via `syncCollapseState`. That cost a full
	// rebuild per press, and each rebuild recreates every container with `expandedSize` at {0, 0} —
	// the same reset behind the 0x0 containers, so this narrows that exposure rather than widening
	// it.
	//
	// Restore straight away rather than leaving it to `executeLayout`. This run may never reach the
	// layout stage — it can go stale, or return early — which would leave the graph zeroed for
	// whatever runs next. If that run does no layout of its own, `getContainerSize` returns zero and
	// the containers render as their borders with their contents outside, persistently.
	if (!state.layoutGraph || isNewBaseStructure) {
		const carriedSizes = state.layoutGraph?.getExpandedContainerSizes();
		state.layoutGraph = LayoutGraph.fromTopology(layoutNodes);
		if (carriedSizes) state.layoutGraph.restoreExpandedSizes(carriedSizes);
		noteRunDetail({ graphRebuilt: true });
	}

	// Defer collapse so ELK runs with everything expanded — only if
	// no expanded size is available from either the graph or the cache.
	//
	// Skipped entirely when collapse is scale-driven. Deferring means mounting
	// every element card in the graph purely to learn expanded container sizes
	// for containers we are about to collapse — the dominant cost of a cold load
	// — and then discovering their collapsed sizes afterwards, which triggers a
	// corrective re-layout. At scale it is far cheaper to learn a container's
	// expanded size lazily, if and when the user expands it.
	let deferCollapse = false;
	if (isNewStructure && collapsed.size > 0 && !isScaleCollapsed(topology.nodes)) {
		for (const id of collapsed) {
			const hasChildren = layoutNodes.some(
				(n) =>
					(n.node_type === 'Element' && (n as Record<string, unknown>).container_id === id) ||
					(n.node_type === 'Container' && (n as Record<string, unknown>).parent_container_id === id)
			);
			const hasExpandedSize =
				prevExpandedSizes?.has(id) || !!state.containerSizeCache.get(id)?.expanded;
			if (hasChildren && !hasExpandedSize) {
				deferCollapse = true;
				break;
			}
		}
	}

	// Sync collapse state from store -> graph
	let collapseChanged = false;
	if (!deferCollapse) {
		collapseChanged = state.layoutGraph.syncCollapseState(collapsed);
	}

	// Force ELK re-layout when a container was expanded but has no cached layout
	let needsElkForExpand = false;
	if (collapseChanged) {
		for (const c of state.layoutGraph.containers.values()) {
			if (!c.collapsed && c.allChildren.length > 0) {
				const hasZeroExpandedSize = c.expandedSize.width === 0;
				const hasUninitializedChildren = c.childElements.some((el) => el.size.y === 0);
				if (hasZeroExpandedSize || hasUninitializedChildren) {
					needsElkForExpand = true;
					state.seenAutoCollapseIds.add(c.id);
				}
			}
		}
	}

	// Compute aggregated edges for collapsed containers
	const aggregatedEdges = computeCollapsedEdges(
		elevatedEdges,
		collapsed,
		layoutNodes,
		hiddenEdgeTypes,
		parentIndex.parentMap
	);

	const visibleNodes = state.layoutGraph.getVisibleNodes(layoutNodes);

	const isViewTransition = isNewStructure && viewChanged && topologyChanged;
	const needsElk = isNewStructure || needsElkForExpand;

	// Clear view size cache on base structure change
	if (isNewBaseStructure) {
		state.viewSizeCache.delete(`${currentView}:${topology.id}`);
	}

	return {
		layoutNodes,
		collapsed,
		elevatedEdges,
		elementToContainer,
		parentIndex,
		topoKey,
		structureKey,
		baseKey,
		isNewStructure,
		isNewBaseStructure,
		viewChanged,
		topologyChanged,
		deferCollapse,
		needsElkForExpand,
		collapseChanged,
		visibleNodes,
		aggregatedEdges,
		hiddenEdgeTypes,
		prevExpandedSizes,
		prevChildPositions,
		currentView,
		topologyId: topology.id ?? '',
		needsElk,
		isViewTransition
	};
}
