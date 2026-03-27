<script lang="ts">
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import type { Service } from '$lib/features/services/types/base';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import VmManagerConfigPanel from './VmManagerConfigPanel.svelte';
	import ContainerManagerConfigPanel from './ContainerManagerConfigPanel.svelte';
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
		common_noServiceSelected,
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
		onServiceChange: (service: Service) => void;
		onVirtualizedHostChange: (host: Host) => void;
	}

	let { virtualizationManagerServices, onServiceChange, onVirtualizedHostChange }: Props = $props();

	// TanStack Query hooks for context data
	// Use limit: 0 to get all hosts for virtualization form
	const hostsQuery = useHostsQuery({ limit: 0 });
	const servicesQuery = useServicesCacheQuery();
	let hostsData = $derived(hostsQuery.data?.items ?? []);
	let servicesData = $derived(servicesQuery.data ?? []);

	// Context for VirtualizationManagerServiceDisplay
	let displayContext: VirtualizationManagerContext = $derived({
		hosts: hostsData,
		services: servicesData
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
							onChange={(updatedHost) => onVirtualizedHostChange(updatedHost)}
						/>
					{:else if virtualizationType === 'containers'}
						<ContainerManagerConfigPanel
							service={selectedItem}
							onChange={(updatedService) => onServiceChange(updatedService)}
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
					title={common_noServiceSelected()}
					subtitle={hosts_virtualization_selectToManage()}
				/>
			{/if}
		</svelte:fragment>
	</ListConfigEditor>
</div>
