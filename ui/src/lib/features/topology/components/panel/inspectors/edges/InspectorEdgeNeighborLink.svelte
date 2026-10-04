<script lang="ts">
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { useTopology, selectedTopologyId } from '$lib/features/topology/context';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import {
		common_source,
		common_target,
		topology_neighborLinkPortsUnknown
	} from '$lib/paraglide/messages';
	import { discoveryProtocols, entities } from '$lib/shared/stores/metadata';
	import type { components } from '$lib/api/schema';
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
			<Tag {...discoveryProtocols.getTag(protocol, common_source())} />
		</div>
	{/if}

	<p class="text-tertiary text-sm">{topology_neighborLinkPortsUnknown()}</p>

	{#if sourceHost}
		<InspectorSection
			id="edge:NeighborLink:source"
			title={common_source()}
			icon={entities.getIconComponent('Host')}
			iconClass={entities.getColorHelper('Host').icon}
			description={null}
		>
			<div class="card card-static">
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
			icon={entities.getIconComponent('Host')}
			iconClass={entities.getColorHelper('Host').icon}
			description={null}
		>
			<div class="card card-static">
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
