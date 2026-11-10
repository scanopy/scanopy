/**
 * Svelte Query configuration
 *
 * Sets up the QueryClient with default options for the application.
 */

import { QueryClient } from '@tanstack/svelte-query';
import { ApiError, AUTH_EMAIL_VERIFICATION_REQUIRED, RequestTimeoutError } from './client';
import type { components } from './schema';

/**
 * Create a QueryClient with application-specific defaults
 */
export function createQueryClient(): QueryClient {
	return new QueryClient({
		defaultOptions: {
			queries: {
				// Data is considered fresh for 30 seconds
				staleTime: 30 * 1000,
				// Keep unused data in cache for 5 minutes
				gcTime: 5 * 60 * 1000,
				// Retry 429s 3 times (global rate limit gate handles waiting), others 2 times
				retry: (failureCount, error) => {
					if (error instanceof ApiError && error.status === 429) {
						return failureCount < 3;
					}
					// A request the server did not answer within its budget has already waited that
					// long and been reported; a retry repeats both.
					if (error instanceof RequestTimeoutError) {
						return false;
					}
					// Only verifying lifts it, and that brings up the current user again.
					if (error instanceof ApiError && error.code === AUTH_EMAIL_VERIFICATION_REQUIRED) {
						return false;
					}
					return failureCount < 2;
				},
				// For 429s, retry almost immediately — the global rate limit gate
				// middleware holds requests until the window passes (+ jitter).
				// Retrying fast ensures retries enter the gate while it's active,
				// preventing stampedes when all retries fire after the window expires.
				retryDelay: (attemptIndex, error) => {
					if (error instanceof ApiError && error.status === 429) {
						return 100;
					}
					return Math.min(1000 * 2 ** attemptIndex, 30000);
				},
				// Refetch on window focus (useful for keeping data fresh)
				refetchOnWindowFocus: true,
				// Don't refetch on mount if data is fresh
				refetchOnMount: true,
				// Tell the component about every change, not only changes to the properties it has
				// read. svelte-query refreshes a component's result only when told, so a property the
				// component first reads after the query settled (a hidden tab's loading check that
				// stops at an earlier condition) would read the stale value and never be corrected.
				notifyOnChangeProps: 'all'
			},
			mutations: {
				// Don't retry mutations by default
				retry: false
			}
		}
	});
}

/**
 * Singleton QueryClient instance for the application
 */
export const queryClient = createQueryClient();

/**
 * Every entity cached under `rootKey`, once each: plain arrays (the services cache, by-id lookups)
 * and paginated results (host lists, summaries). For code outside a component, which reads the
 * cache rather than subscribing to a query.
 */
export function cachedEntities<T extends { id: string }>(rootKey: readonly unknown[]): T[] {
	const byId = new Map<string, T>();
	for (const [, data] of queryClient.getQueriesData<unknown>({ queryKey: rootKey })) {
		const items = Array.isArray(data) ? data : (data as { items?: unknown } | undefined)?.items;
		if (!Array.isArray(items)) continue;
		for (const item of items as T[]) {
			if (item?.id && !byId.has(item.id)) byId.set(item.id, item);
		}
	}
	return [...byId.values()];
}

/**
 * Query key factory for consistent cache key generation
 *
 * Usage:
 *   queryKeys.hosts.all       -> ['hosts']
 *   queryKeys.hosts.detail(id) -> ['hosts', id]
 *   queryKeys.hosts.list(filters) -> ['hosts', 'list', filters]
 */
export const queryKeys = {
	auth: {
		all: ['auth'] as const,
		currentUser: () => [...queryKeys.auth.all, 'currentUser'] as const,
		onboardingState: () => [...queryKeys.auth.all, 'onboardingState'] as const
	},
	invites: {
		all: ['invites'] as const,
		detail: (id: string) => [...queryKeys.invites.all, 'detail', id] as const
	},
	hosts: {
		all: ['hosts'] as const,
		lists: () => [...queryKeys.hosts.all, 'list'] as const,
		list: (filters?: Record<string, unknown>) => [...queryKeys.hosts.lists(), filters] as const,
		details: () => [...queryKeys.hosts.all, 'detail'] as const,
		detail: (id: string) => [...queryKeys.hosts.details(), id] as const,
		fieldValues: () => [...queryKeys.hosts.all, 'field-values'] as const
	},
	ipAddresses: {
		all: ['ipAddresses'] as const,
		byHost: (hostId: string) => [...queryKeys.ipAddresses.all, 'host', hostId] as const,
		detail: (id: string) => [...queryKeys.ipAddresses.all, 'detail', id] as const
	},
	ports: {
		all: ['ports'] as const,
		byHost: (hostId: string) => [...queryKeys.ports.all, 'host', hostId] as const,
		detail: (id: string) => [...queryKeys.ports.all, 'detail', id] as const
	},
	services: {
		all: ['services'] as const,
		byHost: (hostId: string) => [...queryKeys.services.all, 'host', hostId] as const,
		detail: (id: string) => [...queryKeys.services.all, 'detail', id] as const,
		fieldValues: () => [...queryKeys.services.all, 'field-values'] as const
	},
	bindings: {
		all: ['bindings'] as const,
		byService: (serviceId: string) => [...queryKeys.bindings.all, 'service', serviceId] as const,
		detail: (id: string) => [...queryKeys.bindings.all, 'detail', id] as const
	},
	sites: {
		all: ['sites'] as const,
		fieldValues: () => [...queryKeys.sites.all, 'field-values'] as const,
		detail: (id: string) => [...queryKeys.sites.all, 'detail', id] as const
	},
	subnets: {
		all: ['subnets'] as const,
		fieldValues: () => [...queryKeys.subnets.all, 'field-values'] as const,
		bySite: (siteId: string) => [...queryKeys.subnets.all, 'site', siteId] as const,
		detail: (id: string) => [...queryKeys.subnets.all, 'detail', id] as const
	},
	vlans: {
		all: ['vlans'] as const,
		fieldValues: () => [...queryKeys.vlans.all, 'field-values'] as const,
		bySite: (siteId: string) => [...queryKeys.vlans.all, 'site', siteId] as const,
		detail: (id: string) => [...queryKeys.vlans.all, 'detail', id] as const
	},
	dependencies: {
		all: ['dependencies'] as const,
		detail: (id: string) => [...queryKeys.dependencies.all, 'detail', id] as const
	},
	users: {
		all: ['users'] as const,
		// Users and pending invites in one list, full or a page.
		members: () => [...queryKeys.users.all, 'members'] as const,
		me: () => [...queryKeys.users.all, 'me'] as const,
		detail: (id: string) => [...queryKeys.users.all, 'detail', id] as const
	},
	organizations: {
		all: ['organizations'] as const,
		current: () => [...queryKeys.organizations.all, 'current'] as const
	},
	releaseNotes: {
		all: ['releaseNotes'] as const,
		unseen: () => [...queryKeys.releaseNotes.all, 'unseen'] as const
	},
	daemons: {
		all: ['daemons'] as const,
		fieldValues: () => [...queryKeys.daemons.all, 'field-values'] as const,
		detail: (id: string) => [...queryKeys.daemons.all, 'detail', id] as const
	},
	discovery: {
		all: ['discovery'] as const,
		detail: (id: string) => [...queryKeys.discovery.all, 'detail', id] as const,
		sessions: () => [...queryKeys.discovery.all, 'sessions'] as const,
		fieldValues: () => [...queryKeys.discovery.all, 'field-values'] as const
	},
	apiKeys: {
		all: ['apiKeys'] as const,
		fieldValues: () => [...queryKeys.apiKeys.all, 'field-values'] as const,
		detail: (id: string) => [...queryKeys.apiKeys.all, 'detail', id] as const
	},
	userApiKeys: {
		all: ['userApiKeys'] as const,
		fieldValues: () => [...queryKeys.userApiKeys.all, 'field-values'] as const,
		detail: (id: string) => [...queryKeys.userApiKeys.all, 'detail', id] as const
	},
	tags: {
		all: ['tags'] as const,
		fieldValues: () => [...queryKeys.tags.all, 'field-values'] as const,
		detail: (id: string) => [...queryKeys.tags.all, 'detail', id] as const
	},
	tagGroups: {
		all: ['tagGroups'] as const
	},
	credentials: {
		all: ['credentials'] as const,
		detail: (id: string) => [...queryKeys.credentials.all, 'detail', id] as const
	},
	interfaces: {
		all: ['interfaces'] as const,
		byHost: (hostId: string) => [...queryKeys.interfaces.all, 'host', hostId] as const
	},
	topology: {
		all: ['topology'] as const,
		detail: (id: string) => [...queryKeys.topology.all, 'detail', id] as const,
		data: (
			siteId: string,
			snapshotId: string | undefined,
			view: components['schemas']['TopologyView']
		) => [...queryKeys.topology.all, 'data', siteId, snapshotId ?? null, view] as const
	},
	snapshots: {
		all: ['snapshots'] as const,
		bySite: (siteId: string) => [...queryKeys.snapshots.all, 'site', siteId] as const,
		detail: (id: string) => [...queryKeys.snapshots.all, 'detail', id] as const
	},
	billing: {
		all: ['billing'] as const,
		plans: () => [...queryKeys.billing.all, 'plans'] as const,
		saveOfferCoupon: () => [...queryKeys.billing.all, 'save-offer-coupon'] as const,
		invoiceBilling: () => [...queryKeys.billing.all, 'invoice-billing'] as const
	},
	licenses: {
		all: ['licenses'] as const,
		currentKey: () => [...queryKeys.licenses.all, 'current-key'] as const,
		instances: () => [...queryKeys.licenses.all, 'instances'] as const
	},
	shares: {
		all: ['shares'] as const,
		detail: (id: string) => [...queryKeys.shares.all, 'detail', id] as const
	},
	dashboard: {
		all: ['dashboard'] as const,
		summary: () => [...queryKeys.dashboard.all, 'summary'] as const,
		issues: () => [...queryKeys.dashboard.all, 'issues'] as const
	},
	globalSearch: {
		all: ['global-search'] as const,
		results: (text: string, tagIds: string[]) =>
			[...queryKeys.globalSearch.all, text, tagIds] as const
	},
	config: {
		all: ['config'] as const
	},
	metadata: {
		all: ['metadata'] as const
	}
} as const;
