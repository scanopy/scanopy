<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm, validateForm } from '$lib/shared/components/forms/form-context';
	import { Info, ArrowRight } from 'lucide-svelte';
	import type {
		Host,
		HostFormData,
		IfEntry,
		CreateHostWithServicesRequest,
		UpdateHostWithServicesRequest
	} from '$lib/features/hosts/types/base';
	import {
		formDataToHostPrimitive,
		hydrateHostToFormData,
		createEmptyHostFormData
	} from '$lib/features/hosts/queries';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { queryKeys } from '$lib/api/query-client';
	import DetailsForm from './Details/HostDetailsForm.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import InterfacesForm from './Interfaces/InterfacesForm.svelte';
	import ServicesForm from './Services/ServicesForm.svelte';
	import { concepts, entities, serviceDefinitions } from '$lib/shared/stores/metadata';
	import type { Service } from '$lib/features/services/types/base';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import PortsForm from './Ports/PortsForm.svelte';
	import VirtualizationForm from './Virtualization/VirtualizationForm.svelte';
	import SnmpForm from './Snmp/SnmpForm.svelte';
	import IfEntriesForm from './IfEntries/IfEntriesForm.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import { pushError } from '$lib/shared/stores/feedback';
	import {
		common_back,
		common_cancel,
		common_delete,
		common_deleting,
		common_details,
		common_editName,
		common_ifEntries,
		common_interfaces,
		common_next,
		common_ports,
		common_saving,
		common_serviceConfiguration,
		common_services,
		common_virtualization,
		hosts_createHost,
		hosts_editor_basicInfo,
		hosts_editor_interfacesDesc,
		hosts_editor_snmpTabDesc,
		hosts_editor_updateHost,
		hosts_editor_virtualizationDesc,
		hosts_failedToSave,
		hosts_ifEntries_subtitle
	} from '$lib/paraglide/messages';

	interface Props {
		host?: Host | null;
		isOpen?: boolean;
		onCreate: (data: CreateHostWithServicesRequest) => Promise<void> | void;
		onUpdate: (data: UpdateHostWithServicesRequest) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((host: Host) => Promise<void> | void) | null;
		name?: string;
	}

	let {
		host = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined
	}: Props = $props();

	// TanStack Query hooks
	const queryClient = useQueryClient();
	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);
	let defaultNetworkId = $derived(networksData[0]?.id ?? '');

	// Subscribe to ifEntries cache for this host - reactive to cache updates
	// Since ifEntries are read-only (populated by SNMP discovery), we bypass formData
	// and read directly from the cache for both tab visibility and form data
	const ifEntriesQuery = createQuery(() => ({
		queryKey: [...queryKeys.ifEntries.all, 'forHost', host?.id ?? 'none'],
		queryFn: () => {
			const allIfEntries = queryClient.getQueryData<IfEntry[]>(queryKeys.ifEntries.all) ?? [];
			return allIfEntries.filter((e) => e.host_id === host?.id);
		},
		enabled: !!host && isOpen,
		// Check cache frequently since we're reading from another query's cache
		staleTime: 1000,
		refetchInterval: 2000 // Poll while modal is open
	}));

	let hostIfEntries = $derived(ifEntriesQuery.data ?? []);

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(host !== null);
	let title = $derived(
		isEditing ? common_editName({ name: host?.name ?? '' }) : hosts_createHost()
	);

	// formData holds structural data (ids, network_id, tags, etc.)
	// Form fields (name, hostname, description, interface IPs, port numbers) are synced at submission
	let formData = $state<HostFormData>(createEmptyHostFormData());

	// Sync form field values to formData structure
	function syncFormValuesToFormData(values: typeof form.state.values) {
		formData.name = values.name;
		formData.hostname = values.hostname;
		formData.description = values.description;

		// Sync interface field values (ip, mac, name) to formData.interfaces
		if (values.interfaces && formData.interfaces) {
			for (let i = 0; i < formData.interfaces.length && i < values.interfaces.length; i++) {
				const formIface = values.interfaces[i];
				const dataIface = formData.interfaces[i];
				if (formIface && dataIface) {
					dataIface.ip_address = formIface.ip_address ?? '';
					dataIface.mac_address = formIface.mac_address ?? null;
					dataIface.name = formIface.name ?? null;
				}
			}
		}

		// Sync port field values (number, protocol) to formData.ports
		if (values.ports && formData.ports) {
			for (let i = 0; i < formData.ports.length && i < values.ports.length; i++) {
				const formPort = values.ports[i];
				const dataPort = formData.ports[i];
				if (formPort && dataPort) {
					dataPort.number = formPort.number;
					dataPort.protocol = formPort.protocol;
				}
			}
		}
	}

	// Perform the actual submission - defined as a separate function so it
	// has access to latest state (TanStack Form's onSubmit captures state at creation time)
	async function performSubmission(value: typeof form.state.values) {
		// Sync form values to formData structure
		syncFormValuesToFormData(value);

		loading = true;
		try {
			if (isEditing) {
				// Update existing host
				const hostPrimitive = formDataToHostPrimitive(formData);
				const promises = [
					onUpdate({
						host: hostPrimitive,
						interfaces: formData.interfaces,
						ports: formData.ports,
						services: formData.services
					})
				];

				// VM managed hosts - only update host fields, not children
				for (const updatedHost of vmManagedHostUpdates.values()) {
					promises.push(
						onUpdate({
							host: updatedHost,
							interfaces: null,
							ports: null,
							services: null
						})
					);
				}

				await Promise.all(promises);
				handleClose();
			} else {
				// Create and close
				await onCreate({ host: formData, services: formData.services });
				handleClose();
			}
		} catch (error) {
			pushError(error instanceof Error ? error.message : hosts_failedToSave());
		} finally {
			loading = false;
		}
	}

	// TanStack Form - onSubmit delegates to performSubmission for access to latest state
	// Note: We intentionally do NOT recreate the form on modal open. Reassigning `form` causes
	// Field components to remain bound to the old form (TanStack Form is not reactive to Svelte).
	// Instead, we use form.reset() which properly resets values, validation, and touched state.
	// Fields unmount when modal closes ({#if isOpen} in GenericModal) and re-register on open.
	let form = createForm(() => ({
		defaultValues: {
			name: formData.name,
			hostname: formData.hostname || '',
			description: formData.description || '',
			interfaces: formData.interfaces || [],
			ports: formData.ports || [],
			services: formData.services || [],
			credential_mode: ((formData.credential_assignments?.length ?? 0) > 0
				? 'override'
				: 'default') as 'default' | 'override'
		},
		onSubmit: async ({ value }) => {
			await performSubmission(value);
		}
	}));

	// Initialize form data when host changes or modal opens
	let modalInitialized = $state(false);
	function handleOpen() {
		modalInitialized = true;
		lastHostId = host?.id ?? null;
		resetForm();
	}

	async function handleDelete() {
		if (onDelete && host) {
			deleting = true;
			try {
				await onDelete(host);
			} finally {
				deleting = false;
			}
		}
	}

	// Track host ID to detect when host changes WHILE modal is already open
	// Initial open is handled by handleOpen(), so we check modalInitialized.
	let lastHostId = $state<string | null>(null);
	$effect(() => {
		const currentHostId = host?.id ?? null;
		// Only reset if modal was already initialized and host changed
		if (isOpen && modalInitialized && currentHostId !== lastHostId) {
			const currentTab = activeTab;
			resetForm();
			activeTab = currentTab; // Preserve tab position
			lastHostId = currentHostId;
		}
		// Reset flag when modal closes
		if (!isOpen) {
			modalInitialized = false;
		}
	});

	let vmManagerServices = $derived(
		(formData.services || []).filter(
			(s) => serviceDefinitions.getMetadata(s.service_definition).manages_virtualization != null
		)
	);

	function handleVirtualizationServiceChange(updatedService: Service) {
		// Find the actual index in formData.services
		const actualIndex = formData.services.findIndex((s) => s.id === updatedService.id);
		if (actualIndex >= 0) {
			const updatedServices = [...formData.services];
			updatedServices[actualIndex] = updatedService;
			formData.services = updatedServices;
		}
	}

	let vmManagedHostUpdates: SvelteMap<string, Host> = new SvelteMap();

	function handleVirtualizationHostChange(updatedHost: Host) {
		// This is another host; ie not the current
		// Hold on to updates and only make them on submit
		vmManagedHostUpdates.set(updatedHost.id, updatedHost);
	}

	// Tab management
	let activeTab = $state('details');
	let furthestReached = $state(0);

	// Sub-entity deep link: when navigating to a specific interface/port/ifEntry
	let pendingSubEntityId = $state<string | null>(null);

	function handleSubEntityNavigation(subEntityId: string) {
		pendingSubEntityId = subEntityId;
	}
	// Get network for passing to SNMP form
	let currentNetwork = $derived(networksData.find((n) => n.id === formData.network_id) ?? null);

	let tabs = $derived([
		{
			id: 'details',
			label: common_details(),
			icon: Info,
			description: hosts_editor_basicInfo()
		},
		{
			id: 'snmp',
			label: 'Credentials',
			icon: entities.getIconComponent('Credential'),
			description: hosts_editor_snmpTabDesc(),
			disabled: !isEditing && furthestReached < 1
		},
		...(isEditing
			? [
					{
						id: 'if-entries',
						label: common_ifEntries(),
						icon: entities.getIconComponent('IfEntry'),
						description: hosts_ifEntries_subtitle()
					}
				]
			: []),
		{
			id: 'interfaces',
			label: common_interfaces(),
			icon: entities.getIconComponent('Interface'),
			description: hosts_editor_interfacesDesc(),
			disabled: !isEditing && furthestReached < 2
		},
		{
			id: 'ports',
			label: common_ports(),
			icon: entities.getIconComponent('Port'),
			description: common_serviceConfiguration(),
			disabled: !isEditing && furthestReached < 3
		},
		{
			id: 'services',
			label: common_services(),
			icon: entities.getIconComponent('Service'),
			description: common_serviceConfiguration(),
			disabled: !isEditing && furthestReached < 4
		},
		...(isEditing
			? [
					{
						id: 'virtualization',
						label: common_virtualization(),
						icon: concepts.getIconComponent('Virtualization'),
						description: hosts_editor_virtualizationDesc()
					}
				]
			: [])
	]);

	let enabledTabs = $derived(tabs.filter((t) => !t.disabled));
	let currentEnabledIndex = $derived(enabledTabs.findIndex((t) => t.id === activeTab));

	function nextTab() {
		if (currentEnabledIndex < enabledTabs.length - 1) {
			activeTab = enabledTabs[currentEnabledIndex + 1].id;
		}
	}

	function previousTab() {
		if (currentEnabledIndex > 0) {
			activeTab = enabledTabs[currentEnabledIndex - 1].id;
		}
	}

	function resetForm() {
		// Hydrate host to HostFormData for form editing (includes interfaces, ports, services)
		formData = host
			? hydrateHostToFormData(host, queryClient)
			: createEmptyHostFormData(defaultNetworkId);

		// Sort services by position
		if (formData.services) {
			formData.services = formData.services.sort((a, b) => (a.position ?? 0) - (b.position ?? 0));
		}

		// Reset TanStack form
		form.reset({
			name: formData.name,
			hostname: formData.hostname || '',
			description: formData.description || '',
			interfaces: formData.interfaces || [],
			ports: formData.ports || [],
			services: formData.services || [],
			credential_mode: (formData.credential_assignments?.length ?? 0) > 0 ? 'override' : 'default'
		});

		activeTab = 'details'; // Reset to first tab
		furthestReached = 0;
	}

	// Wizard steps for progressive unlock in create mode
	const wizardSteps = ['details', 'snmp', 'interfaces', 'ports', 'services'];
	let isLastWizardStep = $derived(activeTab === wizardSteps[wizardSteps.length - 1]);

	// Handle form-based submission for create flow with steps
	async function handleFormSubmit() {
		if (isEditing || isLastWizardStep) {
			await submitForm(form);
		} else {
			// Validate all fields before advancing to next tab
			const isValid = await validateForm(form);
			if (isValid) {
				const wizardIndex = wizardSteps.indexOf(activeTab);
				if (wizardIndex >= 0 && wizardIndex + 1 > furthestReached) {
					furthestReached = wizardIndex + 1;
				}
				nextTab();
			}
		}
	}

	function handleClose() {
		activeTab = 'details';
		furthestReached = 0;
		onClose();
	}

	function handleFormCancel() {
		if (isEditing || currentEnabledIndex === 0) {
			handleClose();
		} else {
			previousTab();
		}
	}

	// Dynamic labels based on create/edit mode and tab position
	let saveLabel = $derived(
		isEditing ? hosts_editor_updateHost() : isLastWizardStep ? hosts_createHost() : common_next()
	);
	let cancelLabel = $derived(isEditing ? common_cancel() : common_back());
	let showCancel = $derived(isEditing ? true : currentEnabledIndex !== 0);
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={host?.id}
	onClose={handleClose}
	onOpen={handleOpen}
	onSubEntityNavigation={handleSubEntityNavigation}
	size="full"
	showCloseButton={true}
	{tabs}
	{activeTab}
	tabStyle={isEditing ? 'tabs' : 'stepper'}
	onTabChange={(tabId) => (activeTab = tabId)}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon
			Icon={entities.getIconComponent('Host')}
			color={entities.getColorString('Host')}
		/>
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleFormSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<!-- Content -->
		<div class="min-h-0 flex-1 overflow-hidden">
			<!-- Details Tab -->
			{#if activeTab === 'details'}
				<div class="flex h-full flex-col">
					<div class="min-h-0 flex-1 overflow-y-auto">
						<DetailsForm {form} bind:formData {isEditing} />
					</div>
					{#if isEditing && host}
						<EntityMetadataSection entities={[host]} />
					{/if}
				</div>
			{/if}

			<!-- Interfaces Tab -->
			{#if activeTab === 'interfaces'}
				<div class="flex h-full flex-col">
					<InterfacesForm
						bind:formData
						{form}
						{isEditing}
						currentServices={formData.services}
						onServicesChange={(services) => (formData.services = services)}
						bind:targetEntityId={pendingSubEntityId}
					/>
				</div>
			{/if}

			<!-- Ports Tab -->
			{#if activeTab === 'ports'}
				<div class="flex h-full flex-col">
					<PortsForm
						bind:formData
						{form}
						currentServices={formData.services}
						onServicesChange={(services) => (formData.services = services)}
						bind:targetEntityId={pendingSubEntityId}
					/>
				</div>
			{/if}

			<!-- Services Tab -->
			{#if activeTab === 'services'}
				<div class="flex h-full flex-col">
					<ServicesForm bind:formData {form} />
				</div>
			{/if}

			<!-- Virtualization Tab -->
			{#if activeTab === 'virtualization'}
				<div class="flex h-full flex-col">
					<VirtualizationForm
						virtualizationManagerServices={vmManagerServices}
						onServiceChange={handleVirtualizationServiceChange}
						onVirtualizedHostChange={handleVirtualizationHostChange}
					/>
				</div>
			{/if}

			<!-- SNMP Tab -->
			{#if activeTab === 'snmp'}
				<div class="flex h-full flex-col">
					<SnmpForm bind:formData network={currentNetwork} />
				</div>
			{/if}

			<!-- IfEntries Tab -->
			{#if activeTab === 'if-entries'}
				<div class="flex h-full flex-col">
					<IfEntriesForm ifEntries={hostIfEntries} bind:targetEntityId={pendingSubEntityId} />
				</div>
			{/if}
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if isEditing && onDelete}
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
					{#if showCancel}
						<button
							type="button"
							disabled={loading || deleting}
							onclick={handleFormCancel}
							class="btn-secondary"
						>
							{cancelLabel}
						</button>
					{/if}
					<button
						type="submit"
						disabled={loading || deleting}
						class="btn-primary {!isEditing && !isLastWizardStep ? 'btn-primary-lg' : ''}"
					>
						{loading ? common_saving() : saveLabel}
						{#if !isEditing && !isLastWizardStep}
							<ArrowRight class="h-4 w-4" />
						{/if}
					</button>
				</div>
			</div>
		</div>
	</form>
</GenericModal>
