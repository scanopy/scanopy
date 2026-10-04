<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import type { ElementRenderContext } from '$lib/features/topology/resolvers';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { DependencyDisplay } from '$lib/shared/components/forms/selection/display/DependencyDisplay.svelte';
	import {
		common_inbound,
		common_outbound,
		inspector_noDependencies
	} from '$lib/paraglide/messages';
	import InspectorSection from '../shared/InspectorSection.svelte';
	import InspectorSubsection from '../shared/InspectorSubsection.svelte';

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

	let service = $derived(
		elementContext?.elementType === 'Service' && elementContext.services.length > 0
			? elementContext.services[0]
			: null
	);

	// Find dependencies where this service is a member
	let inboundDeps = $derived.by(() => {
		if (!service) return [];
		return topology.dependencies.filter((d) => {
			const members = d.members;
			if (members.type === 'Services') {
				const idx = members.service_ids.indexOf(service!.id);
				return idx > 0;
			}
			if (members.type === 'Bindings') {
				const serviceBindingIds = service!.bindings.map((b) => b.id);
				const idx = members.binding_ids.findIndex((bid) => serviceBindingIds.includes(bid));
				return idx > 0;
			}
			return false;
		});
	});

	let outboundDeps = $derived.by(() => {
		if (!service) return [];
		return topology.dependencies.filter((d) => {
			const members = d.members;
			if (members.type === 'Services') {
				return members.service_ids[0] === service!.id;
			}
			if (members.type === 'Bindings') {
				const serviceBindingIds = service!.bindings.map((b) => b.id);
				return serviceBindingIds.includes(members.binding_ids[0]);
			}
			return false;
		});
	});

	let hasDeps = $derived(inboundDeps.length > 0 || outboundDeps.length > 0);
</script>

<InspectorSection
	id="Dependencies"
	section="Dependencies"
	count={hasDeps ? outboundDeps.length + inboundDeps.length : undefined}
>
	{#if !hasDeps}
		<p class="text-tertiary text-sm">{inspector_noDependencies()}</p>
	{:else}
		<div class="space-y-3">
			{#if outboundDeps.length > 0}
				<InspectorSubsection
					id="Dependencies:outbound"
					title={common_outbound()}
					count={outboundDeps.length}
				>
					<div class="space-y-1">
						{#each outboundDeps as dep (dep.id)}
							<div class="card card-static">
								<EntityDisplayWrapper
									item={dep}
									context={{ compact: true }}
									displayComponent={DependencyDisplay}
								/>
							</div>
						{/each}
					</div>
				</InspectorSubsection>
			{/if}
			{#if inboundDeps.length > 0}
				<InspectorSubsection
					id="Dependencies:inbound"
					title={common_inbound()}
					count={inboundDeps.length}
				>
					<div class="space-y-1">
						{#each inboundDeps as dep (dep.id)}
							<div class="card card-static">
								<EntityDisplayWrapper
									item={dep}
									context={{ compact: true }}
									displayComponent={DependencyDisplay}
								/>
							</div>
						{/each}
					</div>
				</InspectorSubsection>
			{/if}
		</div>
	{/if}
</InspectorSection>
