<script lang="ts">
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import type { PortBinding, Service } from '$lib/features/services/types/base';
	import { createDefaultService } from '$lib/features/services/queries';
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import { ServiceTypeDisplay } from '$lib/shared/components/forms/selection/display/ServiceTypeDisplay.svelte';
	import { pushError } from '$lib/shared/stores/feedback';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import ServiceConfigPanel from './ServiceConfigPanel.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import { ArrowRightLeft } from 'lucide-svelte';
	import {
		common_noServiceSelected,
		common_services,
		hosts_services_emptyMessage,
		hosts_services_helpText,
		hosts_services_invalidIndex,
		hosts_services_placeholder,
		hosts_services_selectToConfig,
		hosts_services_transferBindingsTitle,
		hosts_services_transferPorts
	} from '$lib/paraglide/messages';

	interface Props {
		formData: HostFormData;
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any; setFieldValue: any };
	}

	let { formData = $bindable(), form }: Props = $props();

	let selectedPortBindings = $state<PortBinding[]>([]);

	// Available service types for adding
	let availableServiceTypes = $derived(
		serviceDefinitions
			.getItems()
			?.filter((service) => service.metadata?.can_be_added !== false)
			.sort((a, b) => (a.category ?? '').localeCompare(b.category ?? '', 'en')) || []
	);

	function handleItemSelect() {
		selectedPortBindings = [];
	}

	function handleTransferPorts(transferToService: Service, transferFromService: Service) {
		const bindingIdsToTransfer = new Set(selectedPortBindings.map((b) => b.id));

		// Update bindings with new service_id and network_id for the target service
		const transferredBindings = selectedPortBindings.map((b) => ({
			...b,
			service_id: transferToService.id,
			network_id: transferToService.network_id
		}));

		formData.services = formData.services.map((s) => {
			if (s.id === transferToService.id) {
				return {
					...s,
					bindings: [...s.bindings, ...transferredBindings]
				};
			} else if (s.id === transferFromService.id) {
				return {
					...s,
					bindings: s.bindings.filter((b) => !bindingIdsToTransfer.has(b.id))
				};
			}
			return s;
		});

		selectedPortBindings = [];
	}

	// Event handlers
	function handleAddService(serviceTypeId: string) {
		const serviceMetadata = serviceDefinitions.getItems()?.find((s) => s.id === serviceTypeId);
		if (!serviceMetadata) return;

		const newService: Service = createDefaultService(
			serviceTypeId,
			formData.id,
			formData.network_id
		);

		formData.services = [...formData.services, newService];
	}

	function handleRemoveService(index: number) {
		formData.services = formData.services.filter((_, i) => i !== index);
	}

	function handleServiceChange(service: Service, index: number) {
		if (index >= 0 && index < formData.services.length) {
			const updatedServices = [...formData.services];
			updatedServices[index] = service;
			formData.services = updatedServices;
		} else {
			pushError(hosts_services_invalidIndex());
		}
	}

	function handleServiceReorder(fromIndex: number, toIndex: number) {
		if (fromIndex === toIndex) return;

		const updatedServices = [...formData.services];
		const [movedService] = updatedServices.splice(fromIndex, 1);
		updatedServices.splice(toIndex, 0, movedService);

		formData.services = updatedServices;
		form.setFieldValue('services', formData.services);
	}
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<ListConfigEditor
		items={formData.services}
		onChange={handleServiceChange}
		onReorder={handleServiceReorder}
		onItemSelect={handleItemSelect}
	>
		<svelte:fragment
			slot="list"
			let:items
			let:onEdit
			let:highlightedIndex
			let:highlightedItem
			let:onMoveUp
			let:onMoveDown
			let:onItemSelect
		>
			{@const isTransferringPortBindings = selectedPortBindings.length > 0}
			<ListManager
				label={common_services()}
				helpText={hosts_services_helpText()}
				placeholder={hosts_services_placeholder()}
				emptyMessage={hosts_services_emptyMessage()}
				options={availableServiceTypes}
				itemClickAction="edit"
				showSearch={true}
				{items}
				allowItemRemove={() => !isTransferringPortBindings}
				allowReorder={!isTransferringPortBindings}
				optionDisplayComponent={ServiceTypeDisplay}
				itemDisplayComponent={ServiceDisplay}
				getItemContext={() => ({})}
				onAdd={handleAddService}
				onRemove={handleRemoveService}
				onClick={onItemSelect}
				{onMoveDown}
				{onMoveUp}
				{onEdit}
				{highlightedIndex}
			>
				{#snippet itemSnippet({ item })}
					<div class="flex min-w-0 flex-1 items-center justify-between gap-2">
						<div class="min-w-0 flex-1 overflow-hidden">
							<ListSelectItem
								{item}
								context={{ interfaceId: null }}
								displayComponent={ServiceDisplay}
							/>
						</div>

						{#if selectedPortBindings.length > 0 && item != highlightedItem && highlightedItem != null && !item.bindings.some((b) => b.type == 'Interface')}
							<button
								type="button"
								onclick={(e) => {
									e.stopPropagation();
									handleTransferPorts(item, highlightedItem);
								}}
								class="btn-secondary flex-shrink-0 text-xs"
								title={hosts_services_transferBindingsTitle({
									count: selectedPortBindings.length
								})}
							>
								<ArrowRightLeft size={12} />
								<span>{hosts_services_transferPorts()}</span>
							</button>
						{/if}
					</div>
				{/snippet}
			</ListManager>
		</svelte:fragment>

		<svelte:fragment slot="config" let:selectedItem let:selectedIndex>
			<!-- Render all service config panels to register form fields, but only show the selected one -->
			{#each formData.services as service, index (`${service.id}-${index}`)}
				<div class:hidden={selectedIndex !== index}>
					<ServiceConfigPanel
						host={formData}
						{index}
						{service}
						{form}
						onChange={(updatedService) => handleServiceChange(updatedService, index)}
						bind:selectedPortBindings
						currentServices={formData.services}
					/>
				</div>
			{/each}

			{#if !selectedItem}
				<EntityConfigEmpty
					title={common_noServiceSelected()}
					subtitle={hosts_services_selectToConfig()}
				/>
			{/if}
		</svelte:fragment>
	</ListConfigEditor>

	<EntityMetadataSection entities={formData.services} />
</div>
