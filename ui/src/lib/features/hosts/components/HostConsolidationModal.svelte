<script lang="ts">
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import type { Host } from '../types/base';
	import { useHostsQuery } from '../queries';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import EntityDisplay from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import EntityList from '$lib/shared/components/data/EntityList.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
	import { useInterfacesQuery } from '$lib/features/interfaces/queries';
	import { usePortsQuery } from '$lib/features/ports/queries';
	import {
		common_back,
		common_cancel,
		common_consolidating,
		common_next,
		hosts_consolidateModal_chooseHost,
		hosts_consolidateModal_credentialsMigrated,
		hosts_consolidateModal_hostWillBeDeleted,
		hosts_consolidateModal_interfacesMigrated,
		hosts_consolidateModal_portsMigrated,
		hosts_consolidateModal_previewSubtitle,
		hosts_consolidateModal_previewTitle,
		hosts_consolidateModal_selectHost,
		hosts_consolidateModal_servicesMigrated,
		hosts_consolidateModal_title,
		hosts_consolidateModal_warningBody,
		hosts_consolidateModal_warningTitle
	} from '$lib/paraglide/messages';

	interface Props {
		otherHost?: Host | null;
		isOpen?: boolean;
		onConsolidate: (otherHostId: string, destinationHostId: string) => Promise<void> | void;
		onClose: () => void;
	}

	let { otherHost = null, isOpen = false, onConsolidate, onClose }: Props = $props();

	// TanStack Query hooks
	// Use limit: 0 to get all hosts for consolidation modal dropdown
	const hostsQuery = useHostsQuery({ limit: 0 });
	const servicesQuery = useServicesCacheQuery();
	const interfacesQuery = useInterfacesQuery();
	const portsQuery = usePortsQuery();

	let hostsData = $derived(hostsQuery.data?.items ?? []);
	let servicesData = $derived(servicesQuery.data ?? []);
	let interfacesData = $derived(interfacesQuery.data ?? []);
	let portsData = $derived(portsQuery.data ?? []);

	let selectedDestinationHostId = $state('');
	let loading = $state(false);
	let showPreview = $state(false);

	// Get available hosts (excluding the host being consolidated away)
	let availableHosts = $derived(
		(otherHost
			? hostsData
					.filter((host) => host.id !== otherHost.id)
					.filter((host) => host.network_id == otherHost.network_id)
			: []
		).sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()))
	);

	// Get the selected target host
	let selectedTargetHost = $derived(
		selectedDestinationHostId
			? hostsData.find((host) => host.id === selectedDestinationHostId)
			: null
	);

	// Build consolidation actions list
	let consolidationActions = $derived(
		(() => {
			if (!otherHost || !selectedTargetHost) return [];

			// Get children counts from query data
			const services = servicesData.filter((s) => s.host_id === otherHost.id);
			const interfaces = interfacesData.filter((i) => i.host_id === otherHost.id);
			const ports = portsData.filter((p) => p.host_id === otherHost.id);

			const actions = [
				{
					id: 'delete',
					name: hosts_consolidateModal_hostWillBeDeleted({ name: otherHost.name })
				}
			];

			if (services.length > 0) {
				actions.push({
					id: 'services',
					name: hosts_consolidateModal_servicesMigrated({
						count: services.length,
						source: otherHost.name,
						destination: selectedTargetHost.name
					})
				});
			}

			if (interfaces.length > 0) {
				actions.push({
					id: 'interfaces',
					name: hosts_consolidateModal_interfacesMigrated({
						count: interfaces.length,
						source: otherHost.name,
						destination: selectedTargetHost.name
					})
				});
			}

			if (ports.length > 0) {
				actions.push({
					id: 'ports',
					name: hosts_consolidateModal_portsMigrated({
						count: ports.length,
						source: otherHost.name,
						destination: selectedTargetHost.name
					})
				});
			}

			const credentialCount = otherHost.credential_assignments?.length ?? 0;
			if (credentialCount > 0) {
				actions.push({
					id: 'credentials',
					name: hosts_consolidateModal_credentialsMigrated({
						count: credentialCount,
						source: otherHost.name,
						destination: selectedTargetHost.name
					})
				});
			}

			return actions;
		})()
	);

	// Reset when modal opens/closes
	$effect(() => {
		if (isOpen && otherHost) {
			resetForm();
		}
	});

	function resetForm() {
		selectedDestinationHostId = '';
		showPreview = false;
		loading = false;
	}

	function handleTargetSelection() {
		if (selectedDestinationHostId) {
			showPreview = true;
		}
	}

	function handleBack() {
		showPreview = false;
	}

	async function handleConsolidate() {
		if (!otherHost || !selectedDestinationHostId) return;

		loading = true;
		try {
			await onConsolidate(selectedDestinationHostId, otherHost.id);
			onClose();
		} finally {
			loading = false;
		}
	}

	function handleClose() {
		if (!loading) {
			onClose();
		}
	}

	function handleHostSelect(hostId: string) {
		selectedDestinationHostId = hostId;
	}
</script>

<GenericModal
	{isOpen}
	title={hosts_consolidateModal_title()}
	size="lg"
	onClose={handleClose}
	preventCloseOnClickOutside={loading}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon
			Icon={entities.getIconComponent('Host')}
			color={entities.getColorHelper('Host').color}
		/>
	{/snippet}

	<!-- Main content -->
	<div class="p-6">
		{#if !showPreview}
			<!-- Step 1: Target Selection -->
			<div>
				<!-- Source host info -->
				<div class="card mb-6">
					<EntityDisplay
						context={{
							services: servicesData.filter((s) => (otherHost ? s.host_id == otherHost.id : false))
						}}
						item={otherHost}
						displayComponent={HostDisplay}
					/>
				</div>

				<!-- Target selection -->
				<div>
					<RichSelect
						label={hosts_consolidateModal_selectHost({ hostName: otherHost?.name ?? '' })}
						placeholder={hosts_consolidateModal_chooseHost()}
						selectedValue={selectedDestinationHostId}
						options={availableHosts}
						onSelect={handleHostSelect}
						showSearch={true}
						getOptionContext={(option) => ({
							services: servicesData.filter((s) => s.host_id == option.id)
						})}
						displayComponent={HostDisplay}
					/>
				</div>
			</div>
		{:else}
			<!-- Step 2: Conversion Preview -->
			<div>
				<div class="mb-6 text-center">
					<h3 class="text-primary mb-2 text-lg font-medium">
						{hosts_consolidateModal_previewTitle()}
					</h3>
					<p class="text-secondary text-sm">
						{hosts_consolidateModal_previewSubtitle()}
					</p>
				</div>

				<!-- Details of what will happen -->
				<EntityList title="" items={consolidationActions} />

				<!-- Warning -->
				<div class="mt-4">
					<InlineWarning
						title={hosts_consolidateModal_warningTitle()}
						body={hosts_consolidateModal_warningBody()}
					/>
				</div>
			</div>
		{/if}
	</div>

	{#snippet footer()}
		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					<!-- Empty space for alignment -->
				</div>

				<div class="flex items-center gap-3">
					{#if showPreview}
						<button type="button" disabled={loading} onclick={handleBack} class="btn-secondary">
							{common_back()}
						</button>
					{/if}

					<button type="button" disabled={loading} onclick={handleClose} class="btn-secondary">
						{common_cancel()}
					</button>

					{#if !showPreview}
						<button
							type="button"
							disabled={!selectedDestinationHostId}
							onclick={handleTargetSelection}
							class="btn-primary"
						>
							{common_next()}
						</button>
					{:else}
						<button
							type="button"
							disabled={loading || !selectedDestinationHostId}
							onclick={handleConsolidate}
							class="btn-danger"
						>
							{loading ? common_consolidating() : hosts_consolidateModal_title()}
						</button>
					{/if}
				</div>
			</div>
		</div>
	{/snippet}
</GenericModal>
