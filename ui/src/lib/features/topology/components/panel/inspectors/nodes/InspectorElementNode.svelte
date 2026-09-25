<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import { activeView, topologyReadOnly } from '$lib/features/topology/queries';
	import type { TopologyNode, RenderableTopology } from '$lib/features/topology/types/base';
	import { resolveElementNode } from '$lib/features/topology/resolvers';
	import { useTopology, selectedTopologyId } from '$lib/features/topology/context';
	import { getTopologyEditState } from '$lib/features/topology/state';
	import { getInspectorConfig, getSectionComponent } from '../view-config';

	let { node }: { node: Node } = $props();

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

	let resolved = $derived(
		topology ? resolveElementNode(node.id, node.data as TopologyNode, topology) : null
	);

	// View-driven section config
	let config = $derived(getInspectorConfig($activeView));
	let sections = $derived(config.element_sections);
</script>

{#if topology && resolved}
	<div class="space-y-4">
		{#each sections as section (section)}
			{@const SectionComponent = getSectionComponent(section)}
			<SectionComponent {node} {topology} {editState} elementContext={resolved} />
		{/each}
	</div>
{/if}
