<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import type { RenderableTopology, TopologyNode } from '$lib/features/topology/types/base';
	import type { TopologyEditState } from '$lib/features/topology/state';
	import type { ElementRenderContext } from '$lib/features/topology/resolvers';
	import type { Service } from '$lib/features/services/types/base';
	import { inspector_servicesOnIPAddress, common_containers } from '$lib/paraglide/messages';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import { inspectorServiceSections } from '$lib/features/topology/element-render-data';
	import { hiddenEntityIds } from '$lib/features/topology/interactions';
	import InspectorSection from '../shared/InspectorSection.svelte';
	import InspectorSubsection from '../shared/InspectorSubsection.svelte';

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

	let isReadonly = $derived(editState.isReadonly);

	// An IP-address element lists the services bound to that address; a Host element lists every
	// service on the host, then one subsection per manager box its card draws.
	let isIPAddressElement = $derived(elementContext?.elementType === 'IPAddress');
	// A Service element's own service is the selection, which Identity already shows.
	let sections = $derived(
		inspectorServiceSections(
			node.data as TopologyNode,
			elementContext?.elementType === 'Service' ? [] : (elementContext?.services ?? []),
			topology,
			$hiddenEntityIds,
			isIPAddressElement ? (elementContext?.ipAddressId ?? null) : null
		)
	);
	let total = $derived(
		sections.own.length +
			sections.groups.reduce(
				(n, g) => n + g.services.length + g.hosts.reduce((m, h) => m + h.services.length, 0),
				0
			)
	);

	let serviceContext = $derived({
		ipAddressId: elementContext?.ipAddressId ?? null,
		ports: topology.ports,
		showEntityTagPicker: true,
		tagPickerDisabled: !editState.isEditable,
		entityTags: isReadonly ? (topology.entity_tags ?? []) : undefined,
		compact: true
	});

	function hostContext(services: Service[]) {
		return {
			// The first service supplies the icon, as on every other topology host card.
			services,
			showEntityTagPicker: !isReadonly,
			tagPickerDisabled: !editState.isEditable,
			entityTags: isReadonly ? (topology.entity_tags ?? []) : undefined,
			compact: true
		};
	}
</script>

{#if sections.own.length > 0 || sections.groups.length > 0}
	<InspectorSection
		id="Services"
		section="Services"
		title={isIPAddressElement ? inspector_servicesOnIPAddress() : undefined}
		count={total}
	>
		<div class="space-y-3">
			{#if sections.own.length > 0}
				<div class="space-y-1">
					{#each sections.own as service (service.id)}
						<div class="card card-static">
							<EntityDisplayWrapper
								context={serviceContext}
								item={service}
								displayComponent={ServiceDisplay}
							/>
						</div>
					{/each}
				</div>
			{/if}
			{#each sections.groups as group (group.groupId)}
				{@const definition = group.header?.service_definition ?? null}
				<InspectorSubsection
					id={`inline:${definition ?? group.groupId}`}
					title={group.header?.name ?? common_containers()}
					icon={definition ? serviceDefinitions.getIconComponent(definition) : null}
					description={definition ? serviceDefinitions.getDescription(definition) : null}
					count={group.services.length + group.hosts.length}
				>
					{#each group.services as service (service.id)}
						<div class="card card-static">
							<EntityDisplayWrapper
								context={serviceContext}
								item={service}
								displayComponent={ServiceDisplay}
							/>
						</div>
					{/each}
					{#each group.hosts as member (member.host.id)}
						<div class="card card-static">
							<EntityDisplayWrapper
								context={hostContext(member.services)}
								item={member.host}
								displayComponent={HostDisplay}
							/>
						</div>
						{#if member.services.length > 0}
							<div class="ml-4 space-y-1">
								{#each member.services as service (service.id)}
									<div class="card card-static">
										<EntityDisplayWrapper
											context={serviceContext}
											item={service}
											displayComponent={ServiceDisplay}
										/>
									</div>
								{/each}
							</div>
						{/if}
					{/each}
				</InspectorSubsection>
			{/each}
		</div>
	</InspectorSection>
{/if}
