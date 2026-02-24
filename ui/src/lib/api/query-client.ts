/**
 * Svelte Query configuration
 *
 * Sets up the QueryClient with default options for the application.
 */

import { QueryClient } from '@tanstack/svelte-query';

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
				// Retry failed requests up to 2 times
				retry: 2,
				// Delay between retries (exponential backoff)
				retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 30000),
				// Refetch on window focus (useful for keeping data fresh)
				refetchOnWindowFocus: true,
				// Don't refetch on mount if data is fresh
				refetchOnMount: true
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
		detail: (id: string) => [...queryKeys.hosts.details(), id] as const
	},
	interfaces: {
		all: ['interfaces'] as const,
		byHost: (hostId: string) => [...queryKeys.interfaces.all, 'host', hostId] as const,
		detail: (id: string) => [...queryKeys.interfaces.all, 'detail', id] as const
	},
	ports: {
		all: ['ports'] as const,
		byHost: (hostId: string) => [...queryKeys.ports.all, 'host', hostId] as const,
		detail: (id: string) => [...queryKeys.ports.all, 'detail', id] as const
	},
	services: {
		all: ['services'] as const,
		byHost: (hostId: string) => [...queryKeys.services.all, 'host', hostId] as const,
		detail: (id: string) => [...queryKeys.services.all, 'detail', id] as const
	},
	bindings: {
		all: ['bindings'] as const,
		byService: (serviceId: string) => [...queryKeys.bindings.all, 'service', serviceId] as const,
		detail: (id: string) => [...queryKeys.bindings.all, 'detail', id] as const
	},
	networks: {
		all: ['networks'] as const,
		detail: (id: string) => [...queryKeys.networks.all, 'detail', id] as const
	},
	subnets: {
		all: ['subnets'] as const,
		byNetwork: (networkId: string) => [...queryKeys.subnets.all, 'network', networkId] as const,
		detail: (id: string) => [...queryKeys.subnets.all, 'detail', id] as const
	},
	groups: {
		all: ['groups'] as const,
		detail: (id: string) => [...queryKeys.groups.all, 'detail', id] as const
	},
	users: {
		all: ['users'] as const,
		me: () => [...queryKeys.users.all, 'me'] as const,
		detail: (id: string) => [...queryKeys.users.all, 'detail', id] as const
	},
	organizations: {
		all: ['organizations'] as const,
		current: () => [...queryKeys.organizations.all, 'current'] as const
	},
	daemons: {
		all: ['daemons'] as const,
		detail: (id: string) => [...queryKeys.daemons.all, 'detail', id] as const
	},
	discovery: {
		all: ['discovery'] as const,
		detail: (id: string) => [...queryKeys.discovery.all, 'detail', id] as const,
		sessions: () => [...queryKeys.discovery.all, 'sessions'] as const
	},
	apiKeys: {
		all: ['apiKeys'] as const,
		detail: (id: string) => [...queryKeys.apiKeys.all, 'detail', id] as const
	},
	userApiKeys: {
		all: ['userApiKeys'] as const,
		detail: (id: string) => [...queryKeys.userApiKeys.all, 'detail', id] as const
	},
	tags: {
		all: ['tags'] as const,
		detail: (id: string) => [...queryKeys.tags.all, 'detail', id] as const
	},
	snmpCredentials: {
		all: ['snmpCredentials'] as const,
		detail: (id: string) => [...queryKeys.snmpCredentials.all, 'detail', id] as const
	},
	ifEntries: {
		all: ['ifEntries'] as const,
		byHost: (hostId: string) => [...queryKeys.ifEntries.all, 'host', hostId] as const
	},
	topology: {
		all: ['topology'] as const,
		detail: (id: string) => [...queryKeys.topology.all, 'detail', id] as const
	},
	billing: {
		all: ['billing'] as const,
		plans: () => [...queryKeys.billing.all, 'plans'] as const
	},
	shares: {
		all: ['shares'] as const,
		detail: (id: string) => [...queryKeys.shares.all, 'detail', id] as const
	},
	dashboard: {
		all: ['dashboard'] as const,
		summary: () => [...queryKeys.dashboard.all, 'summary'] as const
	},
	config: {
		all: ['config'] as const
	},
	metadata: {
		all: ['metadata'] as const
	}
} as const;
