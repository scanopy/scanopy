<script lang="ts">
	import { hostIcon } from '#lib/features/hosts/host-icon.ts';
	import EntityDisplayWrapper from '#lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '#lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { useTopology, selectedTopologyId } from '#lib/features/topology/context.ts';
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import type { RenderableTopology } from '#lib/features/topology/types/base.ts';
	import {
		common_source,
		common_target,
		topology_neighborLinkPortsUnknown
	} from '#lib/paraglide/messages.js';
	import { discoveryProtocols, entities } from '#lib/shared/stores/metadata.ts';
	import type { components } from '#lib/api/schema';
	import InspectorSection from '../shared/InspectorSection.svelte';

	let {
		sourceHostId,
		targetHostId,
		protocol
	}: {
		sourceHostId?: string;
		targetHostId?: string;
		protocol?: components['schemas']['DiscoveryProtocol'];
	} = $props();

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					RenderableTopology | undefined)
	);

	let sourceHost = $derived(topology?.hosts.find((h) => h.id === sourceHostId));
	let targetHost = $derived(topology?.hosts.find((h) => h.id === targetHostId));
</script>

<div class="space-y-4">
	{#if protocol}
		<div class="flex items-center gap-2">
			<Tag {...discoveryProtocols.getTag(protocol)} />
		</div>
	{/if}

	<!-- A forwarding-table link says what that evidence is; an LLDP/CDP one that the ports are
	     unknown. -->
	<p class="text-sm text-tertiary">
		{protocol === 'ForwardingTable'
			? discoveryProtocols.getDescription(protocol)
			: topology_neighborLinkPortsUnknown()}
	</p>

	{#if sourceHost}
		<InspectorSection
			id="edge:NeighborLink:source"
			title={common_source()}
			icon={hostIcon(sourceHost)}
			iconClass={entities.getColorHelper('Host').icon}
			description={null}
		>
			<div class="card-static card">
				<EntityDisplayWrapper
					context={{
						services: topology?.services.filter((s) => s.host_id === sourceHost.id) ?? [],
						compact: true,
						hideTags: ['guest']
					}}
					item={sourceHost}
					displayComponent={HostDisplay}
				/>
			</div>
		</InspectorSection>
	{/if}

	{#if targetHost}
		<InspectorSection
			id="edge:NeighborLink:target"
			title={common_target()}
			icon={hostIcon(targetHost)}
			iconClass={entities.getColorHelper('Host').icon}
			description={null}
		>
			<div class="card-static card">
				<EntityDisplayWrapper
					context={{
						services: topology?.services.filter((s) => s.host_id === targetHost.id) ?? [],
						compact: true,
						hideTags: ['guest']
					}}
					item={targetHost}
					displayComponent={HostDisplay}
				/>
			</div>
		</InspectorSection>
	{/if}
</div>
