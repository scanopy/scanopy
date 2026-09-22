/**
 * TanStack Query hooks for Discovery
 */

import {
	createQuery,
	createMutation,
	useQueryClient,
	keepPreviousData
} from '@tanstack/svelte-query';
import { queryClient, queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import { requireSuccess, unwrapData, unwrapEnvelope } from '$lib/api/query-helpers';
import type { Discovery } from './types/base';
import type { components } from '$lib/api/schema';
import type { DiscoveryUpdatePayload } from './types/api';
import type { Organization } from '../organizations/types';
import { pushError, pushSuccess, pushWarning } from '$lib/shared/stores/feedback';
import { BaseSSEManager, type SSEConfig } from '$lib/shared/utils/sse';
import { writable } from 'svelte/store';
import * as m from '$lib/paraglide/messages';
import { networkItems } from '$lib/features/networks/columns';
import { daemonItems } from '$lib/features/daemons/columns';

/**
 * Query hook for fetching all discoveries.
 *
 * Loads the full list, which is fine for scan configurations — there are only
 * ever a handful. Run *history* grows one row per run forever, so that tab uses
 * `useDiscoveryHistoryQuery` instead. This hook stays unpaginated because
 * several callers (the scheduled tab, CreateDaemonModal) need every row.
 */
export function useDiscoveriesQuery(enabled?: () => boolean) {
	return createQuery(() => ({
		queryKey: queryKeys.discovery.all,
		queryFn: async () => {
			return unwrapData(
				await apiClient.GET('/api/v1/discovery', {
					params: { query: { limit: 0 } }
				})
			);
		},
		...(enabled ? { enabled } : {})
	}));
}

/** Query parameters for the paginated discovery-history list. */
export interface DiscoveryHistoryQueryParams {
	limit?: number;
	offset?: number;
	/** Primary ordering field (used for grouping). Always sorts ASC. */
	group_by?: components['schemas']['DiscoveryOrderField'];
	/** Secondary ordering field (sorting within groups or standalone sort). */
	order_by?: components['schemas']['DiscoveryOrderField'];
	/** Direction for order_by field. */
	order_direction?: components['schemas']['OrderDirection'];
	/** Free-text search across the run's name and its daemon's name. */
	search?: string;
	/** Filter by network ID. Several narrows to the union of them. */
	network_ids?: string[];
	/** Filter by daemon ID. Several narrows to the union of them. */
	daemon_ids?: string[];
	/** Only runs of one of these discovery types (raw discriminants). */
	discovery_types?: string[];
}

/** Pagination metadata, derived from the generated schema. */
export type PaginationMeta = components['schemas']['PaginationMeta'];

/**
 * Paginated, server-ordered run history.
 *
 * Separate from `useDiscoveriesQuery` so paginating here doesn't truncate the
 * full list every other caller depends on. `historical: true` replaces what the
 * history tab used to do by filtering `run_type` in the browser — which only
 * ever worked because the whole table was loaded.
 */
export function useDiscoveryHistoryQuery(
	paramsOrGetter: DiscoveryHistoryQueryParams | (() => DiscoveryHistoryQueryParams) = {},
	// Every tab is mounted at once (inactive ones hidden with CSS), so an ungated
	// query fetches on app boot and refetches on every discovery SSE invalidation
	// for a tab nobody is looking at. Gate it on tab activity.
	enabled: () => boolean = () => true
) {
	return createQuery(() => {
		const params = typeof paramsOrGetter === 'function' ? paramsOrGetter() : paramsOrGetter;
		const {
			limit,
			offset,
			group_by,
			order_by,
			order_direction,
			search,
			network_ids,
			daemon_ids,
			discovery_types
		} = params;

		return {
			queryKey: [
				...queryKeys.discovery.all,
				'history',
				{
					limit,
					offset,
					group_by,
					order_by,
					order_direction,
					search,
					network_ids,
					daemon_ids,
					discovery_types
				}
			],
			enabled: enabled(),
			queryFn: async (): Promise<{ items: Discovery[]; pagination: PaginationMeta | null }> => {
				const envelope = unwrapEnvelope(
					await apiClient.GET('/api/v1/discovery', {
						params: {
							query: {
								limit,
								offset,
								group_by,
								order_by,
								order_direction,
								search,
								network_ids,
								daemon_ids,
								discovery_types,
								historical: true
							}
						}
					})
				);
				return {
					items: envelope.data,
					pagination: envelope.meta?.pagination ?? null
				};
			},
			placeholderData: keepPreviousData
		};
	});
}

/**
 * Mutation hook for creating a discovery
 */
export function useCreateDiscoveryMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (discovery: Discovery) => {
			return unwrapData(await apiClient.POST('/api/v1/discovery', { body: discovery }));
		},
		onSuccess: (newDiscovery: Discovery) => {
			queryClient.setQueryData<Discovery[]>(queryKeys.discovery.all, (old) =>
				old ? [...old, newDiscovery] : [newDiscovery]
			);
			// The history tab reads a paginated key under this prefix, which
			// setQueryData above doesn't touch.
			queryClient.invalidateQueries({ queryKey: queryKeys.discovery.all });
		}
	}));
}

/**
 * Mutation hook for updating a discovery
 */
export function useUpdateDiscoveryMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (discovery: Discovery) => {
			return unwrapData(
				await apiClient.PUT('/api/v1/discovery/{id}', {
					params: { path: { id: discovery.id } },
					body: discovery
				})
			);
		},
		onSuccess: (updatedDiscovery: Discovery) => {
			queryClient.setQueryData<Discovery[]>(
				queryKeys.discovery.all,
				(old) => old?.map((d) => (d.id === updatedDiscovery.id ? updatedDiscovery : d)) ?? []
			);
			queryClient.invalidateQueries({ queryKey: queryKeys.discovery.all });
		}
	}));
}

/**
 * Mutation hook for deleting a discovery
 */
export function useDeleteDiscoveryMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (id: string) => {
			requireSuccess(
				await apiClient.DELETE('/api/v1/discovery/{id}', {
					params: { path: { id } }
				})
			);
			return id;
		},
		onSuccess: (id: string) => {
			queryClient.setQueryData<Discovery[]>(
				queryKeys.discovery.all,
				(old) => old?.filter((d) => d.id !== id) ?? []
			);
			queryClient.invalidateQueries({ queryKey: queryKeys.discovery.all });
		}
	}));
}

/**
 * Mutation hook for bulk deleting discoveries
 */
export function useBulkDeleteDiscoveriesMutation() {
	const queryClient = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (ids: string[]) => {
			requireSuccess(await apiClient.POST('/api/v1/discovery/bulk-delete', { body: ids }));
			return ids;
		},
		onSuccess: (ids: string[]) => {
			queryClient.setQueryData<Discovery[]>(
				queryKeys.discovery.all,
				(old) => old?.filter((d) => !ids.includes(d.id)) ?? []
			);
			queryClient.invalidateQueries({ queryKey: queryKeys.discovery.all });
		}
	}));
}

import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';
import type { Daemon } from '../daemons/types/base';
import type { Network } from '../networks/types';
import type { FieldConfig } from '$lib/shared/components/data/types';

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Create empty form data for a new discovery
 */
export function createEmptyDiscoveryFormData(daemon: Daemon | null): Discovery {
	return {
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		tags: [],
		discovery_type: {
			type: 'Unified',
			host_id: daemon ? daemon.host_id : uuidv4Sentinel,
			subnet_ids: null,
			host_naming_fallback: 'BestService',
			scan_settings: {}
		},
		run_type: {
			type: 'Scheduled',
			cron_schedule: '0 0 0 * * 0',
			last_run: null,
			enabled: true,
			timezone: Intl.DateTimeFormat().resolvedOptions().timeZone
		},
		name: '',
		daemon_id: daemon ? daemon.id : uuidv4Sentinel,
		network_id: daemon ? daemon.network_id : uuidv4Sentinel,
		integration_targets: []
	};
}

/**
 * Parse a simple cron expression back to hours
 * Only handles the patterns we generate
 */
export function parseCronToHours(cron: string): number | null {
	const parts = cron.split(' ');
	if (parts.length !== 6) return null;

	const [, , hour, day, ,] = parts;

	// Daily pattern: "0 0 0 * * *"
	if (hour === '0' && day === '*') {
		return 24;
	}

	// Every N days: "0 0 0 */N * *"
	if (hour === '0' && day.startsWith('*/')) {
		const days = parseInt(day.slice(2));
		return days * 24;
	}

	// Every N hours: "0 0 */N * * *"
	if (hour.startsWith('*/')) {
		return parseInt(hour.slice(2));
	}

	// Every hour: "0 0 * * * *"
	if (hour === '*') {
		return 1;
	}

	return null;
}

/**
 * Generate a cron expression for "every N hours"
 * Format: "0 0 *\/N * * *" (second minute hour day month weekday)
 */
export function generateCronSchedule(hours: number): string {
	if (hours === 0) {
		return '0 0 * * * *'; // Every hour as fallback
	}
	if (hours === 1) {
		return '0 0 * * * *'; // Every hour
	}
	if (hours === 24) {
		return '0 0 0 * * *'; // Daily at midnight
	}
	if (hours % 24 === 0) {
		// Every N days at midnight
		const days = hours / 24;
		return `0 0 0 */${days} * *`;
	}
	// Every N hours
	return `0 0 */${hours} * * *`;
}

/**
 * Generate a cron expression from day-of-week + hour + minute
 * Format: "0 minute hour * * day1,day2,..." (second minute hour day month weekday)
 */
export function generateDayTimeCronSchedule(
	daysOfWeek: number[],
	hour: number,
	minute: number
): string {
	const dayStr = daysOfWeek.length === 7 ? '*' : daysOfWeek.join(',');
	return `0 ${minute} ${hour} * * ${dayStr}`;
}

/**
 * Parse a day-of-week + time cron expression back to its components.
 * Returns null for cron expressions that don't match this pattern (legacy interval crons).
 * Expected format: "0 minute hour * * daySpec"
 */
export function parseDayTimeCronSchedule(
	cron: string
): { daysOfWeek: number[]; hour: number; minute: number } | null {
	const parts = cron.split(' ');
	if (parts.length !== 6) return null;

	const [sec, min, hour, day, month, weekday] = parts;

	// Must be: second=0, day=*, month=*
	if (sec !== '0' || day !== '*' || month !== '*') return null;

	// Minute and hour must be plain numbers
	const minuteNum = parseInt(min);
	const hourNum = parseInt(hour);
	if (isNaN(minuteNum) || isNaN(hourNum)) return null;
	if (String(minuteNum) !== min || String(hourNum) !== hour) return null;
	if (minuteNum < 0 || minuteNum > 59 || hourNum < 0 || hourNum > 23) return null;

	// Parse weekday field
	let daysOfWeek: number[];
	if (weekday === '*') {
		daysOfWeek = [0, 1, 2, 3, 4, 5, 6];
	} else {
		daysOfWeek = weekday.split(',').map((d) => parseInt(d));
		if (daysOfWeek.some((d) => isNaN(d) || d < 0 || d > 6)) return null;
	}

	return { daysOfWeek, hour: hourNum, minute: minuteNum };
}

/**
 * Format a schedule for display on cards.
 * Returns a human-readable string like "Mon, Wed, Fri at 03:00 (America/New_York)"
 */
export function formatScheduleDisplay(cron: string, timezone: string | null | undefined): string {
	const tz = timezone || 'UTC';
	const parsed = parseDayTimeCronSchedule(cron);

	if (parsed) {
		const time = `${String(parsed.hour).padStart(2, '0')}:${String(parsed.minute).padStart(2, '0')}`;
		if (parsed.daysOfWeek.length === 7) {
			return `Daily at ${time} (${tz})`;
		}
		const dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
		const days = parsed.daysOfWeek.map((d) => dayNames[d]).join(', ');
		return `${days} at ${time} (${tz})`;
	}

	// Fallback: try legacy hours format
	const hours = parseCronToHours(cron);
	if (hours !== null) {
		return `Every ${hours} Hours`;
	}

	// Raw cron fallback
	return `${cron} (${tz})`;
}

/**
 * Field configuration for the DataTableControls
 */
export const discoveryFields = (
	daemons: Daemon[],
	networks: Network[]
): FieldConfig<Discovery>[] => [
	{
		key: 'name',
		label: m.common_name(),
		type: 'string',
		searchable: true,
		sortable: true,
		// Identity field: grouping by it would render a header per discovery.
		groupable: false,
		getValue: (item: Discovery) => item.name
	},
	{
		key: 'created_at',
		label: m.common_created(),
		type: 'date',
		sortable: true
	},
	{
		key: 'daemon_id',
		label: m.common_daemon(),
		type: 'string',
		searchable: true,
		filterable: true,
		groupable: true,
		getValue: (item: Discovery) =>
			daemons.find((d) => d.id == item.daemon_id)?.name ??
			m.common_unknownEntity({ entity: m.common_daemon() }),
		display: { getItems: (item: Discovery) => daemonItems(item.daemon_id, daemons) }
	},
	{
		key: 'network_id',
		label: m.common_network(),
		type: 'string',
		searchable: true,
		filterable: true,
		groupable: true,
		getValue: (item: Discovery) =>
			networks.find((n) => n.id === item.network_id)?.name ?? m.common_unknownNetwork(),
		display: { getItems: (item: Discovery) => networkItems(item.network_id, networks) }
	},
	{
		key: 'discovery_type',
		label: m.common_type(),
		type: 'string',
		searchable: true,
		filterable: true,
		groupable: true,
		getValue: (item: Discovery) => item.discovery_type.type
	}
];

// ============================================================================
// Discovery Sessions (TanStack Query + SSE)
// ============================================================================

/**
 * Store for tracking which sessions are being cancelled
 * This is UI-only state, not server data
 */
export const cancellingSessions = writable<Map<string, boolean>>(new Map());

/**
 * Query hook for fetching active discovery sessions
 * @param getEnabled - Getter function that returns whether query is enabled (for reactivity with Svelte 5 runes)
 */
export function useActiveSessionsQuery(getEnabled: () => boolean = () => true) {
	return createQuery(() => ({
		queryKey: queryKeys.discovery.sessions(),
		queryFn: async () => {
			return unwrapData(
				await apiClient.GET('/api/v1/discovery/active-sessions', {})
			) as DiscoveryUpdatePayload[];
		},
		// Sessions change frequently, keep fresh
		staleTime: 5 * 1000,
		enabled: getEnabled()
	}));
}

/**
 * Mutation hook for initiating a discovery session
 */
export function useInitiateDiscoveryMutation() {
	const qc = useQueryClient();

	return createMutation(() => ({
		mutationFn: async (discoveryId: string) => {
			return unwrapData(
				await apiClient.POST('/api/v1/discovery/start-session', {
					body: discoveryId
				})
			) as DiscoveryUpdatePayload;
		},
		onSuccess: (session: DiscoveryUpdatePayload) => {
			// Add session to cache
			qc.setQueryData<DiscoveryUpdatePayload[]>(queryKeys.discovery.sessions(), (old) => {
				if (!old) return [session];
				const exists = old.find((s) => s.session_id === session.session_id);
				if (exists) {
					return old.map((s) => (s.session_id === session.session_id ? session : s));
				}
				return [...old, session];
			});

			// Connect SSE to receive updates
			discoverySSEManager.connect();

			pushSuccess(
				`${session.discovery_type.type} discovery session created with session ID ${session.session_id}`
			);
		}
	}));
}

/**
 * Mutation hook for cancelling a discovery session
 */
export function useCancelDiscoveryMutation() {
	return createMutation(() => ({
		mutationFn: async (sessionId: string) => {
			// Mark as cancelling
			cancellingSessions.update((c) => {
				const m = new Map(c);
				m.set(sessionId, true);
				return m;
			});

			const result = await apiClient.POST('/api/v1/discovery/{session_id}/cancel', {
				params: { path: { session_id: sessionId } }
			});

			if (!result.response.ok) {
				// Clear cancelling state on failure, before the throw below
				cancellingSessions.update((c) => {
					const m = new Map(c);
					m.delete(sessionId);
					return m;
				});
			}
			requireSuccess(result);

			return sessionId;
		}
		// Note: Success handling happens via SSE when the "Cancelled" phase is received
	}));
}

// ============================================================================
// Discovery SSE Manager
// ============================================================================

// Track last known progress per session to detect changes
const lastProgress = new Map<string, number>();

// Throttle configuration for query invalidations
const INVALIDATION_THROTTLE_MS = 1000; // At most 1 invalidation per second

class DiscoverySSEManager extends BaseSSEManager<DiscoveryUpdatePayload> {
	private invalidationPending = false;
	private invalidationTimeout: ReturnType<typeof setTimeout> | null = null;

	/**
	 * Throttled query invalidation - batches multiple invalidation requests
	 * and only executes at most once per INVALIDATION_THROTTLE_MS
	 */
	private scheduleInvalidation() {
		if (this.invalidationPending) {
			// Already have a pending invalidation, skip
			return;
		}

		this.invalidationPending = true;
		this.invalidationTimeout = setTimeout(() => {
			this.invalidationPending = false;
			this.invalidationTimeout = null;

			// Invalidate all relevant queries
			queryClient.invalidateQueries({ queryKey: queryKeys.hosts.all });
			queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
			queryClient.invalidateQueries({ queryKey: queryKeys.subnets.all });
			queryClient.invalidateQueries({ queryKey: queryKeys.daemons.all });
		}, INVALIDATION_THROTTLE_MS);
	}

	/**
	 * Clean up resources on disconnect
	 */
	override disconnect() {
		// Clear any pending invalidation
		if (this.invalidationTimeout) {
			clearTimeout(this.invalidationTimeout);
			this.invalidationTimeout = null;
			this.invalidationPending = false;
		}

		// Clear progress tracking for all sessions
		lastProgress.clear();

		super.disconnect();
	}

	protected createConfig(): SSEConfig<DiscoveryUpdatePayload> {
		return {
			url: '/api/v1/discovery/stream',
			onMessage: async (update) => {
				// Check if progress increased
				const last = lastProgress.get(update.session_id) || 0;
				const current = update.progress || 0;

				if (current > last) {
					// Schedule throttled invalidation instead of immediate
					this.scheduleInvalidation();
					lastProgress.set(update.session_id, current);
				}

				// Handle terminal phases
				if (update.phase === 'Complete') {
					// The scan completed either way, so that is what the toast says. Warnings are
					// not toasted: a sticky toast carrying several paragraphs is the wrong surface
					// for detail the user needs to read at their own pace, and the run's history
					// card already tags itself "Warnings" and lists them in full.
					pushSuccess(m.discovery_completed({ type: update.discovery_type.type }));
					// Final refresh on completion - do this immediately, not throttled
					await Promise.all([
						queryClient.invalidateQueries({ queryKey: queryKeys.hosts.all }),
						queryClient.invalidateQueries({ queryKey: queryKeys.services.all }),
						queryClient.invalidateQueries({ queryKey: queryKeys.subnets.all }),
						queryClient.invalidateQueries({ queryKey: queryKeys.daemons.all }),
						queryClient.invalidateQueries({ queryKey: queryKeys.discovery.all })
					]);
				} else if (update.phase === 'Cancelled') {
					pushWarning(m.discovery_cancelled());
				} else if (update.phase === 'Failed' && update.error) {
					pushError(m.discovery_error({ error: update.error }), -1);
				}

				// Invalidate org cache until FirstDiscoveryCompleted milestone appears
				const org = queryClient.getQueryData<Organization>(queryKeys.organizations.current());
				if (org && !org.onboarding.includes('FirstDiscoveryCompleted')) {
					queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
				}

				// Update sessions in TanStack cache
				queryClient.setQueryData<DiscoveryUpdatePayload[]>(
					queryKeys.discovery.sessions(),
					(current) => {
						if (!current) current = [];

						// Cleanup for terminal phases
						if (
							update.phase === 'Complete' ||
							update.phase === 'Cancelled' ||
							update.phase === 'Failed'
						) {
							// Clear cancelling state
							cancellingSessions.update((c) => {
								const m = new Map(c);
								m.delete(update.session_id);
								return m;
							});

							lastProgress.delete(update.session_id);

							// Remove completed/cancelled/failed sessions
							return current.filter((session) => session.session_id !== update.session_id);
						}

						// For non-terminal phases, update or add the session
						const existingIndex = current.findIndex((s) => s.session_id === update.session_id);

						if (existingIndex >= 0) {
							const updated = [...current];
							updated[existingIndex] = update;
							return updated;
						} else {
							return [...current, update];
						}
					}
				);

				// If this session references a discovery not yet in cache
				// (e.g. auto-created on daemon registration), refetch discoveries
				if (update.discovery_id) {
					const discoveries = queryClient.getQueryData<Discovery[]>(queryKeys.discovery.all);
					if (discoveries && !discoveries.some((d) => d.id === update.discovery_id)) {
						queryClient.invalidateQueries({ queryKey: queryKeys.discovery.all });
					}
				}
			},
			onError: (error) => {
				console.error('Discovery SSE error:', error);
				pushError(m.discovery_lostConnection());
			},
			onOpen: () => {}
		};
	}
}

export const discoverySSEManager = new DiscoverySSEManager();
