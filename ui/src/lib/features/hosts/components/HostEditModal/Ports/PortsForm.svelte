<script lang="ts">
	import { type HostFormData, type Port } from '$lib/features/hosts/types/base';
	import { ports } from '$lib/shared/stores/metadata';
	import { PortTypeDisplay } from '$lib/shared/components/forms/selection/display/PortTypeDisplay.svelte';
	import { v4 as uuidv4 } from 'uuid';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { PortDisplay } from '$lib/shared/components/forms/selection/display/PortDisplay.svelte';
	import type { Service } from '$lib/features/services/types/base';
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import PortConfigPanel from './PortConfigPanel.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import ConfirmationDialog from '$lib/shared/components/feedback/ConfirmationDialog.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import {
		common_cancel,
		common_ports,
		hosts_ports_customPort,
		hosts_ports_deleteMessage,
		hosts_ports_deleteTitle,
		hosts_ports_emptyMessage,
		hosts_ports_helpText,
		hosts_ports_noSelected,
		hosts_ports_placeholder,
		hosts_ports_selectToConfig,
		hosts_ports_wellKnownSubtitle,
		hosts_ports_wellKnownTitle
	} from '$lib/paraglide/messages';

	interface Props {
		formData: HostFormData;
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any; setFieldValue: any };
		currentServices?: Service[];
		onServicesChange?: (services: Service[]) => void;
		targetEntityId?: string | null;
	}

	let {
		formData = $bindable(),
		form,
		currentServices = [],
		onServicesChange = () => {},
		targetEntityId = $bindable(null)
	}: Props = $props();

	// Confirmation dialog state
	let showDeleteConfirmation = $state(false);
	let pendingDeleteIndex: number | null = $state(null);
	let affectedServiceNames: string[] = $state([]);

	let selectablePorts = $derived(
		ports
			.getItems()
			.filter(
				(p_type) =>
					p_type.metadata.can_be_added && !formData.ports.some((port) => port.type == p_type.id)
			)
			.sort((a, b) => a.metadata.number - b.metadata.number)
	);

	// Find services that have bindings to a specific port
	function getServicesWithBindingsToPort(portId: string): Service[] {
		return currentServices.filter((service) =>
			service.bindings.some((b) => b.type === 'Port' && b.port_id === portId)
		);
	}

	// Remove bindings to a port from all services
	function removeBindingsToPort(portId: string) {
		const updatedServices = currentServices.map((service) => ({
			...service,
			bindings: service.bindings.filter((b) => !(b.type === 'Port' && b.port_id === portId))
		}));
		onServicesChange(updatedServices);
	}

	function handleCreateNewPort() {
		const newPort: Port = {
			id: uuidv4(), // Temp ID for form - store will detect as new since it's not in ports store
			host_id: formData.id,
			network_id: formData.network_id,
			protocol: 'Tcp',
			number: Math.floor(Math.random() * 65535) + 1,
			type: 'Custom',
			created_at: new Date().toISOString(),
			updated_at: new Date().toISOString()
		};

		formData.ports = [...formData.ports, newPort];
		form.setFieldValue('ports', formData.ports);
	}

	function handleAddPort(portId: string) {
		const portType = ports.getItem(portId);

		if (portType) {
			const newPort: Port = {
				id: uuidv4(), // Temp ID for form - store will detect as new since it's not in ports store
				host_id: formData.id,
				network_id: formData.network_id,
				number: portType.metadata.number as number,
				protocol: portType.metadata.protocol,
				type: portType.id,
				created_at: new Date().toISOString(),
				updated_at: new Date().toISOString()
			};
			formData.ports = [...formData.ports, newPort];
			form.setFieldValue('ports', formData.ports);
		}
	}

	function handleRemovePort(index: number) {
		const port = formData.ports[index];
		const affectedServices = getServicesWithBindingsToPort(port.id);

		if (affectedServices.length > 0) {
			// Show confirmation dialog
			pendingDeleteIndex = index;
			affectedServiceNames = affectedServices.map((s) => s.name);
			showDeleteConfirmation = true;
		} else {
			// No bindings, delete immediately
			formData.ports = formData.ports.filter((_, i) => i !== index);
			form.setFieldValue('ports', formData.ports);
		}
	}

	function confirmDelete() {
		if (pendingDeleteIndex !== null) {
			const port = formData.ports[pendingDeleteIndex];
			// Remove bindings from services first
			removeBindingsToPort(port.id);
			// Then remove the port
			formData.ports = formData.ports.filter((_, i) => i !== pendingDeleteIndex);
			form.setFieldValue('ports', formData.ports);
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

	function handlePortChange(updatedPort: Port, index: number) {
		// Update formData.ports for real-time sync with list display and bindings
		// Note: Don't call form.setFieldValue here - the form field already updated
		// form state via field.handleChange. We only need to sync formData for display.
		const updatedPorts = [...formData.ports];
		updatedPorts[index] = updatedPort;
		formData.ports = updatedPorts;
	}
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<ListConfigEditor bind:items={formData.ports} bind:targetEntityId>
		<svelte:fragment slot="list" let:items let:onEdit let:highlightedIndex>
			<ListManager
				label={common_ports()}
				helpText={hosts_ports_helpText()}
				placeholder={hosts_ports_placeholder()}
				emptyMessage={hosts_ports_emptyMessage()}
				allowReorder={false}
				allowCreateNew={true}
				itemClickAction="edit"
				createNewLabel={hosts_ports_customPort()}
				allowDuplicates={false}
				options={selectablePorts}
				{items}
				optionDisplayComponent={PortTypeDisplay}
				itemDisplayComponent={PortDisplay}
				getItemContext={() => ({ currentServices, interfaces: formData.interfaces })}
				onCreateNew={handleCreateNewPort}
				onAdd={handleAddPort}
				onRemove={handleRemovePort}
				{onEdit}
				{highlightedIndex}
			/>
		</svelte:fragment>

		<svelte:fragment slot="config" let:selectedItem let:selectedIndex>
			{#if selectedItem && selectedItem.type == 'Custom'}
				{#key selectedItem.id}
					<PortConfigPanel
						port={selectedItem}
						index={selectedIndex}
						{form}
						onChange={(updatedPort) => handlePortChange(updatedPort, selectedIndex)}
					/>
				{/key}
			{:else if selectedItem && selectedItem.type != 'Custom'}
				<EntityConfigEmpty
					title={hosts_ports_wellKnownTitle()}
					subtitle={hosts_ports_wellKnownSubtitle()}
				/>
			{:else}
				<EntityConfigEmpty
					title={hosts_ports_noSelected()}
					subtitle={hosts_ports_selectToConfig()}
				/>
			{/if}
		</svelte:fragment>
	</ListConfigEditor>

	<EntityMetadataSection entities={formData.ports} />
</div>

<ConfirmationDialog
	isOpen={showDeleteConfirmation}
	title={hosts_ports_deleteTitle()}
	message={hosts_ports_deleteMessage()}
	details={affectedServiceNames}
	confirmLabel={hosts_ports_deleteTitle()}
	cancelLabel={common_cancel()}
	variant="warning"
	onConfirm={confirmDelete}
	onCancel={cancelDelete}
	onClose={() => (showDeleteConfirmation = false)}
/>
