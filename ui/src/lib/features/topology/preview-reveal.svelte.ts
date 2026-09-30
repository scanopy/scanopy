import { tick } from 'svelte';
import { fromStore } from 'svelte/store';
import { useUpdateNodeInternals, type Edge } from '@xyflow/svelte';
import { previewEdges } from './queries';
import { collectEdgeHandles, previewEdgeHandlesByNode } from './interactions';

const preview = fromStore(previewEdges);

/**
 * The dependency editor's preview edges, released only once SvelteFlow has bounds for the handles
 * they name. Feed `current` into the canvas's `edges`.
 *
 * A new dependency's preview usually names a side no real edge on that node uses, so the node
 * never rendered that handle (`edgeHandlesByNode`) and SvelteFlow drops the edge with "Couldn't
 * create edge for source handle id". Publishing the handles renders their divs, but SvelteFlow
 * re-reads handle bounds from the DOM only when a node's size changes, and adding a handle does
 * not change it. So the endpoints are force-measured, and the preview is revealed on the frame
 * after that lands. A per-caller run counter discards a run the next preview has already
 * superseded.
 *
 * Call it during initialisation of a component inside the `SvelteFlowProvider` whose canvas draws
 * the edges: `useUpdateNodeInternals` measures in that provider's store only.
 *
 * Every caller writes the one global `previewEdgeHandlesByNode`. That is safe because every caller
 * derives it from the one global `previewEdges`, so each write carries the same map, and no caller
 * clears it on unmount, so the dependency tutorial closing cannot strip the main viewer's handles.
 */
export function useRevealedPreviewEdges(): { readonly current: Edge[] } {
	const updateNodeInternals = useUpdateNodeInternals();
	let revealed = $state<Edge[]>([]);
	let run = 0;
	$effect(() => {
		const edges = preview.current;
		const thisRun = ++run;
		const handles = collectEdgeHandles(edges);
		previewEdgeHandlesByNode.set(handles);
		if (edges.length === 0) {
			revealed = [];
			return;
		}
		void (async () => {
			await tick();
			if (thisRun !== run) return;
			updateNodeInternals([...handles.keys()]);
			// The hook measures on the next frame; this callback is queued after it.
			await new Promise((resolve) => requestAnimationFrame(resolve));
			if (thisRun !== run) return;
			revealed = edges;
		})();
	});
	return {
		get current() {
			return revealed;
		}
	};
}
