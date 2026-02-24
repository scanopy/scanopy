<script lang="ts">
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { toColor } from '$lib/shared/utils/styling';
	import { entities } from '$lib/shared/stores/metadata';
	import { formatRelativeTime } from '$lib/shared/utils/formatting';
	import type { components } from '$lib/api/schema';

	type DaemonSummary = components['schemas']['DaemonSummary'];

	let {
		daemons,
		onNavigate
	}: {
		daemons: DaemonSummary[];
		onNavigate: (tab: string) => void;
	} = $props();

	const DaemonIcon = entities.getIconComponent('Daemon');
	const daemonColor = entities.getColorHelper('Daemon').icon;

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

	function hasIssue(daemon: DaemonSummary): boolean {
		return (
			daemon.is_unreachable ||
			daemon.version_status.status === 'Deprecated' ||
			daemon.version_status.status === 'Outdated'
		);
	}

	let healthyCount = $derived(daemons.filter((d) => !hasIssue(d)).length);

	let issueCount = $derived(daemons.length - healthyCount);
</script>

<section>
	<div class="mb-3 flex items-center justify-between">
		<h3 class="text-primary text-base font-semibold">Daemons</h3>
		<span class="text-tertiary text-sm">
			<span class="text-success">{healthyCount} healthy</span>{#if issueCount > 0}<span
					class="text-warning"
				>
					&middot; {issueCount} need attention</span
				>{/if}
		</span>
	</div>
	<div class="grid gap-4 sm:grid-cols-2">
		{#each daemons as daemon (daemon.id)}
			{@const statusTag = getStatusTag(daemon)}
			{@const clickable = hasIssue(daemon)}
			<div
				class="card card-static"
				class:cursor-pointer={clickable}
				class:hover:ring-1={clickable}
				class:hover:ring-gray-700={clickable}
				onclick={clickable ? () => onNavigate('daemons') : undefined}
				onkeydown={clickable
					? (e) => {
							if (e.key === 'Enter' || e.key === ' ') onNavigate('daemons');
						}
					: undefined}
				role={clickable ? 'button' : undefined}
				tabindex={clickable ? 0 : undefined}
			>
				<div class="flex items-center gap-3">
					<DaemonIcon class="h-4 w-4 flex-shrink-0" style="color: {daemonColor}" />
					<span class="text-primary text-sm font-medium">{daemon.name}</span>
				</div>
				<div class="mt-2 flex items-center gap-2">
					{#if daemon.last_seen}
						<Tag
							label="Last seen {formatRelativeTime(daemon.last_seen)}"
							color={toColor('green')}
						/>
					{/if}
					{#if statusTag}
						<Tag label={statusTag.label} color={statusTag.color} />
					{/if}
				</div>
			</div>
		{/each}
	</div>
</section>
