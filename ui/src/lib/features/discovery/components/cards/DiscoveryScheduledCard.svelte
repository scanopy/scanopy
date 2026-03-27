<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import { Edit, Play, Power, Trash2 } from 'lucide-svelte';
	import type { Discovery } from '../../types/base';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { formatScheduleDisplay } from '../../queries';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import type { TagProps } from '$lib/shared/components/data/types';
	import { common_delete, common_legacy } from '$lib/paraglide/messages';

	// Queries
	const daemonsQuery = useDaemonsQuery();
	const networksQuery = useNetworksQuery();
	const hostsQuery = useHostsQuery({ limit: 0 });
	const subnetsQuery = useSubnetsQuery();
	const credentialsQuery = useCredentialsQuery();

	// Derived data
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let hostsData = $derived(hostsQuery.data?.items ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let credentialsData = $derived(credentialsQuery.data ?? []);

	let {
		viewMode,
		discovery,
		onEdit,
		onDelete,
		onRun,
		onToggleEnabled,
		selected,
		onSelectionChange = () => {}
	}: {
		viewMode: 'card' | 'list';
		discovery: Discovery;
		onEdit?: (discovery: Discovery) => void;
		onDelete?: (discovery: Discovery) => void;
		onRun?: (discovery: Discovery) => void;
		onToggleEnabled?: (discovery: Discovery) => void;
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	let isEnabled = $derived(discovery.run_type.type === 'Scheduled' && discovery.run_type.enabled);

	let legacyStatus: TagProps | null = $derived(
		discovery.discovery_type.type !== 'Unified' ? { label: common_legacy(), color: 'Yellow' } : null
	);

	let cardData = $derived({
		title: discovery.name,
		status: legacyStatus,
		iconColor: entities.getColorHelper('Discovery').icon,
		Icon: entities.getIconComponent('Discovery'),
		fields: [
			{
				label: 'Daemon',
				value: (() => {
					const daemon = daemonsData.find((d) => d.id == discovery.daemon_id);
					if (!daemon) return 'Unknown Daemon';
					return [
						{
							id: daemon.id,
							label: daemon.name,
							color: entities.getColorHelper('Daemon').color,
							entityRef: entityRef('Daemon', daemon.id, daemon, {
								hosts: hostsData,
								subnets: subnetsData
							})
						}
					];
				})()
			},
			{
				label: 'Network',
				value: (() => {
					const network = networksData.find((n) => n.id == discovery.network_id);
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
				label: 'Schedule',
				value:
					discovery.run_type.type == 'Scheduled'
						? formatScheduleDisplay(discovery.run_type.cron_schedule, discovery.run_type.timezone)
						: 'Manual'
			},
			{
				label: 'Last Run',
				value:
					discovery.run_type.type != 'Historical' && discovery.run_type.last_run
						? formatTimestamp(discovery.run_type.last_run)
						: 'Never'
			},
			{ label: 'Tags', snippet: tagsSnippet }
		],
		actions: [
			...(onDelete
				? [
						{
							label: common_delete(),
							icon: Trash2,
							class: `btn-icon`,
							onClick: () => onDelete(discovery)
						}
					]
				: []),
			...(onToggleEnabled
				? [
						{
							label: isEnabled ? 'Disable' : 'Enable',
							icon: Power,
							class: isEnabled ? `btn-icon-success` : `btn-icon`,
							onClick: () => onToggleEnabled(discovery)
						}
					]
				: []),
			...(onRun
				? [{ label: 'Run', icon: Play, class: `btn-icon`, onClick: () => onRun(discovery) }]
				: []),
			...(onEdit
				? [{ label: 'Edit', icon: Edit, class: `btn-icon`, onClick: () => onEdit(discovery) }]
				: [])
		]
	});
</script>

{#snippet tagsSnippet()}
	<div class="flex items-center gap-2">
		<span class="text-secondary text-sm">Tags:</span>
		<TagPickerInline
			selectedTagIds={discovery.tags}
			entityId={discovery.id}
			entityType="Discovery"
		/>
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
