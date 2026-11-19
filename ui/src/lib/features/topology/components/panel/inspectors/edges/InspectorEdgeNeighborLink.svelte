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
		topology_focusLink,
		topology_focusNode,
		topology_neighborLinkPortsUnknown
	} from '#lib/paraglide/messages.js';
	import { discoveryProtocols, entities } from '#lib/shared/stores/metadata.ts';
	import type { components } from '#lib/api/schema';
	import InspectorSection from '../shared/InspectorSection.svelte';
	import HostLinkTrace from '../shared/HostLinkTrace.svelte';
	import type { Edge } from '@xyflow/svelte';
	import { useSvelteFlow } from '@xyflow/svelte';
	import { Crosshair } from '@lucide/svelte';
	import { focusLink, focusNodes, outermostNode } from '#lib/features/topology/viewport-fit.ts';

	let {
		edge,
		sourceHostId,
		targetHostId,
		protocol
	}: {
		edge?: Edge;
		sourceHostId?: string;
		targetHostId?: string;
		protocol?: components['schemas']['DiscoveryProtocol'];
	} = $props();

	const flow = useSvelteFlow();

	/** Moves the camera onto the outermost container of one end of the link. */
	function focusEnd(nodeId: string) {
		focusNodes(flow, [outermostNode(nodeId, flow.getInternalNode)]);
	}

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
	<div class="flex items-center gap-2">
		{#if protocol}
			<Tag {...discoveryProtocols.getTag(protocol)} />
		{/if}
		{#if edge}
			{@const current = edge}
			<button
				class="btn-icon ml-auto p-0.5"
				onclick={() => focusLink(flow, current)}
				title={topology_focusLink()}
			>
				<Crosshair class="h-3.5 w-3.5" />
			</button>
		{/if}
	</div>

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
			{#snippet actions()}
				{#if edge}
					{@const end = edge.source}
					<button class="btn-icon p-0.5" onclick={() => focusEnd(end)} title={topology_focusNode()}>
						<Crosshair class="h-3.5 w-3.5" />
					</button>
				{/if}
			{/snippet}
			<div class="space-y-2">
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
				{#if edge && topology}
					<HostLinkTrace
						{edge}
						hostId={sourceHost.id}
						{topology}
						sectionId="edge:NeighborLink:source:links"
					/>
				{/if}
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
			{#snippet actions()}
				{#if edge}
					{@const end = edge.target}
					<button class="btn-icon p-0.5" onclick={() => focusEnd(end)} title={topology_focusNode()}>
						<Crosshair class="h-3.5 w-3.5" />
					</button>
				{/if}
			{/snippet}
			<div class="space-y-2">
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
				{#if edge && topology}
					<HostLinkTrace
						{edge}
						hostId={targetHost.id}
						{topology}
						sectionId="edge:NeighborLink:target:links"
					/>
				{/if}
			</div>
		</InspectorSection>
	{/if}
</div>
