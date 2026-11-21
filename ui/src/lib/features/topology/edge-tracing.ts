import type { Edge } from '@xyflow/svelte';
import type { RenderableTopology, TopologyEdge } from './types/base';
import { hostDisplayName } from '#lib/features/hosts/host-display-name.ts';

/** A link at one end of a selected link: the next hop when tracing a path. */
export interface HostLink {
	edge: Edge;
	/** The port on the host the list is for; unknown on a device-level link. */
	localInterfaceId?: string;
	farHostId: string;
	/** The port at the far end; unknown on a device-level link. */
	farInterfaceId?: string;
}

/** The hosts and ports at each end of a physical or neighbour link, or null for any other edge. */
export function linkEnds(
	data: TopologyEdge,
	hostOfInterface: (interfaceId: string) => string | undefined
): {
	source: { hostId: string; interfaceId?: string };
	target: { hostId: string; interfaceId?: string };
} | null {
	if (data.edge_type === 'PhysicalLink') {
		const sourceHost = hostOfInterface(data.source_entity_id);
		const targetHost = hostOfInterface(data.target_entity_id);
		if (!sourceHost || !targetHost) return null;
		return {
			source: { hostId: sourceHost, interfaceId: data.source_entity_id },
			target: { hostId: targetHost, interfaceId: data.target_entity_id }
		};
	}
	if (data.edge_type === 'NeighborLink') {
		return { source: { hostId: data.source_host_id }, target: { hostId: data.target_host_id } };
	}
	return null;
}

/**
 * Every physical or neighbour link on `hostId` other than `excludeEdgeId`, from the edges drawn,
 * ordered by the far host's name. Bundled edges are skipped: each link they stand for is drawn on
 * its own when the bundle is expanded.
 */
export function linksOfHost(
	hostId: string,
	edges: Edge[],
	topology: Pick<RenderableTopology, 'interfaces' | 'hosts'>,
	excludeEdgeId: string
): HostLink[] {
	const hostOfInterface = new Map(topology.interfaces.map((i) => [i.id, i.host_id]));
	const hostName = new Map(topology.hosts.map((h) => [h.id, hostDisplayName(h)]));
	const links: HostLink[] = [];
	for (const edge of edges) {
		if (edge.id === excludeEdgeId) continue;
		const data = edge.data as (TopologyEdge & { isAggregated?: boolean }) | undefined;
		if (!data || data.isAggregated) continue;
		const ends = linkEnds(data, (id) => hostOfInterface.get(id));
		if (!ends) continue;
		const [local, far] =
			ends.source.hostId === hostId
				? [ends.source, ends.target]
				: ends.target.hostId === hostId
					? [ends.target, ends.source]
					: [null, null];
		if (!local || !far || far.hostId === hostId) continue;
		links.push({
			edge,
			localInterfaceId: local.interfaceId,
			farHostId: far.hostId,
			farInterfaceId: far.interfaceId
		});
	}
	return links.sort((a, b) =>
		(hostName.get(a.farHostId) ?? '').localeCompare(hostName.get(b.farHostId) ?? '')
	);
}
