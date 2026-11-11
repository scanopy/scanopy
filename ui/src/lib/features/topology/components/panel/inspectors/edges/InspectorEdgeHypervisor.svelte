<script lang="ts">
	import { hostIcon } from '#lib/features/hosts/host-icon.ts';
	import type { Edge } from '@xyflow/svelte';
	import EntityDisplayWrapper from '#lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { ServiceDisplay } from '#lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import { HostDisplay } from '#lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { useTopology, selectedTopologyId } from '#lib/features/topology/context.ts';
	import { getTopologyEditState } from '#lib/features/topology/state.ts';
	import { topologyReadOnly } from '#lib/features/topology/queries.ts';
	import type { RenderableTopology } from '#lib/features/topology/types/base.ts';
	import {
		hosts_virtualization_hypervisorService,
		hosts_virtualization_virtualMachines,
		hosts_virtualization_noVmsYet,
		inspector_hypervisorHost
	} from '#lib/paraglide/messages.js';
	import { entities } from '#lib/shared/stores/metadata.ts';
	import InspectorSection from '../shared/InspectorSection.svelte';

	let { edge, hypervisorServiceId }: { edge: Edge; hypervisorServiceId: string } = $props();

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

	let hypervisorService = $derived(
		topology ? topology.services.find((s) => s.id == hypervisorServiceId) : null
	);
	let hypervisorHost = $derived(topology ? topology.hosts.find((h) => h.id == edge.target) : null);

	let managedVms = $derived(
		topology
			? topology.hosts.filter((h) => h.virtualization_service_id === hypervisorServiceId)
			: []
	);
</script>

<div class="space-y-4">
	{#if hypervisorService}
		<InspectorSection
			id="edge:Hypervisor:service"
			title={hosts_virtualization_hypervisorService()}
			icon={entities.getIconComponent('Service')}
			iconClass={entities.getColorHelper('Service').icon}
			description={null}
		>
			<div class="card-static card">
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
		</InspectorSection>
	{/if}

	{#if hypervisorHost}
		<InspectorSection
			id="edge:Hypervisor:host"
			title={inspector_hypervisorHost()}
			icon={hostIcon(hypervisorHost)}
			iconClass={entities.getColorHelper('Host').icon}
			description={null}
		>
			<div class="card-static card">
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
		</InspectorSection>
	{/if}

	<InspectorSection
		id="edge:Hypervisor:guests"
		title={hosts_virtualization_virtualMachines()}
		icon={entities.getIconComponent('Host')}
		iconClass={entities.getColorHelper('Host').icon}
		description={null}
		count={managedVms.length}
	>
		{#if managedVms.length === 0}
			<p class="text-sm text-secondary">{hosts_virtualization_noVmsYet()}</p>
		{:else}
			<div class="space-y-1">
				{#each managedVms as vmHost (vmHost.id)}
					<div class="card-static card">
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
			</div>
		{/if}
	</InspectorSection>
</div>
