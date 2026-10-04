<script lang="ts">
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import IPAddressConfigPanel from './IPAddressConfigPanel.svelte';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { type HostFormData, type IPAddress } from '$lib/features/hosts/types/base';
	import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import { IPAddressDisplay } from '$lib/shared/components/forms/selection/display/IPAddressDisplay.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import InternetIPAddressConfigPanel from './InternetIPAddressConfigPanel.svelte';
	import { v4 as uuidv4 } from 'uuid';
	import type { Service } from '$lib/features/services/types/base';
	import ConfirmationDialog from '$lib/shared/components/feedback/ConfirmationDialog.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import {
		common_cancel,
		common_ipAddress,
		common_ipAddresses,
		common_noEntitySelected,
		hosts_ipAddresses_deleteMessage,
		hosts_ipAddresses_deleteTitle,
		hosts_ipAddresses_emptyMessage,
		hosts_ipAddresses_helpText,
		hosts_ipAddresses_placeholder,
		hosts_ipAddresses_selectToConfig
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
	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);

	// Confirmation dialog state
	let showDeleteConfirmation = $state(false);
	let pendingDeleteIndex: number | null = $state(null);
	let affectedServiceNames: string[] = $state([]);

	// Find services that have bindings to a specific interface
	function getServicesWithBindingsToInterface(interfaceId: string): Service[] {
		return currentServices.filter((service) =>
			service.bindings.some(
				(b) =>
					(b.type === 'IPAddress' && b.ip_address_id === interfaceId) ||
					(b.type === 'Port' && b.ip_address_id === interfaceId)
			)
		);
	}

	// Remove bindings to an interface from all services
	function removeBindingsToInterface(interfaceId: string) {
		const updatedServices = currentServices.map((service) => ({
			...service,
			bindings: service.bindings.filter(
				(b) =>
					!(b.type === 'IPAddress' && b.ip_address_id === interfaceId) &&
					!(b.type === 'Port' && b.ip_address_id === interfaceId)
			)
		}));
		onServicesChange(updatedServices);
	}

	// Computed values
	let interfaces = $derived(formData.ip_addresses || []);

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
			const newInterface: IPAddress = {
				id: uuidv4(), // Temp ID for form - store will detect as new since it's not in interfaces store
				host_id: formData.id,
				network_id: formData.network_id,
				name: subnet.name,
				subnet_id: subnetId,
				ip_address: '203.0.113.' + (Math.floor(Math.random() * 255) + 1).toString(),
				mac_address: undefined,
				created_at: new Date().toISOString(),
				updated_at: new Date().toISOString()
			};

			formData.ip_addresses = [...interfaces, newInterface];
			form.setFieldValue('ip_addresses', formData.ip_addresses);
		} else {
			const newInterface: IPAddress = {
				id: uuidv4(), // Temp ID for form - store will detect as new since it's not in interfaces store
				host_id: formData.id,
				network_id: formData.network_id,
				name: null,
				subnet_id: subnetId,
				ip_address: '',
				mac_address: undefined,
				created_at: new Date().toISOString(),
				updated_at: new Date().toISOString()
			};

			formData.ip_addresses = [...interfaces, newInterface];
			form.setFieldValue('ip_addresses', formData.ip_addresses);
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
			formData.ip_addresses = interfaces.filter((_, i) => i !== index);
			form.setFieldValue('ip_addresses', formData.ip_addresses);
		}
	}

	function confirmDelete() {
		if (pendingDeleteIndex !== null) {
			const iface = interfaces[pendingDeleteIndex];
			// Remove bindings from services first
			removeBindingsToInterface(iface.id);
			// Then remove the interface
			formData.ip_addresses = interfaces.filter((_, i) => i !== pendingDeleteIndex);
			form.setFieldValue('ip_addresses', formData.ip_addresses);
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

	function handleInterfaceChange(updatedInterface: IPAddress, index: number) {
		// Update formData.ip_addresses for real-time sync with list display and bindings
		// Note: Don't call form.setFieldValue here - the form field already updated
		// form state via field.handleChange. We only need to sync formData for display.
		const updatedInterfaces = [...formData.ip_addresses];
		updatedInterfaces[index] = updatedInterface;
		formData.ip_addresses = updatedInterfaces;
	}

	function handleReorder(fromIndex: number, toIndex: number) {
		if (fromIndex === toIndex) return;

		const updatedInterfaces = [...formData.ip_addresses];
		const [movedInterface] = updatedInterfaces.splice(fromIndex, 1);
		updatedInterfaces.splice(toIndex, 0, movedInterface);

		formData.ip_addresses = updatedInterfaces;
		form.setFieldValue('ip_addresses', formData.ip_addresses);
	}
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<ListConfigEditor items={formData.ip_addresses} onReorder={handleReorder} bind:targetEntityId>
		<svelte:fragment
			slot="list"
			let:items
			let:onEdit
			let:highlightedIndex
			let:onMoveUp
			let:onMoveDown
		>
			<ListManager
				label={common_ipAddresses()}
				helpText={hosts_ipAddresses_helpText()}
				placeholder={hosts_ipAddresses_placeholder()}
				emptyMessage={hosts_ipAddresses_emptyMessage()}
				allowReorder={true}
				itemClickAction="edit"
				options={availableSubnets}
				{items}
				optionDisplayComponent={SubnetDisplay}
				itemDisplayComponent={IPAddressDisplay}
				getItemContext={() => ({ subnets: subnetsData, networks: networksData })}
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
						<IPAddressConfigPanel
							{iface}
							{subnet}
							{index}
							{form}
							{isEditing}
							network={networksData.find((n) => n.id === iface.network_id)}
							onChange={(updatedInterface) => handleInterfaceChange(updatedInterface, index)}
						/>
					</div>
				{/if}
			{/each}

			<!-- Show internet interface panel only when selected (no form validation needed) -->
			{#if selectedItem && selectedSubnet && selectedSubnet.cidr === '0.0.0.0/0'}
				<InternetIPAddressConfigPanel
					iface={selectedItem}
					subnet={selectedSubnet}
					onChange={(updatedInterface) => onChange(updatedInterface)}
				/>
			{:else if !selectedItem}
				<EntityConfigEmpty
					title={common_noEntitySelected({ entity: common_ipAddress() })}
					subtitle={hosts_ipAddresses_selectToConfig()}
				/>
			{/if}
		</svelte:fragment>
	</ListConfigEditor>

	<EntityMetadataSection entities={formData.ip_addresses} />
</div>

<ConfirmationDialog
	isOpen={showDeleteConfirmation}
	title={hosts_ipAddresses_deleteTitle()}
	message={hosts_ipAddresses_deleteMessage()}
	details={affectedServiceNames}
	confirmLabel={hosts_ipAddresses_deleteTitle()}
	cancelLabel={common_cancel()}
	variant="warning"
	onConfirm={confirmDelete}
	onCancel={cancelDelete}
	onClose={() => (showDeleteConfirmation = false)}
/>
