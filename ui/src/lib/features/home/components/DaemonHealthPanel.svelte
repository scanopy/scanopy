<script lang="ts">
	import type { Daemon } from '$lib/features/daemons/types/base';
	import HomeDaemonDisplay from './HomeDaemonDisplay.svelte';

	let {
		daemons,
		onNavigate
	}: {
		daemons: Daemon[];
		onNavigate: (tab: string) => void;
	} = $props();

	function hasIssue(daemon: Daemon): boolean {
		return (
			daemon.standby === true ||
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
			<span class="text-success">{healthyCount} healthy</span>
			{#if issueCount > 0}
				<span class="text-warning">&middot; {issueCount} need attention</span>
			{/if}
		</span>
	</div>
	<div class="grid gap-4 sm:grid-cols-2">
		{#each daemons as daemon (daemon.id)}
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
				<HomeDaemonDisplay item={daemon} />
			</div>
		{/each}
	</div>
</section>
