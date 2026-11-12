<script lang="ts">
	import { formatEstimatedRemaining } from '#lib/features/discovery/utils/estimation.ts';
	import DocsHint from '#lib/shared/components/feedback/DocsHint.svelte';
	import { discoveryPhases } from '#lib/shared/stores/metadata.ts';
	import {
		discovery_cancellationWait,
		discovery_foundHostsEstimating,
		discovery_foundHostsRemaining,
		discovery_fullScanRunning,
		discovery_scanningForHosts,
		home_docsDiscoveryTakesLong,
		home_docsDiscoveryTakesLongLinkText
	} from '#lib/paraglide/messages.js';

	interface Props {
		phase: string;
		hosts_discovered?: number | null;
		estimated_remaining_secs?: number | null;
		/** The running session scans all 65,535 ports, which explains a long estimate. */
		is_full_scan?: boolean;
		class?: string;
	}

	let {
		phase,
		hosts_discovered,
		estimated_remaining_secs,
		is_full_scan = false,
		class: className = ''
	}: Props = $props();

	let text = $derived.by(() => {
		// Cancelling: frontend-only overlay during cancel mutation (no backend variant).
		if (phase === 'Cancelling') return discovery_cancellationWait();
		// Scanning: dynamic host count + estimated remaining.
		if (phase === 'Scanning') {
			if (!hosts_discovered) return discovery_scanningForHosts();
			if (estimated_remaining_secs != null)
				return discovery_foundHostsRemaining({
					count: hosts_discovered,
					remaining: formatEstimatedRemaining(estimated_remaining_secs)
				});
			return discovery_foundHostsEstimating({ count: hosts_discovered });
		}
		// All other backend DiscoveryPhase variants source from metadata fixture.
		return discoveryPhases.getDescription(phase) || null;
	});
</script>

{#if text}
	<div class={className}>
		<p class="text-xs text-secondary">{text}</p>
		{#if is_full_scan && phase === 'Scanning'}
			<p class="text-xs text-tertiary">{discovery_fullScanRunning()}</p>
		{/if}
		{#if phase === 'Scanning' && estimated_remaining_secs != null && estimated_remaining_secs > 3600}
			<DocsHint
				text={home_docsDiscoveryTakesLong()}
				href="https://scanopy.net/docs/using-scanopy/discovery/scans/#discovery-duration"
				linkText={home_docsDiscoveryTakesLongLinkText()}
				class="mt-0.5"
			/>
		{/if}
	</div>
{/if}
