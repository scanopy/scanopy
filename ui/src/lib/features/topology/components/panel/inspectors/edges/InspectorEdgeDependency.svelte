<script lang="ts">
	import { et } from '$lib/shared/utils/embed-i18n';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import {
		useUpdateDependencyMutation,
		useUpdateDependencyDescriptionMutation,
		useDeleteDependencyMutation
	} from '$lib/features/dependencies/queries';
	import {
		BindingWithServiceDisplay,
		type BindingWithServiceContext
	} from '$lib/shared/components/forms/selection/display/BindingWithServiceDisplay.svelte';
	import {
		ServiceDisplay,
		type ServiceDisplayContext
	} from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import { DependencyDisplay } from '$lib/shared/components/forms/selection/display/DependencyDisplay.svelte';
	import { ArrowDown, Edit, Trash2 } from 'lucide-svelte';
	import {
		common_delete,
		common_deleting,
		common_confirmDeleteName,
		common_edit
	} from '$lib/paraglide/messages';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import type { Dependency } from '$lib/features/dependencies/types/base';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import {
		editingDependencyId,
		selectedNodes,
		selectedEdge,
		topologyReadOnly
	} from '$lib/features/topology/queries';
	import { useTopology, selectedTopologyId } from '$lib/features/topology/context';
	import { getTopologyEditState } from '$lib/features/topology/state';
	import { clearSelection } from '$lib/features/topology/selection';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import { SvelteSet } from 'svelte/reactivity';

	import type { components } from '$lib/api/schema';
	type TopologyView = components['schemas']['TopologyView'];

	/* eslint-disable @typescript-eslint/no-unused-vars -- component contract props */
	let {
		dependencyId,
		sourceId,
		targetId,
		view = 'L3Logical'
	}: {
		dependencyId: string;
		sourceId: string;
		targetId: string;
		view?: TopologyView;
	} = $props();
	/* eslint-enable @typescript-eslint/no-unused-vars */

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

	let editState = $derived(getTopologyEditState(topology, false, isReadonly));

	// TanStack Query mutations for updating/deleting dependencies
	const updateDependencyMutation = useUpdateDependencyMutation();
	const descriptionMutation = useUpdateDependencyDescriptionMutation();
	const deleteDependencyMutation = useDeleteDependencyMutation();
	let isMutationPending = $derived(updateDependencyMutation.isPending);
	let isDeleting = $derived(deleteDependencyMutation.isPending);

	function handleDelete() {
		if (group && confirm(common_confirmDeleteName({ name: group.name }))) {
			deleteDependencyMutation.mutate(group.id, {
				onSuccess: () => clearSelection()
			});
		}
	}

	let group = $derived(topology ? topology.dependencies.find((g) => g.id == dependencyId) : null);

	// Local copy of dependency for editing
	let localGroup = $state<Dependency | null>(null);

	// Initialize from dependency when it loads
	$effect(() => {
		if (group) {
			localGroup = { ...group };
		}
	});

	// Auto-save when styling changes (only in non-readonly mode)
	// Guard against calling mutate while a mutation is already pending to prevent infinite loops
	$effect(() => {
		if (
			!isReadonly &&
			localGroup &&
			group &&
			!isMutationPending &&
			(localGroup.color !== group.color || localGroup.edge_style !== group.edge_style)
		) {
			updateDependencyMutation.mutate(localGroup);
		}
	});

	let groupColor = $derived(createColorHelper(group?.color || 'Gray'));

	let isRequestPath = $derived(group?.dependency_type == 'RequestPath');

	// TanStack Query for subnets (for isContainerSubnet check)
	const subnetsQuery = useSubnetsQuery();
	let subnetsData = $derived(subnetsQuery.data ?? []);

	// Create isContainerSubnet function from subnets data
	let isContainerSubnetFn = $derived((subnetId: string) => {
		const subnet = subnetsData.find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	});

	// Helper functions to get data from topology
	function getServiceForBindingFromTopology(bindingId: string) {
		if (!topology) return null;
		return topology.services.find((s) => s.bindings.some((b) => b.id === bindingId)) || null;
	}

	function getBindingFromTopology(bindingId: string) {
		if (!topology) return null;
		for (const service of topology.services) {
			const binding = service.bindings.find((b) => b.id === bindingId);
			if (binding) return binding;
		}
		return null;
	}

	function getHostForService(serviceHostId: string) {
		if (!topology) return null;
		return topology.hosts.find((h) => h.id === serviceHostId) || null;
	}

	// Build context for BindingWithServiceDisplay
	let bindingContext: BindingWithServiceContext = $derived({
		services: topology?.services ?? [],
		hosts: topology?.hosts ?? [],
		ip_addresses: topology?.ip_addresses ?? [],
		ports: topology?.ports ?? [],
		isContainerSubnet: isContainerSubnetFn,
		compact: true
	});

	// Edit: push the dep's members into the canvas selection as whatever nodes
	// represent them in the active view, clear the edge selection, and set
	// editingDependencyId so the panel routes to InspectorMultiSelect in edit
	// mode. Set editingDependencyId BEFORE clearing selectedEdge so the panel
	// switches away from InspectorEdge synchronously — otherwise InspectorEdge
	// briefly re-evaluates with a null edge and throws.
	//
	// Member→node mapping is view-aware. In Workloads / Application, Service
	// is a direct element_entity so service IDs match node IDs. In L3, Service
	// is inlined on IPAddress cards, so topology.nodes contains no Service
	// entries — the matching node is the IPAddress the binding lives on.
	// Collect both candidates per member and keep whichever actually exists in
	// topology.nodes for the current view.
	function startEditing() {
		if (!group || !topology) return;

		const candidateNodeIds = new SvelteSet<string>();
		const addServiceCandidates = (serviceId: string) => {
			candidateNodeIds.add(serviceId);
			const svc = topology!.services.find((s) => s.id === serviceId);
			if (!svc) return;
			for (const b of svc.bindings) {
				if (b.ip_address_id) candidateNodeIds.add(b.ip_address_id);
			}
		};

		if (group.members.type === 'Services') {
			for (const sid of group.members.service_ids) addServiceCandidates(sid);
		} else {
			for (const bid of group.members.binding_ids) {
				for (const svc of topology.services) {
					const binding = svc.bindings.find((b) => b.id === bid);
					if (!binding) continue;
					candidateNodeIds.add(svc.id);
					if (binding.ip_address_id) candidateNodeIds.add(binding.ip_address_id);
					break;
				}
			}
		}

		const memberNodes: import('@xyflow/svelte').Node[] = [...candidateNodeIds]
			.map((id) => topology!.nodes.find((n) => n.id === id))
			.filter((n): n is NonNullable<typeof n> => !!n)
			.map((n) => ({
				id: n.id,
				type: n.node_type,
				position: n.position,
				data: n
			}));

		editingDependencyId.set(group.id);
		if (memberNodes.length > 0) {
			selectedNodes.set(memberNodes);
		}
		selectedEdge.set(null);
	}

	// Context for group display with description
	let groupContext = $derived({
		compact: true,
		showEditableEntityDescription: true,
		entityDescription: group?.description ?? null,
		entityDescriptionDisabled: !editState.isEditable,
		onEntityDescriptionSave: (desc: string | null) => {
			if (group) {
				descriptionMutation.mutate({ dependencyId: group.id, description: desc });
			}
		}
	});
</script>

<div class="space-y-3">
	{#if group && localGroup}
		<span class="text-secondary mb-2 block text-sm font-medium">{et('Dependency')}</span>
		<div class="card card-static">
			<EntityDisplayWrapper
				context={groupContext}
				item={group}
				displayComponent={DependencyDisplay}
			/>
		</div>

		{#if !isReadonly && editState.isEditable}
			<button
				type="button"
				class="btn-secondary flex w-full items-center justify-center gap-2 text-xs"
				onclick={startEditing}
			>
				<Edit class="h-4 w-4" />
				{common_edit()}
			</button>
		{/if}

		{#if !isReadonly}
			<button
				type="button"
				disabled={isDeleting}
				onclick={handleDelete}
				class="btn-danger flex w-full items-center justify-center gap-2 text-xs"
			>
				<Trash2 class="h-4 w-4" />
				{isDeleting ? common_deleting() : common_delete()}
			</button>
		{/if}

		<span class="text-secondary mb-2 block text-sm font-medium">{et('Services')}</span>
		{#if group.members.type === 'Bindings'}
			{#each group.members.binding_ids as bindingId (bindingId)}
				{@const bindingService = getServiceForBindingFromTopology(bindingId)}
				{@const bindingHost = bindingService ? getHostForService(bindingService.host_id) : null}
				{@const bindingData = getBindingFromTopology(bindingId)}
				{#if bindingService && bindingHost && bindingData}
					<div
						class={isRequestPath
							? `card card-static ${bindingId == sourceId || bindingId == targetId ? 'ring-1 ring-gray-500' : ''}`
							: `card card-static ${bindingId == sourceId ? `ring-1 ${groupColor.ring}` : bindingId == targetId ? 'ring-1 ring-gray-500' : ''}`}
					>
						<EntityDisplayWrapper
							context={bindingContext}
							item={bindingData}
							displayComponent={BindingWithServiceDisplay}
						/>
					</div>
					{#if bindingId == sourceId && isRequestPath}
						<div class="flex flex-col items-center">
							<ArrowDown class="text-secondary h-5 w-5" />
						</div>
					{/if}
				{/if}
			{/each}
		{:else if group.members.type === 'Services'}
			{#each group.members.service_ids as serviceId (serviceId)}
				{@const service = topology?.services.find((s) => s.id === serviceId)}
				{#if service}
					<div class="card card-static">
						<EntityDisplayWrapper
							context={{ compact: true } satisfies ServiceDisplayContext}
							item={service}
							displayComponent={ServiceDisplay}
						/>
					</div>
				{/if}
			{/each}
		{/if}
	{/if}
</div>
