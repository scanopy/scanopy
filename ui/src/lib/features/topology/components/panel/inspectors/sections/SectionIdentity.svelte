<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import { useSvelteFlow } from '@xyflow/svelte';
	import { Crosshair } from 'lucide-svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { IPAddressDisplay } from '$lib/shared/components/forms/selection/display/IPAddressDisplay.svelte';
	import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import { InterfaceDisplay } from '$lib/shared/components/forms/selection/display/InterfaceDisplay.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import type { RenderableTopology, TopologyNode } from '$lib/features/topology/types/base';
	import type { TopologyEditState } from '$lib/features/topology/state';
	import type {
		ElementRenderContext,
		ContainerRenderContext
	} from '$lib/features/topology/resolvers';
	import { inspector_thisEntity, topology_focusNode } from '$lib/paraglide/messages';
	import { containerTypes, entities } from '$lib/shared/stores/metadata';
	import { activeView } from '$lib/features/topology/queries';
	import InspectorSection from '../shared/InspectorSection.svelte';
	import { useNetworksQuery } from '$lib/features/networks/queries';

	let {
		node,
		topology,
		editState,
		elementContext,
		containerContext
	}: {
		node: Node;
		topology: RenderableTopology;
		editState: TopologyEditState;
		elementContext?: ElementRenderContext;
		containerContext?: ContainerRenderContext;
	} = $props();

	const { fitView } = useSvelteFlow();

	function handleFocus() {
		fitView({ nodes: [{ id: node.id }], padding: 0.5, duration: 300 });
	}

	// Derive the section label from entity/container type metadata
	let sectionLabel = $derived.by(() => {
		if (elementContext) {
			const name = entities.getItem(elementContext.elementType)?.name ?? elementContext.elementType;
			return inspector_thisEntity({ name });
		}
		if (containerContext) {
			const name = containerTypes.getName(containerContext.containerType);
			return inspector_thisEntity({ name });
		}
		return '';
	});

	// For Interface elements: show the interface
	let thisIPAddress = $derived(elementContext?.ipAddress ?? null);
	const networksQuery = useNetworksQuery();
	let interfaceDisplayContext = $derived({
		subnets: topology.subnets,
		networks: networksQuery.data ?? [],
		compact: true
	});

	// For Service elements: show the service
	let thisService = $derived(
		elementContext?.elementType === 'Service' && elementContext.services.length > 0
			? elementContext.services[0]
			: null
	);
	let isApplicationView = $derived($activeView === 'Application');
	let serviceDisplayContext = $derived({
		ipAddressId: isApplicationView ? null : (elementContext?.ipAddressId ?? null),
		ports: isApplicationView ? [] : topology.ports,
		showEntityTagPicker: !editState.isReadonly,
		tagPickerDisabled: !editState.isEditable,
		entityTags: topology.entity_tags,
		compact: true
	});

	// For Interface (SNMP) elements: show the Interface
	let thisInterface = $derived.by(() => {
		if (elementContext?.elementType !== 'Interface') return null;
		const nodeData = node.data as TopologyNode;
		const ifEntryId = 'interface_id' in nodeData ? (nodeData.interface_id as string) : undefined;
		return ifEntryId ? (topology.interfaces.find((e) => e.id === ifEntryId) ?? null) : null;
	});

	// For Host containers: resolve via entity_id on the container node
	let thisHost = $derived.by(() => {
		if (containerContext?.containerType !== 'Host') return null;
		const entityId = (node.data as Record<string, unknown>)?.entity_id as string | undefined;
		if (entityId) {
			return topology.hosts.find((h) => h.id === entityId) ?? null;
		}
		return null;
	});
	let hostDisplayContext = $derived({
		showEntityTagPicker: !editState.isReadonly,
		tagPickerDisabled: !editState.isEditable,
		entityTags: topology.entity_tags,
		compact: true,
		// The first service supplies the icon, as on every other topology host card.
		services: topology.services
	});

	// For containers: show the header/title
	let containerTitle = $derived(containerContext?.title ?? null);

	// A Host element has none of these: SectionHostDetail heads it as the selection instead.
	let hasBody = $derived(
		!!(thisInterface || thisIPAddress || thisService || thisHost || containerTitle)
	);
</script>

{#if hasBody}
	<InspectorSection id="Identity" section="Identity" title={sectionLabel}>
		{#snippet actions()}
			<button class="btn-icon p-0.5" onclick={handleFocus} title={topology_focusNode()}>
				<Crosshair class="h-3.5 w-3.5" />
			</button>
		{/snippet}
		{#if thisInterface}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={{ compact: true }}
					item={thisInterface}
					displayComponent={InterfaceDisplay}
				/>
			</div>
		{:else if thisIPAddress}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={interfaceDisplayContext}
					item={thisIPAddress}
					displayComponent={IPAddressDisplay}
				/>
			</div>
		{:else if thisService}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={serviceDisplayContext}
					item={thisService}
					displayComponent={ServiceDisplay}
				/>
			</div>
		{:else if thisHost}
			<div class="card card-static">
				<EntityDisplayWrapper
					context={hostDisplayContext}
					item={thisHost}
					displayComponent={HostDisplay}
				/>
			</div>
		{:else if containerTitle}
			<div class="card card-static">
				<p class="text-primary text-sm font-medium">{containerTitle}</p>
			</div>
		{/if}
	</InspectorSection>
{/if}
