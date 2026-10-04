<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { IPAddressDisplay } from '$lib/shared/components/forms/selection/display/IPAddressDisplay.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import type { ElementRenderContext } from '$lib/features/topology/resolvers';
	import {
		common_ipAddresses,
		inspector_otherIPAddress,
		inspector_otherIPAddresses
	} from '$lib/paraglide/messages';
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

	let isIPAddressElement = $derived(!!elementContext?.ipAddressId);

	let otherInterfaces = $derived(
		topology.ip_addresses.filter(
			(i) =>
				i.host_id === elementContext?.hostId &&
				(!isIPAddressElement || i.id !== elementContext?.ipAddressId)
		)
	);

	let interfaceContext = $derived({ subnets: topology.subnets, compact: true });
</script>

{#if otherInterfaces.length > 0}
	<InspectorSection
		id="OtherInterfaces"
		section="OtherInterfaces"
		title={isIPAddressElement
			? otherInterfaces.length > 1
				? inspector_otherIPAddresses()
				: inspector_otherIPAddress()
			: common_ipAddresses()}
		count={otherInterfaces.length}
	>
		<div class="space-y-1">
			{#each otherInterfaces as iface (iface.id)}
				<div class="card card-static">
					<EntityDisplayWrapper
						context={interfaceContext}
						item={iface}
						displayComponent={IPAddressDisplay}
					/>
				</div>
			{/each}
		</div>
	</InspectorSection>
{/if}
