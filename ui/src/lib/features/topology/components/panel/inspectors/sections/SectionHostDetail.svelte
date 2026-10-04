<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import type { TopologyEditState } from '$lib/features/topology/state';
	import type { ElementRenderContext } from '$lib/features/topology/resolvers';
	import { useUpdateHostDescriptionMutation } from '$lib/features/hosts/queries';
	import { useSvelteFlow } from '@xyflow/svelte';
	import { Crosshair } from 'lucide-svelte';
	import { inspector_thisEntity, topology_focusNode } from '$lib/paraglide/messages';
	import { entities, inspectorSections } from '$lib/shared/stores/metadata';
	import InspectorSection from '../shared/InspectorSection.svelte';
	import InferredHostNotice from '$lib/features/hosts/components/InferredHostNotice.svelte';

	/* eslint-disable @typescript-eslint/no-unused-vars -- component contract props */
	let {
		node,
		topology,
		editState,
		elementContext
	}: {
		node: Node;
		topology: RenderableTopology;
		editState: TopologyEditState;
		elementContext?: ElementRenderContext;
	} = $props();
	/* eslint-enable @typescript-eslint/no-unused-vars */

	let isReadonly = $derived(editState.isReadonly);
	let host = $derived(elementContext?.host ?? null);
	// On a Host element (Workloads) the host is the selection itself, so this section takes the
	// selection's heading and focus control; Identity leaves Host elements to it.
	let isSelectedHost = $derived(elementContext?.elementType === 'Host');
	const { fitView } = useSvelteFlow();

	const updateHostDescriptionMutation = useUpdateHostDescriptionMutation();

	let hostContext = $derived({
		services: topology.services.filter((s) => host && s.host_id === host.id),
		showEntityTagPicker: true,
		tagPickerDisabled: !editState.isEditable,
		entityTags: isReadonly ? (topology.entity_tags ?? []) : undefined,
		showEditableEntityDescription: true,
		entityDescription: host?.description ?? null,
		entityDescriptionDisabled: !editState.isEditable,
		onEntityDescriptionSave: (desc: string | null) => {
			if (host) {
				updateHostDescriptionMutation.mutate({ host, description: desc });
			}
		},
		compact: true
	});
</script>

{#if host}
	<InspectorSection
		id={isSelectedHost ? 'Identity' : 'HostDetail'}
		section="HostDetail"
		title={isSelectedHost
			? inspector_thisEntity({ name: entities.getItem('Host')?.name ?? 'Host' })
			: undefined}
		description={isSelectedHost ? inspectorSections.getDescription('Identity') : undefined}
	>
		{#snippet actions()}
			{#if isSelectedHost}
				<button
					class="btn-icon p-0.5"
					onclick={() => fitView({ nodes: [{ id: node.id }], padding: 0.5, duration: 300 })}
					title={topology_focusNode()}
				>
					<Crosshair class="h-3.5 w-3.5" />
				</button>
			{/if}
		{/snippet}
		<InferredHostNotice source={host.source} class="mb-2" />
		<div class="card card-static">
			<EntityDisplayWrapper context={hostContext} item={host} displayComponent={HostDisplay} />
		</div>
	</InspectorSection>
{/if}
