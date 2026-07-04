<script lang="ts">
	import { et } from '$lib/shared/utils/embed-i18n';
	import type { Edge } from '@xyflow/svelte';
	import type { TopologyEdge } from '$lib/features/topology/types/base';
	import { activeView, aggregatedEdgeOriginals } from '$lib/features/topology/queries';
	import InspectorEdgeDependency from './edges/InspectorEdgeDependency.svelte';
	import InspectorEdgeIPAddress from './edges/InspectorEdgeIPAddress.svelte';
	import InspectorEdgeHypervisor from './edges/InspectorEdgeHypervisor.svelte';
	import InspectorEdgeContainerRuntime from './edges/InspectorEdgeContainerRuntime.svelte';
	import InspectorEdgePhysicalLink from './edges/InspectorEdgePhysicalLink.svelte';
	import InspectorEdgeAggregated from './edges/InspectorEdgeAggregated.svelte';

	let { edge }: { edge: Edge } = $props();

	let edgeData = $derived(edge?.data as (TopologyEdge & { isAggregated?: boolean }) | undefined);
	let view = $derived($activeView);
	let originalEdges = $derived(
		edgeData?.isAggregated ? $aggregatedEdgeOriginals.get(edge.id) : undefined
	);
</script>

<div class="w-full space-y-4">
	{#if !edgeData}
		<div class="space-y-3">
			<p class="text-tertiary text-sm">{et('Edge data not available')}</p>
		</div>
	{:else if edgeData.isAggregated && originalEdges}
		<InspectorEdgeAggregated edges={originalEdges} />
	{:else if edgeData.edge_type === 'HubAndSpoke' || edgeData.edge_type === 'RequestPath'}
		<InspectorEdgeDependency
			dependencyId={edgeData?.dependency_id}
			sourceId={edgeData?.source_id}
			targetId={edgeData?.target_id}
			{view}
		/>
	{:else if edgeData.edge_type === 'SameHost'}
		<InspectorEdgeIPAddress {edge} hostId={edgeData?.host_id} {view} />
	{:else if edgeData.edge_type === 'Hypervisor'}
		<InspectorEdgeHypervisor {edge} hypervisorServiceId={edgeData?.hypervisor_service_id} />
	{:else if edgeData.edge_type === 'ContainerRuntime'}
		<InspectorEdgeContainerRuntime {edge} serviceId={edgeData?.service_id} />
	{:else if edgeData.edge_type === 'PhysicalLink'}
		<InspectorEdgePhysicalLink
			sourceEntityId={edgeData?.source_entity_id}
			targetEntityId={edgeData?.target_entity_id}
			protocol={edgeData?.protocol}
		/>
	{:else}
		<div class="space-y-3">
			<p class="text-tertiary text-sm">{et('Unable to display edge details')}</p>
		</div>
	{/if}
</div>
