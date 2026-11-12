<script lang="ts">
	import { hostIcon } from '#lib/features/hosts/host-icon.ts';
	import type { Edge } from '@xyflow/svelte';
	import EntityDisplayWrapper from '#lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '#lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { IPAddressDisplay } from '#lib/shared/components/forms/selection/display/IPAddressDisplay.svelte';
	import { useTopology, selectedTopologyId } from '#lib/features/topology/context.ts';
	import { getTopologyEditState } from '#lib/features/topology/state.ts';
	import { topologyReadOnly } from '#lib/features/topology/queries.ts';
	import type { RenderableTopology } from '#lib/features/topology/types/base.ts';
	import { common_host, common_ipAddresses } from '#lib/paraglide/messages.js';
	import { entities } from '#lib/shared/stores/metadata.ts';
	import InspectorSection from '../shared/InspectorSection.svelte';

	import type { components } from '#lib/api/schema';
	type TopologyView = components['schemas']['TopologyView'];

	/* eslint-disable @typescript-eslint/no-unused-vars -- component contract props */
	let {
		edge,
		hostId,
		view = 'L3Logical'
	}: {
		edge: Edge;
		hostId: string;
		view?: TopologyView;
	} = $props();
	/* eslint-enable @typescript-eslint/no-unused-vars */

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

	let host = $derived(topology ? topology.hosts.find((h) => h.id == hostId) : null);

	let sourceInterface = $derived(topology?.ip_addresses.find((i) => i.id == edge.source));
	let targetInterface = $derived(topology?.ip_addresses.find((i) => i.id == edge.target));

	// Context for interface displays
	let interfaceContext = $derived({ subnets: topology?.subnets ?? [], compact: true });
</script>

<div class="space-y-4">
	{#if host}
		<InspectorSection
			id="edge:SameHost:host"
			title={common_host()}
			icon={hostIcon(host)}
			iconClass={entities.getColorHelper('Host').icon}
			description={null}
		>
			<div class="card-static card">
				<EntityDisplayWrapper
					context={{
						services: topology?.services.filter((s) => host && s.host_id == host.id) ?? [],
						showEntityTagPicker: true,
						tagPickerDisabled: !editState.isEditable,
						entityTags: isReadonly ? (topology?.entity_tags ?? []) : undefined,
						compact: true
					}}
					item={host}
					displayComponent={HostDisplay}
				/>
			</div>
		</InspectorSection>
	{/if}
	<InspectorSection
		id="edge:SameHost:ipAddresses"
		title={common_ipAddresses()}
		icon={entities.getIconComponent('IPAddress')}
		iconClass={entities.getColorHelper('IPAddress').icon}
		description={null}
		count={(sourceInterface ? 1 : 0) + (targetInterface ? 1 : 0)}
	>
		<div class="space-y-1">
			{#if sourceInterface}
				<div class="card-static card">
					<EntityDisplayWrapper
						context={interfaceContext}
						item={sourceInterface}
						displayComponent={IPAddressDisplay}
					/>
				</div>
			{/if}

			{#if targetInterface}
				<div class="card-static card">
					<EntityDisplayWrapper
						context={interfaceContext}
						item={targetInterface}
						displayComponent={IPAddressDisplay}
					/>
				</div>
			{/if}
		</div>
	</InspectorSection>
</div>
