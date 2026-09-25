<script lang="ts">
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import { topologyReadOnly } from '$lib/features/topology/queries';
	import { useTopology, selectedTopologyId } from '$lib/features/topology/context';
	import { getTopologyEditState } from '$lib/features/topology/state';
	import { SvelteMap } from 'svelte/reactivity';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import { subnetTypes } from '$lib/shared/stores/metadata';
	import {
		common_containerizedService,
		topology_containerBridgeSubnets
	} from '$lib/paraglide/messages';

	let { serviceId }: { serviceId: string } = $props();

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let isReadonly = $derived(topo.isReadonly || $topologyReadOnly);
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					RenderableTopology | undefined)
	);

	let editState = $derived(getTopologyEditState(topology, false, isReadonly));

	let container = $derived(topology ? topology.services.find((s) => s.id == serviceId) : null);

	// Every container-bridge subnet this one container is reachable at — the relation the edge
	// represents.
	let bridgeSubnets = $derived.by(() => {
		const subnets = new SvelteMap<string, Subnet>();
		if (!topology || !container) return [];

		for (const binding of container.bindings) {
			const ipAddressId =
				binding.type === 'IPAddress' ? binding.ip_address_id : (binding.ip_address_id ?? null);
			if (!ipAddressId) continue;

			const ipAddress = topology.ip_addresses.find((i) => i.id === ipAddressId);
			if (!ipAddress?.subnet_id) continue;

			const subnet = topology.subnets.find((s) => s.id === ipAddress.subnet_id);
			if (subnet && subnetTypes.getMetadata(subnet.subnet_type).is_container_bridge) {
				subnets.set(subnet.id, subnet);
			}
		}

		return Array.from(subnets.values());
	});
</script>

<div class="space-y-3">
	{#if container}
		<span class="text-secondary mb-2 block text-sm font-medium"
			>{common_containerizedService()}</span
		>
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
				item={container}
				displayComponent={ServiceDisplay}
			/>
		</div>
	{/if}

	{#if bridgeSubnets.length > 0}
		<span class="text-secondary mb-2 block text-sm font-medium"
			>{topology_containerBridgeSubnets()}</span
		>
		{#each bridgeSubnets as subnet (subnet.id)}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={{
						showEntityTagPicker: true,
						tagPickerDisabled: !editState.isEditable,
						entityTags: isReadonly ? (topology?.entity_tags ?? []) : undefined,
						compact: true
					}}
					item={subnet}
					displayComponent={SubnetDisplay}
				/>
			</div>
		{/each}
	{/if}
</div>
