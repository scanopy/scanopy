<script lang="ts">
	import { untrack } from 'svelte';
	import { tweened } from 'svelte/motion';
	import { cubicOut } from 'svelte/easing';

	/** `stalled` stops the shimmer and dims the bar, so it stops presenting a silent scan as busy. */
	let { progress, stalled = false }: { progress: number; stalled?: boolean } = $props();

	const animatedProgress = tweened(
		untrack(() => progress),
		{
			duration: 500,
			easing: cubicOut
		}
	);

	$effect(() => {
		animatedProgress.set(progress);
	});
</script>

<div
	class="progress-bar relative h-full overflow-hidden rounded-full bg-blue-500"
	class:opacity-40={stalled}
	style="width: {$animatedProgress}%"
>
	{#if !stalled}
		<div class="progress-shimmer absolute inset-0"></div>
	{/if}
</div>

<style>
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
