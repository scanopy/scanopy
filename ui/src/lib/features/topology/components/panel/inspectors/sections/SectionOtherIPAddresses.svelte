<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { IPAddressDisplay } from '$lib/shared/components/forms/selection/display/IPAddressDisplay.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import type { ElementRenderContext } from '$lib/features/topology/resolvers';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import {
		common_ipAddresses,
		inspector_otherIPAddress,
		inspector_otherIPAddresses
	} from '$lib/paraglide/messages';

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

	const networksQuery = useNetworksQuery();
	let interfaceContext = $derived({
		subnets: topology.subnets,
		networks: networksQuery.data ?? [],
		compact: true
	});
</script>

{#if otherInterfaces.length > 0}
	<div>
		<span class="text-secondary mb-2 block text-sm font-medium">
			{isIPAddressElement
				? otherInterfaces.length > 1
					? inspector_otherIPAddresses()
					: inspector_otherIPAddress()
				: common_ipAddresses()}
		</span>
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
	</div>
{/if}
