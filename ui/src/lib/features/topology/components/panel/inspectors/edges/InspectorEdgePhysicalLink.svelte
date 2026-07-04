<script lang="ts">
	import { et } from '$lib/shared/utils/embed-i18n';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { InterfaceDisplay } from '$lib/shared/components/forms/selection/display/InterfaceDisplay.svelte';
	import { useTopology, selectedTopologyId } from '$lib/features/topology/context';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';

	let {
		sourceEntityId,
		targetEntityId,
		protocol
	}: {
		sourceEntityId?: string;
		targetEntityId?: string;
		protocol?: 'LLDP' | 'CDP';
	} = $props();

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					| RenderableTopology
					| undefined)
	);

	// Derive Interface and Host data
	let sourceInterface = $derived(topology?.interfaces.find((e) => e.id === sourceEntityId));
	let targetInterface = $derived(topology?.interfaces.find((e) => e.id === targetEntityId));
	let sourceHost = $derived(
		sourceInterface ? topology?.hosts.find((h) => h.id === sourceInterface.host_id) : null
	);
	let targetHost = $derived(
		targetInterface ? topology?.hosts.find((h) => h.id === targetInterface.host_id) : null
	);
</script>

<div class="space-y-3">
	{#if protocol}
		<div class="flex items-center gap-2">
			<Tag label={protocol} color={protocol == 'CDP' ? 'Blue' : 'Green'} />
		</div>
	{/if}

	{#if sourceHost || sourceInterface}
		<span class="text-secondary mb-2 block text-sm font-medium">{et('Source')}</span>
		{#if sourceHost}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={{
						services: topology?.services.filter((s) => s.host_id === sourceHost.id) ?? [],
						compact: true
					}}
					item={sourceHost}
					displayComponent={HostDisplay}
				/>
			</div>
		{/if}
		{#if sourceInterface}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={undefined}
					item={sourceInterface}
					displayComponent={InterfaceDisplay}
				/>
			</div>
		{/if}
	{/if}

	{#if targetHost || targetInterface}
		<span class="text-secondary mb-2 block text-sm font-medium">{et('Target')}</span>
		{#if targetHost}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={{
						services: topology?.services.filter((s) => s.host_id === targetHost.id) ?? [],
						compact: true
					}}
					item={targetHost}
					displayComponent={HostDisplay}
				/>
			</div>
		{/if}
		{#if targetInterface}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={undefined}
					item={targetInterface}
					displayComponent={InterfaceDisplay}
				/>
			</div>
		{/if}
	{/if}
</div>
