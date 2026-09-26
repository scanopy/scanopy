/**
 * Collapse state management for C4 Context Zoom.
 *
 * Tracks which containers are collapsed, persists to localStorage,
 * and provides edge aggregation for collapsed containers.
 *
 * Supports leveled collapse/expand with 4 levels:
 *   1 = Fully collapsed
 *   2 = Containers expanded, subcontainers collapsed
 *   3 = Subcontainers expanded (except collapsed-by-default and infrastructure)
 *   4 = Fully expanded
 */

import { get, writable } from 'svelte/store';
import { browser } from '$app/environment';
import type { TopologyEdge, TopologyNode } from './types/base';
import type { ContainerTypeMetadata } from '$lib/shared/stores/metadata';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface AggregatedEdge {
	id: string;
	source: string;
	target: string;
	count: number;
	originalEdges: TopologyEdge[];
}

export type CollapseLevel = 1 | 2 | 3 | 4;

interface ContainerTypesAccessor {
	getMetadata: (id: string | null) => ContainerTypeMetadata;
}

// ---------------------------------------------------------------------------
// Store & persistence
// ---------------------------------------------------------------------------

const COLLAPSED_STORAGE_KEY = 'scanopy_topology_collapsed_containers';
const LEVEL_STORAGE_KEY = 'scanopy_topology_collapse_level';

function loadCollapsedFromStorage(): Set<string> {
	if (!browser) return new Set();
	try {
		const stored = localStorage.getItem(COLLAPSED_STORAGE_KEY);
		if (stored !== null) {
			const arr = JSON.parse(stored);
			if (Array.isArray(arr)) return new Set(arr);
		}
	} catch (error) {
		console.warn('Failed to load collapsed containers from localStorage:', error);
	}
	return new Set();
}

function saveCollapsedToStorage(collapsed: Set<string>): void {
	if (!browser) return;
	try {
		localStorage.setItem(COLLAPSED_STORAGE_KEY, JSON.stringify([...collapsed]));
	} catch (error) {
		console.error('Failed to save collapsed containers to localStorage:', error);
	}
}

function loadLevelFromStorage(): CollapseLevel | null {
	if (!browser) return null;
	try {
		const stored = localStorage.getItem(LEVEL_STORAGE_KEY);
		if (stored !== null) {
			const num = parseInt(stored, 10);
			if (num >= 1 && num <= 4) return num as CollapseLevel;
		}
	} catch {
		// ignore
	}
	return null;
}

function saveLevelToStorage(level: CollapseLevel): void {
	if (!browser) return;
	try {
		localStorage.setItem(LEVEL_STORAGE_KEY, String(level));
	} catch {
		// ignore
	}
}

export const collapsedContainers = writable<Set<string>>(loadCollapsedFromStorage());

// Persist on change (skip first subscription call)
let collapsedInitialized = false;
if (browser) {
	collapsedContainers.subscribe((value) => {
		if (collapsedInitialized) {
			saveCollapsedToStorage(value);
		}
		collapsedInitialized = true;
	});
}

const initialStoredLevel = loadLevelFromStorage();

// Module-load snapshot: was a level persisted in localStorage when the tab opened?
// The pipeline uses this to distinguish a truly fresh session from one where the
// user previously chose a level. Without this signal, an empty collapsed set on
// first render gets inferred as level 4 and overrides the default.
export const hadStoredLevelOnLoad = initialStoredLevel !== null;

export const collapseLevel = writable<CollapseLevel>(initialStoredLevel ?? 3);

if (browser) {
	let levelInitialized = false;
	collapseLevel.subscribe((value) => {
		if (levelInitialized) {
			saveLevelToStorage(value);
		}
		levelInitialized = true;
	});
}

// ---------------------------------------------------------------------------
// Level computation
// ---------------------------------------------------------------------------

/**
 * Check if a container node is an "auto-collapse" candidate:
 * either collapsed_by_default or matches the infrastructure rule.
 */
function isAutoCollapseContainer(
	node: TopologyNode,
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null
): boolean {
	if (node.node_type !== 'Container') return false;
	const data = node as Record<string, unknown>;
	const ct = data.container_type as string | undefined;
	if (ct && containerTypesStore.getMetadata(ct).collapsed_by_default === true) return true;
	if (infraRuleId && data.element_rule_id === infraRuleId) return true;
	return false;
}

/**
 * Element count above which containers start collapsed regardless of type.
 *
 * At a few hundred hosts the expanded graph mounts thousands of element cards —
 * the dominant cost of a cold load — and at the zoom needed to see it all their
 * contents are illegible anyway.
 */
const SCALE_COLLAPSE_ELEMENTS = 300;

/** Whether this graph is large enough for containers to start collapsed. */
export function isScaleCollapsed(allNodes: TopologyNode[]): boolean {
	return allNodes.filter((n) => n.node_type === 'Element').length >= SCALE_COLLAPSE_ELEMENTS;
}

/**
 * Containers that scale-collapse applies to, or an empty set below the threshold.
 *
 * This is the single owner of that policy. Both the automatic path
 * (`prepare.ts` `applyAutoCollapse`, on load) and the manual ladder
 * (`computeCollapsedForLevel`, on a Collapse/Expand press) must agree on it —
 * when they did not, the ladder recomputed a set with no knowledge of
 * scale-collapse, so the first Collapse press *expanded* every host container
 * and `inferCurrentLevel` fell through to its default, leaving the level
 * indicator describing a graph that was never drawn.
 */
export function scaleCollapseCandidates(
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	/** Ignore the size threshold — used by level 1, which means "all of these" at any size. */
	ignoreThreshold = false
): Set<string> {
	if (!ignoreThreshold && !isScaleCollapsed(allNodes)) return new Set();
	return new Set(
		allNodes
			.filter((n) => {
				if (n.node_type !== 'Container') return false;
				const ct = (n as Record<string, unknown>).container_type as string | undefined;
				return containerTypesStore.getMetadata(ct ?? 'Subnet').is_collapsible;
			})
			.map((n) => n.id)
	);
}

/**
 * Compute the set of container IDs that should be collapsed at a given level.
 *
 * Every level must produce a visibly different diagram, which is why scale collapse is
 * deliberately *not* folded in here. Scale collapse closes every collapsible container, so
 * unioning it into levels 2 and 3 made them identical to level 1 — three rungs of the ladder
 * rendering the same graph, and the indicator reading 3 while the diagram was fully collapsed.
 * Instead, level 1 *is* that state, so a scale-collapsed load simply infers level 1 and every
 * step from there genuinely expands.
 */
export function computeCollapsedForLevel(
	level: CollapseLevel,
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null
): Set<string> {
	const containers = allNodes.filter((n) => n.node_type === 'Container');

	switch (level) {
		case 1: {
			// Every container that *can* be collapsed. Restricted to collapsible ones so this
			// is exactly the set `applyAutoCollapse` produces at scale — that identity is what
			// lets `inferCurrentLevel` recognise a scale-collapsed load as level 1 instead of
			// falling through to its default. Listing a container that cannot be collapsed
			// would also be a lie about the rendered state.
			return scaleCollapseCandidates(allNodes, containerTypesStore, true);
		}
		case 2: {
			// Root containers expanded except auto-collapse ones (collapsed_by_default
			// / infrastructure); all subcontainers collapsed. Auto-collapse containers
			// stay collapsed at every level except 4 (fully expanded).
			const collapsed = new Set<string>();
			for (const node of containers) {
				const data = node as Record<string, unknown>;
				const ct = data.container_type as string | undefined;
				const isSub = ct ? containerTypesStore.getMetadata(ct).is_subcontainer : false;
				if (isSub || isAutoCollapseContainer(node, containerTypesStore, infraRuleId)) {
					collapsed.add(node.id);
				}
			}
			return collapsed;
		}
		case 3: {
			// Subcontainers expanded except collapsed-by-default and infrastructure
			const collapsed = new Set<string>();
			for (const node of containers) {
				if (isAutoCollapseContainer(node, containerTypesStore, infraRuleId)) {
					collapsed.add(node.id);
				}
			}
			return collapsed;
		}
		case 4: {
			// Everything expanded
			return new Set();
		}
	}
}

/**
 * Infer the closest collapse level from the current collapsed set.
 * Returns exact match only; defaults to 1 if no match.
 */
export function inferCurrentLevel(
	collapsed: Set<string>,
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null
): CollapseLevel {
	// Check from most expanded to most collapsed
	for (const level of [4, 3, 2, 1] as CollapseLevel[]) {
		const expected = computeCollapsedForLevel(level, allNodes, containerTypesStore, infraRuleId);
		if (setsEqual(collapsed, expected)) return level;
	}
	// No exact match — determine closest by checking which level would expand more
	// If nothing is collapsed, that's level 4
	if (collapsed.size === 0) return 4;
	// If all containers are collapsed, that's level 1
	const allContainers = allNodes.filter((n) => n.node_type === 'Container');
	if (allContainers.every((n) => collapsed.has(n.id))) return 1;
	// Default: level 3 (collapsed-by-default subcontainers only).
	// This is the natural state — persisted set may not exactly match
	// any level due to stale IDs or manual user collapses.
	return 3;
}

function setsEqual(a: Set<string>, b: Set<string>): boolean {
	if (a.size !== b.size) return false;
	for (const item of a) {
		if (!b.has(item)) return false;
	}
	return true;
}

/**
 * Get the IDs of auto-collapse containers (for marking as seen when at level 4).
 */
export function getAutoCollapseIds(
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null
): string[] {
	return allNodes
		.filter((n) => isAutoCollapseContainer(n, containerTypesStore, infraRuleId))
		.map((n) => n.id);
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export function toggleCollapse(containerId: string, allNodes?: TopologyNode[]): void {
	collapsedContainers.update((set) => {
		const next = new Set(set);
		if (next.has(containerId)) {
			// Expanding: also expand child containers (subgroups)
			next.delete(containerId);
			if (allNodes) {
				for (const node of allNodes) {
					if (node.node_type === 'Container') {
						const parentId = (node as Record<string, unknown>).parent_container_id as
							string | undefined;
						if (parentId === containerId) next.delete(node.id);
					}
				}
			}
		} else {
			// Collapsing: also collapse child containers (subgroups)
			next.add(containerId);
			if (allNodes) {
				for (const node of allNodes) {
					if (node.node_type === 'Container') {
						const parentId = (node as Record<string, unknown>).parent_container_id as
							string | undefined;
						if (parentId === containerId) next.add(node.id);
					}
				}
			}
		}
		return next;
	});
}

/**
 * Step expand level up by one. Returns the new level and the set of
 * auto-collapse IDs that should be marked as "seen" (relevant at level 4).
 */
/**
 * The next level in `direction` that would actually change the diagram, or `null` if none would.
 *
 * A level's set can be indistinguishable from the current one, so stepping blindly produces
 * presses that appear to do nothing. It happens whenever a *root* is collapsed: every deeper
 * level only adds containers already hidden inside it. The Application view shows this at its
 * worst — with no application tags every service lands in one `ApplicationUngrouped` root, which
 * is `collapsed_by_default`, so levels 3, 2 and 1 all render identically and only level 4 differs.
 *
 * Skipping is preferred to redefining the levels: how many distinct states a view *has* depends
 * on its container depth, so no fixed four-rung ladder can always fill them, and changing what a
 * level means would desynchronise `inferCurrentLevel` from what the loader produces.
 */
export function nextEffectiveLevel(
	direction: 'collapse' | 'expand',
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null
): CollapseLevel | null {
	const rendered = get(collapsedContainers);
	// Start from what is actually drawn, not from the stored level. The two can disagree —
	// `prepare` infers a level before auto-collapse has been applied, so a scale-collapsed load
	// keeps a lower number than its diagram — and stepping from a stale number walks the wrong
	// way: pressing Collapse from a fully-collapsed graph "advanced" to a level that expands it.
	const from = inferCurrentLevel(rendered, allNodes, containerTypesStore, infraRuleId);
	const step = direction === 'collapse' ? -1 : 1;

	for (let level = from + step; level >= 1 && level <= 4; level += step) {
		const candidate = computeCollapsedForLevel(
			level as CollapseLevel,
			allNodes,
			containerTypesStore,
			infraRuleId
		);
		// Collapse must only ever collapse and expand must only ever expand, whatever the
		// numbering implies. Requiring a strict superset (or subset) of what is drawn makes the
		// button's direction the guarantee, rather than something the level ordering happens to
		// deliver — and it is what disables the button once there is genuinely nowhere to go.
		const moves =
			direction === 'collapse'
				? isStrictSuperset(candidate, rendered)
				: isStrictSuperset(rendered, candidate);
		if (moves) return level as CollapseLevel;
	}
	return null;
}

/** True when `a` contains every member of `b` and at least one more. */
function isStrictSuperset(a: Set<string>, b: Set<string>): boolean {
	if (a.size <= b.size) return false;
	for (const value of b) {
		if (!a.has(value)) return false;
	}
	return true;
}

export function stepExpand(
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null,
	/**
	 * Called with the auto-collapse ids *before* the stores are written.
	 *
	 * Writing `collapsedContainers` notifies its subscribers synchronously, and the viewer's
	 * subscriber runs the pipeline far enough to reach `prepare` before `set` returns — so by the
	 * time this function returned its ids, `applyAutoCollapse` had already re-collapsed every
	 * `collapsed_by_default` container it was about to be told to leave alone. The level advanced
	 * to 4, the collapsed set did not, `collapseChanged` came out false, no layout ran, and the
	 * diagram stayed on the previous level with anything new to it unsized at 0x0. Pressing expand
	 * again worked because the ids had been marked seen by then, which is what made this look like
	 * a sizing bug rather than an ordering one.
	 */
	onBeforeApply?: (idsLeftExpanded: string[]) => void
): { newLevel: CollapseLevel; autoCollapseIds: string[] } {
	const newLevel = nextEffectiveLevel('expand', allNodes, containerTypesStore, infraRuleId);
	if (newLevel === null) return { newLevel: get(collapseLevel), autoCollapseIds: [] };

	const collapsed = computeCollapsedForLevel(newLevel, allNodes, containerTypesStore, infraRuleId);
	const idsLeftExpanded = autoCollapseCandidatesLeftExpanded(
		allNodes,
		containerTypesStore,
		infraRuleId,
		collapsed
	);
	onBeforeApply?.(idsLeftExpanded);

	collapsedContainers.set(collapsed);
	collapseLevel.set(newLevel);

	return { newLevel, autoCollapseIds: idsLeftExpanded };
}

/**
 * Candidates the chosen level deliberately leaves open.
 *
 * Auto-collapse — both `collapsed_by_default` and scale collapse — is one-shot per container,
 * gated on `seenAutoCollapseIds`. That set lives in memory while `collapsedContainers` is
 * persisted to `localStorage`, so after a reload the stored graph is collapsed and the "already
 * had its shot" record is empty. Stepping the ladder then wrote the new level's collapsed set and
 * the very next pipeline run put every scale-collapse candidate straight back.
 *
 * The visible result was a ladder that took two presses per rung: the first moved the number while
 * the graph stayed put, and the second moved the graph while the number stayed put — because
 * `nextEffectiveLevel` re-derives the current level from the collapsed set, which had not moved.
 *
 * Choosing a level is an explicit instruction about every container that level covers, so
 * everything it leaves open is marked seen and auto-collapse leaves it alone. That is the same
 * promise `applyAutoCollapse` already makes for a container the user expands by hand.
 */
function autoCollapseCandidatesLeftExpanded(
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null,
	collapsed: Set<string>
): string[] {
	const candidates = new Set<string>([
		// `ignoreThreshold` so the set is the same whether or not this graph is large enough to
		// scale-collapse — below the threshold there is simply nothing to protect against.
		...scaleCollapseCandidates(allNodes, containerTypesStore, true),
		...getAutoCollapseIds(allNodes, containerTypesStore, infraRuleId)
	]);
	return [...candidates].filter((id) => !collapsed.has(id));
}

export function stepCollapse(
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null,
	/** See `stepExpand` — same ordering and same one-shot bookkeeping. */
	onBeforeApply?: (idsLeftExpanded: string[]) => void
): { newLevel: CollapseLevel } {
	const newLevel = nextEffectiveLevel('collapse', allNodes, containerTypesStore, infraRuleId);
	if (newLevel === null) return { newLevel: get(collapseLevel) };

	const collapsed = computeCollapsedForLevel(newLevel, allNodes, containerTypesStore, infraRuleId);
	onBeforeApply?.(
		autoCollapseCandidatesLeftExpanded(allNodes, containerTypesStore, infraRuleId, collapsed)
	);

	collapsedContainers.set(collapsed);
	collapseLevel.set(newLevel);
	return { newLevel };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Build a mapping from element node ID to its parent container ID.
 */
export function buildElementToContainer(nodes: TopologyNode[]): Map<string, string> {
	const map = new Map<string, string>();
	for (const node of nodes) {
		if (node.node_type === 'Element') {
			const parentId =
				(node as Record<string, unknown>).container_id ??
				(node as Record<string, unknown>).subnet_id;
			if (typeof parentId === 'string') {
				map.set(node.id, parentId);
			}
		}
	}
	return map;
}

/**
 * Resolve a node ID to its nearest collapsed ancestor.
 * Walks up the parent chain (element → container → parent_container → …)
 * and returns the outermost collapsed container, or null if none.
 */
export function resolveCollapsedAncestor(
	nodeId: string,
	collapsed: Set<string>,
	parentMap: Map<string, string>
): string | null {
	let current = nodeId;
	let result: string | null = null;
	const visited = new Set<string>();

	while (current && !visited.has(current)) {
		visited.add(current);
		if (collapsed.has(current)) {
			result = current;
		}
		const parent = parentMap.get(current);
		if (!parent || parent === current) break;
		current = parent;
	}

	return result;
}

/**
 * Build a parent map covering both Element and Container nodes.
 * Elements map to their container_id/subnet_id, containers map to parent_container_id.
 */
export function buildFullParentMap(nodes: TopologyNode[]): Map<string, string> {
	const parentMap = new Map<string, string>();
	for (const node of nodes) {
		if (node.node_type === 'Element') {
			const parentId =
				(node as Record<string, unknown>).container_id ??
				(node as Record<string, unknown>).subnet_id;
			if (typeof parentId === 'string') {
				parentMap.set(node.id, parentId);
			}
		} else if (node.node_type === 'Container') {
			const parentId = (node as Record<string, unknown>).parent_container_id as string | undefined;
			if (parentId) {
				parentMap.set(node.id, parentId);
			}
		}
	}
	return parentMap;
}

/**
 * Compute aggregated edges for collapsed containers.
 *
 * - Resolves each edge endpoint to its nearest collapsed ancestor
 *   (works for elements, subcontainers, and containers at any nesting depth)
 * - Groups edges between the same pair of (resolved) nodes
 * - Returns aggregated edges with count
 */
export function computeCollapsedEdges(
	edges: TopologyEdge[],
	collapsed: Set<string>,
	nodes: TopologyNode[],
	hiddenEdgeTypes: string[],
	prebuiltParentMap?: Map<string, string>
): AggregatedEdge[] {
	if (collapsed.size === 0) return [];

	const parentMap = prebuiltParentMap ?? buildFullParentMap(nodes);

	const hiddenSet = new Set(hiddenEdgeTypes);

	// Cache resolved ancestors
	const ancestorCache = new Map<string, string | null>();
	function getCollapsedAncestor(nodeId: string): string | null {
		if (ancestorCache.has(nodeId)) return ancestorCache.get(nodeId)!;
		const result = resolveCollapsedAncestor(nodeId, collapsed, parentMap);
		ancestorCache.set(nodeId, result);
		return result;
	}

	// Group by resolved (source, target) pair
	const groups = new Map<string, TopologyEdge[]>();

	for (const edge of edges) {
		let src = edge.source as string;
		let tgt = edge.target as string;

		// Remap to nearest collapsed ancestor
		const srcAncestor = getCollapsedAncestor(src);
		if (srcAncestor) src = srcAncestor;
		const tgtAncestor = getCollapsedAncestor(tgt);
		if (tgtAncestor) tgt = tgtAncestor;

		// Skip if neither endpoint was remapped (edge is fully outside collapsed containers)
		if (!srcAncestor && !tgtAncestor) continue;

		// Skip self-loops (both endpoints inside same collapsed container)
		if (src === tgt) continue;

		// Normalize key so (A,B) and (B,A) are the same group
		const key = src < tgt ? `${src}->${tgt}` : `${tgt}->${src}`;

		let group = groups.get(key);
		if (!group) {
			group = [];
			groups.set(key, group);
		}
		group.push(edge);
	}

	const result: AggregatedEdge[] = [];
	let idx = 0;

	for (const [key, groupEdges] of groups) {
		// If all edges in this group are hidden types, skip
		const visibleEdges = groupEdges.filter((e) => !hiddenSet.has(e.edge_type));
		if (visibleEdges.length === 0) continue;

		const [src, tgt] = key.split('->');
		result.push({
			id: `collapsed-edge-${idx++}`,
			source: src,
			target: tgt,
			count: visibleEdges.length,
			originalEdges: visibleEdges
		});
	}

	return result;
}
