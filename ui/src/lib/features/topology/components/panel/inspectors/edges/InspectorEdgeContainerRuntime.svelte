<script lang="ts">
	import type { Edge } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import { topologyReadOnly } from '$lib/features/topology/queries';
	import { useTopology, selectedTopologyId } from '$lib/features/topology/context';
	import { getTopologyEditState } from '$lib/features/topology/state';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import type { RenderableTopology, TopologyEdge } from '$lib/features/topology/types/base';
	import { containerHostsOfEdge } from '$lib/features/topology/resolvers';
	import {
		common_containerizedService,
		common_containerizedServices,
		hosts_virtualization_containerHosts,
		topology_containerBridgeSubnet,
		topology_containerBridgeSubnets,
		topology_containerHost,
		topology_containerService
	} from '$lib/paraglide/messages';
	import { entities } from '$lib/shared/stores/metadata';
	import InspectorSection from '../shared/InspectorSection.svelte';

	let { edge, serviceId }: { edge: Edge; serviceId: string } = $props();

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let isReadonly = $derived(topo.isReadonly || $topologyReadOnly);
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					RenderableTopology | undefined)
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

	// The edge names the containers it stands for — the ones on the bridge subnet(s) it
	// reaches, already narrowed for the current grouping. Resolving them here from the edge's
	// endpoint can't work: the endpoint is elevated onto the subnet box before it reaches us.
	let containerizedServiceIds = $derived(
		((edge.data as Record<string, unknown> | undefined)?.containerized_service_ids as
			string[] | undefined) ?? []
	);
	let containerizedServices = $derived(
		topology
			? containerizedServiceIds.flatMap((id) => topology.services.find((s) => s.id === id) ?? [])
			: []
	);

	// An edge to a container host (macvlan, ipvlan) names no containerized services.
	let containerHosts = $derived(
		topology && edge.data ? containerHostsOfEdge(topology, edge.data as TopologyEdge) : []
	);

	// The bridges this edge reaches — one when they render as separate boxes, all of them when
	// merged. Walking the listed containers' bindings instead would pull in every other bridge
	// a multi-attached container happens to sit on, which this edge does not connect.
	let allBridgeSubnets = $derived(
		topology
			? (
					((edge.data as Record<string, unknown> | undefined)?.subnet_ids as
						string[] | undefined) ?? []
				).flatMap((id) => topology.subnets.find((s) => s.id === id) ?? [])
			: []
	);
</script>

<div class="space-y-4">
	{#if containerizingHost}
		<InspectorSection
			id="edge:ContainerRuntime:host"
			title={topology_containerHost()}
			icon={entities.getIconComponent('Host')}
			iconClass={entities.getColorHelper('Host').icon}
			description={null}
		>
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
		</InspectorSection>
	{/if}
	{#if containerizingService}
		<InspectorSection
			id="edge:ContainerRuntime:service"
			title={topology_containerService()}
			icon={entities.getIconComponent('Service')}
			iconClass={entities.getColorHelper('Service').icon}
			description={null}
		>
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
		</InspectorSection>
	{/if}

	{#if containerHosts.length > 0}
		<!-- An edge to a container host: the subnet it reaches is the LAN, not a bridge. -->
		<InspectorSection
			id="edge:ContainerRuntime:containerHosts"
			title={hosts_virtualization_containerHosts()}
			icon={entities.getIconComponent('Host')}
			iconClass={entities.getColorHelper('Host').icon}
			description={null}
			count={containerHosts.length}
		>
			<div class="space-y-1">
				{#each containerHosts as host (host.id)}
					<div class="card card-static">
						<EntityDisplayWrapper
							context={{
								services: topology?.services.filter((s) => s.host_id == host.id) ?? [],
								showEntityTagPicker: true,
								tagPickerDisabled: !editState.isEditable,
								entityTags: isReadonly ? (topology?.entity_tags ?? []) : undefined,
								compact: true
							}}
							item={host}
							displayComponent={HostDisplay}
						/>
					</div>
				{/each}
			</div>
		</InspectorSection>
	{/if}
	{#if containerizedServices.length > 0}
		<InspectorSection
			id="edge:ContainerRuntime:containers"
			title={containerizedServices.length === 1
				? common_containerizedService()
				: common_containerizedServices()}
			icon={entities.getIconComponent('Service')}
			iconClass={entities.getColorHelper('Service').icon}
			description={null}
			count={containerizedServices.length}
		>
			<div class="space-y-1">
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
			</div>
		</InspectorSection>
	{/if}

	{#if allBridgeSubnets.length > 0 && containerHosts.length === 0}
		<InspectorSection
			id="edge:ContainerRuntime:subnets"
			title={allBridgeSubnets.length > 1
				? topology_containerBridgeSubnets()
				: topology_containerBridgeSubnet()}
			icon={entities.getIconComponent('Subnet')}
			iconClass={entities.getColorHelper('Subnet').icon}
			description={null}
			count={allBridgeSubnets.length}
		>
			<div class="space-y-1">
				{#each allBridgeSubnets as subnet (subnet.id)}
					<div class="card card-static">
						<EntityDisplayWrapper
							context={{ compact: true }}
							item={subnet}
							displayComponent={SubnetDisplay}
						/>
					</div>
				{/each}
			</div>
		</InspectorSection>
	{/if}
</div>
