<script lang="ts">
	import {
		topology_loadStageData,
		topology_loadStageDraw,
		topology_loadStageLayout,
		topology_loadStageMeasure
	} from '#lib/paraglide/messages.js';
	import { loadProgress, type LoadStage } from '../../pipeline/load-progress';

	const LABELS: Record<LoadStage, () => string> = {
		data: topology_loadStageData,
		measure: topology_loadStageMeasure,
		layout: topology_loadStageLayout,
		layoutRefine: topology_loadStageLayout,
		draw: topology_loadStageDraw
	};

	// The fill stops short of the end of its slice until the stage ends, so a stage that overruns
	// its expected time shows as a bar waiting at the line, never as one that crossed it.
	const HELD_BACK = 0.08;

	let progress = $derived($loadProgress);
	let target = $derived(progress ? progress.to - (progress.to - progress.from) * HELD_BACK : 0);
</script>

{#if progress}
	<span class="load-progress">
		<span class="load-progress-label">{LABELS[progress.stage]()}</span>
		<span
			class="load-progress-track"
			role="progressbar"
			aria-valuemin={0}
			aria-valuemax={100}
			aria-valuenow={Math.round(progress.from * 100)}
			aria-label={LABELS[progress.stage]()}
		>
			<span
				class="load-progress-fill"
				style:transform="scaleX({target})"
				style:transition-duration="{Math.round(progress.expectedMs)}ms"
			></span>
		</span>
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

	.load-progress-track {
		display: block;
		width: 140px;
		height: 4px;
		overflow: hidden;
		border-radius: 9999px;
		background: var(--color-topology-lod-container);
	}

	/*
	 * Moved with `transform` and nothing else. A layout pass holds the main thread for seconds, and
	 * a transform transition on a promoted element is the one animation the compositor runs without
	 * it. A width transition would stop where the layout started.
	 */
	.load-progress-fill {
		display: block;
		width: 100%;
		height: 100%;
		border-radius: 9999px;
		background: #3b82f6;
		transform-origin: left center;
		transition-property: transform;
		transition-timing-function: cubic-bezier(0.2, 0.6, 0.4, 1);
		will-change: transform;
	}
</style>
