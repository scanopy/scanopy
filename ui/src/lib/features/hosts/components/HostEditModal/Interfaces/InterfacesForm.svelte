<script lang="ts">
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import InterfaceConfigPanel from './InterfaceConfigPanel.svelte';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { type HostFormData, type Interface } from '$lib/features/hosts/types/base';
	import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import { InterfaceDisplay } from '$lib/shared/components/forms/selection/display/InterfaceDisplay.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import InternetInterfaceConfigPanel from './InternetInterfaceConfigPanel.svelte';
	import { v4 as uuidv4 } from 'uuid';
	import type { Service } from '$lib/features/services/types/base';
	import ConfirmationDialog from '$lib/shared/components/feedback/ConfirmationDialog.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import {
		common_cancel,
		common_interfaces,
		hosts_interfaces_deleteMessage,
		hosts_interfaces_deleteTitle,
		hosts_interfaces_emptyMessage,
		hosts_interfaces_helpText,
		hosts_interfaces_noSelected,
		hosts_interfaces_placeholder,
		hosts_interfaces_selectToConfig
	} from '$lib/paraglide/messages';

	interface Props {
		formData: HostFormData;
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any; setFieldValue: any };
		currentServices?: Service[];
		onServicesChange?: (services: Service[]) => void;
		isEditing?: boolean;
		targetEntityId?: string | null;
	}

	let {
		formData = $bindable(),
		form,
		currentServices = [],
		onServicesChange = () => {},
		isEditing = false,
		targetEntityId = $bindable(null)
	}: Props = $props();

	// TanStack Query for subnets
	const subnetsQuery = useSubnetsQuery();
	let subnetsData = $derived(subnetsQuery.data ?? []);

	// Confirmation dialog state
	let showDeleteConfirmation = $state(false);
	let pendingDeleteIndex: number | null = $state(null);
	let affectedServiceNames: string[] = $state([]);

	// Find services that have bindings to a specific interface
	function getServicesWithBindingsToInterface(interfaceId: string): Service[] {
		return currentServices.filter((service) =>
			service.bindings.some(
				(b) =>
					(b.type === 'Interface' && b.interface_id === interfaceId) ||
					(b.type === 'Port' && b.interface_id === interfaceId)
			)
		);
	}

	// Remove bindings to an interface from all services
	function removeBindingsToInterface(interfaceId: string) {
		const updatedServices = currentServices.map((service) => ({
			...service,
			bindings: service.bindings.filter(
				(b) =>
					!(b.type === 'Interface' && b.interface_id === interfaceId) &&
					!(b.type === 'Port' && b.interface_id === interfaceId)
			)
		}));
		onServicesChange(updatedServices);
	}

	// Computed values
	let interfaces = $derived(formData.interfaces || []);

	let availableSubnets = $derived(subnetsData.filter((s) => s.network_id == formData.network_id));

	// Helper function to find subnet by ID
	function findSubnetById(subnetId: string) {
		return subnetsData.find((s) => s.id === subnetId) || null;
	}

	// Event handlers
	function handleAddInterface(subnetId: string) {
		const subnet = findSubnetById(subnetId);
		if (!subnet) return;

		if (subnet.cidr == '0.0.0.0/0') {
			const newInterface: Interface = {
				id: uuidv4(), // Temp ID for form - store will detect as new since it's not in interfaces store
				host_id: formData.id,
				network_id: formData.network_id,
				name: subnet.name,
				subnet_id: subnetId,
				ip_address: '203.0.113.' + (Math.floor(Math.random() * 255) + 1).toString(),
				mac_address: null,
				created_at: new Date().toISOString(),
				updated_at: new Date().toISOString()
			};

			formData.interfaces = [...interfaces, newInterface];
			form.setFieldValue('interfaces', formData.interfaces);
		} else {
			const newInterface: Interface = {
				id: uuidv4(), // Temp ID for form - store will detect as new since it's not in interfaces store
				host_id: formData.id,
				network_id: formData.network_id,
				name: null,
				subnet_id: subnetId,
				ip_address: '',
				mac_address: null,
				created_at: new Date().toISOString(),
				updated_at: new Date().toISOString()
			};

			formData.interfaces = [...interfaces, newInterface];
			form.setFieldValue('interfaces', formData.interfaces);
		}
	}

	function handleRemoveInterface(index: number) {
		const iface = interfaces[index];
		const affectedServices = getServicesWithBindingsToInterface(iface.id);

		if (affectedServices.length > 0) {
			// Show confirmation dialog
			pendingDeleteIndex = index;
			affectedServiceNames = affectedServices.map((s) => s.name);
			showDeleteConfirmation = true;
		} else {
			// No bindings, delete immediately
			formData.interfaces = interfaces.filter((_, i) => i !== index);
			form.setFieldValue('interfaces', formData.interfaces);
		}
	}

	function confirmDelete() {
		if (pendingDeleteIndex !== null) {
			const iface = interfaces[pendingDeleteIndex];
			// Remove bindings from services first
			removeBindingsToInterface(iface.id);
			// Then remove the interface
			formData.interfaces = interfaces.filter((_, i) => i !== pendingDeleteIndex);
			form.setFieldValue('interfaces', formData.interfaces);
		}
		// Reset dialog state
		showDeleteConfirmation = false;
		pendingDeleteIndex = null;
		affectedServiceNames = [];
	}

	function cancelDelete() {
		showDeleteConfirmation = false;
		pendingDeleteIndex = null;
		affectedServiceNames = [];
	}

	function handleInterfaceChange(updatedInterface: Interface, index: number) {
		// Update formData.interfaces for real-time sync with list display and bindings
		// Note: Don't call form.setFieldValue here - the form field already updated
		// form state via field.handleChange. We only need to sync formData for display.
		const updatedInterfaces = [...formData.interfaces];
		updatedInterfaces[index] = updatedInterface;
		formData.interfaces = updatedInterfaces;
	}

	function handleReorder(fromIndex: number, toIndex: number) {
		if (fromIndex === toIndex) return;

		const updatedInterfaces = [...formData.interfaces];
		const [movedInterface] = updatedInterfaces.splice(fromIndex, 1);
		updatedInterfaces.splice(toIndex, 0, movedInterface);

		formData.interfaces = updatedInterfaces;
		form.setFieldValue('interfaces', formData.interfaces);
	}
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<ListConfigEditor items={formData.interfaces} onReorder={handleReorder} bind:targetEntityId>
		<svelte:fragment
			slot="list"
			let:items
			let:onEdit
			let:highlightedIndex
			let:onMoveUp
			let:onMoveDown
		>
			<ListManager
				label={common_interfaces()}
				helpText={hosts_interfaces_helpText()}
				placeholder={hosts_interfaces_placeholder()}
				emptyMessage={hosts_interfaces_emptyMessage()}
				allowReorder={true}
				itemClickAction="edit"
				options={availableSubnets}
				{items}
				optionDisplayComponent={SubnetDisplay}
				itemDisplayComponent={InterfaceDisplay}
				getItemContext={() => ({ subnets: subnetsData })}
				onAdd={handleAddInterface}
				onRemove={handleRemoveInterface}
				{onMoveUp}
				{onMoveDown}
				{onEdit}
				{highlightedIndex}
			/>
		</svelte:fragment>

		<svelte:fragment slot="config" let:selectedItem let:selectedIndex let:onChange>
			{@const selectedSubnet = selectedItem ? findSubnetById(selectedItem.subnet_id) : null}

			<!-- Render all interface config panels to register form fields, but only show the selected one -->
			<!-- Key includes index to force re-mount when position changes (reordering) -->
			{#each interfaces as iface, index (`${iface.id}-${index}`)}
				{@const subnet = findSubnetById(iface.subnet_id)}
				{#if subnet && subnet.cidr !== '0.0.0.0/0'}
					<div class:hidden={selectedIndex !== index}>
						<InterfaceConfigPanel
							{iface}
							{subnet}
							{index}
							{form}
							{isEditing}
							onChange={(updatedInterface) => handleInterfaceChange(updatedInterface, index)}
						/>
					</div>
				{/if}
			{/each}

			<!-- Show internet interface panel only when selected (no form validation needed) -->
			{#if selectedItem && selectedSubnet && selectedSubnet.cidr === '0.0.0.0/0'}
				<InternetInterfaceConfigPanel
					iface={selectedItem}
					subnet={selectedSubnet}
					onChange={(updatedInterface) => onChange(updatedInterface)}
				/>
			{:else if !selectedItem}
				<EntityConfigEmpty
					title={hosts_interfaces_noSelected()}
					subtitle={hosts_interfaces_selectToConfig()}
				/>
			{/if}
		</svelte:fragment>
	</ListConfigEditor>

	<EntityMetadataSection entities={formData.interfaces} />
</div>

<ConfirmationDialog
	isOpen={showDeleteConfirmation}
	title={hosts_interfaces_deleteTitle()}
	message={hosts_interfaces_deleteMessage()}
	details={affectedServiceNames}
	confirmLabel={hosts_interfaces_deleteTitle()}
	cancelLabel={common_cancel()}
	variant="warning"
	onConfirm={confirmDelete}
	onCancel={cancelDelete}
	onClose={() => (showDeleteConfirmation = false)}
/>
