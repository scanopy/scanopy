<script lang="ts">
	import { et } from '$lib/shared/utils/embed-i18n';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import type { TopologyEdge, RenderableTopology } from '$lib/features/topology/types/base';
	import { useTopology, selectedTopologyId } from '$lib/features/topology/context';
	import { edgeTypes } from '$lib/shared/stores/metadata';
	import { topology_connectionsCount, common_dependenciesLabel } from '$lib/paraglide/messages';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { SameHostEdgeDisplay } from '$lib/shared/components/forms/selection/display/SameHostEdgeDisplay.svelte';
	import { PhysicalLinkEdgeDisplay } from '$lib/shared/components/forms/selection/display/PhysicalLinkEdgeDisplay.svelte';
	import { HypervisorEdgeDisplay } from '$lib/shared/components/forms/selection/display/HypervisorEdgeDisplay.svelte';
	import { DependencyDisplay } from '$lib/shared/components/forms/selection/display/DependencyDisplay.svelte';
	import {
		ServiceDisplay,
		type ServiceDisplayContext
	} from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import type { Dependency } from '$lib/features/dependencies/types/base';

	let { edges }: { edges: TopologyEdge[] } = $props();

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					| RenderableTopology
					| undefined)
	);

	// Group edges by type
	let edgesByType = $derived.by(() => {
		const groups = new SvelteMap<string, TopologyEdge[]>();
		for (const edge of edges) {
			const type = edge.edge_type;
			const existing = groups.get(type);
			if (existing) {
				existing.push(edge);
			} else {
				groups.set(type, [edge]);
			}
		}
		return groups;
	});

	// Reactive dependency resolution — re-evaluates when topology loads
	let dependencyEdgeGroups = $derived.by(() => {
		if (!topology) return new SvelteMap<string, Dependency[]>();
		const result = new SvelteMap<string, Dependency[]>();
		for (const [edgeType, typeEdges] of edgesByType) {
			if (edgeType !== 'HubAndSpoke' && edgeType !== 'RequestPath') continue;
			const seen = new SvelteSet<string>();
			const deps: Dependency[] = [];
			for (const edge of typeEdges) {
				if (!('dependency_id' in edge)) continue;
				const depId = edge.dependency_id as string;
				if (seen.has(depId)) continue;
				seen.add(depId);
				const dep = topology.dependencies.find((d) => d.id === depId);
				if (dep) deps.push(dep);
			}
			result.set(edgeType, deps);
		}
		return result;
	});

	// Reactive ContainerRuntime resolution
	let svcVirtData = $derived.by(() => {
		const typeEdges = edgesByType.get('ContainerRuntime');
		if (!typeEdges || !topology) return null;

		// Group by service_id
		const byContainerizer = new SvelteMap<string, TopologyEdge[]>();
		for (const edge of typeEdges) {
			if (!('service_id' in edge)) continue;
			const id = edge.service_id as string;
			const existing = byContainerizer.get(id);
			if (existing) existing.push(edge);
			else byContainerizer.set(id, [edge]);
		}

		if (byContainerizer.size === 1) {
			// Single Docker service — show detailed view
			const [containerizingId] = [...byContainerizer.keys()];
			const containerizer = topology.services.find((s) => s.id === containerizingId);
			const containerized = topology.services.filter(
				(s) =>
					s.virtualization &&
					s.virtualization.type === 'Docker' &&
					s.virtualization.details.service_id === containerizingId
			);
			return { mode: 'single' as const, containerizer, containerized };
		} else {
			// Multiple Docker services — show summary per host
			const hosts = new SvelteMap<
				string,
				{ host: (typeof topology.hosts)[0]; containerCount: number }
			>();
			for (const [containerizingId] of byContainerizer) {
				const service = topology.services.find((s) => s.id === containerizingId);
				if (!service) continue;
				const host = topology.hosts.find((h) => h.id === service.host_id);
				if (!host) continue;
				const existing = hosts.get(host.id);
				if (existing) {
					existing.containerCount += topology.services.filter(
						(s) =>
							s.virtualization &&
							s.virtualization.type === 'Docker' &&
							s.virtualization.details.service_id === containerizingId
					).length;
				} else {
					hosts.set(host.id, {
						host,
						containerCount: topology.services.filter(
							(s) =>
								s.virtualization &&
								s.virtualization.type === 'Docker' &&
								s.virtualization.details.service_id === containerizingId
						).length
					});
				}
			}
			return { mode: 'multi' as const, hosts: [...hosts.values()] };
		}
	});

	function getDisplayComponent(edgeType: string) {
		switch (edgeType) {
			case 'Interface':
				return SameHostEdgeDisplay;
			case 'PhysicalLink':
				return PhysicalLinkEdgeDisplay;
			case 'Hypervisor':
				return HypervisorEdgeDisplay;
			default:
				return null;
		}
	}

	function isDependencyEdge(edgeType: string) {
		return edgeType === 'HubAndSpoke' || edgeType === 'RequestPath';
	}

	function isContainerRuntime(edgeType: string) {
		return edgeType === 'ContainerRuntime';
	}
</script>

<div class="space-y-4">
	<span class="text-secondary block text-sm font-medium">
		{topology_connectionsCount({ count: edges.length })}
	</span>

	<div class="max-h-96 space-y-3 overflow-y-auto">
		{#each [...edgesByType.entries()] as [edgeType, typeEdges] (edgeType)}
			{@const typeName = isDependencyEdge(edgeType)
				? common_dependenciesLabel()
				: edgeTypes.getName(edgeType)}
			{@const displayComponent = getDisplayComponent(edgeType)}

			{#if typeEdges.length > 1}
				<span class="text-tertiary block text-xs font-medium uppercase tracking-wide">
					{typeName} ({typeEdges.length})
				</span>
			{:else}
				<span class="text-tertiary block text-xs font-medium uppercase tracking-wide">
					{typeName}
				</span>
			{/if}

			{#if isDependencyEdge(edgeType)}
				{@const uniqueDeps = dependencyEdgeGroups.get(edgeType) ?? []}
				{#each uniqueDeps as dep (dep.id)}
					<div class="card card-static">
						<EntityDisplayWrapper
							item={dep}
							context={{ compact: true }}
							displayComponent={DependencyDisplay}
						/>
					</div>
					{#if dep.members.type === 'Services'}
						{#each dep.members.service_ids as serviceId (serviceId)}
							{@const service = topology?.services.find((s) => s.id === serviceId)}
							{#if service}
								<div class="card card-static ml-3">
									<EntityDisplayWrapper
										context={{ compact: true } satisfies ServiceDisplayContext}
										item={service}
										displayComponent={ServiceDisplay}
									/>
								</div>
							{/if}
						{/each}
					{:else if dep.members.type === 'Bindings'}
						{#each dep.members.binding_ids as bindingId (bindingId)}
							{@const bindingService = topology?.services.find((s) =>
								s.bindings.some((b) => b.id === bindingId)
							)}
							{#if bindingService}
								<div class="card card-static ml-3">
									<EntityDisplayWrapper
										context={{ compact: true } satisfies ServiceDisplayContext}
										item={bindingService}
										displayComponent={ServiceDisplay}
									/>
								</div>
							{/if}
						{/each}
					{/if}
				{:else}
					<div class="card card-static">
						<div class="px-2 py-1">
							<span class="text-secondary text-sm">{typeName}</span>
						</div>
					</div>
				{/each}
			{:else if isContainerRuntime(edgeType) && svcVirtData}
				{#if svcVirtData.mode === 'single'}
					{#if svcVirtData.containerizer}
						<span class="text-secondary mb-1 block text-sm font-medium">{et('Docker Service')}</span>
						<div class="card card-static">
							<EntityDisplayWrapper
								item={svcVirtData.containerizer}
								context={{ ipAddressId: null, ports: topology?.ports ?? [], compact: true }}
								displayComponent={ServiceDisplay}
							/>
						</div>
					{/if}
					{#if svcVirtData.containerized.length > 0}
						<span class="text-secondary mb-1 block text-sm font-medium">
							Containerized Services ({svcVirtData.containerized.length})
						</span>
						{#each svcVirtData.containerized as service (service.id)}
							<div class="card card-static">
								<EntityDisplayWrapper
									item={service}
									context={{ ipAddressId: null, ports: topology?.ports ?? [], compact: true }}
									displayComponent={ServiceDisplay}
								/>
							</div>
						{/each}
					{/if}
				{:else}
					{#each svcVirtData.hosts as { host, containerCount } (host.id)}
						<div class="card card-static">
							<EntityDisplayWrapper
								item={host}
								context={{
									services: topology?.services.filter((s) => s.host_id === host.id) ?? [],
									compact: true
								}}
								displayComponent={HostDisplay}
							/>
							<div class="flex items-center gap-2 px-3 pb-2">
								<Tag label="Docker" color="Indigo" />
								<span class="text-tertiary text-xs"
									>{containerCount} container{containerCount !== 1 ? 's' : ''}</span
								>
							</div>
						</div>
					{/each}
				{/if}
			{:else}
				{#each typeEdges as edge (edge.id)}
					<div class="card card-static">
						{#if displayComponent}
							<EntityDisplayWrapper
								item={edge}
								context={{ topology: topology ?? undefined }}
								{displayComponent}
							/>
						{:else}
							<div class="px-2 py-1">
								<span class="text-secondary text-sm">{typeName}</span>
							</div>
						{/if}
					</div>
				{/each}
			{/if}
		{/each}
	</div>
</div>
