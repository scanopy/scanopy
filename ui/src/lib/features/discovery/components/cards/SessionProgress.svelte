<script lang="ts">
	import AnimatedProgressBar from './AnimatedProgressBar.svelte';
	import StallTag from '../StallTag.svelte';
	import ProgressTrack from '$lib/shared/components/data/ProgressTrack.svelte';
	import { sessionStallMinutes } from '$lib/features/discovery/utils/staleness';

	interface Props {
		session_id: string;
		/** The session's own phase, not a display overlay such as the cancelling label. */
		phase: string;
		progress: number;
		cancelling?: boolean;
	}

	let { session_id, phase, progress, cancelling = false }: Props = $props();

	// The bar and the tag read one value, so a dimmed bar always comes with the tag that says why.
	let stalled = $derived(!cancelling && $sessionStallMinutes(session_id, phase) !== null);
</script>

<div class="flex items-center gap-2">
	<ProgressTrack class="flex-1">
		<AnimatedProgressBar {progress} {stalled} />
	</ProgressTrack>
	<span class="text-secondary text-xs">{progress}%</span>
	<StallTag {session_id} {phase} {cancelling} />
</div>
