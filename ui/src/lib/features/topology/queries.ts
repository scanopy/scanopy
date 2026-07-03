/**
 * TanStack Query hooks for Topology
 *
 * Note: UI state (selected nodes/edges, options panel, localStorage preferences)
 * remains in local component state or a separate UI store.
 */

import { createQuery, createMutation } from '@tanstack/svelte-query';
import { queryClient, queryKeys } from '$lib/api/query-client';
import { apiClient, API_BASE_PATH } from '$lib/api/client';
import type { Topology, TopologyEdge, TopologyOptions, RenderableTopology } from './types/base';
import type { ContainerGraphRule, ElementGraphRule, ElementRule } from './types/grouping';
import { makeGraphRule } from './types/grouping';
import type { ContainerRule } from './types/grouping';
import _containerRuleTypes from '$lib/data/container-rule-types.json';
import _elementRuleTypes from '$lib/data/element-rule-types.json';
import type { Organization } from '$lib/features/organizations/types';
import { BaseSSEManager, type SSEConfig } from '$lib/shared/utils/sse';
import { writable, derived, get } from 'svelte/store';
import { UNTAGGED_SENTINEL } from './interactions';
import { getDefaultHiddenEdgeTypes } from './layout/edge-classification';
import type { components } from '$lib/api/schema';
import viewsJson from '$lib/data/views.json';
import { getIrrelevantServiceCategories } from '$lib/shared/stores/metadata';
import { common_infrastructure } from '$lib/paraglide/messages';

export type TopologyView = components['schemas']['TopologyView'];

/** Strip UI-only sentinel values from options before sending to the API */
export function sanitizeOptionsForApi(options: TopologyOptions): TopologyOptions {
	const tf = options.local?.tag_filter;
	const isSentinel = (id: string) => id === UNTAGGED_SENTINEL;
	return {
		...options,
		local: {
			...options.local,
			tag_filter: {
				hidden_host_tag_ids: (tf?.hidden_host_tag_ids ?? []).filter((id) => !isSentinel(id)),
				hidden_service_tag_ids: (tf?.hidden_service_tag_ids ?? []).filter((id) => !isSentinel(id)),
				hidden_subnet_tag_ids: (tf?.hidden_subnet_tag_ids ?? []).filter((id) => !isSentinel(id))
			}
		}
	};
}

type ServiceCategory = components['schemas']['ServiceCategory'];
type TopologyLocalOptions = components['schemas']['TopologyLocalOptions'];

/** Get the org's use case from the query cache, defaulting to 'other' */
export function getOrgUseCase(): string {
	const org = queryClient.getQueryData<Organization>(queryKeys.organizations.current());
	return org?.use_case ?? 'other';
}

/**
 * Get categories that are irrelevant for the org's use case (for grouping into
 * the "Infrastructure Services" ByServiceCategory element rule).
 */
function getIrrelevantCategories(useCase: string): ServiceCategory[] {
	return [...getIrrelevantServiceCategories(useCase)] as ServiceCategory[];
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
 * that drives the layout pipeline — so on a network switch it can briefly lag.
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
 * so it stays correct across a same-view network switch.
 */
export function getInfrastructureRuleIdForTopology(
	topology: Topology | RenderableTopology
): string | null {
	return findInfraRuleId(topology.options?.request?.element_rules);
}

const ALL_VIEWS: TopologyView[] = viewsJson.map((p) => p.id as TopologyView);

/** Default local options for a given view (UI-only, not sent to backend as rules) */
function getDefaultLocalOptions(view: TopologyView): TopologyLocalOptions {
	return {
		hide_edge_types: getDefaultHiddenEdgeTypes(view),
		no_fade_edges: false,
		bundle_edges: true,
		tag_filter: {
			hidden_host_tag_ids: [],
			hidden_service_tag_ids: [],
			hidden_subnet_tag_ids: []
		},
		show_minimap: true
	};
}

/** Build default per-view local options */
function initDefaultLocalOptions(): Record<TopologyView, TopologyLocalOptions> {
	return Object.fromEntries(ALL_VIEWS.map((p) => [p, getDefaultLocalOptions(p)])) as Record<
		TopologyView,
		TopologyLocalOptions
	>;
}

/**
 * Default request options matching the backend's TopologyRequestOptions::default().
 * Container rules and hidden categories are per-view HashMaps.
 * Element rules are shared cross-view.
 */
function defaultRequestOptions(): components['schemas']['TopologyRequestOptions'] {
	// Build container rules per view from fixture metadata
	const containerRules: Record<string, ContainerGraphRule[]> = {};
	for (const p of ALL_VIEWS) {
		containerRules[p] = _containerRuleTypes
			.filter((r) => (r.metadata as { views?: string[] })?.views?.includes(p))
			.map((r) => {
				if (r.id === 'ByApplication') {
					return makeGraphRule({ ByApplication: { tag_ids: [] } } as ContainerRule);
				}
				return makeGraphRule(r.id as ContainerRule);
			});
	}

	// Element rules: one of each type (shared cross-view)
	const seen = new Set<string>();
	const elementRules: ElementGraphRule[] = [];
	for (const r of _elementRuleTypes) {
		if (!seen.has(r.id)) {
			seen.add(r.id);
			if (r.id === 'ByServiceCategory') {
				const rule = makeGraphRule({
					ByServiceCategory: {
						categories: getIrrelevantCategories(getOrgUseCase()),
						title: common_infrastructure(),
						is_infra_rule: true
					}
				});
				elementRules.push(rule);
			} else if (r.id === 'ByTag') {
				elementRules.push(makeGraphRule({ ByTag: { tag_ids: [], title: null } }));
			} else {
				elementRules.push(makeGraphRule(r.id as ElementRule));
			}
		}
	}

	// Default: OpenPorts hidden under Service.Category for every view
	// (use-case-aware filtering is handled by the ByServiceCategory element
	// rule; this is the chip-level toggle state). Shape matches the nested
	// hide_metadata_values HashMap serialized by the backend.
	const hideMetadataValues: Record<string, Record<string, Record<string, string[]>>> = {};
	for (const p of ALL_VIEWS) {
		hideMetadataValues[p] = { Service: { Category: ['OpenPorts'] } };
	}

	return {
		hide_entities: {},
		hide_metadata_values: hideMetadataValues,
		container_rules: containerRules,
		element_rules: elementRules
	};
}

/**
 * Query hook for fetching all topologies
 */
export function useTopologiesQuery(enabled?: () => boolean) {
	return createQuery(() => ({
		queryKey: queryKeys.topology.all,
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/v1/topology', {
				params: { query: { limit: 0 } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch topologies');
			}
			return data.data;
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
 * so live vs snapshot data don't collide. The SSE `live_topology_updates_stream`
 * consumer invalidates this query on live-view network updates.
 */
export function useTopologyDataQuery(
	networkId: () => string | undefined,
	snapshotId: () => string | undefined
) {
	return createQuery(() => ({
		queryKey: queryKeys.topology.data(networkId() ?? '', snapshotId()),
		queryFn: async () => {
			const network_id = networkId();
			if (!network_id) {
				throw new Error('No network ID provided');
			}
			const snapshot_id = snapshotId();
			const { data } = await apiClient.GET('/api/v1/topology/data', {
				params: { query: { network_id, snapshot_id } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch topology data');
			}
			return data.data;
		},
		enabled: () => !!networkId()
	}));
}

/**
 * One-shot GET that records the `FirstTopologyRebuild` onboarding milestone ("user viewed
 * their topology"). Sends `mark_viewed=true`, which the backend only honours for a live
 * topology that has hosts and a completed discovery. Deliberately NOT routed through
 * `useTopologyDataQuery` (which never sets the flag) so the milestone only fires from an
 * explicit on-tab view, never from background fetches on other tabs.
 */
export async function markTopologyViewed(networkId: string): Promise<void> {
	await apiClient.GET('/api/v1/topology/data', {
		params: { query: { network_id: networkId, mark_viewed: true } }
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
			const { data } = await apiClient.GET('/api/v1/topology/{id}', {
				params: { path: { id: topologyId } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch topology');
			}
			return data.data;
		},
		enabled: () => !!id()
	}));
}

/**
 * Mutation hook for updating a topology's layout state.
 *
 * After the snapshot refactor, the PUT endpoint only accepts updates to
 * `nodes` / `edges` / `options` — there's no separate rebuild/refresh/lock
 * surface. Live data updates flow through `live_topology_updates_stream`
 * (see TopologySSEManager below); option/layout edits are saved by callers
 * via this mutation.
 */
export function useUpdateTopologyMutation() {
	return createMutation(() => ({
		mutationFn: async (topology: Topology) => {
			await apiClient.PUT('/api/v1/topology/{id}', {
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
			networkId: string;
			view: TopologyView;
			nodeId: string;
			position: { x: number; y: number };
		}) => {
			const { data } = await apiClient.POST('/api/v1/topology/{id}/node-position', {
				params: { path: { id: params.topologyId } },
				body: {
					network_id: params.networkId,
					view: params.view,
					node_id: params.nodeId,
					position: params.position
				}
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to update node position');
			}
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
			networkId: string;
			view: TopologyView;
			nodeId: string;
			size: { x: number; y: number };
			position: { x: number; y: number };
		}) => {
			const { data } = await apiClient.POST('/api/v1/topology/{id}/node-resize', {
				params: { path: { id: params.topologyId } },
				body: {
					network_id: params.networkId,
					view: params.view,
					node_id: params.nodeId,
					size: params.size,
					position: params.position
				}
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to resize node');
			}
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
			networkId: string;
			view: TopologyView;
			edgeId: string;
			sourceHandle: 'Top' | 'Bottom' | 'Left' | 'Right';
			targetHandle: 'Top' | 'Bottom' | 'Left' | 'Right';
		}) => {
			const { data } = await apiClient.POST('/api/v1/topology/{id}/edge-handles', {
				params: { path: { id: params.topologyId } },
				body: {
					network_id: params.networkId,
					view: params.view,
					edge_id: params.edgeId,
					source_handle: params.sourceHandle,
					target_handle: params.targetHandle
				}
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to update edge handles');
			}
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
const PREFERRED_NETWORK_KEY = 'scanopy_preferred_network_id';
const SELECTED_NETWORK_KEY = 'scanopy_topology_selected_network_id';

// UI-only state
export const selectedTopologyId = writable<string | null>(null);
/** Currently selected network in the topology tab. The tab now owns the
 *  network choice (no global network selector exists yet). */
export const selectedNetworkId = writable<string | null>(null);
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
 *  Driven by `TopologyTab` from `isReadOnly || a snapshot being selected` — a
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

/** Read topology ID and view from current URL search params. */
export function getTopologyParamsFromUrl(): {
	topologyId: string | null;
	view: TopologyView | null;
} {
	if (!browser) return { topologyId: null, view: null };
	const params = new URLSearchParams(window.location.search);
	const topologyId = params.get('topologyId');
	const viewParam = params.get('view');
	const view = viewParam && VALID_VIEWS.has(viewParam) ? (viewParam as TopologyView) : null;
	return { topologyId, view };
}

/** Update URL search params to reflect current topology state. Uses replaceState (no history entry). */
function syncTopologyParamsToUrl(topologyId: string | null, view: TopologyView): void {
	if (!browser) return;
	const url = new URL(window.location.href);
	if (topologyId) {
		url.searchParams.set('topologyId', topologyId);
	} else {
		url.searchParams.delete('topologyId');
	}
	url.searchParams.set('view', view);
	window.history.replaceState(window.history.state, '', url.toString());
}

/** Push a new history entry with updated topology params. For user-initiated changes. */
export function pushTopologyParams(topologyId: string | null, view: TopologyView): void {
	if (!browser) return;
	const url = new URL(window.location.href);
	if (topologyId) {
		url.searchParams.set('topologyId', topologyId);
	} else {
		url.searchParams.delete('topologyId');
	}
	url.searchParams.set('view', view);
	window.history.pushState({}, '', url.toString());
}

// Single source of truth for topology options.
// request: backend state (container_rules/hide_service_categories are per-view HashMaps)
// perViewLocal: UI-only local options per view
const topologyOptionsStore = writable<{
	request: components['schemas']['TopologyRequestOptions'];
	perViewLocal: Record<TopologyView, TopologyLocalOptions>;
}>({
	request: defaultRequestOptions(),
	perViewLocal: initDefaultLocalOptions()
});

// Derived: element rules from the single store (for GroupingRuleEditor)
export const sharedElementRules = derived(topologyOptionsStore, ($store) => {
	return ($store.request.element_rules ?? []) as ElementGraphRule[];
});

// Public derived store: projects the active view's slice of topology options
export const topologyOptions = derived([topologyOptionsStore, activeView], ([$store, $view]) => ({
	local: $store.perViewLocal[$view],
	request: {
		...$store.request
	}
}));

// Helper to update the active view's local options or request scalars
export function updateTopologyOptions(
	updater: (current: TopologyOptions) => TopologyOptions
): void {
	const view = get(activeView);
	topologyOptionsStore.update((store) => {
		const currentOpts: TopologyOptions = {
			local: store.perViewLocal[view],
			request: { ...store.request }
		};
		const updated = updater(currentOpts);
		return {
			request: { ...updated.request },
			perViewLocal: {
				...store.perViewLocal,
				[view]: updated.local
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
	const view = get(activeView);
	return sanitizeOptionsForApi({
		local: store.perViewLocal[view],
		request: {
			...store.request
		}
	});
}

/**
 * Hydrate stores from a topology's backend-stored options.
 * Called on initial topology selection and SSE updates.
 * SSE updates preserve the user's view and local options for other views.
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

		// The active view is no longer persisted on the row — it's driven by the
		// URL (`?view=`) with an L3Logical default (see TopologyTab) for the app,
		// and by the explicit view prop for shares. Hydration only restores the
		// view-agnostic request options + the active view's local options.
		if (isInitial) {
			const request = { ...opts.request };

			// Auto-populate infra rule categories if empty (new topology or migration).
			// Only targets is_infra_rule rules with no categories — preserves user edits.
			const elementRules = [...(request.element_rules ?? [])];
			for (let i = 0; i < elementRules.length; i++) {
				const rule = elementRules[i].rule;
				if (
					typeof rule === 'object' &&
					'ByServiceCategory' in rule &&
					rule.ByServiceCategory.is_infra_rule &&
					(!rule.ByServiceCategory.categories || rule.ByServiceCategory.categories.length === 0)
				) {
					const useCase = getOrgUseCase();
					elementRules[i] = {
						...elementRules[i],
						rule: {
							ByServiceCategory: {
								...rule.ByServiceCategory,
								categories: getIrrelevantCategories(useCase),
								title: common_infrastructure()
							}
						}
					};
					break;
				}
			}
			request.element_rules = elementRules;

			// Full hydration: use backend request options + default local options.
			// When useDefaultLocal is true (share/embed), always use view defaults —
			// the viewer has no stored preferences and the creator's shouldn't leak.
			topologyOptionsStore.set({
				request,
				perViewLocal: {
					...initDefaultLocalOptions(),
					...(useDefaultLocal ? {} : { [get(activeView)]: opts.local })
				}
			});
		} else {
			// SSE update or topology switch: update request options, preserve
			// all client-side local options. Local options (hide_edge_types,
			// bundle_edges, etc.) are client-side state — the server returns
			// whatever was last sent, which may be stale.
			topologyOptionsStore.update((current) => ({
				request: opts.request,
				perViewLocal: current.perViewLocal
			}));
		}
	} finally {
		hydrating = false;
	}
}

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
 * Set a preferred network to select when topology loads.
 * Used after onboarding to ensure the scanned network's topology is shown.
 */
export function setPreferredNetwork(networkId: string): void {
	if (browser) {
		localStorage.setItem(PREFERRED_NETWORK_KEY, networkId);
	}
}

/**
 * Get and clear the preferred network (one-time use)
 */
export function consumePreferredNetwork(): string | null {
	if (!browser) return null;
	const preferred = localStorage.getItem(PREFERRED_NETWORK_KEY);
	if (preferred) {
		localStorage.removeItem(PREFERRED_NETWORK_KEY);
	}
	return preferred;
}

export function resetTopologyOptions(): void {
	topologyOptionsStore.set({
		request: defaultRequestOptions(),
		perViewLocal: initDefaultLocalOptions()
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

/** Persist the topology tab's selected network id across reloads. */
export function loadSelectedNetworkFromStorage(): string | null {
	if (!browser) return null;
	try {
		return localStorage.getItem(SELECTED_NETWORK_KEY);
	} catch (error) {
		console.warn('Failed to load selected network from localStorage:', error);
		return null;
	}
}

function saveSelectedNetworkToStorage(networkId: string | null): void {
	if (!browser) return;
	try {
		if (networkId) localStorage.setItem(SELECTED_NETWORK_KEY, networkId);
		else localStorage.removeItem(SELECTED_NETWORK_KEY);
	} catch (error) {
		console.error('Failed to save selected network to localStorage:', error);
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
			.PUT('/api/v1/topology/{id}', {
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
let networkInitialized = false;

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

	selectedNetworkId.subscribe((id) => {
		if (networkInitialized) {
			saveSelectedNetworkToStorage(id);
		}
		networkInitialized = true;
	});

	// NOTE: the persisted network selection is NOT hydrated here. It is validated
	// against the accessible networks list and applied by the init `$effect` in
	// TopologyTab — hydrating a stale id here would fire a 404/403 topology fetch
	// before validation could run.

	// NOTE: switching the active view does NOT persist options or hit the
	// network — every view's node/edge slice is pre-built on the row, so a
	// view switch is a pure client-side slice selection (see toRenderableTopology).
	// View deep-linking is handled via URL params below.

	// Sync stores → URL (replaceState, no history entry)
	// User-initiated changes use pushTopologyParams from TopologyTab instead.
	selectedTopologyId.subscribe((id) => {
		if (id !== null) {
			syncTopologyParamsToUrl(id, get(activeView));
		}
	});
	activeView.subscribe((view) => {
		const id = get(selectedTopologyId);
		if (id !== null) {
			syncTopologyParamsToUrl(id, view);
		}
	});
}

// ============================================================================
// Topology SSE Manager — live data updates
// ============================================================================

/**
 * Payload of the `live_topology_updates_stream` SSE: the network whose
 * live entity set just changed. The frontend invalidates the topology +
 * snapshot lists for that network so TanStack Query refetches and xyflow
 * re-renders.
 */
interface LiveTopologyUpdate {
	network_id: string;
}

class TopologySSEManager extends BaseSSEManager<LiveTopologyUpdate> {
	protected createConfig(): SSEConfig<LiveTopologyUpdate> {
		return {
			url: `${API_BASE_PATH}/api/v1/topology/stream`,
			onMessage: (update) => {
				// Live data changed for this network — invalidate the topology
				// list (the live row's nodes/edges may have shifted), the
				// snapshots-for-network list (taking a snapshot would have
				// added a row), and the LIVE entity bundle for the affected
				// network. Snapshot bundles are immutable; predicate keeps
				// their cache entries intact.
				queryClient.invalidateQueries({
					predicate: (query) => {
						const key = query.queryKey as readonly unknown[];
						if (key[0] !== 'topology') return false;
						if (key[1] === 'data') {
							// key shape: ['topology', 'data', networkId, snapshotId | null]
							return key[2] === update.network_id && key[3] == null;
						}
						return true;
					}
				});
				queryClient.invalidateQueries({
					queryKey: queryKeys.snapshots.byNetwork(update.network_id)
				});

				// Invalidate org cache until FirstTopologyRebuild milestone appears
				const org = queryClient.getQueryData<Organization>(queryKeys.organizations.current());
				if (org && !org.onboarding.includes('FirstTopologyRebuild')) {
					queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
				}
			},
			onError: (error) => {
				console.error('Topology SSE error:', error);
			},
			onOpen: () => {}
		};
	}
}

export const topologySSEManager = new TopologySSEManager();
