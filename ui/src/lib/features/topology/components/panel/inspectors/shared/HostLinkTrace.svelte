<script lang="ts">
	import type { Edge } from '@xyflow/svelte';
	import { useSvelteFlow } from '@xyflow/svelte';
	import { ArrowRight } from '@lucide/svelte';
	import type { RenderableTopology } from '#lib/features/topology/types/base.ts';
	import { linksOfHost, type HostLink } from '#lib/features/topology/edge-tracing.ts';
	import { focusLink } from '#lib/features/topology/viewport-fit.ts';
	import { selectEdge, useSelectionStores } from '#lib/features/topology/selection.ts';
	import { hostDisplayName } from '#lib/features/hosts/host-display-name.ts';
	import { interfaceDisplayName } from '#lib/features/hosts/interface-display-name.ts';
	import { topology_otherLinks } from '#lib/paraglide/messages.js';
	import InspectorSubsection from './InspectorSubsection.svelte';

	/**
	 * The other links on one end of a selected link. Choosing one selects it and moves the camera
	 * onto it, so a path can be followed one hop at a time.
	 */
	let {
		edge,
		hostId,
		topology,
		sectionId
	}: {
		edge: Edge;
		hostId: string;
		topology: RenderableTopology;
		/** Collapse key of the subsection; persisted. */
		sectionId: string;
	} = $props();

	const flow = useSvelteFlow();
	const stores = useSelectionStores();

	let links = $derived(linksOfHost(hostId, flow.getEdges(), topology, edge.id));
	let interfaceById = $derived(new Map(topology.interfaces.map((i) => [i.id, i])));
	let hostById = $derived(new Map(topology.hosts.map((h) => [h.id, h])));

	function portName(id: string | undefined): string | null {
		const iface = id ? interfaceById.get(id) : undefined;
		return iface ? interfaceDisplayName(iface) : null;
	}

	function follow(link: HostLink) {
		selectEdge(link.edge, stores);
		focusLink(flow, link.edge);
	}
</script>

{#if links.length > 0}
	<InspectorSubsection id={sectionId} title={topology_otherLinks()} count={links.length}>
		<ul class="space-y-0.5">
			{#each links as link (link.edge.id)}
				{@const farHost = hostById.get(link.farHostId)}
				{@const localPort = portName(link.localInterfaceId)}
				{@const farPort = portName(link.farInterfaceId)}
				<li>
					<button
						type="button"
						class="flex w-full min-w-0 items-center gap-1.5 rounded px-1 py-0.5 text-left text-sm hover:bg-[var(--color-bg-surface-hover)]"
						onclick={() => follow(link)}
					>
						{#if localPort}
							<span class="shrink-0 text-tertiary">{localPort}</span>
						{/if}
						<ArrowRight class="h-3 w-3 shrink-0 text-tertiary" />
						<span class="truncate text-secondary">
							{farHost ? hostDisplayName(farHost) : link.farHostId}
						</span>
						{#if farPort}
							<span class="shrink-0 text-tertiary">{farPort}</span>
						{/if}
					</button>
				</li>
			{/each}
		</ul>
	</InspectorSubsection>
{/if}
