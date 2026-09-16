<script lang="ts">
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { InterfaceDisplay } from '$lib/shared/components/forms/selection/display/InterfaceDisplay.svelte';
	import { useTopology, selectedTopologyId } from '$lib/features/topology/context';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import { common_source, common_target, topology_neighborEvidence } from '$lib/paraglide/messages';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { neighborEvidenceTag } from '$lib/shared/utils/freshness';
	import { formatRelativeTime } from '$lib/shared/utils/formatting';

	let {
		sourceEntityId,
		targetEntityId,
		protocol
	}: {
		sourceEntityId?: string;
		targetEntityId?: string;
		protocol?: 'LLDP' | 'CDP';
	} = $props();

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					| RenderableTopology
					| undefined)
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
	const networksQuery = useNetworksQuery();
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
	let evidenceNetwork = $derived(
		(networksQuery.data ?? []).find((n) => n.id === evidenceInterface?.network_id)
	);
	let evidenceTag = $derived(
		evidenceRow ? neighborEvidenceTag(evidenceRow, evidenceNetwork) : null
	);
</script>

<div class="space-y-3">
	{#if protocol}
		<div class="flex items-center gap-2">
			<Tag label={protocol} color={protocol == 'CDP' ? 'Blue' : 'Green'} />
		</div>
	{/if}

	{#if evidenceRow?.neighbor_seen_at}
		<div class="flex items-center gap-2 text-sm">
			<span class="text-secondary font-medium">{topology_neighborEvidence()}</span>
			<span>{formatRelativeTime(evidenceRow.neighbor_seen_at)}</span>
			{#if evidenceTag}
				<Tag {...evidenceTag} pill />
			{/if}
		</div>
	{/if}

	{#if sourceHost || sourceInterface}
		<span class="text-secondary mb-2 block text-sm font-medium">{common_source()}</span>
		{#if sourceHost}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={{
						services: topology?.services.filter((s) => s.host_id === sourceHost.id) ?? [],
						compact: true
					}}
					item={sourceHost}
					displayComponent={HostDisplay}
				/>
			</div>
		{/if}
		{#if sourceInterface}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={undefined}
					item={sourceInterface}
					displayComponent={InterfaceDisplay}
				/>
			</div>
		{/if}
	{/if}

	{#if targetHost || targetInterface}
		<span class="text-secondary mb-2 block text-sm font-medium">{common_target()}</span>
		{#if targetHost}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={{
						services: topology?.services.filter((s) => s.host_id === targetHost.id) ?? [],
						compact: true
					}}
					item={targetHost}
					displayComponent={HostDisplay}
				/>
			</div>
		{/if}
		{#if targetInterface}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={undefined}
					item={targetInterface}
					displayComponent={InterfaceDisplay}
				/>
			</div>
		{/if}
	{/if}
</div>
