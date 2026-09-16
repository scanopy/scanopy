import { describe, it, expect } from 'vitest';
import {
	reloadInputsDiff,
	snapshotReloadInputs,
	type ReloadInputs
} from '$lib/features/topology/pipeline/reload-guard';

function inputs(overrides: Partial<ReloadInputs> = {}): ReloadInputs {
	return {
		collapsed: new Set(),
		expandedBundles: new Set(),
		expandedPorts: new Set(),
		bundleEdges: false,
		hiddenEdgeTypes: '',
		tagHidden: new Set(),
		hiddenEntities: new Set(),
		hiddenMetadata: '',
		topology: null,
		...overrides
	};
}

describe('pipeline reload guard', () => {
	/**
	 * The reported bug: "Show everything" cleared a server-side filter, the view stayed empty, and
	 * only a page refresh brought the nodes back.
	 *
	 * Clearing the filter rewrites the options at once, which starts a run against the bundle the
	 * server filtered. The bundle carrying the restored entities lands a round trip later, while
	 * that run is still going, and queues a reload. When the run ends every filter input already
	 * matches — the run consumed them — so without the topology in the comparison the reload was
	 * suppressed and the new bundle was never laid out.
	 */
	it('re-runs when the topology changed during the run, even with every filter unchanged', () => {
		const filteredBundle = { nodes: ['host-a'] };
		const restoredBundle = { nodes: ['host-a', 'port-1', 'port-2'] };

		const consumed = snapshotReloadInputs(inputs({ topology: filteredBundle }));
		const current = inputs({ topology: restoredBundle });

		expect(reloadInputsDiff(consumed, current)).toEqual(['topology']);
	});

	/** The other half of the guard's job: an identical write mid-run must still be suppressed. */
	it('suppresses a reload when nothing the run consumed has changed', () => {
		const bundle = { nodes: ['host-a'] };
		const consumed = snapshotReloadInputs(
			inputs({
				topology: bundle,
				tagHidden: new Set(['n1']),
				hiddenMetadata: 'Interface.LinkState='
			})
		);
		// Fresh sets with the same members — what a hydration re-emit produces.
		const current = inputs({
			topology: bundle,
			tagHidden: new Set(['n1']),
			hiddenMetadata: 'Interface.LinkState='
		});

		expect(reloadInputsDiff(consumed, current)).toEqual([]);
	});
});
