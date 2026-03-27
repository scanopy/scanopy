<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import { useRetryDaemonConnectionMutation } from '$lib/features/daemons/queries';
	import { entities, subnetTypes } from '$lib/shared/stores/metadata';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { ArrowBigUp, RefreshCw, Trash2 } from 'lucide-svelte';
	import { common_delete } from '$lib/paraglide/messages';
	import { getDaemonStatusTag } from '$lib/features/daemons/utils';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useApiKeysQuery } from '$lib/features/daemon_api_keys/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import type { TagProps } from '$lib/shared/components/data/types';
	import { entityRef } from '$lib/shared/components/data/types';
	import DaemonUpgradeModal from './DaemonUpgradeModal.svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import { modalState, openModal, closeModal } from '$lib/shared/stores/modal-registry';

	// Modal state — supports deep linking via ?modal=upgrade-daemon&id=<daemon-id>
	let upgradeModalOpen = $state(false);

	// Auto-open when deep-linked
	$effect(() => {
		if (
			$modalState.name === 'upgrade-daemon' &&
			$modalState.id === daemon.id &&
			!upgradeModalOpen
		) {
			upgradeModalOpen = true;
		}
	});

	function handleOpenUpgrade() {
		upgradeModalOpen = true;
		openModal('upgrade-daemon', { id: daemon.id });
	}

	function handleCloseUpgrade() {
		upgradeModalOpen = false;
		closeModal();
	}

	// Queries
	const networksQuery = useNetworksQuery();
	const retryConnectionMutation = useRetryDaemonConnectionMutation();
	// Use limit: 0 to get all hosts for daemon card lookups
	const hostsQuery = useHostsQuery({ limit: 0 });
	const subnetsQuery = useSubnetsQuery();
	const apiKeysQuery = useApiKeysQuery();
	const credentialsQuery = useCredentialsQuery();

	// Derived data
	let networksData = $derived(networksQuery.data ?? []);
	let hostsData = $derived(hostsQuery.data?.items ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let apiKeysData = $derived(apiKeysQuery.data ?? []);
	let credentialsData = $derived(credentialsQuery.data ?? []);

	let {
		daemon,
		onDelete,
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: {
		daemon: Daemon;
		onDelete?: (daemon: Daemon) => void;
		viewMode: 'card' | 'list';
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	let host = $derived(hostsData.find((h) => h.id === daemon.host_id) ?? null);
	let linkedApiKey = $derived(
		daemon.api_key_id ? apiKeysData.find((k) => k.id === daemon.api_key_id) : null
	);

	let status: TagProps = $derived(getDaemonStatusTag(daemon));

	let hasUpdateAvailable = $derived(
		daemon.version_status.status === 'Outdated' || daemon.version_status.status === 'Deprecated'
	);

	let retryPending = $derived(retryConnectionMutation.isPending);

	let upgradeButtonClass = $derived.by(() => {
		switch (daemon.version_status.status) {
			case 'Deprecated':
				return 'btn-icon-info';
			case 'Outdated':
				return 'btn-icon-info';
			default:
				return 'btn-icon';
		}
	});

	// Get version string from version_status
	let version = $derived(daemon.version_status.version ?? 'Unknown');

	// Build card data
	let cardData = $derived({
		title: daemon.name,
		iconColor: entities.getColorHelper('Daemon').icon,
		Icon: entities.getIconComponent('Daemon'),
		status,
		fields: [
			{
				label: 'Network',
				value: (() => {
					const network = networksData.find((n) => n.id == daemon.network_id);
					if (!network) return 'Unknown Network';
					return [
						{
							id: network.id,
							label: network.name,
							color: entities.getColorHelper('Network').color,
							entityRef: entityRef('Network', network.id, network, {
								credentials: credentialsData
							})
						}
					];
				})()
			},
			{
				label: 'Host',
				value: (() => {
					if (!host) return 'Unknown Host';
					return [
						{
							id: host.id,
							label: host.name,
							color: entities.getColorHelper('Host').color,
							entityRef: entityRef('Host', host.id, host)
						}
					];
				})()
			},
			{
				label: 'Version',
				value: version
			},
			{
				label: 'Last Seen',
				value: daemon.last_seen ? formatTimestamp(daemon.last_seen) : 'Never'
			},
			{
				label: 'Mode',
				value: daemon.mode
			},
			...(linkedApiKey
				? [
						{
							label: 'API Key',
							value: [
								{
									id: linkedApiKey.id,
									label: linkedApiKey.name,
									color: entities.getColorHelper('DaemonApiKey').color,
									entityRef: entityRef('DaemonApiKey', linkedApiKey.id, linkedApiKey)
								}
							]
						}
					]
				: [
						{
							label: 'API Key',
							value: daemon.mode == 'server_poll' ? 'Not Found' : 'N/A'
						}
					]),
			{
				label: 'Has Docker Socket',
				value: daemon.capabilities.has_docker_socket ? 'Yes' : 'No'
			},
			{
				label: 'Interfaces With',
				value: daemon.capabilities.interfaced_subnet_ids
					.map((s) => subnetsData.find((subnet) => subnet.id == s))
					.filter((s) => s != undefined)
					.filter((s) => !subnetTypes.getMetadata(s.subnet_type).hide_from_subnet_list)
					.map((s) => ({
						id: s.id,
						label: s.name,
						color: entities.getColorHelper('Subnet').color,
						entityRef: entityRef('Subnet', s.id, s)
					})),
				emptyText: 'No subnet interfaces'
			},
			{ label: 'Tags', snippet: tagsSnippet }
		],
		actions: [
			...(onDelete
				? [
						{
							label: common_delete(),
							icon: Trash2,
							class: 'btn-icon-danger',
							onClick: () => onDelete(daemon)
						}
					]
				: []),
			...(hasUpdateAvailable && daemon.is_unreachable !== true
				? [
						{
							label: 'Update',
							icon: ArrowBigUp,
							class: upgradeButtonClass,
							onClick: handleOpenUpgrade,
							disabled: false,
							forceLabel: true
						}
					]
				: []),
			// Show retry button for unreachable ServerPoll daemons
			...(daemon.is_unreachable === true && daemon.mode === 'server_poll'
				? [
						{
							label: 'Retry Connection',
							icon: RefreshCw,
							class: 'btn-icon-info',
							onClick: () => retryConnectionMutation.mutate(daemon.id),
							disabled: retryPending,
							forceLabel: true
						}
					]
				: [])
		]
	});
</script>

{#snippet tagsSnippet()}
	<div class="flex items-center gap-2">
		<span class="text-secondary text-sm">Tags:</span>
		<TagPickerInline selectedTagIds={daemon.tags} entityId={daemon.id} entityType="Daemon" />
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />

<DaemonUpgradeModal isOpen={upgradeModalOpen} onClose={handleCloseUpgrade} {daemon} />
