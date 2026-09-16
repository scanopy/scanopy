<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import type { TopologyEditState } from '$lib/features/topology/state';
	import type { ElementRenderContext } from '$lib/features/topology/resolvers';
	import { useUpdateHostDescriptionMutation } from '$lib/features/hosts/queries';
	import { inspector_hostDetail } from '$lib/paraglide/messages';
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
	<div>
		<span class="text-secondary mb-2 block text-sm font-medium">{inspector_hostDetail()}</span>
		<InferredHostNotice source={host.source} class="mb-2" />
		<div class="card card-static">
			<EntityDisplayWrapper context={hostContext} item={host} displayComponent={HostDisplay} />
		</div>
	</div>
{/if}
