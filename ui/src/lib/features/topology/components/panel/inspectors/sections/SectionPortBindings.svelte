<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import type { ElementRenderContext } from '$lib/features/topology/resolvers';
	import InspectorSection from '../shared/InspectorSection.svelte';

	/* eslint-disable @typescript-eslint/no-unused-vars -- component contract props */
	let {
		node,
		topology,
		elementContext
	}: {
		node: Node;
		topology: RenderableTopology;
		elementContext?: ElementRenderContext;
	} = $props();
	/* eslint-enable @typescript-eslint/no-unused-vars */

	// For Service elements, show port bindings
	let service = $derived(
		elementContext?.elementType === 'Service' && elementContext.services.length > 0
			? elementContext.services[0]
			: null
	);

	let portBindings = $derived.by(() => {
		if (!service) return [];
		return service.bindings
			.filter((b) => b.type === 'Port')
			.map((b) => {
				const port = topology.ports.find((p) => p.id === b.port_id);
				return { binding: b, port };
			});
	});
</script>

{#if portBindings.length > 0}
	<InspectorSection id="PortBindings" section="PortBindings" count={portBindings.length}>
		<div class="card card-static space-y-1">
			{#each portBindings as { binding, port } (binding.id)}
				<div class="flex items-center gap-2 text-sm">
					<span class="text-primary font-mono">
						{port ? `${port.number}/${port.protocol.toLowerCase()}` : binding.port_id}
					</span>
				</div>
			{/each}
		</div>
	</InspectorSection>
{/if}
