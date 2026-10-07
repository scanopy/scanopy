<script lang="ts">
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import type { TopologyEdge, RenderableTopology } from '$lib/features/topology/types/base';
	import { useTopology, selectedTopologyId } from '$lib/features/topology/context';
	import { edgeTypes, hostVirtualizations, serviceDefinitions } from '$lib/shared/stores/metadata';
	import { containerHostsOfEdge, identityHostsOfEdge } from '$lib/features/topology/resolvers';
	import {
		topology_connectionsCount,
		topology_containerCount,
		common_containerizedServices,
		common_dependenciesLabel,
		common_presentedBy,
		hosts_virtualization_containerHosts,
		inspector_dockerService
	} from '$lib/paraglide/messages';
	import InspectorSection from '../shared/InspectorSection.svelte';
	import InspectorSubsection from '../shared/InspectorSubsection.svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { SameHostEdgeDisplay } from '$lib/shared/components/forms/selection/display/SameHostEdgeDisplay.svelte';
	import { PhysicalLinkEdgeDisplay } from '$lib/shared/components/forms/selection/display/PhysicalLinkEdgeDisplay.svelte';
	import { HypervisorEdgeDisplay } from '$lib/shared/components/forms/selection/display/HypervisorEdgeDisplay.svelte';
	import { DependencyDisplay } from '$lib/shared/components/forms/selection/display/DependencyDisplay.svelte';
	import {
		ServiceDisplay,
		type ServiceDisplayContext
	} from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import {
		HostDisplay,
		type HostDisplayContext,
		type HostTagRole
	} from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import type { Dependency } from '$lib/features/dependencies/types/base';

	let { edges }: { edges: TopologyEdge[] } = $props();

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					RenderableTopology | undefined)
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

	/** The containers the given runtime edges stand for — the union of what each names. */
	function containersOf(edges: TopologyEdge[]): string[] {
		const ids = new SvelteSet<string>();
		for (const edge of edges) {
			const edgeIds = (edge as unknown as Record<string, unknown>).containerized_service_ids;
			if (!Array.isArray(edgeIds)) continue;
			for (const id of edgeIds as string[]) ids.add(id);
		}
		return [...ids];
	}

	/** The container hosts (macvlan, ipvlan) the given runtime edges reach, each once. */
	function containerHostsOf(topology: RenderableTopology, edges: TopologyEdge[]) {
		const hosts = new SvelteMap<string, RenderableTopology['hosts'][number]>();
		for (const edge of edges) {
			for (const host of containerHostsOfEdge(topology, edge)) hosts.set(host.id, host);
		}
		return [...hosts.values()];
	}

	// Reactive ContainerRuntime resolution. Each edge names the containers it reaches (services on
	// bridge networks, or one container host on a macvlan/ipvlan network), so the aggregate is
	// their union — not every container the runtime happens to host.
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
			const containerizerEdges = byContainerizer.get(containerizingId) ?? [];
			const containerized = containersOf(containerizerEdges).flatMap(
				(id) => topology.services.find((s) => s.id === id) ?? []
			);
			const containerHosts = containerHostsOf(topology, containerizerEdges);
			return { mode: 'single' as const, containerizer, containerized, containerHosts };
		} else {
			// Multiple Docker services — show summary per host
			const hosts = new SvelteMap<
				string,
				{ host: (typeof topology.hosts)[0]; containerCount: number; runtimes: string[] }
			>();
			for (const [containerizingId, containerizerEdges] of byContainerizer) {
				const service = topology.services.find((s) => s.id === containerizingId);
				if (!service) continue;
				const host = topology.hosts.find((h) => h.id === service.host_id);
				if (!host) continue;
				const containerCount =
					containersOf(containerizerEdges).length +
					containerHostsOf(topology, containerizerEdges).length;
				const existing = hosts.get(host.id);
				// The host's runtimes by service definition (Docker, Podman), one tag each.
				if (existing) {
					existing.containerCount += containerCount;
					if (!existing.runtimes.includes(service.service_definition)) {
						existing.runtimes.push(service.service_definition);
					}
				} else {
					hosts.set(host.id, { host, containerCount, runtimes: [service.service_definition] });
				}
			}
			return { mode: 'multi' as const, hosts: [...hosts.values()] };
		}
	});

	// Reactive NetworkIdentity resolution: per guest (its Network Identities service), the guest
	// host and the union of the identity hosts its edges reach.
	let identityGroups = $derived.by(() => {
		const typeEdges = edgesByType.get('NetworkIdentity');
		if (!typeEdges || !topology) return [];
		const groups = new SvelteMap<
			string,
			{
				guest: RenderableTopology['hosts'][number] | undefined;
				identities: SvelteMap<string, RenderableTopology['hosts'][number]>;
			}
		>();
		for (const edge of typeEdges) {
			if (edge.edge_type !== 'NetworkIdentity') continue;
			const serviceId = edge.identities_service_id;
			let group = groups.get(serviceId);
			if (!group) {
				const service = topology.services.find((s) => s.id === serviceId);
				group = {
					guest: service ? topology.hosts.find((h) => h.id === service.host_id) : undefined,
					identities: new SvelteMap()
				};
				groups.set(serviceId, group);
			}
			for (const host of identityHostsOfEdge(topology, edge)) group.identities.set(host.id, host);
		}
		return [...groups.entries()].map(([serviceId, { guest, identities }]) => ({
			serviceId,
			guest,
			identities: [...identities.values()]
		}));
	});

	function getDisplayComponent(edgeType: string) {
		switch (edgeType) {
			case 'Interface':
				return SameHostEdgeDisplay;
			case 'PhysicalLink':
			case 'NeighborLink':
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

	function hostContext(hostId: string, hideTags: HostTagRole[] = []): HostDisplayContext {
		return {
			services: topology?.services.filter((s) => s.host_id === hostId) ?? [],
			compact: true,
			hideTags
		};
	}
</script>

<InspectorSection
	id="edge:Aggregated:connections"
	title={topology_connectionsCount({ count: edges.length })}
	description={null}
>
	<div class="max-h-96 space-y-3 overflow-y-auto">
		{#each [...edgesByType.entries()] as [edgeType, typeEdges] (edgeType)}
			{@const typeName = isDependencyEdge(edgeType)
				? common_dependenciesLabel()
				: edgeTypes.getName(edgeType)}
			{@const displayComponent = getDisplayComponent(edgeType)}

			<InspectorSubsection
				id={`edge:Aggregated:${edgeType}`}
				title={typeName}
				icon={edgeTypes.getIconComponent(edgeType)}
				iconClass={edgeTypes.getColorHelper(edgeType).icon}
				count={typeEdges.length > 1 ? typeEdges.length : undefined}
			>
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
							<span class="text-tertiary mb-1 block text-xs font-medium"
								>{inspector_dockerService()}</span
							>
							<div class="card card-static">
								<EntityDisplayWrapper
									item={svcVirtData.containerizer}
									context={{ ipAddressId: null, ports: topology?.ports ?? [], compact: true }}
									displayComponent={ServiceDisplay}
								/>
							</div>
						{/if}
						{#if svcVirtData.containerized.length > 0}
							<span class="text-tertiary mb-1 block text-xs font-medium">
								{common_containerizedServices()} ({svcVirtData.containerized.length})
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
						{#if svcVirtData.containerHosts.length > 0}
							<span class="text-tertiary mb-1 block text-xs font-medium">
								{hosts_virtualization_containerHosts()} ({svcVirtData.containerHosts.length})
							</span>
							{#each svcVirtData.containerHosts as host (host.id)}
								<div class="card card-static">
									<EntityDisplayWrapper
										item={host}
										context={hostContext(host.id)}
										displayComponent={HostDisplay}
									/>
								</div>
							{/each}
						{/if}
					{:else}
						{#each svcVirtData.hosts as { host, containerCount, runtimes } (host.id)}
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
									{#each runtimes as runtime (runtime)}
										<Tag {...serviceDefinitions.getTag(runtime)} />
									{/each}
									<span class="text-tertiary text-xs"
										>{topology_containerCount({ count: containerCount })}</span
									>
								</div>
							</div>
						{/each}
					{/if}
				{:else if edgeType === 'NetworkIdentity' && identityGroups.length > 0}
					{#each identityGroups as group (group.serviceId)}
						{#if group.guest}
							<span class="text-tertiary mb-1 block text-xs font-medium"
								>{common_presentedBy()}</span
							>
							<div class="card card-static">
								<EntityDisplayWrapper
									item={group.guest}
									context={hostContext(group.guest.id)}
									displayComponent={HostDisplay}
								/>
							</div>
						{/if}
						<span class="text-tertiary mb-1 block text-xs font-medium">
							{hostVirtualizations.getName('NetworkIdentity')} ({group.identities.length})
						</span>
						{#each group.identities as host (host.id)}
							<div class="card card-static">
								<EntityDisplayWrapper
									item={host}
									context={hostContext(host.id, ['guest'])}
									displayComponent={HostDisplay}
								/>
							</div>
						{/each}
					{/each}
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
			</InspectorSubsection>
		{/each}
	</div>
</InspectorSection>
