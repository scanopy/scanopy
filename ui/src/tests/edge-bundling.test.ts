import { describe, it, expect } from 'vitest';
import { bundleEdges } from '$lib/features/topology/layout/edge-bundling';
import type { components } from '$lib/api/schema';

type TopologyEdge = components['schemas']['Edge'];

const PROXY_BOX = 'box-proxy';
const DB_BOX = 'box-db';

/**
 * `relation_key` is what the backend computes for this edge type (`EdgeType::relation_key`):
 * the identity of the thing the edge stands for, or null when the edge is one of several
 * interchangeable connections of its type.
 */
function edge(
	source: string,
	target: string,
	edgeType: string,
	relationKey: string | null = null
): TopologyEdge {
	return {
		id: `${edgeType}-${source}-${target}`,
		edge_type: edgeType,
		source,
		target,
		source_handle: 'Bottom',
		target_handle: 'Top',
		is_multi_hop: false,
		label: null,
		relation_key: relationKey
	} as unknown as TopologyEdge;
}

/** Both containers have a card in each of the two subnet boxes. */
const elementToContainer = new Map<string, string>([
	['api-proxy', PROXY_BOX],
	['api-db', DB_BOX],
	['worker-proxy', PROXY_BOX],
	['worker-db', DB_BOX]
]);

describe('edge bundling', () => {
	it('keeps each container’s SameContainer edge separate', () => {
		// Two different containers each spanning the same pair of subnets. Merging them would
		// produce a bundle claiming the subnets are linked while dropping which container links
		// them — and bundles render without a label and highlight only as a group.
		const { bundles, unbundled } = bundleEdges(
			[
				edge('api-proxy', 'api-db', 'SameContainer', 'svc-api'),
				edge('worker-proxy', 'worker-db', 'SameContainer', 'svc-worker')
			],
			elementToContainer
		);

		expect(bundles).toHaveLength(0);
		expect(unbundled).toHaveLength(2);
	});

	it('still bundles interchangeable edges of the same type between the same containers', () => {
		// ContainerRuntime elevates onto its containers, so several of them land on identical
		// endpoints and draw one line over another. Merging is the clutter reduction bundling
		// exists for — and the only way to show there is more than one.
		const { bundles } = bundleEdges(
			[
				edge('api-proxy', 'api-db', 'ContainerRuntime'),
				edge('worker-proxy', 'worker-db', 'ContainerRuntime')
			],
			elementToContainer
		);

		expect(bundles).toHaveLength(1);
		expect(bundles[0].count).toBe(2);
	});

	it('keeps separate dependencies apart', () => {
		const { bundles, unbundled } = bundleEdges(
			[
				edge('api-proxy', 'api-db', 'RequestPath', 'dep-1'),
				edge('worker-proxy', 'worker-db', 'RequestPath', 'dep-2')
			],
			elementToContainer
		);

		expect(bundles).toHaveLength(0);
		expect(unbundled).toHaveLength(2);
	});

	it('keeps every cable separate when physical links cross the same pair of subnets', () => {
		// A switch in the management subnet cabled to three hosts that all sit in the servers
		// subnet. The cables are distinct relationships, so all three have to draw — bundling
		// them renders only the first and the other two vanish from the canvas while still
		// highlighting, which is the bug this guards.
		const { bundles, unbundled } = bundleEdges(
			[
				edge('api-proxy', 'api-db', 'PhysicalLink', 'if-switch-3:if-hv01-1'),
				edge('api-proxy', 'worker-db', 'PhysicalLink', 'if-switch-4:if-hv02-1'),
				edge('api-proxy', 'api-db', 'PhysicalLink', 'if-switch-5:if-docker-1')
			],
			elementToContainer
		);

		expect(bundles).toHaveLength(0);
		expect(unbundled).toHaveLength(3);
	});

	it('keeps each host’s addresses on their own line', () => {
		// Two hosts that each have an address in both subnets. One line per host, or the survivor
		// claims the other host's spread as its own.
		const { bundles, unbundled } = bundleEdges(
			[
				edge('api-proxy', 'api-db', 'SameHost', 'host-a'),
				edge('worker-proxy', 'worker-db', 'SameHost', 'host-b')
			],
			elementToContainer
		);

		expect(bundles).toHaveLength(0);
		expect(unbundled).toHaveLength(2);
	});

	it('gives each relation its own bundle id', () => {
		// Expansion state is keyed by bundle id, so two bundles between the same pair of
		// containers sharing an id would expand and collapse as one.
		const { bundles } = bundleEdges(
			[
				edge('api-proxy', 'api-db', 'RequestPath', 'dep-1'),
				edge('worker-proxy', 'worker-db', 'RequestPath', 'dep-1'),
				edge('api-proxy', 'worker-db', 'RequestPath', 'dep-2'),
				edge('worker-proxy', 'api-db', 'RequestPath', 'dep-2')
			],
			elementToContainer
		);

		expect(bundles).toHaveLength(2);
		expect(new Set(bundles.map((b) => b.id)).size).toBe(2);
	});
});
