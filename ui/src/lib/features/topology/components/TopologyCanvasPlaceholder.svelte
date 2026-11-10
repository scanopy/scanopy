<script lang="ts">
	import './visualization/topology-viewer.css';
	import TopologyLoadProgress from './visualization/TopologyLoadProgress.svelte';
	import { beginLoad, LOAD_STAGES } from '../pipeline/load-progress';

	/**
	 * The canvas as it stands before there is a topology to draw: the viewer's frame and
	 * background, with the load's progress where the viewer shows it. The viewer cannot mount until
	 * the data has arrived, and a spinner in its place made the page change shape twice on the way
	 * to the graph.
	 */
	let { scope }: { scope: string } = $props();

	$effect(() => {
		beginLoad(scope, LOAD_STAGES);
	});
</script>

<div class="relative h-[calc(100vh-120px)] w-full">
	<div class="card card-static topology-canvas-placeholder h-full w-full overflow-hidden p-0!">
		<div class="topology-status-pill">
			<span class="topology-status-row"><TopologyLoadProgress /></span>
		</div>
	</div>
</div>

<style>
	/* The dotted background SvelteFlow draws for the viewer: 1px dots on a 50px grid. */
	.topology-canvas-placeholder {
		position: relative;
		background-color: var(--color-topology-bg);
		background-image: radial-gradient(
			circle,
			var(--color-topology-lod-stroke) 0.5px,
			transparent 1px
		);
		background-size: 50px 50px;
	}
</style>
