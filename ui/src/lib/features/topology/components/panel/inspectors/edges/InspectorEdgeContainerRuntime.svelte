<script lang="ts">
	import { et } from '$lib/shared/utils/embed-i18n';
	import type { Edge } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import { topologyOptions, activeView, topologyReadOnly } from '$lib/features/topology/queries';
	import { useTopology, selectedTopologyId } from '$lib/features/topology/context';
	import { getTopologyEditState } from '$lib/features/topology/state';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import type { RenderableTopology } from '$lib/features/topology/types/base';

	let { edge, serviceId }: { edge: Edge; serviceId: string } = $props();

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let isReadonly = $derived(topo.isReadonly || $topologyReadOnly);
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					| RenderableTopology
					| undefined)
	);

	// Unified edit state
	let editState = $derived(getTopologyEditState(topology, false, isReadonly));

	let containerizingService = $derived(
		topology ? topology.services.find((s) => s.id == serviceId) : null
	);

	let containerizingHost = $derived(
		containerizingService && topology
			? topology.hosts.find((h) => h.id == containerizingService.host_id)
			: null
	);

	// Target can be either a subnet (grouped) or a service (not grouped)
	let isGrouped = $derived(
		(
			(($topologyOptions.request.container_rules ?? {}) as Record<string, { rule: unknown }[]>)[
				$activeView
			] ?? []
		).some((r) => r.rule === 'MergeDockerBridges')
	);
	// Get containerized services - all if grouped, or just the one in edge.target if not
	let containerizedServices = $derived(
		topology
			? isGrouped
				? topology.services.filter(
						(s) =>
							s.virtualization &&
							s.virtualization.type === 'Docker' &&
							s.virtualization.details.service_id === serviceId
					)
				: topology.services.filter((s) => s.bindings.some((b) => b.ip_address_id == edge.target))
			: []
	);

	// Helper to get interface from topology
	function getInterfaceFromTopology(ipAddressId: string) {
		if (!topology) return null;
		return topology.ip_addresses.find((i) => i.id === ipAddressId) ?? null;
	}

	// Helper to get subnet from topology
	function getSubnetFromTopology(subnetId: string) {
		if (!topology) return null;
		return topology.subnets.find((s) => s.id === subnetId) || null;
	}

	// Get all Docker Bridge subnets for those containerized services
	let allDockerSubnets = $derived.by(() => {
		const subnets = new SvelteMap<string, Subnet>(); // Use Map to deduplicate by subnet ID

		for (const service of containerizedServices) {
			for (const binding of service.bindings) {
				// Get interface_id based on binding type
				let ipAddressId: string | null = null;
				if (binding.type === 'IPAddress') {
					ipAddressId = binding.ip_address_id;
				} else if (binding.type === 'Port') {
					ipAddressId = binding.ip_address_id ?? null;
				}

				if (!ipAddressId) continue;

				const iface = getInterfaceFromTopology(ipAddressId);
				if (!iface?.subnet_id) continue;

				const subnet = getSubnetFromTopology(iface.subnet_id);
				if (subnet?.subnet_type === 'DockerBridge') {
					subnets.set(subnet.id, subnet);
				}
			}
		}

		return Array.from(subnets.values());
	});
</script>

<div class="space-y-3">
	{#if containerizingHost}
		<span class="text-secondary mb-2 block text-sm font-medium">{et('Docker Host')}</span>
		<div class="card card-static">
			<EntityDisplayWrapper
				context={{
					services:
						topology?.services.filter((s) =>
							containerizingHost ? s.host_id == containerizingHost.id : false
						) ?? [],
					showEntityTagPicker: true,
					tagPickerDisabled: !editState.isEditable,
					entityTags: isReadonly ? (topology?.entity_tags ?? []) : undefined,
					compact: true
				}}
				item={containerizingHost}
				displayComponent={HostDisplay}
			/>
		</div>
	{/if}
	{#if containerizingService}
		<span class="text-secondary mb-2 block text-sm font-medium">{et('Docker Service')}</span>
		<div class="card card-static">
			<EntityDisplayWrapper
				context={{
					ipAddressId: null,
					ports: topology?.ports ?? [],
					showEntityTagPicker: true,
					tagPickerDisabled: !editState.isEditable,
					entityTags: isReadonly ? (topology?.entity_tags ?? []) : undefined,
					compact: true
				}}
				item={containerizingService}
				displayComponent={ServiceDisplay}
			/>
		</div>
	{/if}

	<span class="text-secondary mb-2 block text-sm font-medium">
		{isGrouped ? 'Containerized Services' : 'Containerized Service'}
	</span>
	{#each containerizedServices as service (service.id)}
		<div class="card card-static">
			<EntityDisplayWrapper
				context={{
					ipAddressId: null,
					ports: topology?.ports ?? [],
					showEntityTagPicker: true,
					tagPickerDisabled: !editState.isEditable,
					entityTags: isReadonly ? (topology?.entity_tags ?? []) : undefined,
					compact: true
				}}
				item={service}
				displayComponent={ServiceDisplay}
			/>
		</div>
	{/each}

	{#if allDockerSubnets.length > 0}
		<span class="text-secondary mb-2 block text-sm font-medium"
			>Docker Bridge Subnet{allDockerSubnets.length > 1 ? 's' : ''}</span
		>
		{#each allDockerSubnets as subnet (subnet.id)}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={{ compact: true }}
					item={subnet}
					displayComponent={SubnetDisplay}
				/>
			</div>
		{/each}
	{/if}
</div>
