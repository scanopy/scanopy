<script lang="ts">
	import RichSelect from '#lib/shared/components/forms/selection/RichSelect.svelte';
	import type { Host, HostWithAddresses } from '../types/base';
	import { hostDisplayName } from '../host-display-name';
	import { useHostPicker, useHostServices, hostDisplayContext } from '../host-picker.svelte';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import EntityDisplay from '#lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { HostDisplay } from '#lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import InlineWarning from '#lib/shared/components/feedback/InlineWarning.svelte';
	import EntityList from '#lib/shared/components/data/EntityList.svelte';
	import { entities } from '#lib/shared/stores/metadata.ts';
	import ModalHeaderIcon from '#lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { useServicesCacheQuery } from '#lib/features/services/queries.ts';
	import { usePortsQuery } from '#lib/features/ports/queries.ts';
	import { useIPAddressesQuery } from '#lib/features/ip-addresses/queries.ts';
	import {
		common_back,
		common_cancel,
		common_merging,
		common_next,
		hosts_mergeModal_chooseHost,
		hosts_mergeModal_credentialsMigrated,
		hosts_mergeModal_hostWillBeDeleted,
		hosts_mergeModal_addressesMigrated,
		hosts_mergeModal_portsMigrated,
		hosts_mergeModal_previewSubtitle,
		hosts_mergeModal_selectHost,
		hosts_mergeModal_servicesMigrated,
		hosts_mergeModal_title,
		hosts_mergeModal_warningBody,
		hosts_mergeModal_warningTitle
	} from '#lib/paraglide/messages.js';

	interface Props {
		otherHost?: Host | null;
		isOpen?: boolean;
		onMerge: (otherHostId: string, destinationHostId: string) => Promise<void> | void;
		onClose: () => void;
	}

	let { otherHost = null, isOpen = false, onMerge, onClose }: Props = $props();

	// TanStack Query hooks
	//
	// The destination picker only offers hosts on the same site as the host
	// being merged away, a page at a time, searched on the server.
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

	// Every host on the page but the one being merged away. The server orders by the same
	// title the dropdown renders.
	let availableHosts = $derived(
		otherHost ? hostPicker.options.filter((host) => host.id !== otherHost.id) : []
	);

	// What the merge moves, then the deletion
	let mergeActions = $derived(
		(() => {
			if (!otherHost || !selectedTargetHost) return [];

			// Get children counts from query data
			const services = servicesData.filter((s) => s.host_id === otherHost.id);
			const addresses = ipAddressesData.filter((i) => i.host_id === otherHost.id);
			const ports = portsData.filter((p) => p.host_id === otherHost.id);

			const actions: { id: string; name: string }[] = [];

			if (services.length > 0) {
				actions.push({
					id: 'services',
					name: hosts_mergeModal_servicesMigrated({
						count: services.length,
						source: hostDisplayName(otherHost),
						destination: hostDisplayName(selectedTargetHost)
					})
				});
			}

			if (addresses.length > 0) {
				actions.push({
					id: 'addresses',
					name: hosts_mergeModal_addressesMigrated({
						count: addresses.length,
						source: hostDisplayName(otherHost),
						destination: hostDisplayName(selectedTargetHost)
					})
				});
			}

			if (ports.length > 0) {
				actions.push({
					id: 'ports',
					name: hosts_mergeModal_portsMigrated({
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
					name: hosts_mergeModal_credentialsMigrated({
						count: credentialCount,
						source: hostDisplayName(otherHost),
						destination: hostDisplayName(selectedTargetHost)
					})
				});
			}

			actions.push({
				id: 'delete',
				name: hosts_mergeModal_hostWillBeDeleted({ name: hostDisplayName(otherHost) })
			});

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

	async function handleMerge() {
		if (!otherHost || !selectedDestinationHostId) return;

		loading = true;
		try {
			await onMerge(selectedDestinationHostId, otherHost.id);
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

	let actions = $derived<ModalAction[]>([
		...(showPreview
			? [
					{
						role: 'secondary' as const,
						label: common_back(),
						onClick: handleBack,
						disabled: loading
					}
				]
			: []),
		{ role: 'cancel', label: common_cancel(), onClick: handleClose, disabled: loading },
		!showPreview
			? {
					role: 'primary',
					label: common_next(),
					onClick: handleTargetSelection,
					disabled: !selectedDestinationHostId
				}
			: {
					role: 'primary',
					tone: 'danger',
					label: loading ? common_merging() : hosts_mergeModal_title(),
					onClick: handleMerge,
					disabled: loading || !selectedDestinationHostId
				}
	]);
</script>

<GenericModal
	{isOpen}
	title={hosts_mergeModal_title()}
	size="lg"
	onClose={handleClose}
	preventCloseOnClickOutside={loading}
	{actions}
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
				<div class="card-static card mb-6">
					<EntityDisplay
						context={hostDisplayContext(ipAddressesData, otherHostServices.services)}
						item={otherHost}
						displayComponent={HostDisplay}
					/>
				</div>

				<!-- Target selection -->
				<div>
					<RichSelect
						label={hosts_mergeModal_selectHost({
							hostName: otherHost ? hostDisplayName(otherHost) : ''
						})}
						placeholder={hosts_mergeModal_chooseHost()}
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
					title={hosts_mergeModal_warningTitle()}
					body={otherHost && selectedTargetHost
						? hosts_mergeModal_warningBody({
								source: hostDisplayName(otherHost),
								destination: hostDisplayName(selectedTargetHost)
							})
						: ''}
				/>

				<p class="text-sm text-secondary">
					{hosts_mergeModal_previewSubtitle()}
				</p>

				<!-- Details of what will happen -->
				<EntityList title="" items={mergeActions} />
			</div>
		{/if}
	</div>
</GenericModal>
