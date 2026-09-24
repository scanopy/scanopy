<script lang="ts">
	import { Clock } from 'lucide-svelte';
	import { sessionStallMinutes } from '$lib/features/discovery/utils/staleness';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { toColor } from '$lib/shared/utils/styling';
	import { discovery_noUpdatesMinutes, discovery_noUpdatesTooltip } from '$lib/paraglide/messages';

	interface Props {
		session_id: string;
		phase: string;
		/** A cancel in flight explains the silence, so the tag stays out of the way until it ends. */
		cancelling?: boolean;
	}

	let { session_id, phase, cancelling = false }: Props = $props();

	let minutes = $derived(cancelling ? null : $sessionStallMinutes(session_id, phase));
</script>

{#if minutes !== null}
	<Tag
		icon={Clock}
		color={toColor('amber')}
		label={discovery_noUpdatesMinutes({ minutes })}
		title={discovery_noUpdatesTooltip({ minutes })}
	/>
{/if}
