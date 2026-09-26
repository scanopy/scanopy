<script lang="ts">
	import type { Edge } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import { topologyReadOnly } from '$lib/features/topology/queries';
	import { useTopology, selectedTopologyId } from '$lib/features/topology/context';
	import { getTopologyEditState } from '$lib/features/topology/state';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import {
		common_containerizedService,
		common_containerizedServices,
		topology_containerBridgeSubnet,
		topology_containerBridgeSubnets,
		topology_containerHost,
		topology_containerService
	} from '$lib/paraglide/messages';

	let { edge, serviceId }: { edge: Edge; serviceId: string } = $props();

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let isReadonly = $derived(topo.isReadonly || $topologyReadOnly);
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					RenderableTopology | undefined)
	);

	// Unified edit state
	let editState = $derived(getTopologyEditState(topology, false, isReadonly));

	let containerizingService = $derived(
		topology ? topology.services.find((s) => s.id == serviceId) : null
	);

	let containerizingHost = $derived(
		containerizingService && topology
			? topology.hosts.find((h) => h.id == containerizingService.host_id)
			: null
	);

	// The edge names the containers it stands for — the ones on the bridge subnet(s) it
	// reaches, already narrowed for the current grouping. Resolving them here from the edge's
	// endpoint can't work: the endpoint is elevated onto the subnet box before it reaches us.
	let containerizedServiceIds = $derived(
		((edge.data as Record<string, unknown> | undefined)?.containerized_service_ids as
			string[] | undefined) ?? []
	);
	let containerizedServices = $derived(
		topology
			? containerizedServiceIds.flatMap((id) => topology.services.find((s) => s.id === id) ?? [])
			: []
	);

	// The bridges this edge reaches — one when they render as separate boxes, all of them when
	// merged. Walking the listed containers' bindings instead would pull in every other bridge
	// a multi-attached container happens to sit on, which this edge does not connect.
	let allBridgeSubnets = $derived(
		topology
			? (
					((edge.data as Record<string, unknown> | undefined)?.subnet_ids as
						string[] | undefined) ?? []
				).flatMap((id) => topology.subnets.find((s) => s.id === id) ?? [])
			: []
	);
</script>

<div class="space-y-3">
	{#if containerizingHost}
		<span class="text-secondary mb-2 block text-sm font-medium">{topology_containerHost()}</span>
		<div class="card card-static">
			<EntityDisplayWrapper
				context={{
					services:
						topology?.services.filter((s) =>
							containerizingHost ? s.host_id == containerizingHost.id : false
						) ?? [],
					showEntityTagPicker: true,
					tagPickerDisabled: !editState.isEditable,
					entityTags: isReadonly ? (topology?.entity_tags ?? []) : undefined,
					compact: true
				}}
				item={containerizingHost}
				displayComponent={HostDisplay}
			/>
		</div>
	{/if}
	{#if containerizingService}
		<span class="text-secondary mb-2 block text-sm font-medium">{topology_containerService()}</span>
		<div class="card card-static">
			<EntityDisplayWrapper
				context={{
					ipAddressId: null,
					ports: topology?.ports ?? [],
					showEntityTagPicker: true,
					tagPickerDisabled: !editState.isEditable,
					entityTags: isReadonly ? (topology?.entity_tags ?? []) : undefined,
					compact: true
				}}
				item={containerizingService}
				displayComponent={ServiceDisplay}
			/>
		</div>
	{/if}

	<span class="text-secondary mb-2 block text-sm font-medium">
		{containerizedServices.length === 1
			? common_containerizedService()
			: common_containerizedServices()}
	</span>
	{#each containerizedServices as service (service.id)}
		<div class="card card-static">
			<EntityDisplayWrapper
				context={{
					ipAddressId: null,
					ports: topology?.ports ?? [],
					showEntityTagPicker: true,
					tagPickerDisabled: !editState.isEditable,
					entityTags: isReadonly ? (topology?.entity_tags ?? []) : undefined,
					compact: true
				}}
				item={service}
				displayComponent={ServiceDisplay}
			/>
		</div>
	{/each}

	{#if allBridgeSubnets.length > 0}
		<span class="text-secondary mb-2 block text-sm font-medium"
			>{allBridgeSubnets.length > 1
				? topology_containerBridgeSubnets()
				: topology_containerBridgeSubnet()}</span
		>
		{#each allBridgeSubnets as subnet (subnet.id)}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={{ compact: true }}
					item={subnet}
					displayComponent={SubnetDisplay}
				/>
			</div>
		{/each}
	{/if}
</div>
