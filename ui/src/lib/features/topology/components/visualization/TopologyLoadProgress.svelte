<script lang="ts">
	import {
		topology_loadStageData,
		topology_loadStageDraw,
		topology_loadStageLayout,
		topology_loadStageMeasure
	} from '#lib/paraglide/messages.js';
	import AnimatedProgressBar from '#lib/shared/components/data/AnimatedProgressBar.svelte';
	import ProgressTrack from '#lib/shared/components/data/ProgressTrack.svelte';
	import { loadProgress, stageAnimation, type LoadStage } from '../../pipeline/load-progress';

	const LABELS: Record<LoadStage, () => string> = {
		data: topology_loadStageData,
		measure: topology_loadStageMeasure,
		layout: topology_loadStageLayout,
		layoutRefine: topology_loadStageLayout,
		draw: topology_loadStageDraw
	};

	let progress = $derived($loadProgress);
	let animation = $derived(
		progress ? { ...stageAnimation(progress), startedAt: progress.startedAt } : undefined
	);
</script>

{#if progress}
	<span class="load-progress">
		<span class="load-progress-label">{LABELS[progress.stage]()}</span>
		<ProgressTrack
			class="w-[140px] shrink-0"
			role="progressbar"
			aria-valuemin={0}
			aria-valuemax={100}
			aria-valuenow={Math.round(progress.from * 100)}
			aria-label={LABELS[progress.stage]()}
		>
			<AnimatedProgressBar progress={progress.from * 100} {animation} />
		</ProgressTrack>
	</span>
{/if}

<style>
	.load-progress {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.load-progress-label {
		min-width: 5.5em;
	}
</style>
