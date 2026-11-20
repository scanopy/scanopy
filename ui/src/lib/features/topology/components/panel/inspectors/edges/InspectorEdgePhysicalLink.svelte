<script lang="ts">
	import EntityDisplayWrapper from '#lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '#lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { InterfaceDisplay } from '#lib/shared/components/forms/selection/display/InterfaceDisplay.svelte';
	import { useTopology, selectedTopologyId } from '#lib/features/topology/context.ts';
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import type { RenderableTopology } from '#lib/features/topology/types/base.ts';
	import {
		common_source,
		common_speed,
		common_target,
		topology_focusLink,
		topology_focusNode,
		topology_neighborEvidence,
		topology_speedMismatchNote
	} from '#lib/paraglide/messages.js';
	import { discoveryProtocols, entities } from '#lib/shared/stores/metadata.ts';
	import type { components } from '#lib/api/schema';
	import InspectorSection from '../shared/InspectorSection.svelte';
	import HostLinkTrace from '../shared/HostLinkTrace.svelte';
	import type { Edge } from '@xyflow/svelte';
	import { useSvelteFlow } from '@xyflow/svelte';
	import { Crosshair } from '@lucide/svelte';
	import { focusLink, focusNodes, outermostNode } from '#lib/features/topology/viewport-fit.ts';
	import { useSitesQuery } from '#lib/features/sites/queries.ts';
	import { neighborEvidenceTag } from '#lib/shared/utils/freshness.ts';
	import { formatBitsPerSecond, formatRelativeTime } from '#lib/shared/utils/formatting.ts';

	let {
		edge,
		sourceEntityId,
		targetEntityId,
		protocol
	}: {
		edge?: Edge;
		sourceEntityId?: string;
		targetEntityId?: string;
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

	// Derive Interface and Host data
	let sourceInterface = $derived(topology?.interfaces.find((e) => e.id === sourceEntityId));
	let targetInterface = $derived(topology?.interfaces.find((e) => e.id === targetEntityId));
	let sourceHost = $derived(
		sourceInterface ? topology?.hosts.find((h) => h.id === sourceInterface.host_id) : null
	);
	let targetHost = $derived(
		targetInterface ? topology?.hosts.find((h) => h.id === targetInterface.host_id) : null
	);
	// Each end's reported speed, formatted. Shown as one when the ends agree or only one reports.
	let sourceSpeed = $derived(
		sourceInterface?.speed_bps ? formatBitsPerSecond(sourceInterface.speed_bps) : null
	);
	let targetSpeed = $derived(
		targetInterface?.speed_bps ? formatBitsPerSecond(targetInterface.speed_bps) : null
	);

	// When the adjacency itself was last reported, as opposed to when its ports were last observed
	// — the two diverge the moment a neighbour record stops arriving, and the ports go on being
	// seen every scan. Labelled "Neighbor report" rather than "Last seen" for exactly that reason:
	// on a link, "last seen" reads as a claim about the port, which is the confusion this row
	// exists to end. Shown whether or not it is stale, so the timestamp is reachable here even
	// where the edge label (and its chip) was stripped.
	//
	// GH #701: `neighbor_seen_at` moved off `Interface` onto the per-adjacency `InterfaceNeighborRow`
	// — a port can resolve to several neighbours now, so "the row for this edge" is the one naming
	// the *other* endpoint specifically, not a generic per-interface value. Checked from both
	// directions since a link is recorded on one side and either endpoint's row could be it.
	const sitesQuery = useSitesQuery();
	let evidenceRow = $derived(
		(topology?.neighbours ?? [])
			.filter(
				(n) =>
					((n.interface_id === sourceEntityId && n.neighbor.id === targetEntityId) ||
						(n.interface_id === targetEntityId && n.neighbor.id === sourceEntityId)) &&
					n.neighbor.type === 'Interface'
			)
			.filter((n) => n.neighbor_seen_at)
			.sort((a, b) => (a.neighbor_seen_at! < b.neighbor_seen_at! ? -1 : 1))[0]
	);
	let evidenceInterface = $derived(
		evidenceRow ? topology?.interfaces.find((i) => i.id === evidenceRow!.interface_id) : undefined
	);
	let evidenceSite = $derived(
		(sitesQuery.data ?? []).find((n) => n.id === evidenceInterface?.site_id)
	);
	let evidenceTag = $derived(evidenceRow ? neighborEvidenceTag(evidenceRow, evidenceSite) : null);
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

	{#if evidenceRow?.neighbor_seen_at}
		<div class="flex items-center gap-2 text-sm">
			<span class="font-medium text-secondary">{topology_neighborEvidence()}</span>
			<span>{formatRelativeTime(evidenceRow.neighbor_seen_at)}</span>
			{#if evidenceTag}
				<Tag {...evidenceTag} pill />
			{/if}
		</div>
	{/if}

	{#if sourceSpeed || targetSpeed}
		<div class="flex items-center gap-2 text-sm">
			<span class="font-medium text-secondary">{common_speed()}</span>
			{#if sourceSpeed && targetSpeed && sourceSpeed !== targetSpeed}
				<span>{topology_speedMismatchNote({ source: sourceSpeed, target: targetSpeed })}</span>
			{:else}
				<span>{sourceSpeed ?? targetSpeed}</span>
			{/if}
		</div>
	{/if}

	{#if sourceHost || sourceInterface}
		<InspectorSection
			id="edge:PhysicalLink:source"
			title={common_source()}
			icon={entities.getIconComponent('Interface')}
			iconClass={entities.getColorHelper('Interface').icon}
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
			<div class="space-y-1">
				{#if sourceHost}
					<div class="card-static card">
						<EntityDisplayWrapper
							context={{
								services: topology?.services.filter((s) => s.host_id === sourceHost.id) ?? [],
								compact: true,
								// The link is about which port meets which; the guest type is noise.
								hideTags: ['guest']
							}}
							item={sourceHost}
							displayComponent={HostDisplay}
						/>
					</div>
				{/if}
				{#if sourceInterface}
					<div class="card-static card">
						<EntityDisplayWrapper
							context={{ compact: true }}
							item={sourceInterface}
							displayComponent={InterfaceDisplay}
						/>
					</div>
				{/if}
				{#if edge && topology && sourceHost}
					<HostLinkTrace
						{edge}
						hostId={sourceHost.id}
						{topology}
						sectionId="edge:PhysicalLink:source:links"
					/>
				{/if}
			</div>
		</InspectorSection>
	{/if}

	{#if targetHost || targetInterface}
		<InspectorSection
			id="edge:PhysicalLink:target"
			title={common_target()}
			icon={entities.getIconComponent('Interface')}
			iconClass={entities.getColorHelper('Interface').icon}
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
			<div class="space-y-1">
				{#if targetHost}
					<div class="card-static card">
						<EntityDisplayWrapper
							context={{
								services: topology?.services.filter((s) => s.host_id === targetHost.id) ?? [],
								compact: true,
								// The link is about which port meets which; the guest type is noise.
								hideTags: ['guest']
							}}
							item={targetHost}
							displayComponent={HostDisplay}
						/>
					</div>
				{/if}
				{#if targetInterface}
					<div class="card-static card">
						<EntityDisplayWrapper
							context={{ compact: true }}
							item={targetInterface}
							displayComponent={InterfaceDisplay}
						/>
					</div>
				{/if}
				{#if edge && topology && targetHost}
					<HostLinkTrace
						{edge}
						hostId={targetHost.id}
						{topology}
						sectionId="edge:PhysicalLink:target:links"
					/>
				{/if}
			</div>
		</InspectorSection>
	{/if}
</div>
