<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import { toColor } from '$lib/shared/utils/styling';
	import { Info } from 'lucide-svelte';
	import type { Discovery } from '../../types/base';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { formatDuration, formatTimestamp } from '$lib/shared/utils/formatting';
	import type { TagProps } from '$lib/shared/components/data/types';
	import { entityRef } from '$lib/shared/components/data/types';

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
		onView = () => {},
		selected,
		onSelectionChange = () => {}
	}: {
		viewMode: 'card' | 'list';
		discovery: Discovery;
		onView?: (discovery: Discovery) => void;
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	let results = $derived(
		discovery.run_type.type == 'Historical' ? discovery.run_type.results : null
	);

	let status = $derived.by((): TagProps | null => {
		const phase = results?.phase ?? null;
		if (!phase) return null;
		switch (phase) {
			case 'Complete':
				return { label: 'Complete', color: toColor('green') };
			case 'Failed':
				return { label: 'Failed', color: toColor('red') };
			case 'Cancelled':
				return { label: 'Cancelled', color: toColor('yellow') };
			default:
				return { label: phase, color: toColor('blue') };
		}
	});

	let cardData = $derived({
		title: discovery.name,
		iconColor: entities.getColorHelper('Discovery').icon,
		Icon: entities.getIconComponent('Discovery'),
		status,
		fields: [
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
				label: 'Started',
				value: results && results.started_at ? formatTimestamp(results.started_at) : 'Unknown'
			},
			{
				label: 'Finished',
				value: results && results.finished_at ? formatTimestamp(results.finished_at) : 'Unknown'
			},
			{
				label: 'Duration',
				value: (() => {
					const results =
						discovery.run_type.type == 'Historical' ? discovery.run_type.results : null;
					if (results && results.finished_at && results.started_at) {
						return formatDuration(results.started_at, results.finished_at);
					}
					return 'Unknown';
				})()
			}
		],
		actions: [
			{
				label: 'Details',
				icon: Info,
				class: `btn-icon`,
				onClick: () => onView(discovery)
			}
		]
	});
</script>

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
