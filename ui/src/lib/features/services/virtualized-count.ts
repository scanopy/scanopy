import { apiClient } from '#lib/api/client.ts';
import { unwrapEnvelope } from '#lib/api/query-helpers.ts';
import { serviceDefinitions } from '#lib/shared/stores/metadata.ts';
import {
	hosts_virtualization_identityCount,
	hosts_virtualization_vmCount,
	topology_containerCount
} from '#lib/paraglide/messages.js';
import type { Service } from './types/base';

/**
 * `count` guests, named for what `serviceDefinition` runs: a runtime's containers, a hypervisor's
 * VMs, a host's network identities.
 */
export function guestCountLabel(serviceDefinition: string, count: number): string {
	switch (serviceDefinitions.getMetadata(serviceDefinition).manages_virtualization) {
		case 'containers':
			return topology_containerCount({ count });
		case 'identities':
			return hosts_virtualization_identityCount({ count });
		default:
			return hosts_virtualization_vmCount({ count });
	}
}

/** How many services and hosts one service virtualizes. */
async function countChildren(serviceId: string): Promise<number> {
	const query = { virtualization_service_ids: [serviceId], limit: 1 };
	const [services, hosts] = await Promise.all([
		apiClient.GET('/api/v1/services', { params: { query } }),
		apiClient.GET('/api/v1/hosts', { params: { query } })
	]);
	return (
		(unwrapEnvelope(services).meta?.pagination?.total_count ?? 0) +
		(unwrapEnvelope(hosts).meta?.pagination?.total_count ?? 0)
	);
}

/**
 * What deleting `services` deletes with them, named per runtime or hypervisor ("3 containers,
 * 2 VMs"), or `null` when they run nothing.
 */
export async function virtualizedSummary(
	services: Pick<Service, 'id' | 'service_definition'>[]
): Promise<string | null> {
	const virtualizers = services.filter(
		(s) => serviceDefinitions.getMetadata(s.service_definition).manages_virtualization
	);
	const counts = await Promise.all(
		virtualizers.map(async (s) => ({ service: s, count: await countChildren(s.id) }))
	);
	const labels = counts
		.filter(({ count }) => count > 0)
		.map(({ service, count }) => guestCountLabel(service.service_definition, count));
	return labels.length > 0 ? labels.join(', ') : null;
}

/** What deleting the hosts in `hostIds` deletes through the services on them, or `null`. */
export async function virtualizedSummaryOnHosts(hostIds: string[]): Promise<string | null> {
	if (hostIds.length === 0) return null;
	const services = unwrapEnvelope(
		await apiClient.GET('/api/v1/services', { params: { query: { host_ids: hostIds, limit: 0 } } })
	).data;
	return virtualizedSummary(services);
}
