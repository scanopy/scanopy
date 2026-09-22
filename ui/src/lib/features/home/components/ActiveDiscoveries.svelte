<script lang="ts">
	import SessionProgress from '$lib/features/discovery/components/cards/SessionProgress.svelte';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import type { DiscoveryUpdatePayload } from '$lib/features/discovery/types/api';
	import DiscoveryEstimation from '$lib/features/discovery/components/DiscoveryEstimation.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import { entities } from '$lib/shared/stores/metadata';

	let {
		sessions,
		onNavigate
	}: {
		sessions: DiscoveryUpdatePayload[];
		onNavigate: () => void;
	} = $props();

	const daemonsQuery = useDaemonsQuery();
	let daemons = $derived(daemonsQuery.data ?? []);

	// Only show sessions in Scanning phase
	let scanningSessions = $derived(sessions.filter((s) => s.phase === 'Scanning'));
</script>

{#if scanningSessions.length > 0}
	<section>
		<h3 class="text-primary mb-3 text-base font-semibold">Active Discoveries</h3>
		<div class="grid grid-cols-[repeat(auto-fill,minmax(360px,1fr))] gap-4">
			{#each scanningSessions as session (session.session_id)}
				{@const daemon = daemons.find((d) => d.id == session.daemon_id)}
				<div
					class="card card-static cursor-pointer"
					onclick={onNavigate}
					onkeydown={(e) => {
						if (e.key === 'Enter' || e.key === ' ') onNavigate();
					}}
					role="button"
					tabindex={0}
				>
					<div class="mb-2 flex items-center justify-between">
						<span class="text-primary text-sm font-medium">
							{session.discovery_type.type} Discovery
						</span>
						<EntityTag
							entityRef={{
								entityId: session.daemon_id,
								entityType: 'Daemon',
								data: daemon
							}}
							label={daemon?.name ?? 'Unknown Daemon'}
							color={entities.getColorHelper('Daemon').color}
						/>
					</div>
					<DiscoveryEstimation
						phase={session.phase}
						hosts_discovered={session.hosts_discovered}
						estimated_remaining_secs={session.estimated_remaining_secs}
						class="mb-1"
					/>
					<SessionProgress
						session_id={session.session_id}
						phase={session.phase}
						progress={session.progress}
					/>
				</div>
			{/each}
		</div>
	</section>
{/if}
