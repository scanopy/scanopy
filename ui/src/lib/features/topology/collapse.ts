/**
 * Collapse state management for C4 Context Zoom.
 *
 * Tracks which containers are collapsed, persists to localStorage,
 * and provides edge aggregation for collapsed containers.
 *
 * Supports leveled collapse/expand with 5 levels:
 *   1 = Fully collapsed
 *   2 = Containers expanded, subcontainers collapsed
 *   3 = Subcontainers expanded (except collapsed-by-default and infrastructure)
 *   4 = As 3, with the manager boxes on host cards opened
 *   5 = Fully expanded: collapsed-by-default and infrastructure containers too, always last
 */

import { get, writable } from 'svelte/store';
import { browser } from '$app/env';
import type { TopologyEdge, TopologyNode } from './types/base';
import type { ContainerTypeMetadata } from '#lib/shared/stores/metadata.ts';
import { isDisabledEdge } from './layout/edge-classification';
import { persistedSet, toggleInSet } from '#lib/shared/stores/persisted-set.ts';

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

export type CollapseLevel = 1 | 2 | 3 | 4 | 5;

interface ContainerTypesAccessor {
	getMetadata: (id: string | null) => ContainerTypeMetadata;
}

// ---------------------------------------------------------------------------
// Manager boxes on host cards
// ---------------------------------------------------------------------------

/**
 * Manager boxes on host cards the viewer opened, keyed by `inlineGroupKey`. A box is collapsed
 * unless its key is here, so boxes start collapsed; collapse level 4 opens every box. Persisted,
 * so a box the viewer opened stays open across reloads.
 */
export const expandedInlineGroups = persistedSet('scanopy_topology_expanded_inline_groups');

/**
 * Key for one manager box on one card. Starts with the card's node id followed by `|`, which is
 * how the re-layout finds the card whose height a toggle changed.
 */
export function inlineGroupKey(nodeId: string, groupId: string): string {
	return `${nodeId}|${groupId}`;
}

/** Every manager box drawn in `nodes`: one key per (element, inline group). */
export function allInlineGroupKeys(nodes: TopologyNode[]): string[] {
	const keys = new Set<string>();
	for (const node of nodes) {
		if (node.node_type !== 'Element') continue;
		const groups = (node as { inline_groups?: { group_id: string }[] }).inline_groups ?? [];
		for (const g of groups) keys.add(inlineGroupKey(node.id, g.group_id));
	}
	return [...keys];
}

export function toggleInlineGroup(nodeId: string, groupId: string): void {
	toggleInSet(expandedInlineGroups, inlineGroupKey(nodeId, groupId));
}

/** The manager boxes in `nodes` that are currently collapsed. */
function collapsedBoxKeys(nodes: TopologyNode[]): Set<string> {
	const expanded = get(expandedInlineGroups);
	return new Set(allInlineGroupKeys(nodes).filter((key) => !expanded.has(key)));
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
			if (num >= 1 && num <= 5) return num as CollapseLevel;
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

const SCOPE_STORAGE_KEY = 'scanopy_topology_collapse_scope';

/** Names the topology and view a persisted collapsed set and level were saved for. */
export function collapseScope(topologyId: string, view: string): string {
	return `${topologyId}|${view}`;
}

function loadScopeFromStorage(): string | null {
	if (!browser) return null;
	try {
		return localStorage.getItem(SCOPE_STORAGE_KEY);
	} catch {
		return null;
	}
}

let savedScope = loadScopeFromStorage();

/**
 * The scope the collapse state describes now: the one it was persisted with until a topology has
 * rendered in this tab, then the last one rendered. `null` when none was recorded, which includes
 * state saved before the scope was persisted.
 *
 * Read live, not once at load. The viewer is unmounted while another site's topology is fetched
 * and mounts again with no memory of what it drew, so "which topology and view is this state
 * for" has to be answerable on any mount, not only the tab's first.
 */
export function currentCollapseScope(): string | null {
	return savedScope;
}

/** Record the topology and view the persisted collapse state now describes. */
export function saveCollapseScope(scope: string): void {
	if (scope === savedScope) return;
	savedScope = scope;
	if (!browser) return;
	try {
		localStorage.setItem(SCOPE_STORAGE_KEY, scope);
	} catch {
		// ignore
	}
}

const OPEN_CANDIDATES_STORAGE_KEY = 'scanopy_topology_open_candidates';

function loadOpenCandidatesFromStorage(): Set<string> {
	if (!browser) return new Set();
	try {
		const stored = localStorage.getItem(OPEN_CANDIDATES_STORAGE_KEY);
		if (stored !== null) {
			const arr = JSON.parse(stored);
			if (Array.isArray(arr)) return new Set(arr);
		}
	} catch {
		// ignore
	}
	return new Set();
}

let openCandidates = loadOpenCandidatesFromStorage();
let savedOpenCandidates = [...openCandidates].sort().join(',');

/**
 * The auto-collapse candidates that were open when the collapse state was last saved: the
 * containers the user opened by hand, or that a chosen level leaves open.
 *
 * The persisted collapsed set cannot say this by itself. A candidate missing from it is either one
 * the user opened or one that did not exist yet, and auto-collapse must leave the first alone and
 * collapse the second. Without this record a reload, or a remount of the viewer, put every
 * hand-opened container back.
 */
export function currentOpenCandidates(): ReadonlySet<string> {
	return openCandidates;
}

/** Record the auto-collapse candidates that are open now. Writes only when the set changed. */
export function saveOpenCandidates(ids: Iterable<string>): void {
	const list = [...ids].sort();
	const joined = list.join(',');
	if (joined === savedOpenCandidates) return;
	savedOpenCandidates = joined;
	openCandidates = new Set(list);
	if (!browser) return;
	try {
		localStorage.setItem(OPEN_CANDIDATES_STORAGE_KEY, JSON.stringify(list));
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
// first render gets inferred as level 5 and overrides the default.
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
 * The collapsed set to open with when the viewer mounts on a graph at scale-collapse size and the
 * collapse state describes another topology or view. `null` when the state applies as it stands.
 *
 * Container ids are per topology and view, so a set saved in one names nothing in another.
 * Stripped of those ids it is empty, and an empty set reads as "everything expanded": a reload or
 * a shared link into Workloads after viewing L2, or a switch from a small site to a 4,000-host
 * one, mounted every node. A view switch inside a session opens a view of this size at level 1,
 * and every other way into it does the same.
 */
export function collapsedForEntryFromOtherScope(
	storedScope: string | null,
	scope: string,
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor
): Set<string> | null {
	if (storedScope === scope) return null;
	const candidates = scaleCollapseCandidates(allNodes, containerTypesStore);
	return candidates.size > 0 ? candidates : null;
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
			// stay collapsed at every level except 5 (fully expanded).
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
		case 3:
		case 4: {
			// Subcontainers expanded except collapsed-by-default and infrastructure. Level 4
			// differs only in the manager boxes on host cards, which are not containers (see
			// `collapsedBoxKeys`).
			const collapsed = new Set<string>();
			for (const node of containers) {
				if (isAutoCollapseContainer(node, containerTypesStore, infraRuleId)) {
					collapsed.add(node.id);
				}
			}
			return collapsed;
		}
		case 5: {
			// Every container expanded, the auto-collapse ones last of all.
			return new Set();
		}
	}
}

/**
 * Infer the closest collapse level from the current collapsed set, and from the manager boxes.
 * Containers matching level 3 read as level 4 when every box is open. Every container open reads
 * as level 5 unless boxes are closed: then, with no auto-collapse container to tell 5 from 3, it
 * reads as 3, so expanding still has the boxes to open.
 */
export function inferCurrentLevel(
	collapsed: Set<string>,
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null
): CollapseLevel {
	const boxesOpen = collapsedBoxKeys(allNodes).size === 0;
	const hasBoxes = allInlineGroupKeys(allNodes).length > 0;
	const level = inferContainerLevel(collapsed, allNodes, containerTypesStore, infraRuleId);
	const matchesThree = setsEqual(
		collapsed,
		computeCollapsedForLevel(3, allNodes, containerTypesStore, infraRuleId)
	);
	if (level === 5 && (boxesOpen || !matchesThree)) return 5;
	if (matchesThree) return hasBoxes && boxesOpen ? 4 : 3;
	return level;
}

/** The collapse level the containers alone match (1, 2, 3 or 5). */
function inferContainerLevel(
	collapsed: Set<string>,
	allNodes: TopologyNode[],
	containerTypesStore: ContainerTypesAccessor,
	infraRuleId: string | null
): CollapseLevel {
	// Check from most expanded to most collapsed. Level 4 has level 3's containers.
	for (const level of [5, 3, 2, 1] as CollapseLevel[]) {
		const expected = computeCollapsedForLevel(level, allNodes, containerTypesStore, infraRuleId);
		if (setsEqual(collapsed, expected)) return level;
	}
	// No exact match — determine closest by checking which level would expand more
	// If nothing is collapsed, that's level 5
	if (collapsed.size === 0) return 5;
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
 * Get the IDs of auto-collapse containers (for marking as seen when at level 5).
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
 * auto-collapse IDs that should be marked as "seen" (relevant at level 5).
 */
/**
 * The next level in `direction` that would actually change the diagram, or `null` if none would.
 *
 * A level's set can be indistinguishable from the current one, so stepping blindly produces
 * presses that appear to do nothing. It happens whenever a *root* is collapsed: every deeper
 * level only adds containers already hidden inside it. The Application view shows this at its
 * worst — with no application tags every service lands in one `ApplicationUngrouped` root, which
 * is `collapsed_by_default`, so levels 4 down to 1 all render identically and only level 5 differs.
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
	const renderedContainers = get(collapsedContainers);
	// Start from what is actually drawn, not from the stored level. The two can disagree —
	// `prepare` infers a level before auto-collapse has been applied, so a scale-collapsed load
	// keeps a lower number than its diagram — and stepping from a stale number walks the wrong
	// way: pressing Collapse from a fully-collapsed graph "advanced" to a level that expands it.
	const from = inferCurrentLevel(renderedContainers, allNodes, containerTypesStore, infraRuleId);
	const step = direction === 'collapse' ? -1 : 1;

	// Manager boxes count as collapsible entities alongside containers, so the same strict
	// superset rule keeps both directions honest for them too.
	const renderedBoxes = collapsedBoxKeys(allNodes);
	const rendered = new Set([...renderedContainers, ...renderedBoxes]);

	const hasBoxes = allInlineGroupKeys(allNodes).length > 0;
	const candidateFor = (level: CollapseLevel) =>
		new Set([
			...computeCollapsedForLevel(level, allNodes, containerTypesStore, infraRuleId),
			...boxesForLevel(level, direction, allNodes, renderedBoxes)
		]);

	for (let level = from + step; level >= 1 && level <= 5; level += step) {
		const candidate = candidateFor(level as CollapseLevel);
		// Collapse must only ever collapse and expand must only ever expand, whatever the
		// numbering implies. Requiring a strict superset (or subset) of what is drawn makes the
		// button's direction the guarantee, rather than something the level ordering happens to
		// deliver — and it is what disables the button once there is genuinely nowhere to go.
		const moves =
			direction === 'collapse'
				? isStrictSuperset(candidate, rendered)
				: isStrictSuperset(rendered, candidate);
		if (!moves) continue;
		// Level 4 differs from 3 only in manager boxes, so a view without them has no level 4 of
		// its own: a collapse lands on 3, and an expand runs on to the furthest level drawing the
		// same diagram (5, when the view has no auto-collapse containers either). Either way the
		// level is the one `inferCurrentLevel` reads back.
		if (direction === 'collapse') {
			if (level === 4 && !hasBoxes) continue;
			return level as CollapseLevel;
		}
		let landed = level;
		for (let next = level + 1; next <= 5; next++) {
			if (!setsEqual(candidateFor(next as CollapseLevel), candidate)) break;
			landed = next;
		}
		return landed as CollapseLevel;
	}
	return null;
}

/**
 * The manager boxes a step to `level` leaves collapsed. Levels 4 and 5 open every box. A collapse
 * to 1-3 closes every box. An expand to 1-3 leaves them as drawn, so it never closes a box the
 * viewer opened by hand.
 */
function boxesForLevel(
	level: CollapseLevel,
	direction: 'collapse' | 'expand',
	allNodes: TopologyNode[],
	renderedBoxes: Set<string>
): Set<string> {
	if (level >= 4) return new Set();
	return direction === 'collapse' ? new Set(allInlineGroupKeys(allNodes)) : renderedBoxes;
}

/** Write the boxes a step to `level` leaves open. */
function applyBoxesForLevel(
	level: CollapseLevel,
	direction: 'collapse' | 'expand',
	allNodes: TopologyNode[]
): void {
	if (level < 4 && direction === 'expand') return;
	// Only this view's boxes: the set also holds boxes the viewer opened in other views.
	const keys = allInlineGroupKeys(allNodes);
	expandedInlineGroups.update((set) => {
		const next = new Set(set);
		for (const key of keys) {
			if (level >= 4) next.add(key);
			else next.delete(key);
		}
		return next;
	});
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
	applyBoxesForLevel(newLevel, 'expand', allNodes);
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

	applyBoxesForLevel(newLevel, 'collapse', allNodes);
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
		// A type the view disables never draws expanded (`buildFlowEdges` drops it), so it can't
		// draw merged into a collapsed container either.
		if (isDisabledEdge(edge)) continue;

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
