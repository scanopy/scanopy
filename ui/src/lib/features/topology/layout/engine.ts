import type { TopologyNode, TopologyEdge, RenderableTopology } from '../types/base';
import type { TopologyView } from '../queries';
import type { TopologyParentIndex } from '../topology-parent-index';
import { computeElkLayout } from './elk-layout';

export interface LayoutInput {
	nodes: TopologyNode[];
	edges: TopologyEdge[];
	topology: RenderableTopology;
	/** Active view being rendered — drives per-view layout (L2 vertical, Workloads sort). */
	view: TopologyView;
	parentIndex?: TopologyParentIndex;
	collapsedContainers?: Set<string>;
	expandedContainerSizes?: Map<string, { width: number; height: number }>;
	elementNodeSizes?: Map<string, { x: number; y: number }>;
	hiddenEdgeTypes?: string[];
	/**
	 * Keep every container's children in the order they appear in `nodes`, which the server has
	 * already sorted. Off, the layout arranges them itself to fit each container compactly.
	 */
	preserveChildOrder?: boolean;
}

export interface LayoutResult {
	nodePositions: Map<string, { x: number; y: number }>;
	containerSizes: Map<string, { width: number; height: number }>;
	elementNodeSizes: Map<string, { x: number; y: number }>;
}

export interface LayoutEngine {
	compute(input: LayoutInput): Promise<LayoutResult>;
}

export class ElkLayoutEngine implements LayoutEngine {
	async compute(input: LayoutInput): Promise<LayoutResult> {
		return computeElkLayout(input);
	}
}
