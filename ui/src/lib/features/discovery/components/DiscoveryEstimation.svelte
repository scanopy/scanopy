<script lang="ts">
	import { Clock } from 'lucide-svelte';
	import { formatEstimatedRemaining } from '$lib/features/discovery/utils/estimation';
	import { sessionStallMinutes } from '$lib/features/discovery/utils/staleness';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { discoveryPhases } from '$lib/shared/stores/metadata';
	import { toColor } from '$lib/shared/utils/styling';
	import {
		discovery_cancellationWait,
		discovery_foundHostsEstimating,
		discovery_foundHostsRemaining,
		discovery_noUpdatesFor,
		discovery_scanningForHosts,
		home_docsDiscoveryTakesLong,
		home_docsDiscoveryTakesLongLinkText
	} from '$lib/paraglide/messages';

	interface Props {
		phase: string;
		/** Enables the no-updates tag, which tracks the session's stream messages. */
		session_id?: string;
		hosts_discovered?: number | null;
		estimated_remaining_secs?: number | null;
		class?: string;
	}

	let {
		phase,
		session_id,
		hosts_discovered,
		estimated_remaining_secs,
		class: className = ''
	}: Props = $props();

	let stallMinutes = $derived(session_id ? $sessionStallMinutes(session_id, phase) : null);

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

{#if text || stallMinutes !== null}
	<div class={className}>
		{#if stallMinutes !== null}
			<Tag
				icon={Clock}
				color={toColor('amber')}
				label={discovery_noUpdatesFor({ minutes: stallMinutes })}
			/>
		{/if}
		{#if text}
			<p class="text-secondary text-xs">{text}</p>
		{/if}
		{#if phase === 'Scanning' && estimated_remaining_secs != null && estimated_remaining_secs > 3600}
			<DocsHint
				text={home_docsDiscoveryTakesLong()}
				href="https://scanopy.net/docs/setting-up-daemons/troubleshooting-scans/scan-performance/#discovery-takes-hours"
				linkText={home_docsDiscoveryTakesLongLinkText()}
				class="mt-0.5"
			/>
		{/if}
	</div>
{/if}
