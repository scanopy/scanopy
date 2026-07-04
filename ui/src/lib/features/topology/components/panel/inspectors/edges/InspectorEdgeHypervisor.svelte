<script lang="ts">
	import { et } from '$lib/shared/utils/embed-i18n';
	import type { Edge } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { useTopology, selectedTopologyId } from '$lib/features/topology/context';
	import { getTopologyEditState } from '$lib/features/topology/state';
	import { topologyReadOnly } from '$lib/features/topology/queries';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import {
		hosts_virtualization_hypervisorService,
		hosts_virtualization_virtualMachines,
		hosts_virtualization_noVmsYet
	} from '$lib/paraglide/messages';

	let { edge, hypervisorServiceId }: { edge: Edge; hypervisorServiceId: string } = $props();

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let isReadonly = $derived(topo.isReadonly || $topologyReadOnly);
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					| RenderableTopology
					| undefined)
	);

	let editState = $derived(getTopologyEditState(topology, false, isReadonly));

	let hypervisorService = $derived(
		topology ? topology.services.find((s) => s.id == hypervisorServiceId) : null
	);
	let hypervisorHost = $derived(topology ? topology.hosts.find((h) => h.id == edge.target) : null);

	let managedVms = $derived(
		topology
			? topology.hosts.filter(
					(h) =>
						h.virtualization?.type === 'Proxmox' &&
						h.virtualization.details.service_id === hypervisorServiceId
				)
			: []
	);
</script>

<div class="space-y-3">
	{#if hypervisorService}
		<span class="text-secondary mb-2 block text-sm font-medium"
			>{hosts_virtualization_hypervisorService()}</span
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
				item={hypervisorService}
				displayComponent={ServiceDisplay}
			/>
		</div>
	{/if}

	{#if hypervisorHost}
		<span class="text-secondary mb-2 block text-sm font-medium">{et('Hypervisor Host')}</span>
		<div class="card card-static">
			<EntityDisplayWrapper
				context={{
					services:
						topology?.services.filter((s) =>
							hypervisorHost ? s.host_id == hypervisorHost.id : false
						) ?? [],
					showEntityTagPicker: true,
					tagPickerDisabled: !editState.isEditable,
					entityTags: isReadonly ? (topology?.entity_tags ?? []) : undefined,
					compact: true
				}}
				item={hypervisorHost}
				displayComponent={HostDisplay}
			/>
		</div>
	{/if}

	<span class="text-secondary mb-2 block text-sm font-medium"
		>{hosts_virtualization_virtualMachines()}</span
	>
	{#if managedVms.length === 0}
		<p class="text-secondary text-sm">{hosts_virtualization_noVmsYet()}</p>
	{:else}
		{#each managedVms as vmHost (vmHost.id)}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={{
						services: topology?.services.filter((s) => s.host_id == vmHost.id) ?? [],
						showEntityTagPicker: true,
						tagPickerDisabled: !editState.isEditable,
						entityTags: isReadonly ? (topology?.entity_tags ?? []) : undefined,
						compact: true
					}}
					item={vmHost}
					displayComponent={HostDisplay}
				/>
			</div>
		{/each}
	{/if}
</div>
