<script lang="ts">
	/**
	 * A running scan's phase, estimate and progress bar: the Scans tab's
	 * progress cell, and anywhere else a scan's progress shows the same way.
	 */
	import SessionProgress from './SessionProgress.svelte';
	import DiscoveryEstimation from '../DiscoveryEstimation.svelte';
	import { cancellingSessions } from '../../queries';
	import { isFullScan } from '../../utils/fullScan';
	import type { DiscoveryUpdatePayload } from '../../types/api';
	import { common_cancelling, common_phase } from '#lib/paraglide/messages.js';

	let { session }: { session: DiscoveryUpdatePayload } = $props();

	let isCancelling = $derived($cancellingSessions.get(session.session_id) === true);
	let phase = $derived(isCancelling ? common_cancelling() : session.phase);
</script>

<div class="min-w-0 space-y-2">
	<div class="flex flex-wrap items-center gap-x-2">
		<span class="text-sm font-medium text-secondary">{common_phase()}:</span>
		<span class="text-sm font-medium text-accent">{phase}</span>
	</div>

	<DiscoveryEstimation
		{phase}
		hosts_discovered={session.hosts_discovered}
		estimated_remaining_secs={session.estimated_remaining_secs}
		is_full_scan={isFullScan(session.discovery_type)}
	/>

	<SessionProgress
		session_id={session.session_id}
		phase={session.phase}
		progress={session.progress}
		cancelling={isCancelling}
	/>
</div>
