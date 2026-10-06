<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import { useSvelteFlow } from '@xyflow/svelte';
	import { Crosshair } from 'lucide-svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import type { TopologyEditState } from '$lib/features/topology/state';
	import { useSubnetsQuery, useUpdateSubnetMutation } from '$lib/features/subnets/queries';
	import { nestingItems, subnetNesting } from '$lib/features/subnets/nesting';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { inspector_thisSubnet, topology_focusNode } from '$lib/paraglide/messages';
	import InspectorSection from '../shared/InspectorSection.svelte';

	let {
		node,
		topology,
		editState
	}: {
		node: Node;
		topology: RenderableTopology;
		editState: TopologyEditState;
	} = $props();

	const { fitView } = useSvelteFlow();
	const updateSubnetMutation = useUpdateSubnetMutation();

	let isReadonly = $derived(editState.isReadonly);
	let subnet = $derived(topology.subnets.find((s) => s.id === node.id) ?? null);

	// Nesting is derived server-side across the whole site, so it comes from the live subnet list
	// rather than the topology payload. Not in a read-only view: a shared link has
	// no session to fetch with, and a snapshot's past state would be paired with today's counts.
	const subnetsQuery = useSubnetsQuery(undefined, undefined, () => !editState.isReadonly);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let listed = $derived(isReadonly ? null : (subnetsData.find((s) => s.id === node.id) ?? null));
	let nestedLabels = $derived(listed ? nestingItems(listed, subnetNesting(subnetsData)) : []);

	function handleFocus() {
		fitView({ nodes: [{ id: node.id }], padding: 0.5, duration: 300 });
	}

	let subnetContext = $derived({
		showEntityTagPicker: true,
		tagPickerDisabled: !editState.isEditable,
		entityTags: isReadonly ? (topology.entity_tags ?? []) : undefined,
		showEditableEntityDescription: true,
		entityDescription: subnet?.description ?? null,
		entityDescriptionDisabled: !editState.isEditable,
		onEntityDescriptionSave: (desc: string | null) => {
			if (subnet) {
				updateSubnetMutation.mutate({ ...subnet, description: desc });
			}
		},
		compact: true
	});
</script>

{#if subnet}
	<InspectorSection id="SubnetDetail" section="SubnetDetail" title={inspector_thisSubnet()}>
		{#snippet actions()}
			<button class="btn-icon p-0.5" onclick={handleFocus} title={topology_focusNode()}>
				<Crosshair class="h-3.5 w-3.5" />
			</button>
		{/snippet}
		<div class="card card-static">
			<EntityDisplayWrapper
				context={subnetContext}
				item={subnet}
				displayComponent={SubnetDisplay}
			/>
			{#if nestedLabels.length > 0}
				<div class="mt-2 flex flex-wrap gap-1">
					{#each nestedLabels as label (label.id)}
						<Tag label={label.label} color={label.color} title={label.title} />
					{/each}
				</div>
			{/if}
		</div>
	</InspectorSection>
{/if}
