<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import type { TopologyEditState } from '$lib/features/topology/state';
	import type { ElementRenderContext } from '$lib/features/topology/resolvers';
	import { inspector_servicesOnIPAddress, common_services } from '$lib/paraglide/messages';
	import { getTopologyIndex } from '$lib/features/topology/entity-index';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import type { components } from '$lib/api/schema';

	type InlineGroup = components['schemas']['InlineGroup'];

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

	// Filter services bound to this specific IP address
	let servicesOnInterface = $derived(
		(elementContext?.services ?? []).filter((s) =>
			s.bindings.some(
				(b) => b.ip_address_id === elementContext?.ipAddressId || b.ip_address_id === null
			)
		)
	);

	// Hosts inlined on this host's card (a guest's network identities, a runtime's macvlan
	// containers), each with its own services.
	let memberHosts = $derived.by(() => {
		if (elementContext?.elementType !== 'Host') return [];
		const groups = ((node.data as { inline_groups?: InlineGroup[] }).inline_groups ?? []).filter(
			(g) => g.entity_type === 'Host'
		);
		const { hostsById, servicesByHostId } = getTopologyIndex(topology);
		return groups.flatMap((g) => {
			const host = hostsById.get(g.entity_id);
			const services = servicesByHostId.get(g.entity_id) ?? [];
			return host && services.length > 0 ? [{ host, services }] : [];
		});
	});

	let serviceContext = $derived({
		ipAddressId: elementContext?.ipAddressId ?? null,
		ports: topology.ports,
		showEntityTagPicker: true,
		tagPickerDisabled: !editState.isEditable,
		entityTags: isReadonly ? (topology.entity_tags ?? []) : undefined,
		compact: true
	});
</script>

{#if servicesOnInterface.length > 0}
	<div>
		<span class="text-secondary mb-2 block text-sm font-medium">
			{elementContext?.elementType === 'Host' ? common_services() : inspector_servicesOnIPAddress()}
		</span>
		<div class="space-y-1">
			{#each servicesOnInterface as service (service.id)}
				<div class="card card-static">
					<EntityDisplayWrapper
						context={serviceContext}
						item={service}
						displayComponent={ServiceDisplay}
					/>
				</div>
			{/each}
		</div>
	</div>
{/if}
{#each memberHosts as member (member.host.id)}
	<div>
		<span class="text-secondary mb-2 block text-sm font-medium">
			{hostDisplayName(member.host)}
		</span>
		<div class="space-y-1">
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
	</div>
{/each}
