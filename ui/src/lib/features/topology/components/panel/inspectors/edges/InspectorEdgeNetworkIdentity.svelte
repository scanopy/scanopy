<script lang="ts">
	import type { Edge } from '@xyflow/svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { useTopology, selectedTopologyId } from '$lib/features/topology/context';
	import { getTopologyEditState } from '$lib/features/topology/state';
	import { topologyReadOnly } from '$lib/features/topology/queries';
	import type { RenderableTopology, TopologyEdge } from '$lib/features/topology/types/base';
	import { identityHostsOfEdge } from '$lib/features/topology/resolvers';
	import type { Host } from '$lib/features/hosts/types/base';
	import { entities, hostVirtualizations } from '$lib/shared/stores/metadata';
	import { common_interface, common_presentedBy } from '$lib/paraglide/messages';
	import InspectorSection from '../shared/InspectorSection.svelte';

	let { edge, identitiesServiceId }: { edge: Edge; identitiesServiceId: string } = $props();

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let isReadonly = $derived(topo.isReadonly || $topologyReadOnly);
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					RenderableTopology | undefined)
	);

	let editState = $derived(getTopologyEditState(topology, false, isReadonly));

	// The guest presenting the identity: the host of its Network Identities service.
	let identitiesService = $derived(
		topology?.services.find((s) => s.id === identitiesServiceId) ?? null
	);
	let guestHost = $derived(
		identitiesService
			? (topology?.hosts.find((h) => h.id === identitiesService.host_id) ?? null)
			: null
	);

	// The edge runs from the guest's address to the identity's address on the same subnet.
	let identityHosts = $derived(
		topology && edge.data ? identityHostsOfEdge(topology, edge.data as TopologyEdge) : []
	);
	let identityInterface = $derived.by(() => {
		if (identityHosts.length !== 1) return null;
		const virtualization = identityHosts[0].virtualization_metadata;
		return virtualization?.type === 'NetworkIdentity' ? virtualization.details.interface : null;
	});

	function hostContext(host: Host) {
		return {
			services: topology?.services.filter((s) => s.host_id === host.id) ?? [],
			showEntityTagPicker: true,
			tagPickerDisabled: !editState.isEditable,
			entityTags: isReadonly ? (topology?.entity_tags ?? []) : undefined,
			compact: true
		};
	}
</script>

<div class="space-y-4">
	{#if guestHost}
		<InspectorSection
			id="edge:NetworkIdentity:presentedBy"
			title={common_presentedBy()}
			icon={entities.getIconComponent('Host')}
			iconClass={entities.getColorHelper('Host').icon}
			description={null}
		>
			<div class="card card-static">
				<EntityDisplayWrapper
					context={hostContext(guestHost)}
					item={guestHost}
					displayComponent={HostDisplay}
				/>
			</div>
		</InspectorSection>
	{/if}

	{#if identityHosts.length > 0}
		<InspectorSection
			id="edge:NetworkIdentity:identities"
			title={hostVirtualizations.getName('NetworkIdentity')}
			icon={entities.getIconComponent('Host')}
			iconClass={entities.getColorHelper('Host').icon}
			description={hostVirtualizations.getDescription('NetworkIdentity') || null}
			count={identityHosts.length}
		>
			<div class="space-y-1">
				{#each identityHosts as identityHost (identityHost.id)}
					<div class="card card-static">
						<EntityDisplayWrapper
							context={hostContext(identityHost)}
							item={identityHost}
							displayComponent={HostDisplay}
						/>
					</div>
				{/each}
			</div>
		</InspectorSection>
	{/if}

	{#if identityInterface}
		<InspectorSection
			id="edge:NetworkIdentity:interface"
			title={common_interface()}
			icon={entities.getIconComponent('Interface')}
			iconClass={entities.getColorHelper('Interface').icon}
			description={null}
		>
			<p class="text-primary font-mono text-sm">{identityInterface}</p>
		</InspectorSection>
	{/if}
</div>
