<script lang="ts" module>
	/** One animation of the fill, from `fromScale` to `toScale` of the track, started at `startedAt`. */
	export interface FillAnimation {
		fromScale: number;
		toScale: number;
		durationMs: number;
		easing: string;
		/** `performance.now()` when the animation began. A remounted bar resumes from there. */
		startedAt: number;
	}
</script>

<script lang="ts">
	/**
	 * `stalled` stops the shimmer and dims the bar, so it stops presenting a silent scan as busy.
	 * `animation`, when set, drives the fill instead of `progress`.
	 */
	let {
		progress,
		stalled = false,
		animation
	}: { progress: number; stalled?: boolean; animation?: FillAnimation } = $props();

	let fill: HTMLDivElement | undefined = $state();
	let running: Animation | undefined;

	function scaleOf(element: HTMLElement): number {
		const transform = getComputedStyle(element).transform;
		return transform && transform !== 'none' ? new DOMMatrixReadOnly(transform).a : 0;
	}

	$effect(() => {
		if (!fill || !animation || typeof fill.animate !== 'function') return;
		const { toScale, durationMs, easing, startedAt } = animation;
		// The bar only moves forward: a new animation that would start behind the fill as it
		// stands (a run re-planned mid-stage) starts from the fill instead.
		const shown = running ? scaleOf(fill) : 0;
		const fromScale = Math.min(Math.max(animation.fromScale, shown), toScale);
		running?.cancel();
		running = fill.animate(
			[{ transform: `scaleX(${fromScale})` }, { transform: `scaleX(${toScale})` }],
			{ duration: durationMs, easing, fill: 'forwards' }
		);
		if (fromScale === animation.fromScale) {
			running.currentTime = Math.max(0, performance.now() - startedAt);
		}
	});

	$effect(() => () => running?.cancel());
</script>

<div
	bind:this={fill}
	class="progress-bar relative h-full w-full overflow-hidden rounded-full bg-blue-500"
	class:progress-bar-eased={!animation}
	class:opacity-40={stalled}
	style:transform="scaleX({animation ? animation.fromScale : progress / 100})"
>
	{#if !stalled}
		<div class="progress-shimmer absolute inset-0"></div>
	{/if}
</div>

<style>
	/*
	 * The fill and the shimmer move with `transform` and nothing else. A long script (a topology
	 * layout pass holds the main thread for seconds) stops every JS-driven or `width` animation,
	 * and a transform animation on a promoted element is the one the compositor keeps running.
	 */
	.progress-bar {
		transform-origin: left center;
		will-change: transform;
	}

	/* `cubicOut` over 500 ms, the scan bar's motion between progress updates. */
	.progress-bar-eased {
		transition: transform 500ms cubic-bezier(0.33, 1, 0.68, 1);
	}

	.progress-shimmer {
		background: linear-gradient(
			90deg,
			transparent 0%,
			rgba(255, 255, 255, 0.15) 50%,
			transparent 100%
		);
		animation: shimmer 1.5s infinite;
	}

	@keyframes shimmer {
		0% {
			transform: translateX(-100%);
		}
		100% {
			transform: translateX(100%);
		}
	}
</style>
