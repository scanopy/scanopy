<script lang="ts">
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import ProgressTrack from '$lib/shared/components/data/ProgressTrack.svelte';
	import AnimatedProgressBar from './AnimatedProgressBar.svelte';
	import { cancellingSessions } from '$lib/features/discovery/queries';
	import { entities } from '$lib/shared/stores/metadata';
	import { Loader2, X } from 'lucide-svelte';
	import type { DiscoveryUpdatePayload } from '../../types/api';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { entityRef } from '$lib/shared/components/data/types';
	import DiscoveryEstimation from '../DiscoveryEstimation.svelte';
	import {
		common_notApplicable,
		discovery_scanModeFull,
		discovery_scanModeLight
	} from '$lib/paraglide/messages';

	// Props
	let {
		viewMode,
		session,
		onCancel
	}: {
		viewMode: 'card' | 'list';
		session: DiscoveryUpdatePayload;
		onCancel?: (sessionId: string) => void;
	} = $props();

	// Queries
	const daemonsQuery = useDaemonsQuery();
	const hostsQuery = useHostsQuery({ limit: 0 });
	const subnetsQuery = useSubnetsQuery();

	// Derived data
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let hostsData = $derived(hostsQuery.data?.items ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let daemon = $derived(daemonsData.find((d) => d.id == session.daemon_id));
	let isCancelling = $derived(
		session?.session_id ? $cancellingSessions.get(session.session_id) === true : false
	);

	async function handleCancelDiscovery() {
		if (onCancel) {
			await onCancel(session.session_id);
		}
	}

	// Build card data
	let cardData = $derived({
		title: session.discovery_type.type + ' Discovery',
		iconColor: entities.getColorHelper('Discovery').icon,
		Icon: entities.getIconComponent('Discovery'),
		fields: [
			{
				label: 'Daemon',
				value: (() => {
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
				value: session.started_at ? formatTimestamp(session.started_at) : 'Not Yet'
			},
			{
				label: 'Scan Mode',
				value:
					session.discovery_type.type === 'Unified'
						? session.discovery_type.scan_settings?.is_full_scan
							? discovery_scanModeFull()
							: discovery_scanModeLight()
						: common_notApplicable()
			},
			{
				label: 'Session ID',
				value: session.session_id
			},
			{
				label: '', // No label needed for snippet
				snippet: progressSnippet
			}
		],
		actions: [
			...(onCancel
				? [
						{
							label: 'Cancel Discovery',
							icon: isCancelling ? Loader2 : X,
							class: 'btn-icon-danger',
							animation: isCancelling ? 'animate-spin' : '',
							onClick: isCancelling ? () => {} : () => handleCancelDiscovery()
						}
					]
				: [])
		]
	});
</script>

{#snippet progressSnippet()}
	<div class="flex items-center justify-between gap-3">
		<div class="flex-1 space-y-2">
			<div class="flex items-center gap-3">
				<span class={`text-secondary ${viewMode == 'list' ? 'text-xs' : 'text-sm'} font-medium`}
					>Phase:
				</span>
				<span class={`text-accent ${viewMode == 'list' ? 'text-xs' : 'text-sm'} font-medium`}
					>{isCancelling ? 'Cancelling' : session.phase}</span
				>
			</div>

			<DiscoveryEstimation
				phase={isCancelling ? 'Cancelling' : session.phase}
				hosts_discovered={session.hosts_discovered}
				estimated_remaining_secs={session.estimated_remaining_secs}
				class="mb-1"
			/>

			<div class="flex items-center gap-2">
				<ProgressTrack class="flex-1">
					<AnimatedProgressBar progress={session.progress} />
				</ProgressTrack>
				<span class="text-secondary text-xs">{session.progress}%</span>
			</div>
		</div>
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} selectable={false} />
