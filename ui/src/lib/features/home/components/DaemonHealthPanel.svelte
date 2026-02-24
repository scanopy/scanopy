<script lang="ts">
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { toColor } from '$lib/shared/utils/styling';
	import { entities } from '$lib/shared/stores/metadata';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import type { components } from '$lib/api/schema';

	type DaemonSummary = components['schemas']['DaemonSummary'];

	let { daemons }: { daemons: DaemonSummary[] } = $props();

	const DaemonIcon = entities.getIconComponent('Daemon');

	function getStatusTag(daemon: DaemonSummary) {
		if (daemon.is_unreachable) {
			return { label: 'Unreachable', color: toColor('red') };
		}
		switch (daemon.version_status.status) {
			case 'Deprecated':
				return { label: 'Deprecated', color: toColor('red') };
			case 'Outdated':
				return { label: 'Outdated', color: toColor('yellow') };
			default:
				return null;
		}
	}

	let healthyCount = $derived(
		daemons.filter(
			(d) =>
				!d.is_unreachable &&
				d.version_status.status !== 'Deprecated' &&
				d.version_status.status !== 'Outdated'
		).length
	);

	let issueCount = $derived(daemons.length - healthyCount);
</script>

<section>
	<div class="mb-3 flex items-center justify-between">
		<h3 class="text-primary text-base font-semibold">Daemons</h3>
		<span class="text-tertiary text-sm">
			{healthyCount} healthy{#if issueCount > 0}<span class="text-warning">
					&middot; {issueCount} need attention</span
				>{/if}
		</span>
	</div>
	<div class="space-y-2">
		{#each daemons as daemon (daemon.id)}
			{@const statusTag = getStatusTag(daemon)}
			<div
				class="flex items-center justify-between rounded-lg border border-gray-700 bg-gray-800/50 px-4 py-3"
			>
				<div class="flex items-center gap-3">
					<DaemonIcon
						class="h-4 w-4 flex-shrink-0"
						style="color: {entities.getColorHelper('Daemon').icon}"
					/>
					<span class="text-primary text-sm font-medium">{daemon.name}</span>
				</div>
				<div class="flex items-center gap-3">
					{#if daemon.last_seen}
						<span class="text-tertiary text-xs">{formatTimestamp(daemon.last_seen)}</span>
					{/if}
					{#if statusTag}
						<Tag label={statusTag.label} color={statusTag.color} />
					{/if}
				</div>
			</div>
		{/each}
	</div>
</section>
