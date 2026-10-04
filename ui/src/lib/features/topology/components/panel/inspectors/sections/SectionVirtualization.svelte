<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import type { TopologyEditState } from '$lib/features/topology/state';
	import type { ElementRenderContext } from '$lib/features/topology/resolvers';
	import { common_hypervisor, common_presentedBy } from '$lib/paraglide/messages';
	import { containerTypes, serviceDefinitions } from '$lib/shared/stores/metadata';
	import InspectorSection from '../shared/InspectorSection.svelte';

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

	// The service virtualizing this element's host, and the host that service runs on.
	let virtualizerService = $derived.by(() => {
		const id = elementContext?.host?.virtualization_service_id;
		return id ? (topology.services.find((s) => s.id === id) ?? null) : null;
	});
	let virtualizerHost = $derived.by(() => {
		const hostId = virtualizerService?.host_id;
		return hostId ? (topology.hosts.find((h) => h.id === hostId) ?? null) : null;
	});

	// Named by what the managing service does: a VM's hypervisor, a container's runtime, or the
	// guest that presents a network identity.
	let title = $derived.by(() => {
		if (!virtualizerService) return undefined;
		switch (
			serviceDefinitions.getMetadata(virtualizerService.service_definition).manages_virtualization
		) {
			case 'containers':
				return containerTypes.getName('ContainerRuntime');
			case 'identities':
				return common_presentedBy();
			default:
				return common_hypervisor();
		}
	});

	let hostContext = $derived({
		services: virtualizerHost
			? topology.services.filter((s) => s.host_id === virtualizerHost.id)
			: [],
		showEntityTagPicker: !editState.isReadonly,
		tagPickerDisabled: !editState.isEditable,
		entityTags: isReadonly ? (topology.entity_tags ?? []) : undefined,
		compact: true
	});
</script>

{#if virtualizerHost}
	<InspectorSection id="Virtualization" section="Virtualization" {title}>
		<div class="card card-static">
			<EntityDisplayWrapper
				context={hostContext}
				item={virtualizerHost}
				displayComponent={HostDisplay}
			/>
		</div>
	</InspectorSection>
{/if}
