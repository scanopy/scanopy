<script lang="ts">
	import EntityDisplayWrapper from '#lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '#lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import type { RenderableTopology } from '#lib/features/topology/types/base.ts';
	import type { TopologyEditState } from '#lib/features/topology/state.ts';
	import type { ElementRenderContext } from '#lib/features/topology/resolvers.ts';
	import { useUpdateHostDescriptionMutation } from '#lib/features/hosts/queries.ts';
	import InspectorSection from '../shared/InspectorSection.svelte';
	import HostInspectorActions from '#lib/features/hosts/components/HostInspectorActions.svelte';

	let {
		topology,
		editState,
		elementContext
	}: {
		topology: RenderableTopology;
		editState: TopologyEditState;
		elementContext?: ElementRenderContext;
	} = $props();

	let isReadonly = $derived(editState.isReadonly);
	// On a Host element (Workloads) the host is the selection itself, which Identity draws.
	let host = $derived(
		elementContext?.elementType === 'Host' ? null : (elementContext?.host ?? null)
	);

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
	<InspectorSection id="HostDetail" section="HostDetail">
		<HostInspectorActions {host} editable={editState.isEditable} class="mb-2" />
		<div class="card-static card">
			<EntityDisplayWrapper context={hostContext} item={host} displayComponent={HostDisplay} />
		</div>
	</InspectorSection>
{/if}
