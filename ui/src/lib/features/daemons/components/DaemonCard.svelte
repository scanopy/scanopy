<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import {
		getDaemonIsRunningDiscovery,
		useRetryDaemonConnectionMutation
	} from '$lib/features/daemons/queries';
	import { useActiveSessionsQuery } from '$lib/features/discovery/queries';
	import { concepts, entities } from '$lib/shared/stores/metadata';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { toColor } from '$lib/shared/utils/styling';
	import { ArrowBigUp, RefreshCw, Trash2 } from 'lucide-svelte';
	import { getDaemonStatusTag } from '$lib/features/daemons/utils';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useApiKeysQuery } from '$lib/features/daemon_api_keys/queries';
	import type { TagProps } from '$lib/shared/components/data/types';
	import DaemonUpgradeModal from './DaemonUpgradeModal.svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';

	// Modal state
	let upgradeModalOpen = $state(false);

	// Queries
	const networksQuery = useNetworksQuery();
	const retryConnectionMutation = useRetryDaemonConnectionMutation();
	// Use limit: 0 to get all hosts for daemon card lookups
	const hostsQuery = useHostsQuery({ limit: 0 });
	const subnetsQuery = useSubnetsQuery();
	const sessionsQuery = useActiveSessionsQuery();
	const apiKeysQuery = useApiKeysQuery();

	// Derived data
	let networksData = $derived(networksQuery.data ?? []);
	let hostsData = $derived(hostsQuery.data?.items ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let sessionsData = $derived(sessionsQuery.data ?? []);
	let apiKeysData = $derived(apiKeysQuery.data ?? []);

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
	let daemonIsRunningDiscovery = $derived(getDaemonIsRunningDiscovery(daemon.id, sessionsData));
	let linkedApiKey = $derived(
		daemon.api_key_id ? apiKeysData.find((k) => k.id === daemon.api_key_id) : null
	);

	let status: TagProps | null = $derived(getDaemonStatusTag(daemon));

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
				value: networksData.find((n) => n.id == daemon.network_id)?.name || 'Unknown Network'
			},
			{
				label: 'Host',
				value: host ? host.name : 'Unknown Host'
			},
			{
				label: 'Version',
				value: version
			},
			{
				label: 'Created',
				value: formatTimestamp(daemon.created_at)
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
							value: linkedApiKey.name
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
				value: [
					daemon.capabilities.has_docker_socket
						? {
								id: daemon.id,
								label: 'True',
								color: concepts.getColorHelper('Virtualization').color
							}
						: {
								id: daemon.id,
								label: 'False',
								color: toColor('gray')
							}
				]
			},
			{
				label: 'Interfaces With',
				value:
					daemon.capabilities.interfaced_subnet_ids.length > 0
						? daemon.capabilities.interfaced_subnet_ids
								.map((s) => subnetsData.find((subnet) => subnet.id == s))
								.filter((s) => s != undefined)
								.map((s) => {
									return {
										id: s.id,
										label: s.name,
										color: entities.getColorHelper('Subnet').color
									};
								})
						: [
								{
									id: daemon.id,
									label: 'No subnet interfaces',
									color: toColor('gray')
								}
							],
				emptyText: 'No subnet interfaces'
			},
			{ label: 'Tags', snippet: tagsSnippet }
		],
		actions: [
			...(onDelete
				? [
						{
							label: 'Delete',
							icon: Trash2,
							class: 'btn-icon-danger',
							onClick: () => onDelete(daemon),
							disabled: daemonIsRunningDiscovery
						}
					]
				: []),
			...(hasUpdateAvailable
				? [
						{
							label: 'Update',
							icon: ArrowBigUp,
							class: upgradeButtonClass,
							onClick: () => (upgradeModalOpen = true),
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

<DaemonUpgradeModal isOpen={upgradeModalOpen} onClose={() => (upgradeModalOpen = false)} {daemon} />
