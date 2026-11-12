/**
 * Checks that the element shape key actually predicts card height.
 *
 * Sampling the measure pass rests on one assumption: element nodes sharing a
 * shape key measure to the same height. If that is ever false, nodes are laid
 * out with a wrong height and the graph overlaps — with no error, because
 * nothing threw. This turns that silent failure into a reported one.
 *
 * It runs against a *full* measurement (every node mounted and measured), so it
 * validates the key without depending on sampling being correct. That makes it
 * usable both as a pre-flight check before sampling is trusted, and afterwards
 * as a regression guard when someone changes how cards render.
 *
 * Enabled by `window.__topoVerifyShapes`; off by default, since it needs the
 * expensive full measurement it exists to justify replacing.
 */

import { browser } from '$app/env';
import type { RenderableTopology, TopologyNode } from '../types/base';
import type { XY } from './types';
import {
	buildElementRender,
	currentElementRenderContext,
	elementShapeKey
} from '../element-render-data';

/**
 * Heights must agree exactly.
 *
 * `offsetHeight` is an integer, so any difference is a real structural
 * difference in the card — not measurement noise. Tolerating a few pixels
 * would mean tolerating a cause we haven't identified, and the whole point of
 * this check is that unexplained height variation is where silent layout
 * corruption comes from.
 */
const HEIGHT_TOLERANCE_PX = 0;

/**
 * Fill sizes for elements the cache missed, using a measured card of the same shape key.
 *
 * Returns how many are *still* missing — the caller falls back to a full measurement only if
 * that is non-zero.
 *
 * The shape key exists precisely so card height need not be measured per card: cards sharing a
 * key measure to the same height, which [`verifyShapeKeys`] is here to keep true. That makes it
 * the right source for a card that was never mounted — as happens when containers start
 * collapsed at scale and one is then expanded. The alternative, re-measuring the whole graph,
 * means mounting every card to learn sizes most of them already have, which is the cold-load
 * cost the sampling was introduced to avoid.
 */
export function fillMissingSizesByShapeKey(
	visibleNodes: TopologyNode[],
	topology: RenderableTopology,
	measured: Map<string, XY>,
	/**
	 * Ids of elements no shape key could resolve, collected when supplied.
	 *
	 * The count alone only supports "give up and re-measure everything", which at 19,095 nodes is a
	 * ~665MB full pass to learn a handful of sizes. The ids let the caller measure just those.
	 */
	unresolvedIds?: Set<string>
): number {
	const context = currentElementRenderContext();
	const keyOf = (node: TopologyNode): string | null => {
		try {
			return elementShapeKey(buildElementRender({ nodeId: node.id, node, topology, ...context }));
		} catch {
			return null;
		}
	};

	// Only elements: a container's size comes from its own cache or from ELK's minimum.
	const elements = visibleNodes.filter((n) => n.node_type === 'Element');
	const sizeByKey = new Map<string, XY>();
	for (const node of elements) {
		const size = measured.get(node.id);
		if (!size) continue;
		const key = keyOf(node);
		if (key && !sizeByKey.has(key)) sizeByKey.set(key, size);
	}

	let stillMissing = 0;
	// One id per unresolved key, not every unresolved node.
	//
	// The caller measures what this reports, and cards sharing a shape key measure identically —
	// that is the whole premise of the key. Reporting all of them made a cold cache ask for the
	// entire graph, which is a full measurement pass by another name: 746MB and 5.5s at 19,095
	// nodes, where a handful of representatives would do. The caller re-runs this fill afterwards to
	// spread each measured size across its key.
	const representativeFor = new Set<string>();
	for (const node of elements) {
		if (measured.has(node.id)) continue;
		const key = keyOf(node);
		const known = key ? sizeByKey.get(key) : undefined;
		if (known) {
			measured.set(node.id, known);
		} else {
			stillMissing++;
			// A node with no shape key at all cannot be spoken for by anything else, so it is always
			// its own representative.
			if (key === null || !representativeFor.has(key)) {
				if (key !== null) representativeFor.add(key);
				unresolvedIds?.add(node.id);
			}
		}
	}
	return stillMissing;
}

export interface ShapeKeyDisagreement {
	shapeKey: string;
	/** Distinct measured heights seen for this key, with an example node each. */
	heights: { height: number; nodeId: string; count: number }[];
}

export interface ShapeVerifyReport {
	elementsChecked: number;
	distinctKeys: number;
	disagreements: ShapeKeyDisagreement[];
	/** Nodes whose key could not be computed; these must be measured individually. */
	unkeyedNodeIds: string[];
}

export function shapeVerifyEnabled(): boolean {
	if (!browser) return false;
	return (window as unknown as { __topoVerifyShapes?: boolean }).__topoVerifyShapes === true;
}

/**
 * Group measured element nodes by shape key and report keys that map to more
 * than one height.
 */
export function verifyShapeKeys(
	visibleNodes: TopologyNode[],
	topology: RenderableTopology,
	measured: Map<string, XY>
): ShapeVerifyReport {
	const context = currentElementRenderContext();
	// key -> exact height -> { count, exampleNodeId }
	const byKey = new Map<string, Map<number, { count: number; nodeId: string }>>();
	const unkeyedNodeIds: string[] = [];
	let elementsChecked = 0;

	for (const node of visibleNodes) {
		if (node.node_type !== 'Element') continue;
		const size = measured.get(node.id);
		if (!size) continue;

		let key: string;
		try {
			key = elementShapeKey(buildElementRender({ nodeId: node.id, node, topology, ...context }));
		} catch {
			unkeyedNodeIds.push(node.id);
			continue;
		}

		elementsChecked++;
		const heights = byKey.get(key) ?? new Map<number, { count: number; nodeId: string }>();
		const existing = heights.get(size.y);
		if (existing) existing.count++;
		else heights.set(size.y, { count: 1, nodeId: node.id });
		byKey.set(key, heights);
	}

	const disagreements: ShapeKeyDisagreement[] = [];
	for (const [key, heights] of byKey) {
		if (heights.size <= 1) continue;
		// Compare the spread, not adjacent buckets: bucketing would split two
		// heights one pixel apart whenever they straddled a boundary.
		const observed = [...heights.keys()];
		const spread = Math.max(...observed) - Math.min(...observed);
		if (spread <= HEIGHT_TOLERANCE_PX) continue;
		disagreements.push({
			shapeKey: key,
			heights: [...heights.entries()]
				.map(([height, v]) => ({ height, nodeId: v.nodeId, count: v.count }))
				.sort((a, b) => b.count - a.count)
		});
	}

	return { elementsChecked, distinctKeys: byKey.size, disagreements, unkeyedNodeIds };
}

/**
 * Run the check and surface the result. Exposed on `window` so the Playwright
 * harness can assert on it rather than scraping console output.
 */
export function reportShapeVerification(
	visibleNodes: TopologyNode[],
	topology: RenderableTopology,
	measured: Map<string, XY>
): void {
	const report = verifyShapeKeys(visibleNodes, topology, measured);
	(window as unknown as { __topoShapeReport?: ShapeVerifyReport }).__topoShapeReport = report;

	if (report.disagreements.length > 0) {
		console.warn(
			`[topology] ${report.disagreements.length} shape key(s) map to more than one card height. ` +
				`Sampling the measure pass would lay these out wrongly.`,
			report.disagreements
		);
	}
}
