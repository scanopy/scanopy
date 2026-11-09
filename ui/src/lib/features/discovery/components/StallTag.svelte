<script lang="ts">
	import { Clock } from '@lucide/svelte';
	import { sessionStall } from '$lib/features/discovery/utils/staleness';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { toColor } from '$lib/shared/utils/styling';
	import {
		discovery_noProgressTooltip,
		discovery_noUpdatesMinutes,
		discovery_noUpdatesTooltip
	} from '$lib/paraglide/messages';

	interface Props {
		session_id: string;
		phase: string;
		/** A cancel in flight explains the silence, so the tag stays out of the way until it ends. */
		cancelling?: boolean;
	}

	let { session_id, phase, cancelling = false }: Props = $props();

	let stall = $derived(cancelling ? null : $sessionStall(session_id, phase));
</script>

{#if stall !== null}
	<Tag
		icon={Clock}
		color={toColor('amber')}
		label={discovery_noUpdatesMinutes({ minutes: stall.minutes })}
		title={stall.source === 'activity'
			? discovery_noProgressTooltip({ minutes: stall.minutes })
			: discovery_noUpdatesTooltip({ minutes: stall.minutes })}
	/>
{/if}
