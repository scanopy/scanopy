<script lang="ts">
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import type { Service } from '$lib/features/services/types/base';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import VmManagerConfigPanel from './VmManagerConfigPanel.svelte';
	import ContainerManagerConfigPanel from './ContainerManagerConfigPanel.svelte';
	import NetworkIdentitiesConfigPanel from './NetworkIdentitiesConfigPanel.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import {
		VirtualizationManagerServiceDisplay,
		type VirtualizationManagerContext
	} from '$lib/shared/components/forms/selection/display/VirtualizationManagerServiceDisplay.svelte';
	import type { Host } from '$lib/features/hosts/types/base';
	import { uuidv4Sentinel } from '$lib/shared/utils/formatting';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
	import {
		common_noEntitySelected,
		common_service,
		hosts_virtualization_emptyMessage,
		hosts_virtualization_emptySubtitle,
		hosts_virtualization_emptyTitle,
		hosts_virtualization_pleaseSave,
		hosts_virtualization_pleaseSaveBody,
		hosts_virtualization_selectToManage,
		hosts_virtualization_servicesHelp,
		hosts_virtualization_servicesLabel,
		hosts_virtualization_unknownType,
		hosts_virtualization_unknownTypeSubtitle
	} from '$lib/paraglide/messages';

	interface Props {
		virtualizationManagerServices: Service[];
		/** Staged (unsaved) VM-host reassignments, keyed by host id. */
		managedHostUpdates: Map<string, Host>;
		/** The host being edited's current (staged) services. */
		editedServices: Service[];
		onServiceChange: (service: Service) => void;
		onVirtualizedHostChange: (host: Host) => void;
	}

	let {
		virtualizationManagerServices,
		managedHostUpdates,
		editedServices,
		onServiceChange,
		onVirtualizedHostChange
	}: Props = $props();

	// TanStack Query hooks for context data
	//
	// Still the full nested query, deliberately. The VM picker labels each
	// candidate host with its services (`VmManagerConfigPanel.getHostServices`),
	// and those come from the services cache — which has no fetcher of its own and
	// is populated as a side effect of *this* query. Switching to a children-free
	// host list would leave the picker's service chips empty for any host not on
	// the hosts tab's current page.
	//
	// Fixing that properly means giving the services cache a real source, which is
	// the deferred child-cache re-architecture (see
	// planned-work/child-cache-rearchitecture.md). This surface is lazy — it only
	// mounts inside the host editor's workloads tab — so it no longer costs
	// anything on page load.
	const hostsQuery = useHostsQuery({ limit: 0 });
	const servicesQuery = useServicesCacheQuery();
	let hostsData = $derived(hostsQuery.data?.items ?? []);
	let servicesData = $derived(servicesQuery.data ?? []);

	// "Effective" lists = saved server state overlaid with the staged (unsaved)
	// edits, so assignment counts and panel lists reflect in-progress changes.
	let effectiveHosts = $derived(hostsData.map((h) => managedHostUpdates.get(h.id) ?? h));
	// Overlay staged service edits over the cached list (edited entries win).
	let effectiveServices = $derived([
		...servicesData.filter((s) => !editedServices.some((e) => e.id === s.id)),
		...editedServices
	]);

	// Context for VirtualizationManagerServiceDisplay (drives the per-manager count)
	let displayContext: VirtualizationManagerContext = $derived({
		hosts: effectiveHosts,
		services: effectiveServices
	});
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<ListConfigEditor items={virtualizationManagerServices} onChange={onServiceChange}>
		<svelte:fragment slot="list" let:items let:onEdit let:highlightedIndex>
			<ListManager
				label={hosts_virtualization_servicesLabel()}
				helpText={hosts_virtualization_servicesHelp()}
				emptyMessage={hosts_virtualization_emptyMessage()}
				{items}
				itemClickAction="edit"
				allowItemRemove={() => false}
				allowReorder={false}
				allowAddFromOptions={false}
				options={[] as Service[]}
				itemDisplayComponent={VirtualizationManagerServiceDisplay}
				optionDisplayComponent={VirtualizationManagerServiceDisplay}
				getItemContext={() => displayContext}
				{onEdit}
				{highlightedIndex}
			/>
		</svelte:fragment>

		<svelte:fragment slot="config" let:selectedItem>
			{#if selectedItem}
				{#if selectedItem.id == uuidv4Sentinel}
					<InlineWarning
						title={hosts_virtualization_pleaseSave({ name: selectedItem.name })}
						body={hosts_virtualization_pleaseSaveBody({ name: selectedItem.name })}
					/>
				{:else}
					{@const virtualizationType = serviceDefinitions.getMetadata(
						selectedItem.service_definition
					).manages_virtualization}
					{#if virtualizationType === 'vms'}
						<VmManagerConfigPanel
							service={selectedItem}
							hosts={effectiveHosts}
							services={effectiveServices}
							onChange={(updatedHost) => onVirtualizedHostChange(updatedHost)}
						/>
					{:else if virtualizationType === 'containers'}
						<ContainerManagerConfigPanel
							service={selectedItem}
							hosts={effectiveHosts}
							services={effectiveServices}
							onChange={(updatedService) => onServiceChange(updatedService)}
						/>
					{:else if virtualizationType === 'identities'}
						<NetworkIdentitiesConfigPanel
							service={selectedItem}
							hosts={effectiveHosts}
							services={effectiveServices}
						/>
					{:else}
						<EntityConfigEmpty
							title={hosts_virtualization_unknownType()}
							subtitle={hosts_virtualization_unknownTypeSubtitle()}
						/>
					{/if}
				{/if}
			{:else if virtualizationManagerServices.length === 0}
				<EntityConfigEmpty
					title={hosts_virtualization_emptyTitle()}
					subtitle={hosts_virtualization_emptySubtitle()}
				/>
			{:else}
				<EntityConfigEmpty
					title={common_noEntitySelected({ entity: common_service() })}
					subtitle={hosts_virtualization_selectToManage()}
				/>
			{/if}
		</svelte:fragment>
	</ListConfigEditor>
</div>
