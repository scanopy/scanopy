<script lang="ts">
	import type { Component } from 'svelte';
	import type { Edge } from '@xyflow/svelte';
	import type { components } from '#lib/api/schema';
	import type { TopologyEdge } from '#lib/features/topology/types/base.ts';
	import { activeView, aggregatedEdgeOriginals } from '#lib/features/topology/queries.ts';
	import InspectorEdgeDependency from './edges/InspectorEdgeDependency.svelte';
	import InspectorEdgeIPAddress from './edges/InspectorEdgeIPAddress.svelte';
	import InspectorEdgeHypervisor from './edges/InspectorEdgeHypervisor.svelte';
	import InspectorEdgeNetworkIdentity from './edges/InspectorEdgeNetworkIdentity.svelte';
	import InspectorEdgeContainerRuntime from './edges/InspectorEdgeContainerRuntime.svelte';
	import InspectorEdgePhysicalLink from './edges/InspectorEdgePhysicalLink.svelte';
	import InspectorEdgeNeighborLink from './edges/InspectorEdgeNeighborLink.svelte';
	import InspectorEdgeSameContainer from './edges/InspectorEdgeSameContainer.svelte';
	import InspectorEdgeAggregated from './edges/InspectorEdgeAggregated.svelte';
	import { inspector_edgeDataUnavailable } from '#lib/paraglide/messages.js';

	type EdgeType = components['schemas']['EdgeTypeDiscriminants'];
	type EdgeOf<K extends EdgeType> = Extract<TopologyEdge, { edge_type: K }>;
	interface EdgeInspector<K extends EdgeType> {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		component: Component<any>;
		props: (edge: Edge, data: EdgeOf<K>, view: string) => Record<string, unknown>;
	}

	const dependency = {
		component: InspectorEdgeDependency,
		props: (_edge: Edge, data: EdgeOf<'RequestPath' | 'HubAndSpoke'>, view: string) => ({
			dependencyId: data.dependency_id,
			sourceId: data.source_id,
			targetId: data.target_id,
			view
		})
	};

	/**
	 * The inspector for one edge, by edge type. Keyed by every edge type, so a new one fails to
	 * compile until it has an inspector.
	 */
	const EDGE_INSPECTORS: { [K in EdgeType]: EdgeInspector<K> } = {
		RequestPath: dependency,
		HubAndSpoke: dependency,
		SameHost: {
			component: InspectorEdgeIPAddress,
			props: (edge, data, view) => ({ edge, hostId: data.host_id, view })
		},
		Hypervisor: {
			component: InspectorEdgeHypervisor,
			props: (edge, data) => ({ edge, hypervisorServiceId: data.hypervisor_service_id })
		},
		NetworkIdentity: {
			component: InspectorEdgeNetworkIdentity,
			props: (edge, data) => ({ edge, identitiesServiceId: data.identities_service_id })
		},
		ContainerRuntime: {
			component: InspectorEdgeContainerRuntime,
			props: (edge, data) => ({ edge, serviceId: data.service_id })
		},
		SameContainer: {
			component: InspectorEdgeSameContainer,
			props: (_edge, data) => ({ serviceId: data.service_id })
		},
		PhysicalLink: {
			component: InspectorEdgePhysicalLink,
			props: (_edge, data) => ({
				sourceEntityId: data.source_entity_id,
				targetEntityId: data.target_entity_id,
				protocol: data.protocol
			})
		},
		NeighborLink: {
			component: InspectorEdgeNeighborLink,
			props: (_edge, data) => ({
				sourceHostId: data.source_host_id,
				targetHostId: data.target_host_id,
				protocol: data.protocol
			})
		}
	};

	/** The inspector and its props for an edge of type `K`. */
	function inspectorFor<K extends EdgeType>(edge: Edge, data: EdgeOf<K>, view: string) {
		const entry = EDGE_INSPECTORS[data.edge_type as K];
		return { component: entry.component, props: entry.props(edge, data, view) };
	}

	let { edge }: { edge: Edge } = $props();

	let edgeData = $derived(edge?.data as (TopologyEdge & { isAggregated?: boolean }) | undefined);
	let view = $derived($activeView);
	let originalEdges = $derived(
		edgeData?.isAggregated ? $aggregatedEdgeOriginals.get(edge.id) : undefined
	);
	let inspector = $derived(edgeData ? inspectorFor(edge, edgeData, view) : null);
</script>

<div class="w-full space-y-4">
	{#if !edgeData}
		<div class="space-y-3">
			<p class="text-sm text-tertiary">{inspector_edgeDataUnavailable()}</p>
		</div>
	{:else if edgeData.isAggregated && originalEdges}
		<InspectorEdgeAggregated edges={originalEdges} />
	{:else if inspector}
		<inspector.component {...inspector.props} />
	{/if}
</div>
