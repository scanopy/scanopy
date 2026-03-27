<script lang="ts">
	import { untrack } from 'svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { validateForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Service } from '../types/base';
	import ServiceConfigPanel from '$lib/features/hosts/components/HostEditModal/Services/ServiceConfigPanel.svelte';
	import type { Host, HostFormData } from '$lib/features/hosts/types/base';
	import { useInterfacesQuery } from '$lib/features/interfaces/queries';
	import { usePortsQuery } from '$lib/features/ports/queries';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
	import {
		common_cancel,
		common_delete,
		common_deleting,
		common_updating,
		services_updateService
	} from '$lib/paraglide/messages';

	// TanStack Query hooks to get child entities for hydrating host form data
	const interfacesQuery = useInterfacesQuery();
	const portsQuery = usePortsQuery();
	const servicesQuery = useServicesCacheQuery();
	let interfacesData = $derived(interfacesQuery.data ?? []);
	let portsData = $derived(portsQuery.data ?? []);
	let servicesData = $derived(servicesQuery.data ?? []);

	// Hydrate host to form data for ServiceConfigPanel
	function hydrateHostToFormData(host: Host): HostFormData {
		const hostInterfaces = interfacesData.filter((i) => i.host_id === host.id);
		const hostPorts = portsData.filter((p) => p.host_id === host.id);
		const hostServices = servicesData.filter((s) => s.host_id === host.id);

		return {
			...host,
			interfaces: hostInterfaces,
			ports: hostPorts,
			services: hostServices,
			// SNMP fields - spread from host, default to null if not present
			sys_descr: host.sys_descr ?? null,
			sys_object_id: host.sys_object_id ?? null,
			sys_location: host.sys_location ?? null,
			sys_contact: host.sys_contact ?? null,
			management_url: host.management_url ?? null,
			chassis_id: host.chassis_id ?? null,
			credential_assignments: host.credential_assignments ?? [],
			if_entries: [] // IfEntries not available in this context
		};
	}

	interface Props {
		service: Service;
		host: Host;
		isOpen?: boolean;
		onUpdate: (id: string, data: Service) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
	}

	let {
		service,
		host,
		isOpen = false,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined
	}: Props = $props();

	let loading = $state(false);
	let deleting = $state(false);
	let formData = $state(untrack(() => service));

	// Hydrate host to form data for ServiceConfigPanel
	let hostFormData = $derived(hydrateHostToFormData(host));

	let title = $derived(`Edit ${service.name}`);

	// TanStack Form for validation
	let form = createForm(() => ({
		defaultValues: {
			services: [formData]
		},
		onSubmit: async () => {
			// Actual submission handled by handleSubmit
		}
	}));

	function handleOpen() {
		formData = { ...service };
		form.reset();
	}

	async function handleSubmit() {
		// Validate form first
		const isValid = await validateForm(form);
		if (!isValid) return;

		// Clean up the data before sending
		const serviceData: Service = {
			...formData,
			name: formData.name.trim()
		};

		loading = true;
		try {
			await onUpdate(service.id, serviceData);
			onClose();
		} finally {
			loading = false;
		}
	}

	async function handleDelete() {
		if (onDelete && service) {
			deleting = true;
			try {
				await onDelete(service.id);
			} finally {
				deleting = false;
			}
		}
	}

	function handleServiceUpdate(updatedService: Service) {
		formData = { ...updatedService };
	}
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={service?.id}
	{onClose}
	onOpen={handleOpen}
	size="xl"
>
	{#snippet headerIcon()}
		<ModalHeaderIcon
			Icon={serviceDefinitions.getIconComponent(service.service_definition)}
			color={serviceDefinitions.getColorHelper(service.service_definition).color}
		/>
	{/snippet}

	<!-- Content -->
	<div class="flex h-full flex-col overflow-hidden">
		<div class="min-h-0 flex-1 overflow-y-auto">
			<div class="space-y-8 p-6">
				<ServiceConfigPanel
					host={hostFormData}
					service={formData}
					{form}
					index={0}
					onChange={handleServiceUpdate}
				/>
			</div>
		</div>
		<EntityMetadataSection entities={[service]} />
	</div>

	{#snippet footer()}
		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if onDelete}
						<button
							type="button"
							disabled={deleting || loading}
							onclick={handleDelete}
							class="btn-danger"
						>
							{deleting ? common_deleting() : common_delete()}
						</button>
					{/if}
				</div>
				<div class="flex items-center gap-3">
					<button type="button" onclick={onClose} class="btn-secondary">
						{common_cancel()}
					</button>
					<button
						type="button"
						onclick={handleSubmit}
						disabled={loading || deleting}
						class="btn-primary"
					>
						{loading ? common_updating() : services_updateService()}
					</button>
				</div>
			</div>
		</div>
	{/snippet}
</GenericModal>
