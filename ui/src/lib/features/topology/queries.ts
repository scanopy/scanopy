/**
 * TanStack Query hooks for Topology
 *
 * Note: UI state (selected nodes/edges, options panel, localStorage preferences)
 * remains in local component state or a separate UI store.
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryClient, queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { Topology, TopologyOptions } from './types/base';
import type { Organization } from '$lib/features/organizations/types';
import { uuidv4Sentinel, utcTimeZoneSentinel } from '$lib/shared/utils/formatting';
import { BaseSSEManager, type SSEConfig } from '$lib/shared/utils/sse';
import { writable, get } from 'svelte/store';
import { UNTAGGED_SENTINEL } from './interactions';

/** Strip UI-only sentinel values from options before sending to the API */
export function sanitizeOptionsForApi(options: TopologyOptions): TopologyOptions {
	const sanitized = structuredClone(options);
	const tf = sanitized.local?.tag_filter;
	if (tf) {
		if (tf.hidden_host_tag_ids) {
			tf.hidden_host_tag_ids = tf.hidden_host_tag_ids.filter((id) => id !== UNTAGGED_SENTINEL);
		}
		if (tf.hidden_service_tag_ids) {
			tf.hidden_service_tag_ids = tf.hidden_service_tag_ids.filter(
				(id) => id !== UNTAGGED_SENTINEL
			);
		}
		if (tf.hidden_subnet_tag_ids) {
			tf.hidden_subnet_tag_ids = tf.hidden_subnet_tag_ids.filter((id) => id !== UNTAGGED_SENTINEL);
		}
	}
	return sanitized;
}

// Default options for new topologies
export const defaultTopologyOptions: TopologyOptions = {
	local: {
		left_zone_title: 'Infrastructure',
		hide_edge_types: ['HostVirtualization'],
		no_fade_edges: false,
		hide_resize_handles: false,
		tag_filter: {
			hidden_host_tag_ids: [],
			hidden_service_tag_ids: [],
			hidden_subnet_tag_ids: []
		},
		show_minimap: true
	},
	request: {
		group_docker_bridges_by_host: true,
		hide_ports: false,
		hide_vm_title_on_docker_container: false,
		show_gateway_in_left_zone: true,
		left_zone_service_categories: ['DNS', 'ReverseProxy'],
		hide_service_categories: ['OpenPorts']
	}
};

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
 * Mutation hook for creating a topology
 */
export function useCreateTopologyMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (topology: Topology) => {
			const { data } = await apiClient.POST('/api/v1/topology', { body: topology });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to create topology');
			}
			return data.data;
		},
		onSuccess: (newTopology: Topology) => {
			queryClient.setQueryData<Topology[]>(queryKeys.topology.all, (old) =>
				old ? [...old, newTopology] : [newTopology]
			);
		}
	}));
}

/**
 * Mutation hook for updating a topology
 * Note: Updated topology returns through SSE, so we don't update cache here
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

/**
 * Mutation hook for deleting a topology
 */
export function useDeleteTopologyMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			const { data } = await apiClient.DELETE('/api/v1/topology/{id}', {
				params: { path: { id } }
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to delete topology');
			}
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Topology[]>(
				queryKeys.topology.all,
				(old) => old?.filter((t) => t.id !== id) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for refreshing a topology
 * Note: Updated topology returns through SSE
 * Uses lightweight request - only sends fields the server actually needs
 */
export function useRefreshTopologyMutation() {
	return createMutation(() => ({
		mutationFn: async (topology: Topology) => {
			await apiClient.POST('/api/v1/topology/{id}/refresh', {
				params: { path: { id: topology.id } },
				body: {
					network_id: topology.network_id,
					options: sanitizeOptionsForApi(get(topologyOptions)),
					nodes: [],
					edges: []
				}
			});
			return topology.id;
		}
	}));
}

/**
 * Mutation hook for rebuilding a topology
 * Note: Updated topology returns through SSE
 * Uses lightweight request - only sends fields the server actually needs
 * (network_id, options, nodes/edges for position preservation)
 */
export function useRebuildTopologyMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (topology: Topology) => {
			await apiClient.POST('/api/v1/topology/{id}/rebuild', {
				params: { path: { id: topology.id } },
				body: {
					network_id: topology.network_id,
					options: sanitizeOptionsForApi(get(topologyOptions)),
					nodes: topology.nodes,
					edges: topology.edges
				}
			});
			return topology.id;
		},
		onSuccess: () => {
			const org = queryClient.getQueryData<Organization>(queryKeys.organizations.current());
			if (org && !org.onboarding.includes('FirstTopologyRebuild')) {
				queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
			}
		}
	}));
}

/**
 * Mutation hook for locking a topology
 */
export function useLockTopologyMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (topology: Topology) => {
			const { data } = await apiClient.POST('/api/v1/topology/{id}/lock', {
				params: { path: { id: topology.id } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to lock topology');
			}
			return data.data;
		},
		onSuccess: (updatedTopology: Topology) => {
			queryClient.setQueryData<Topology[]>(
				queryKeys.topology.all,
				(old) => old?.map((t) => (t.id === updatedTopology.id ? updatedTopology : t)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for unlocking a topology
 */
export function useUnlockTopologyMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (topology: Topology) => {
			const { data } = await apiClient.POST('/api/v1/topology/{id}/unlock', {
				params: { path: { id: topology.id } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to unlock topology');
			}
			return data.data;
		},
		onSuccess: (updatedTopology: Topology) => {
			queryClient.setQueryData<Topology[]>(
				queryKeys.topology.all,
				(old) => old?.map((t) => (t.id === updatedTopology.id ? updatedTopology : t)) ?? []
			);
		}
	}));
}

/**
 * Mutation hook for updating a single node's position
 * Lightweight endpoint - only sends node ID and position instead of full topology
 * Fixes HTTP 413 errors on drag operations for large topologies
 */
export function useUpdateNodePositionMutation() {
	return createMutation(() => ({
		mutationFn: async (params: {
			topologyId: string;
			networkId: string;
			nodeId: string;
			position: { x: number; y: number };
		}) => {
			const { data } = await apiClient.POST('/api/v1/topology/{id}/node-position', {
				params: { path: { id: params.topologyId } },
				body: {
					network_id: params.networkId,
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
 */
export function useUpdateNodeResizeMutation() {
	return createMutation(() => ({
		mutationFn: async (params: {
			topologyId: string;
			networkId: string;
			nodeId: string;
			size: { x: number; y: number };
			position: { x: number; y: number };
		}) => {
			const { data } = await apiClient.POST('/api/v1/topology/{id}/node-resize', {
				params: { path: { id: params.topologyId } },
				body: {
					network_id: params.networkId,
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
 */
export function useUpdateEdgeHandlesMutation() {
	return createMutation(() => ({
		mutationFn: async (params: {
			topologyId: string;
			networkId: string;
			edgeId: string;
			sourceHandle: 'Top' | 'Bottom' | 'Left' | 'Right';
			targetHandle: 'Top' | 'Bottom' | 'Left' | 'Right';
		}) => {
			const { data } = await apiClient.POST('/api/v1/topology/{id}/edge-handles', {
				params: { path: { id: params.topologyId } },
				body: {
					network_id: params.networkId,
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

/**
 * Mutation hook for updating topology metadata (name, parent)
 * Lightweight endpoint - only sends metadata fields instead of full topology
 * Fixes HTTP 413 errors on metadata edit operations for large topologies
 */
export function useUpdateMetadataMutation() {
	return createMutation(() => ({
		mutationFn: async (params: {
			topologyId: string;
			networkId: string;
			name: string;
			parentId: string | null;
		}) => {
			const { data } = await apiClient.POST('/api/v1/topology/{id}/metadata', {
				params: { path: { id: params.topologyId } },
				body: {
					network_id: params.networkId,
					name: params.name,
					parent_id: params.parentId
				}
			});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to update topology metadata');
			}
		}
	}));
}

/**
 * Helper to update topologies in the query cache (for SSE updates)
 */
export function updateTopologyInCache(
	queryClient: ReturnType<typeof useQueryClient>,
	topology: Topology
) {
	queryClient.setQueryData<Topology[]>(
		queryKeys.topology.all,
		(old) => old?.map((t) => (t.id === topology.id ? topology : t)) ?? []
	);
}

/**
 * Create empty topology form data
 */
export function createEmptyTopologyFormData(networkId: string): Topology {
	return {
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		name: '',
		network_id: networkId,
		edges: [],
		nodes: [],
		options: structuredClone(defaultTopologyOptions),
		hosts: [],
		interfaces: [],
		services: [],
		subnets: [],
		groups: [],
		ports: [],
		bindings: [],
		is_stale: false,
		last_refreshed: utcTimeZoneSentinel,
		is_locked: false,
		removed_groups: [],
		removed_hosts: [],
		removed_interfaces: [],
		removed_services: [],
		removed_subnets: [],
		removed_bindings: [],
		removed_ports: [],
		if_entries: [],
		removed_if_entries: [],
		locked_at: null,
		locked_by: null,
		parent_id: null,
		tags: [],
		entity_tags: []
	};
}

// ============================================================================
// UI State (not server data - kept as Svelte stores)
// ============================================================================

import { browser } from '$app/environment';
import { type Edge, type Node } from '@xyflow/svelte';
import deepmerge from 'deepmerge';

const OPTIONS_STORAGE_KEY = 'scanopy_topology_options';
const EXPANDED_STORAGE_KEY = 'scanopy_topology_options_expanded_state';
const AUTO_REBUILD_STORAGE_KEY = 'scanopy_topology_auto_rebuild';
const PREFERRED_NETWORK_KEY = 'scanopy_preferred_network_id';

// UI-only state
export const selectedTopologyId = writable<string | null>(null);
export const selectedNode = writable<Node | null>(null);
export const selectedEdge = writable<Edge | null>(null);
export const selectedNodes = writable<Node[]>([]);
export const previewEdges = writable<Edge[]>([]);
export const autoRebuild = writable<boolean>(loadAutoRebuildFromStorage());
export const topologyOptions = writable<TopologyOptions>(loadOptionsFromStorage());
export const optionsPanelExpanded = writable<boolean>(loadExpandedFromStorage());

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
	topologyOptions.set(structuredClone(defaultTopologyOptions));
	if (browser) {
		localStorage.removeItem(OPTIONS_STORAGE_KEY);
		localStorage.removeItem(EXPANDED_STORAGE_KEY);
	}
}

export function hasConflicts(topology: Topology): boolean {
	return (
		topology.removed_hosts.length > 0 ||
		topology.removed_services.length > 0 ||
		topology.removed_subnets.length > 0 ||
		topology.removed_bindings.length > 0 ||
		topology.removed_ports.length > 0 ||
		topology.removed_interfaces.length > 0 ||
		topology.removed_groups.length > 0
	);
}

// localStorage helpers
function loadOptionsFromStorage(): TopologyOptions {
	if (!browser) return defaultTopologyOptions;

	try {
		const stored = localStorage.getItem(OPTIONS_STORAGE_KEY);
		if (stored) {
			const parsed = JSON.parse(stored);
			return deepmerge(defaultTopologyOptions, parsed, {
				arrayMerge: (_, sourceArray) => sourceArray
			});
		}
	} catch (error) {
		console.warn('Failed to load topology options from localStorage:', error);
	}
	return defaultTopologyOptions;
}

function saveOptionsToStorage(options: TopologyOptions): void {
	if (!browser) return;

	try {
		localStorage.setItem(OPTIONS_STORAGE_KEY, JSON.stringify(options));
	} catch (error) {
		console.error('Failed to save topology options to localStorage:', error);
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

function loadAutoRebuildFromStorage(): boolean {
	if (!browser) return true;

	try {
		const stored = localStorage.getItem(AUTO_REBUILD_STORAGE_KEY);
		if (stored !== null) {
			return JSON.parse(stored);
		}
	} catch (error) {
		console.warn('Failed to load auto rebuild state from localStorage:', error);
	}
	return true;
}

function saveAutoRebuildToStorage(value: boolean): void {
	if (!browser) return;

	try {
		localStorage.setItem(AUTO_REBUILD_STORAGE_KEY, JSON.stringify(value));
	} catch (error) {
		console.error('Failed to save auto rebuild state to localStorage:', error);
	}
}

// Set up subscriptions for localStorage persistence
let optionsInitialized = false;
let expandedInitialized = false;
let autoRebuildInitialized = false;

if (browser) {
	let optionsRebuildTimeout: ReturnType<typeof setTimeout>;
	topologyOptions.subscribe((options) => {
		if (optionsInitialized) {
			saveOptionsToStorage(options);

			// Trigger a rebuild when request options change (replaces the old
			// debounced PUT that was lost in the TanStack migration)
			clearTimeout(optionsRebuildTimeout);
			optionsRebuildTimeout = setTimeout(() => {
				if (!get(autoRebuild)) return;
				const topologyId = get(selectedTopologyId);
				if (!topologyId) return;

				const topologies = queryClient.getQueryData<Topology[]>(queryKeys.topology.all);
				const topology = topologies?.find((t) => t.id === topologyId);
				if (!topology) return;

				apiClient.POST('/api/v1/topology/{id}/rebuild', {
					params: { path: { id: topologyId } },
					body: {
						network_id: topology.network_id,
						options: sanitizeOptionsForApi(options),
						nodes: topology.nodes,
						edges: topology.edges
					}
				});
			}, 500);
		}
		optionsInitialized = true;
	});

	optionsPanelExpanded.subscribe((expanded) => {
		if (expandedInitialized) {
			saveExpandedToStorage(expanded);
		}
		expandedInitialized = true;
	});

	autoRebuild.subscribe((value) => {
		if (autoRebuildInitialized) {
			saveAutoRebuildToStorage(value);
		}
		autoRebuildInitialized = true;
	});
}

// ============================================================================
// Topology SSE Manager
// ============================================================================

class TopologySSEManager extends BaseSSEManager<Topology> {
	private stalenessTimers: Map<string, ReturnType<typeof setTimeout>> = new Map();
	private readonly DEBOUNCE_MS = 300;
	private readonly REBUILD_DEBOUNCE_MS = 2000;

	protected createConfig(): SSEConfig<Topology> {
		return {
			url: '/api/v1/topology/stream',
			onMessage: (update) => {
				// If the update says it's NOT stale, apply immediately (it's a full refresh)
				if (!update.is_stale) {
					this.applyFullUpdate(update);
					return;
				}

				// For stale updates with autoRebuild enabled, trigger a debounced rebuild
				if (get(autoRebuild)) {
					const currentId = get(selectedTopologyId);
					if (currentId === update.id && !update.is_locked) {
						const timerKey = `rebuild:${update.id}`;
						const existingTimer = this.stalenessTimers.get(timerKey);
						if (existingTimer) {
							clearTimeout(existingTimer);
						}
						const timer = setTimeout(() => {
							apiClient.POST('/api/v1/topology/{id}/rebuild', {
								params: { path: { id: update.id } },
								body: {
									network_id: update.network_id,
									options: sanitizeOptionsForApi(get(topologyOptions)),
									nodes: update.nodes,
									edges: update.edges
								}
							});
							this.stalenessTimers.delete(timerKey);
						}, this.REBUILD_DEBOUNCE_MS);
						this.stalenessTimers.set(timerKey, timer);
					}
					return;
				}

				// For staleness updates, debounce them
				const existingTimer = this.stalenessTimers.get(update.id);
				if (existingTimer) {
					clearTimeout(existingTimer);
				}

				const timer = setTimeout(() => {
					this.applyPartialUpdate(update.id, {
						removed_groups: update.removed_groups,
						removed_hosts: update.removed_hosts,
						removed_services: update.removed_services,
						removed_subnets: update.removed_subnets,
						removed_bindings: update.removed_bindings,
						removed_interfaces: update.removed_interfaces,
						removed_ports: update.removed_ports,
						is_stale: update.is_stale,
						options: update.options
					});
					this.stalenessTimers.delete(update.id);
				}, this.DEBOUNCE_MS);

				this.stalenessTimers.set(update.id, timer);
			},
			onError: (error) => {
				console.error('Topology SSE error:', error);
			},
			onOpen: () => {}
		};
	}

	private applyFullUpdate(update: Topology) {
		queryClient.setQueryData<Topology[]>(queryKeys.topology.all, (old) => {
			if (!old) return [update];
			return old.map((topo) => (topo.id === update.id ? update : topo));
		});

		// Invalidate org cache until FirstTopologyRebuild milestone appears
		const org = queryClient.getQueryData<Organization>(queryKeys.organizations.current());
		if (org && !org.onboarding.includes('FirstTopologyRebuild')) {
			queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
		}
	}

	private applyPartialUpdate(topologyId: string, updates: Partial<Topology>) {
		queryClient.setQueryData<Topology[]>(queryKeys.topology.all, (old) => {
			if (!old) return [];
			return old.map((topo) => (topo.id === topologyId ? { ...topo, ...updates } : topo));
		});
	}
}

export const topologySSEManager = new TopologySSEManager();
