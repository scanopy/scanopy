<script lang="ts">
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import type { Host, HostWithAddresses } from '../types/base';
	import { hostDisplayName } from '../host-display-name';
	import { useHostPicker, useHostServices, hostDisplayContext } from '../host-picker.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import EntityDisplay from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import EntityList from '$lib/shared/components/data/EntityList.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
	import { usePortsQuery } from '$lib/features/ports/queries';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
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
	//
	// The destination picker only offers hosts on the same site as the host
	// being consolidated away, a page at a time, searched on the server.
	//
	// It is also gated on `isOpen`. This modal is rendered unconditionally by
	// HostTab, so its script runs on first paint; as an unpaginated org-wide
	// `useHostsQuery({ limit: 0 })` it therefore pulled ~1.9MB on page load for a
	// dropdown nobody had opened, and shared that key with every other consumer.
	const hostPicker = useHostPicker(() => ({
		siteId: otherHost?.site_id,
		// Also guards against `site_id: undefined` meaning "every site".
		enabled: isOpen && !!otherHost
	}));
	const otherHostServices = useHostServices(() => (isOpen && otherHost ? [otherHost.id] : []));
	const servicesQuery = useServicesCacheQuery();
	const ipAddressesQuery = useIPAddressesQuery();
	const portsQuery = usePortsQuery();

	let servicesData = $derived(servicesQuery.data ?? []);
	let ipAddressesData = $derived(ipAddressesQuery.data ?? []);
	let portsData = $derived(portsQuery.data ?? []);

	let selectedTargetHost = $state<HostWithAddresses | null>(null);
	let selectedDestinationHostId = $derived(selectedTargetHost?.id ?? '');
	let loading = $state(false);
	let showPreview = $state(false);

	// Every host on the page but the one being consolidated away. The server orders by the same
	// title the dropdown renders.
	let availableHosts = $derived(
		otherHost ? hostPicker.options.filter((host) => host.id !== otherHost.id) : []
	);

	// Build consolidation actions list
	let consolidationActions = $derived(
		(() => {
			if (!otherHost || !selectedTargetHost) return [];

			// Get children counts from query data
			const services = servicesData.filter((s) => s.host_id === otherHost.id);
			const interfaces = ipAddressesData.filter((i) => i.host_id === otherHost.id);
			const ports = portsData.filter((p) => p.host_id === otherHost.id);

			const actions = [
				{
					id: 'delete',
					name: hosts_consolidateModal_hostWillBeDeleted({ name: hostDisplayName(otherHost) })
				}
			];

			if (services.length > 0) {
				actions.push({
					id: 'services',
					name: hosts_consolidateModal_servicesMigrated({
						count: services.length,
						source: hostDisplayName(otherHost),
						destination: hostDisplayName(selectedTargetHost)
					})
				});
			}

			if (interfaces.length > 0) {
				actions.push({
					id: 'interfaces',
					name: hosts_consolidateModal_interfacesMigrated({
						count: interfaces.length,
						source: hostDisplayName(otherHost),
						destination: hostDisplayName(selectedTargetHost)
					})
				});
			}

			if (ports.length > 0) {
				actions.push({
					id: 'ports',
					name: hosts_consolidateModal_portsMigrated({
						count: ports.length,
						source: hostDisplayName(otherHost),
						destination: hostDisplayName(selectedTargetHost)
					})
				});
			}

			const credentialCount = otherHost.credential_assignments?.length ?? 0;
			if (credentialCount > 0) {
				actions.push({
					id: 'credentials',
					name: hosts_consolidateModal_credentialsMigrated({
						count: credentialCount,
						source: hostDisplayName(otherHost),
						destination: hostDisplayName(selectedTargetHost)
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
		selectedTargetHost = null;
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
		selectedTargetHost = availableHosts.find((host) => host.id === hostId) ?? null;
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
				<div class="card card-static mb-6">
					<EntityDisplay
						context={hostDisplayContext(ipAddressesData, otherHostServices.services)}
						item={otherHost}
						displayComponent={HostDisplay}
					/>
				</div>

				<!-- Target selection -->
				<div>
					<RichSelect
						label={hosts_consolidateModal_selectHost({
							hostName: otherHost ? hostDisplayName(otherHost) : ''
						})}
						placeholder={hosts_consolidateModal_chooseHost()}
						selectedValue={selectedDestinationHostId}
						selectedOption={selectedTargetHost ?? undefined}
						options={availableHosts}
						onSelect={handleHostSelect}
						showSearch={true}
						onSearchChange={hostPicker.onSearchChange}
						onLoadMore={hostPicker.onLoadMore}
						hasMore={hostPicker.hasMore}
						loading={hostPicker.loading}
						getOptionContext={() => hostPicker.context()}
						displayComponent={HostDisplay}
					/>
				</div>
			</div>
		{:else}
			<!-- Step 2: Conversion Preview -->
			<div class="space-y-4">
				<InlineWarning
					title={hosts_consolidateModal_warningTitle()}
					body={hosts_consolidateModal_warningBody()}
				/>

				<p class="text-secondary text-sm">
					{hosts_consolidateModal_previewSubtitle()}
				</p>

				<!-- Details of what will happen -->
				<EntityList title="" items={consolidationActions} />
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
