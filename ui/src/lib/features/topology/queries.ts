/**
 * TanStack Query hooks for Topology
 *
 * Note: UI state (selected nodes/edges, options panel, localStorage preferences)
 * remains in local component state or a separate UI store.
 */

import { createQuery, createMutation } from '@tanstack/svelte-query';
import { queryClient, queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { unwrapData } from '$lib/api/query-helpers';
import type { Topology, TopologyEdge, TopologyOptions, RenderableTopology } from './types/base';
import type { ElementGraphRule } from './types/grouping';
import type { Organization } from '$lib/features/organizations/types';
import throttle from 'just-throttle';
import { writable, derived, get } from 'svelte/store';
import { registerUrlOwner, syncUrl } from '$lib/shared/stores/url-state';
import { UNTAGGED_SENTINEL } from './interactions';
import { getDefaultHiddenEdgeTypes } from './layout/edge-classification';
import type { components } from '$lib/api/schema';
import viewsJson from '$lib/data/views.json';
import { declaredMetadataFilters, filtersFor, type HiddenMetadataValues } from './view-filters';

export type TopologyView = components['schemas']['TopologyView'];

/** Strip UI-only sentinel values from options before sending to the API */
export function sanitizeOptionsForApi(options: TopologyOptions): TopologyOptions {
	const isSentinel = (id: string) => id === UNTAGGED_SENTINEL;
	return {
		...options,
		local: {
			...options.local,
			tag_filter: Object.fromEntries(
				Object.entries(options.local?.tag_filter ?? {}).map(([view, tf]) => [
					view,
					{
						hidden_tag_ids: Object.fromEntries(
							Object.entries(tf?.hidden_tag_ids ?? {}).map(([entityType, ids]) => [
								entityType,
								(ids ?? []).filter((id) => !isSentinel(id))
							])
						)
					}
				])
			)
		}
	};
}

type TopologyLocalOptions = components['schemas']['TopologyLocalOptions'];
type EdgeTypeDiscriminants = components['schemas']['EdgeTypeDiscriminants'];
type TopologyTagFilter = components['schemas']['TopologyTagFilter'];

/** Get the org's use case from the query cache, defaulting to 'other' */
export function getOrgUseCase(): string {
	const org = queryClient.getQueryData<Organization>(queryKeys.organizations.current());
	return org?.use_case ?? 'other';
}

/** Scan a set of element rules for the ByServiceCategory rule flagged is_infra_rule. */
export function findInfraRuleId(
	elementRules: TopologyOptions['request']['element_rules'] | undefined
): string | null {
	for (const rule of elementRules ?? []) {
		if (
			typeof rule.rule === 'object' &&
			'ByServiceCategory' in rule.rule &&
			rule.rule.ByServiceCategory.is_infra_rule
		) {
			return rule.id;
		}
	}
	return null;
}

/**
 * Find the infrastructure rule ID from the current topology options store
 * by looking for the ByServiceCategory rule with is_infra_rule: true.
 *
 * Reads the global store, which is hydrated out-of-band from the topology bundle
 * that drives the layout pipeline — so on a site switch it can briefly lag.
 * Layout/auto-collapse code must use {@link getInfrastructureRuleIdForTopology}
 * with the bundle it is rendering; this store-based variant is for UI surfaces
 * (e.g. the grouping-rule editor) that have no topology in hand.
 */
export function getInfrastructureRuleId(): string | null {
	return findInfraRuleId(get(topologyOptionsStore).request.element_rules);
}

/**
 * Infrastructure rule ID derived from a specific topology bundle's options
 * rather than the global store. Always in sync with the nodes being laid out,
 * so it stays correct across a same-view site switch.
 */
export function getInfrastructureRuleIdForTopology(
	topology: Topology | RenderableTopology
): string | null {
	return findInfraRuleId(topology.options?.request?.element_rules);
}

const ALL_VIEWS: TopologyView[] = viewsJson.map((p) => p.id as TopologyView);

/**
 * What one entity's hide-set becomes when its filters are cleared: every filter the view declares,
 * emptied, plus an empty list for any stored key the view no longer declares.
 *
 * Explicit empty lists rather than removed keys. An absent key means "no opinion", which the
 * server refills from the view's defaults on the next read
 * (`TopologyRequestOptions::merge_missing_hide_defaults`); an empty list means "show everything",
 * which it leaves alone. Dropping the key would make a cleared filter re-hide itself on reload —
 * so the view's declared filters are the source here, not just the keys that happen to be stored.
 *
 * Defaults are cleared like anything else. A default Clear all put straight back was a button that
 * visibly did nothing, and it left a user who wanted to see unlinked ports no way to get them.
 */
export function clearedHideSetFor(
	view: string,
	entityType: string,
	existing: Record<string, string[]> | undefined
): Record<string, string[]> {
	const cleared: Record<string, string[]> = {};
	for (const filter of filtersFor(view, entityType)) {
		cleared[filter.filter_type] = [];
	}
	for (const filterType of Object.keys(existing ?? {})) cleared[filterType] = [];
	return cleared;
}

/**
 * A whole view's hide-set with every filter cleared: each entity some filter covers, and each
 * entity with a stored entry, through `clearedHideSetFor`.
 */
export function clearedViewHideSet(
	view: string,
	existing: HiddenMetadataValues | undefined
): HiddenMetadataValues {
	const cleared: HiddenMetadataValues = {};
	const entityTypes = new Set([
		...declaredMetadataFilters(view).flatMap((f) => f.entities as string[]),
		...Object.keys(existing ?? {})
	]);
	for (const entityType of entityTypes) {
		cleared[entityType] = clearedHideSetFor(view, entityType, existing?.[entityType]);
	}
	return cleared;
}

/**
 * Local options for a viewer with no saved preferences: share/embed viewers, and the store
 * before the first topology hydrates it. Mirrors `TopologyLocalOptions::default` on the backend,
 * with each view's hidden edge types read from the same view fixture.
 */
function defaultLocalOptions(): TopologyLocalOptions {
	return {
		hide_edge_types: Object.fromEntries(
			ALL_VIEWS.map((view) => [view, getDefaultHiddenEdgeTypes(view)])
		),
		no_fade_edges: false,
		bundle_edges: true,
		tag_filter: {},
		show_minimap: true
	};
}

/**
 * Request options held before any topology has hydrated the store: no rules and no hides.
 *
 * Product defaults live on the backend (they depend on the organization's use case), and every
 * topology row carries its own. The render pipeline waits for hydration before its first layout
 * (`topologyOptionsHydrated`), so this placeholder is never drawn.
 */
function emptyRequestOptions(): components['schemas']['TopologyRequestOptions'] {
	return {
		hide_entities: {},
		hide_metadata_values: {},
		container_rules: {},
		element_rules: [],
		element_sort: {}
	};
}

/**
 * Query hook for fetching all topologies
 */
export function useTopologiesQuery(enabled?: () => boolean) {
	return createQuery(() => ({
		queryKey: queryKeys.topology.all,
		queryFn: async () => {
			return unwrapData(
				await apiClient.GET('/api/v1/topologies', {
					params: { query: { limit: 0 } }
				})
			);
		},
		...(enabled ? { enabled } : {})
	}));
}

/**
 * Query hook for fetching the topology entity bundle for the selected view.
 *
 * `snapshot_id = undefined` → live entity set.
 * `snapshot_id = <id>`      → entity set as-of that snapshot's `taken_at`.
 *
 * Single endpoint, single code path. The cache key includes the snapshot id
 * so live vs snapshot data don't collide, and the view, since the server
 * builds only the requested view's graph. The live-update stream's topology
 * events (`topologyStream`) invalidate this query on live-view site updates.
 *
 * `enabled` exists so the caller can hold the fetch while the topology tab is
 * off-screen. Every tab in the app is mounted at once (inactive ones are hidden
 * with CSS), so without it this bundle — every host, service, ip-address and port
 * on the site — loads on app boot and, because it is invalidated by the
 * live-update stream, refetches on a throttle for the whole of every scan, on
 * pages that are not showing a graph. A disabled query is skipped by
 * `invalidateQueries` (which refetches active queries only) and refetches when it
 * is re-enabled and stale.
 */
export function useTopologyDataQuery(
	siteId: () => string | undefined,
	snapshotId: () => string | undefined,
	view: () => TopologyView,
	enabled?: () => boolean
) {
	return createQuery(() => ({
		...topologyDataQueryOptions(siteId() ?? '', snapshotId(), view()),
		enabled: () => !!siteId() && (enabled?.() ?? true),
		// A held site refetches only when the user asks (see `heldTopologySites`).
		refetchOnWindowFocus: () => !get(heldTopologySites).has(siteId() ?? '')
	}));
}

/**
 * How long a view's bundle stays cached once nothing reads it.
 *
 * Each view's bundle carries the site's whole entity set, so a cached view the user has left
 * costs as much memory as the one on screen. A minute covers switching back and forth without a
 * refetch, then lets the copy go.
 */
const TOPOLOGY_DATA_GC_MS = 60_000;

/**
 * The bundle for one view of a site. The server builds only that view's graph, so each view is
 * its own cache entry.
 */
function topologyDataQueryOptions(
	siteId: string,
	snapshotId: string | undefined,
	view: TopologyView
) {
	return {
		queryKey: queryKeys.topology.data(siteId, snapshotId, view),
		queryFn: async () => {
			if (!siteId) {
				throw new Error('No site ID provided');
			}
			return unwrapData(
				await apiClient.GET('/api/v1/topologies/data', {
					params: { query: { site_id: siteId, snapshot_id: snapshotId, view } }
				})
			);
		},
		gcTime: TOPOLOGY_DATA_GC_MS
	};
}

/**
 * Load a view's bundle before switching to it, so the current graph stays on screen until the
 * next one can replace it. Resolves at once when the bundle is already cached and fresh.
 */
export function loadTopologyData(
	siteId: string,
	snapshotId: string | undefined,
	view: TopologyView
) {
	return queryClient.ensureQueryData(topologyDataQueryOptions(siteId, snapshotId, view));
}

/**
 * One-shot GET that records the `OpenTopology` setup item ("user viewed their topology"), and
 * `ExploreViews` when `view` is not the default view.
 * Sends `mark_viewed=true`, which the backend only honours for a live topology that has hosts
 * once the first scan is done. Deliberately NOT routed through `useTopologyDataQuery` (which
 * never sets the flag) so it is only recorded from an explicit on-tab view, never from
 * background fetches on other tabs.
 */
export async function markTopologyViewed(siteId: string, view: TopologyView): Promise<void> {
	await apiClient.GET('/api/v1/topologies/data', {
		params: { query: { site_id: siteId, mark_viewed: true, view } }
	});
}

/**
 * Query hook for fetching a single topology
 */
export function useTopologyQuery(id: () => string | undefined) {
	return createQuery(() => ({
		queryKey: queryKeys.topology.detail(id() ?? ''),
		queryFn: async () => {
			const topologyId = id();
			if (!topologyId) {
				throw new Error('No topology ID provided');
			}
			return unwrapData(
				await apiClient.GET('/api/v1/topologies/{id}', {
					params: { path: { id: topologyId } }
				})
			);
		},
		enabled: () => !!id()
	}));
}

/**
 * Mutation hook for updating a topology's layout state.
 *
 * After the snapshot refactor, the PUT endpoint only accepts updates to
 * `nodes` / `edges` / `options` — there's no separate rebuild/refresh/lock
 * surface. Live data updates flow through the live-update stream
 * (see TopologyStream below); option/layout edits are saved by callers
 * via this mutation.
 */
export function useUpdateTopologyMutation() {
	return createMutation(() => ({
		mutationFn: async (topology: Topology) => {
			await apiClient.PUT('/api/v1/topologies/{id}', {
				params: { path: { id: topology.id } },
				body: topology
			});
			return topology;
		}
	}));
}

// === DISABLED: layout-override mutations ===
// Node position / container resize / edge handle reconnect are no longer
// persisted — the graph builds on request and ELK re-lays out every render, so
// there is no mechanism to save these. The backend endpoints are likewise
// commented out (see topology `handlers.rs`). Kept here for revival.
/*
/**
 * Mutation hook for updating a single node's position
 * Lightweight endpoint - only sends node ID and position instead of full topology
 * Fixes HTTP 413 errors on drag operations for large topologies
 *\/
export function useUpdateNodePositionMutation() {
	return createMutation(() => ({
		mutationFn: async (params: {
			topologyId: string;
			siteId: string;
			view: TopologyView;
			nodeId: string;
			position: { x: number; y: number };
		}) => {
			requireSuccess(
				await apiClient.POST('/api/v1/topologies/{id}/node-position', {
					params: { path: { id: params.topologyId } },
					body: {
						site_id: params.siteId,
						view: params.view,
						node_id: params.nodeId,
						position: params.position
					}
				})
			);
		}
	}));
}

/**
 * Mutation hook for updating a node's size and position (resize)
 * Lightweight endpoint - only sends node ID, size, and position instead of full topology
 * Fixes HTTP 413 errors on resize operations for large topologies
 *\/
export function useUpdateNodeResizeMutation() {
	return createMutation(() => ({
		mutationFn: async (params: {
			topologyId: string;
			siteId: string;
			view: TopologyView;
			nodeId: string;
			size: { x: number; y: number };
			position: { x: number; y: number };
		}) => {
			requireSuccess(
				await apiClient.POST('/api/v1/topologies/{id}/node-resize', {
					params: { path: { id: params.topologyId } },
					body: {
						site_id: params.siteId,
						view: params.view,
						node_id: params.nodeId,
						size: params.size,
						position: params.position
					}
				})
			);
		}
	}));
}

/**
 * Mutation hook for updating an edge's handles
 * Lightweight endpoint - only sends edge ID and handles instead of full topology
 * Fixes HTTP 413 errors on edge reconnect operations for large topologies
 *\/
export function useUpdateEdgeHandlesMutation() {
	return createMutation(() => ({
		mutationFn: async (params: {
			topologyId: string;
			siteId: string;
			view: TopologyView;
			edgeId: string;
			sourceHandle: 'Top' | 'Bottom' | 'Left' | 'Right';
			targetHandle: 'Top' | 'Bottom' | 'Left' | 'Right';
		}) => {
			requireSuccess(
				await apiClient.POST('/api/v1/topologies/{id}/edge-handles', {
					params: { path: { id: params.topologyId } },
					body: {
						site_id: params.siteId,
						view: params.view,
						edge_id: params.edgeId,
						source_handle: params.sourceHandle,
						target_handle: params.targetHandle
					}
				})
			);
		}
	}));
}
*/

// ============================================================================
// UI State (not server data - kept as Svelte stores)
// ============================================================================

import { browser } from '$app/environment';
import { type Edge, type Node } from '@xyflow/svelte';

const EXPANDED_STORAGE_KEY = 'scanopy_topology_options_expanded_state';
const PREFERRED_SITE_KEY = 'scanopy_preferred_site_id';
const SELECTED_SITE_KEY = 'scanopy_topology_selected_site_id';

// UI-only state
export const selectedTopologyId = writable<string | null>(null);
/** Currently selected site in the topology tab. The tab now owns the
 *  site choice (no global site selector exists yet). */
export const selectedSiteId = writable<string | null>(null);
/** Currently selected snapshot id, or `null` for the live view. */
export const selectedSnapshotId = writable<string | null>(null);
export const selectedNode = writable<Node | null>(null);
export const selectedEdge = writable<Edge | null>(null);
export const selectedNodes = writable<Node[]>([]);
/** When set, the multi-select inspector renders as an editor for the dependency
 *  with this ID instead of a create form. Set by the dep edge inspector's Edit
 *  button; cleared on Update or Cancel. */
export const editingDependencyId = writable<string | null>(null);
export const previewEdges = writable<Edge[]>([]);

/** Source of truth for real (non-preview) edges. Written by the topology
 *  rebuild pipeline in `BaseTopologyViewer`. Consumed by the merge effect
 *  that derives the xyflow `edges` store (also in BaseTopologyViewer) and
 *  by the dependency editor (for looking up real-edge handles when building
 *  preview edges for the same source/target pair). */
export const baseFlowEdges = writable<Edge[]>([]);
export const activeView = writable<TopologyView>('L3Logical');

/** When true, the topology is view-only: entity edits (tags, dependencies,
 *  descriptions), grouping-rule edits, and canvas layout edits are disabled.
 *  Driven by `TopologyTab` from `can('Topology', 'Write')` and a snapshot being selected. A
 *  snapshot is historical, so it gets the same view-only treatment as an embed.
 *  The single reactive source the inspectors / grouping editor / edit-mode read. */
export const topologyReadOnly = writable(false);

// Tutorial / hint flags (set by nudges, consumed by topology components)
export const showViewSwitcherHint = writable(false);
export const showDependencyTutorial = writable(false);

// ============================================================================
// URL Param Sync
// ============================================================================

const VALID_VIEWS: Set<string> = new Set(viewsJson.map((v) => v.id));

/** The topology and view a link names. A view this build doesn't have is ignored. */
export function topologyParamsFrom(params: URLSearchParams): {
	topologyId: string | null;
	view: TopologyView | null;
} {
	const topologyId = params.get('topologyId');
	const viewParam = params.get('view');
	const view = viewParam && VALID_VIEWS.has(viewParam) ? (viewParam as TopologyView) : null;
	return { topologyId, view };
}

/** Step to the topology and view of a history entry the user went back or forward to. */
function restoreTopologyParams(params: URLSearchParams): void {
	const { topologyId, view } = topologyParamsFrom(params);
	if (topologyId && topologyId !== get(selectedTopologyId)) {
		selectedTopologyId.set(topologyId);
		selectedNodes.set([]);
	}
	if (view && view !== get(activeView)) {
		activeView.set(view);
	}
}

/** Add a history entry for a topology change the user should be able to step back from. */
export function pushTopologyParams(): void {
	syncUrl({ push: true });
}

// Single source of truth for topology options, saved as one value for every view.
// Per-view settings are maps keyed by view inside it (request.container_rules,
// local.hide_edge_types, ...); everything else applies to every view.
const topologyOptionsStore = writable<TopologyOptions>({
	request: emptyRequestOptions(),
	local: defaultLocalOptions()
});

// Derived: element rules from the single store (for GroupingRuleEditor)
export const sharedElementRules = derived(topologyOptionsStore, ($store) => {
	return ($store.request.element_rules ?? []) as ElementGraphRule[];
});

export const topologyOptions = derived(topologyOptionsStore, ($store) => ({
	local: $store.local,
	request: {
		...$store.request
	}
}));

/** Edge types hidden in the active view. */
export const hiddenEdgeTypes = derived(
	[topologyOptionsStore, activeView],
	([$store, $view]) => $store.local.hide_edge_types[$view] ?? []
);

export function updateTopologyOptions(
	updater: (current: TopologyOptions) => TopologyOptions
): void {
	topologyOptionsStore.update((store) => {
		const updated = updater({ local: store.local, request: { ...store.request } });
		return { request: { ...updated.request }, local: updated.local };
	});
}

/** The tag filter of the active view. */
export const activeTagFilter = derived(
	[topologyOptionsStore, activeView],
	([$store, $view]): TopologyTagFilter => $store.local.tag_filter[$view] ?? { hidden_tag_ids: {} }
);

/** Replace the active view's tag filter, leaving other views' as they are. */
export function updateActiveTagFilter(
	updater: (current: TopologyTagFilter) => TopologyTagFilter
): void {
	const view = get(activeView);
	updateTopologyOptions((opts) => ({
		...opts,
		local: {
			...opts.local,
			tag_filter: {
				...opts.local.tag_filter,
				[view]: updater(opts.local.tag_filter[view] ?? { hidden_tag_ids: {} })
			}
		}
	}));
}

/** Replace the active view's hidden edge types, leaving other views' entries as they are. */
export function updateHiddenEdgeTypes(
	updater: (current: EdgeTypeDiscriminants[]) => EdgeTypeDiscriminants[]
): void {
	const view = get(activeView);
	updateTopologyOptions((opts) => ({
		...opts,
		local: {
			...opts.local,
			hide_edge_types: {
				...opts.local.hide_edge_types,
				[view]: updater(opts.local.hide_edge_types[view] ?? [])
			}
		}
	}));
}

/**
 * The entity types the viewer hid in `view` with the filter panel's eye toggle, limited to those
 * the view still draws, as an element or inline on one. Saved options can name a type the view
 * has stopped drawing (L3's ports); that hide has nothing left to act on.
 */
export function hiddenEntityTypesIn(options: TopologyOptions, view: string): string[] {
	const hidden = ((options.request.hide_entities ?? {}) as Record<string, string[]>)[view] ?? [];
	const drawn = new Set(
		viewsJson
			.find((v) => v.id === view)
			?.metadata.element_config.element_entities.flatMap((e) => [
				e.entity_type,
				...e.inline_entities
			]) ?? []
	);
	return hidden.filter((type) => drawn.has(type));
}

/**
 * Clear every control that can hide a node in `view`: the metadata hide-set, the entity-type
 * hides, and the tag filter.
 *
 * Broader than the options panel's Clear all, which covers metadata and tags. This one backs the
 * empty state's way out, so it has to guarantee the view repopulates — leaving an eye toggle set
 * would put the user back where they started, on a button that said it would fix it.
 *
 * Every filter the view declares is written as an explicit empty list rather than having its key
 * removed: an absent key means "no opinion", which the server refills from the view's defaults on
 * the next read (`TopologyRequestOptions::merge_missing_hide_defaults`), while an empty list means
 * "show everything" and is left alone.
 */
export function showEverythingIn(view: string): void {
	updateTopologyOptions((opts) => {
		const hideMeta = { ...(opts.request.hide_metadata_values ?? {}) } as Record<
			string,
			HiddenMetadataValues
		>;
		hideMeta[view] = clearedViewHideSet(view, hideMeta[view]);

		const hideEntities = { ...(opts.request.hide_entities ?? {}) };
		hideEntities[view] = [];

		return {
			...opts,
			request: {
				...opts.request,
				hide_metadata_values: hideMeta,
				hide_entities: hideEntities
			},
			local: {
				...opts.local,
				tag_filter: { ...opts.local.tag_filter, [view]: { hidden_tag_ids: {} } }
			}
		};
	});
}

// Update shared element rules (cross-view)
export function updateSharedElementRules(
	updater: (current: ElementGraphRule[]) => ElementGraphRule[]
): void {
	topologyOptionsStore.update((store) => ({
		...store,
		request: {
			...store.request,
			element_rules: updater((store.request.element_rules ?? []) as ElementGraphRule[])
		}
	}));
}

/**
 * Build options for API requests. Reads directly from the source store —
 * container_rules and hide_service_categories are already per-view HashMaps.
 */
function buildOptionsForApi(): TopologyOptions {
	const store = get(topologyOptionsStore);
	return sanitizeOptionsForApi({
		local: store.local,
		request: {
			...store.request
		}
	});
}

/**
 * Hydrate stores from a topology's backend-stored options.
 * Called on initial topology selection and SSE updates.
 * SSE updates preserve the user's local options.
 */
let hydrating = false;
/**
 * @param useDefaultLocal If true, ignore the topology's stored local options and use
 *   view-appropriate defaults. Used by share/embed views where the viewer has no
 *   stored preferences and the creator's local options shouldn't leak through.
 */
export function hydrateStoresFromTopology(
	topology: Topology | RenderableTopology,
	isInitial = true,
	useDefaultLocal = false
): void {
	hydrating = true;
	try {
		const opts = topology.options;

		// The active view is not persisted on the row: the URL (`?view=`) drives it in the app,
		// and the view prop does for shares. Options cover every view, so hydration does not
		// depend on which view is active and may run before the URL view is applied.
		if (isInitial) {
			// When useDefaultLocal is true (share/embed), use defaults: the viewer has no
			// stored preferences and the creator's must not leak.
			topologyOptionsStore.set({
				request: { ...opts.request },
				local: useDefaultLocal ? defaultLocalOptions() : opts.local
			});
		} else {
			// SSE update or topology switch: update request options, preserve
			// all client-side local options. Local options (hide_edge_types,
			// bundle_edges, etc.) are client-side state — the server returns
			// whatever was last sent, which may be stale.
			topologyOptionsStore.update((current) => ({
				request: opts.request,
				local: current.local
			}));
		}
	} finally {
		hydrating = false;
		topologyOptionsHydrated.set(true);
	}
}

/**
 * True once `hydrateStoresFromTopology` has run at least once.
 *
 * The render pipeline reads `hide_edge_types` and the element rules out of
 * these stores, but hydration happens in an effect — so the viewer's own effect
 * could start a full layout against pre-hydration defaults, then be forced to
 * throw it away and re-run once the real options landed. That cost two
 * `elk.layout()` calls and a full DOM measure pass on every cold load.
 *
 * Deliberately latched rather than reset per topology: the race only exists
 * before the first hydration. Later option changes are ordinary input changes
 * and are handled by the pipeline's reload guard.
 */
export const topologyOptionsHydrated = writable<boolean>(false);

export const optionsPanelExpanded = writable<boolean>(loadExpandedFromStorage());

/** Expanded options panel width in px (Tailwind w-80 = 320). Used by the panel and panel-aware fitView. */
export const OPTIONS_PANEL_WIDTH_PX = 320;

/** Left offset of the options panel (Tailwind left-4 = 16px). */
export const OPTIONS_PANEL_LEFT_OFFSET_PX = 16;

/** Total left padding for fitView when panel is open: panel width + offset + gap. */
export const OPTIONS_PANEL_FITVIEW_PADDING_PX =
	OPTIONS_PANEL_WIDTH_PX + OPTIONS_PANEL_LEFT_OFFSET_PX + 16;

/** Minimap dimensions. Used by MiniMap component and minimap-aware fitView. */
export const MINIMAP_WIDTH_PX = 200;
export const MINIMAP_HEIGHT_PX = 150;
export const MINIMAP_OFFSET_PX = 5;

/** Total bottom-left padding for fitView when minimap is visible. */
export const MINIMAP_FITVIEW_BOTTOM_PX = MINIMAP_HEIGHT_PX + MINIMAP_OFFSET_PX + 16;
export const MINIMAP_FITVIEW_LEFT_PX = MINIMAP_WIDTH_PX + MINIMAP_OFFSET_PX + 16;

/** Lookup map from aggregated edge ID to its original edges. Populated by BaseTopologyViewer during collapse. */
export const aggregatedEdgeOriginals = writable<Map<string, TopologyEdge[]>>(new Map());

/**
 * Set a preferred site to select when topology loads.
 * Used after onboarding to ensure the scanned site's topology is shown.
 */
export function setPreferredSite(siteId: string): void {
	if (browser) {
		localStorage.setItem(PREFERRED_SITE_KEY, siteId);
	}
}

/**
 * Get and clear the preferred site (one-time use)
 */
export function consumePreferredSite(): string | null {
	if (!browser) return null;
	const preferred = localStorage.getItem(PREFERRED_SITE_KEY);
	if (preferred) {
		localStorage.removeItem(PREFERRED_SITE_KEY);
	}
	return preferred;
}

/** Drop the signed-out user's options. The next topology to load hydrates the store again. */
export function resetTopologyOptions(): void {
	topologyOptionsStore.set({
		request: emptyRequestOptions(),
		local: defaultLocalOptions()
	});
	if (browser) {
		localStorage.removeItem(EXPANDED_STORAGE_KEY);
	}
}

function loadExpandedFromStorage(): boolean {
	if (!browser) return false;

	try {
		const stored = localStorage.getItem(EXPANDED_STORAGE_KEY);
		if (stored) {
			return JSON.parse(stored);
		}
	} catch (error) {
		console.warn('Failed to load topology expanded state from localStorage:', error);
	}
	return false;
}

function saveExpandedToStorage(expanded: boolean): void {
	if (!browser) return;

	try {
		localStorage.setItem(EXPANDED_STORAGE_KEY, JSON.stringify(expanded));
	} catch (error) {
		console.error('Failed to save topology expanded state to localStorage:', error);
	}
}

/** Persist the topology tab's selected site id across reloads. */
export function loadSelectedSiteFromStorage(): string | null {
	if (!browser) return null;
	try {
		return localStorage.getItem(SELECTED_SITE_KEY);
	} catch (error) {
		console.warn('Failed to load selected site from localStorage:', error);
		return null;
	}
}

function saveSelectedSiteToStorage(siteId: string | null): void {
	if (!browser) return;
	try {
		if (siteId) localStorage.setItem(SELECTED_SITE_KEY, siteId);
		else localStorage.removeItem(SELECTED_SITE_KEY);
	} catch (error) {
		console.error('Failed to save selected site to localStorage:', error);
	}
}

// Save options/layout edits when the user changes options. Bound to the
// `topologyOptionsStore` change subscription with debouncing so rapid
// toggles in the options panel collapse into a single PUT.
let saveOptionsTimer: ReturnType<typeof setTimeout> | undefined;
function saveOptionsForCurrentTopology(): void {
	if (!browser) return;
	// View-only (snapshot / embed): never persist option/layout edits — they'd
	// mutate the snapshot row.
	if (get(topologyReadOnly)) return;
	clearTimeout(saveOptionsTimer);
	saveOptionsTimer = setTimeout(() => {
		const topologyId = get(selectedTopologyId);
		if (!topologyId) return;
		const topologies = queryClient.getQueryData<Topology[]>(queryKeys.topology.all);
		const topology = topologies?.find((t) => t.id === topologyId);
		if (!topology) return;
		void apiClient
			.PUT('/api/v1/topologies/{id}', {
				params: { path: { id: topologyId } },
				body: { ...topology, options: buildOptionsForApi() }
			})
			.then(() => {
				// Grouping/hide-rule edits trigger a server-side rebuild of every
				// view's node/edge slice. Invalidate the topology list so the
				// rebuilt row flows back and the active slice re-renders.
				void queryClient.invalidateQueries({ queryKey: queryKeys.topology.all });
			});
	}, 500);
}

// Set up subscriptions for UI pref persistence + option-change auto-save
let optionsInitialized = false;
let expandedInitialized = false;
let siteInitialized = false;

if (browser) {
	topologyOptionsStore.subscribe(() => {
		if (optionsInitialized && !hydrating) {
			saveOptionsForCurrentTopology();
		}
		optionsInitialized = true;
	});

	optionsPanelExpanded.subscribe((expanded) => {
		if (expandedInitialized) {
			saveExpandedToStorage(expanded);
		}
		expandedInitialized = true;
	});

	selectedSiteId.subscribe((id) => {
		if (siteInitialized) {
			saveSelectedSiteToStorage(id);
		}
		siteInitialized = true;
	});

	// NOTE: the persisted site selection is NOT hydrated here. It is validated
	// against the accessible sites list and applied by the init `$effect` in
	// TopologyTab — hydrating a stale id here would fire a 404/403 topology fetch
	// before validation could run.

	// NOTE: switching the active view does NOT persist options or hit the
	// network — every view's node/edge slice is pre-built on the row, so a
	// view switch is a pure client-side slice selection (see toRenderableTopology).
	// View deep-linking is handled via URL params below.

	// The topology and view show in the URL on the topology tab only. Store changes replace the
	// current entry; a view the user picks pushes one, through `pushTopologyParams`.
	registerUrlOwner({
		scope: 'topology',
		isActive: () => true,
		params: () => {
			const id = get(selectedTopologyId);
			const params = new URLSearchParams({ view: get(activeView) });
			if (id) params.set('topologyId', id);
			return params;
		},
		restore: restoreTopologyParams
	});
	selectedTopologyId.subscribe(() => syncUrl());
	activeView.subscribe(() => syncUrl());
}

// ============================================================================
// Topology live updates
// ============================================================================

/** A site whose live entity set changed, from the live-update stream. */
type LiveTopologyUpdate = components['schemas']['LiveTopologyUpdate'];

// During discovery the server pings this stream ~5×/sec/site; each ping
// otherwise refetches the uncached, full-rebuild `/topology/data`. Coalesce a
// burst of pings into at most one refetch per window (trailing edge, so the
// final state is always fetched), while still reflecting new hosts within ~1s.
const TOPOLOGY_INVALIDATION_THROTTLE_MS = 1000;

/**
 * Sites whose live topology keeps its drawn layout while a scan runs.
 *
 * A scan changes the site several times a second, and on a large site every refetch that adds a
 * host or service is a full layout run: seconds of main-thread work and hundreds of megabytes,
 * most of it thrown away when the next refetch lands. A held site's live bundle is not refetched
 * on those changes; `heldTopologyUpdates` records that one arrived instead.
 *
 * Only the change stream is held. The user's own edits invalidate the topology queries directly
 * and still refetch.
 */
export const heldTopologySites = writable<ReadonlySet<string>>(new Set());

/** Held sites that changed since their layout was drawn. */
export const heldTopologyUpdates = writable<ReadonlySet<string>>(new Set());

/** Matches the live bundles (every view) of the given sites. */
function isLiveTopologyData(key: readonly unknown[], siteIds: ReadonlySet<string>): boolean {
	// key shape: ['topology', 'data', siteId, snapshotId | null, view]
	return (
		key[0] === 'topology' &&
		key[1] === 'data' &&
		typeof key[2] === 'string' &&
		siteIds.has(key[2]) &&
		key[3] == null
	);
}

/**
 * Live data changed for these sites. Invalidate the topology list (a live row's nodes/edges may
 * have shifted) plus the live bundle of each site that is not held. Snapshot bundles are
 * immutable, so their cache entries stay.
 */
export function invalidateLiveTopology(siteIds: ReadonlySet<string>) {
	const held = get(heldTopologySites);
	const heldChanged = [...siteIds].filter((id) => held.has(id));
	if (heldChanged.length > 0) {
		heldTopologyUpdates.update((pending) => new Set([...pending, ...heldChanged]));
	}
	const refetchable = new Set([...siteIds].filter((id) => !held.has(id)));

	queryClient.invalidateQueries({
		predicate: (query) => {
			const key = query.queryKey as readonly unknown[];
			if (key[0] !== 'topology') return false;
			if (key[1] === 'data') return isLiveTopologyData(key, refetchable);
			return true;
		}
	});
}

/**
 * Live data may have changed on any site, as after the stream was closed. Held sites are marked
 * changed instead of refetched, as `invalidateLiveTopology` does for named sites.
 */
export function invalidateAllLiveTopology() {
	const held = get(heldTopologySites);
	if (held.size > 0) {
		heldTopologyUpdates.update((pending) => new Set([...pending, ...held]));
	}

	queryClient.invalidateQueries({
		predicate: (query) => {
			const key = query.queryKey as readonly unknown[];
			if (key[0] !== 'topology') return false;
			if (key[1] !== 'data') return true;
			return key[3] == null && !(typeof key[2] === 'string' && held.has(key[2]));
		}
	});
	queryClient.invalidateQueries({ queryKey: queryKeys.snapshots.all });
}

/** Fetch a held site's live bundle now. The site stays held. */
export function refreshHeldTopology(siteId: string) {
	heldTopologyUpdates.update((pending) => {
		const next = new Set(pending);
		next.delete(siteId);
		return next;
	});
	queryClient.invalidateQueries({
		predicate: (query) =>
			isLiveTopologyData(query.queryKey as readonly unknown[], new Set([siteId]))
	});
}

/** Hold or release a site. Releasing a site that changed while held fetches it once. */
export function setTopologyHeld(siteId: string, hold: boolean) {
	const held = get(heldTopologySites);
	if (held.has(siteId) === hold) return;
	heldTopologySites.set(
		new Set(hold ? [...held, siteId] : [...held].filter((id) => id !== siteId))
	);
	if (!hold && get(heldTopologyUpdates).has(siteId)) refreshHeldTopology(siteId);
}

/** Topology events on the live-update stream (`liveUpdatesSSEManager`). */
class TopologyStream {
	/** Site ids pinged since the last flush; drained by `flushInvalidations`. */
	private pendingSiteIds = new Set<string>();

	/**
	 * Trailing throttle: coalesces the pings accumulated in `pendingSiteIds`
	 * into a single invalidation pass per window. Trailing (not leading) so the
	 * last ping of a burst is never dropped — the view always converges.
	 */
	private flushInvalidations = throttle(
		() => this.runInvalidations(),
		TOPOLOGY_INVALIDATION_THROTTLE_MS,
		{
			leading: false,
			trailing: true
		}
	);

	private runInvalidations() {
		if (this.pendingSiteIds.size === 0) return;
		const siteIds = this.pendingSiteIds;
		this.pendingSiteIds = new Set<string>();

		invalidateLiveTopology(siteIds);

		for (const siteId of siteIds) {
			// Snapshots-for-site list (taking a snapshot would have added a row).
			queryClient.invalidateQueries({ queryKey: queryKeys.snapshots.bySite(siteId) });
		}

		// Invalidate org cache until OpenTopology is recorded.
		const org = queryClient.getQueryData<Organization>(queryKeys.organizations.current());
		if (org && !org.setup.includes('OpenTopology')) {
			queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
		}
	}

	handleUpdate(update: LiveTopologyUpdate) {
		this.pendingSiteIds.add(update.site_id);
		this.flushInvalidations();
	}

	/** The stream closed: drop pings not yet flushed. */
	reset() {
		this.flushInvalidations.cancel();
		this.pendingSiteIds.clear();
	}
}

export const topologyStream = new TopologyStream();
