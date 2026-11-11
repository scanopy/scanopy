<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import { useSvelteFlow } from '@xyflow/svelte';
	import { focusNodes } from '#lib/features/topology/viewport-fit.ts';
	import { Crosshair } from '@lucide/svelte';
	import EntityDisplayWrapper from '#lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { IPAddressDisplay } from '#lib/shared/components/forms/selection/display/IPAddressDisplay.svelte';
	import { ServiceDisplay } from '#lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import { InterfaceDisplay } from '#lib/shared/components/forms/selection/display/InterfaceDisplay.svelte';
	import { HostDisplay } from '#lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { SubnetDisplay } from '#lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import type { RenderableTopology, TopologyNode } from '#lib/features/topology/types/base.ts';
	import { entityGlyph } from '#lib/features/topology/entity-glyph.ts';
	import DeviceGlyph from '#lib/features/hosts/components/DeviceGlyph.svelte';
	import SoftwareGlyph from '#lib/features/topology/components/visualization/SoftwareGlyph.svelte';
	import type { TopologyEditState } from '#lib/features/topology/state.ts';
	import {
		nodeEntity,
		interfaceRelations,
		type ElementRenderContext,
		type ContainerRenderContext
	} from '#lib/features/topology/resolvers.ts';
	import {
		common_utilization,
		inspector_thisEntity,
		subnets_resolveRange,
		topology_focusNode
	} from '#lib/paraglide/messages.js';
	import {
		containerTypes,
		entities,
		inspectorSections,
		views
	} from '#lib/shared/stores/metadata.ts';
	import { activeView } from '#lib/features/topology/queries.ts';
	import { useSitesQuery } from '#lib/features/sites/queries.ts';
	import { useUpdateHostDescriptionMutation } from '#lib/features/hosts/queries.ts';
	import { useSubnetsQuery, useUpdateSubnetMutation } from '#lib/features/subnets/queries.ts';
	import { nestingItems, subnetNesting } from '#lib/features/subnets/nesting.ts';
	import HostInspectorActions from '#lib/features/hosts/components/HostInspectorActions.svelte';
	import InterfaceRelationRows from '#lib/features/hosts/components/InterfaceRelationRows.svelte';
	import SubnetUtilization from '#lib/features/subnets/components/SubnetUtilization.svelte';
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import UnconfirmedNotice from '#lib/shared/components/confirmation/UnconfirmedNotice.svelte';
	import { openModal } from '#lib/shared/stores/modal-registry.ts';
	import InspectorSection from '../shared/InspectorSection.svelte';
	import EntityDetailRows from '../shared/EntityDetailRows.svelte';
	import { getInspectorConfig } from '../view-config';

	/**
	 * The selection itself: its card, then its registry fields, plus the interface relations and
	 * subnet utilization the registry cannot express.
	 */
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

	const flow = useSvelteFlow();
	const sitesQuery = useSitesQuery();
	const updateHostDescriptionMutation = useUpdateHostDescriptionMutation();
	const updateSubnetMutation = useUpdateSubnetMutation();

	let isReadonly = $derived(editState.isReadonly);

	let sectionLabel = $derived.by(() => {
		if (elementContext) {
			const name = entities.getItem(elementContext.elementType)?.name ?? elementContext.elementType;
			return inspector_thisEntity({ name });
		}
		if (containerContext) {
			return inspector_thisEntity({ name: containerTypes.getName(containerContext.containerType) });
		}
		return '';
	});

	// The selection's own glyph heads the section. A device tile belongs to the host it names: an
	// entity that sits on a host (an address) keeps its own entity icon here, and the host's device type
	// shows beside the host itself, in its card below.
	let selectionGlyph = $derived.by(() => {
		const subject = nodeEntity(node.data as TopologyNode, topology);
		if (!subject) return null;
		const glyph = entityGlyph(subject.entityType, subject.entity, topology);
		return glyph?.type === 'device' && 'host_id' in subject.entity ? null : glyph;
	});
	let selectionEntityType = $derived(
		nodeEntity(node.data as TopologyNode, topology)?.entityType ?? null
	);

	let sections = $derived.by(() => {
		const config = getInspectorConfig($activeView);
		return elementContext ? config.element_sections : config.container_sections;
	});

	// Elements
	let thisInterface = $derived(elementContext?.snmpInterface ?? null);
	let thisIPAddress = $derived(elementContext?.ipAddress ?? null);
	let thisService = $derived(
		elementContext?.elementType === 'Service' && elementContext.services.length > 0
			? elementContext.services[0]
			: null
	);
	let thisHostElement = $derived(
		elementContext?.elementType === 'Host' ? (elementContext.host ?? null) : null
	);

	// Containers
	let thisSubnet = $derived(
		containerContext?.containerType === 'Subnet'
			? (topology.subnets.find((s) => s.id === node.id) ?? null)
			: null
	);
	let thisHostContainer = $derived.by(() => {
		if (containerContext?.containerType !== 'Host') return null;
		const entityId = (node.data as Record<string, unknown>)?.entity_id as string | undefined;
		return entityId ? (topology.hosts.find((h) => h.id === entityId) ?? null) : null;
	});
	let containerTitle = $derived(containerContext?.title ?? null);

	let ipAddressDisplayContext = $derived({
		subnets: topology.subnets,
		sites: sitesQuery.data ?? [],
		compact: true
	});

	let showNetworkDetail = $derived(
		views.getMetadata($activeView).inspector_config?.show_service_network_detail ?? true
	);
	let serviceDisplayContext = $derived({
		ipAddressId: showNetworkDetail ? (elementContext?.ipAddressId ?? null) : null,
		ports: showNetworkDetail ? topology.ports : [],
		showEntityTagPicker: !isReadonly,
		tagPickerDisabled: !editState.isEditable,
		entityTags: topology.entity_tags,
		compact: true
	});

	let hostElementContext = $derived({
		services: topology.services.filter((s) => thisHostElement && s.host_id === thisHostElement.id),
		showEntityTagPicker: true,
		tagPickerDisabled: !editState.isEditable,
		entityTags: isReadonly ? (topology.entity_tags ?? []) : undefined,
		showEditableEntityDescription: true,
		entityDescription: thisHostElement?.description ?? null,
		entityDescriptionDisabled: !editState.isEditable,
		onEntityDescriptionSave: (desc: string | null) => {
			if (thisHostElement) {
				updateHostDescriptionMutation.mutate({ host: thisHostElement, description: desc });
			}
		},
		compact: true
	});

	let hostContainerContext = $derived({
		showEntityTagPicker: !isReadonly,
		tagPickerDisabled: !editState.isEditable,
		entityTags: topology.entity_tags,
		compact: true,
		// The first service supplies the icon, as on every other topology host card.
		services: topology.services
	});

	let subnetContext = $derived({
		showEntityTagPicker: true,
		tagPickerDisabled: !editState.isEditable,
		entityTags: isReadonly ? (topology.entity_tags ?? []) : undefined,
		showEditableEntityDescription: true,
		entityDescription: thisSubnet?.description ?? null,
		entityDescriptionDisabled: !editState.isEditable,
		onEntityDescriptionSave: (desc: string | null) => {
			if (thisSubnet) {
				updateSubnetMutation.mutate({ ...thisSubnet, description: desc });
			}
		},
		compact: true
	});

	// Nesting and utilization are derived server-side across the whole site, so they come from the
	// live subnet list rather than the topology payload. Not in a read-only view: a shared link has
	// no session to fetch with, and a snapshot's past state would be paired with today's counts.
	const subnetsQuery = useSubnetsQuery(undefined, () => !!thisSubnet && !isReadonly);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let listedSubnet = $derived(
		thisSubnet && !isReadonly ? (subnetsData.find((s) => s.id === thisSubnet!.id) ?? null) : null
	);
	let nestedLabels = $derived(
		listedSubnet ? nestingItems(listedSubnet, subnetNesting(subnetsData)) : []
	);

	let relations = $derived(thisInterface ? interfaceRelations(thisInterface, topology) : null);

	let hasBody = $derived(
		!!(
			thisInterface ||
			thisIPAddress ||
			thisService ||
			thisHostElement ||
			thisSubnet ||
			thisHostContainer ||
			containerTitle
		)
	);
</script>

{#if hasBody}
	<InspectorSection id="Identity" section="Identity" title={sectionLabel}>
		{#snippet glyph()}
			{#if selectionGlyph?.type === 'device'}
				<DeviceGlyph deviceType={selectionGlyph.deviceType} />
			{:else if selectionGlyph?.type === 'software'}
				<SoftwareGlyph definition={selectionGlyph.definition} />
			{:else}
				{@const SelectionIcon = selectionEntityType
					? entities.getIconComponent(selectionEntityType)
					: inspectorSections.getIconComponent('Identity')}
				<SelectionIcon
					class="h-4 w-4 shrink-0 {selectionEntityType
						? entities.getColorHelper(selectionEntityType).icon
						: inspectorSections.getColorHelper('Identity').icon}"
				/>
			{/if}
		{/snippet}
		{#snippet actions()}
			<button
				class="btn-icon p-0.5"
				onclick={() => focusNodes(flow, [node.id])}
				title={topology_focusNode()}
			>
				<Crosshair class="h-3.5 w-3.5" />
			</button>
		{/snippet}
		<div class="space-y-2">
			{#if thisInterface}
				<div class="card-static card">
					<EntityDisplayWrapper
						context={{ compact: true }}
						item={thisInterface}
						displayComponent={InterfaceDisplay}
					/>
				</div>
			{:else if thisIPAddress}
				<div class="card-static card">
					<EntityDisplayWrapper
						context={ipAddressDisplayContext}
						item={thisIPAddress}
						displayComponent={IPAddressDisplay}
					/>
				</div>
			{:else if thisService}
				<div class="card-static card">
					<EntityDisplayWrapper
						context={serviceDisplayContext}
						item={thisService}
						displayComponent={ServiceDisplay}
					/>
				</div>
			{:else if thisHostElement}
				<HostInspectorActions host={thisHostElement} editable={editState.isEditable} />
				<div class="card-static card">
					<EntityDisplayWrapper
						context={hostElementContext}
						item={thisHostElement}
						displayComponent={HostDisplay}
					/>
				</div>
			{:else if thisSubnet}
				<UnconfirmedNotice
					entity={thisSubnet}
					resolveLabel={subnets_resolveRange()}
					onResolve={editState.isEditable
						? () => {
								if (!thisSubnet) return;
								window.location.hash = 'subnets';
								openModal('provisional-range', {
									id: thisSubnet.id,
									returnUrl: window.location.href
								});
							}
						: undefined}
				/>
				<div class="card-static card">
					<EntityDisplayWrapper
						context={subnetContext}
						item={thisSubnet}
						displayComponent={SubnetDisplay}
					/>
					{#if nestedLabels.length > 0}
						<div class="mt-2 flex flex-wrap gap-1">
							{#each nestedLabels as label (label.id)}
								<Tag label={label.label} color={label.color} title={label.title} />
							{/each}
						</div>
					{/if}
				</div>
			{:else if thisHostContainer}
				<div class="card-static card">
					<EntityDisplayWrapper
						context={hostContainerContext}
						item={thisHostContainer}
						displayComponent={HostDisplay}
					/>
				</div>
			{:else if containerTitle}
				<div class="card-static card">
					<p class="text-sm font-medium text-primary">{containerTitle}</p>
				</div>
			{/if}

			{#if relations}
				<EntityDetailRows {node} {topology} {sections} allFields>
					<InterfaceRelationRows {...relations} />
				</EntityDetailRows>
			{:else if listedSubnet}
				<EntityDetailRows {node} {topology} {sections}>
					<div class="space-y-1">
						<span class="text-sm text-secondary">{common_utilization()}</span>
						<SubnetUtilization subnet={listedSubnet} />
					</div>
				</EntityDetailRows>
			{:else}
				<EntityDetailRows {node} {topology} {sections} />
			{/if}
		</div>
	</InspectorSection>
{/if}
