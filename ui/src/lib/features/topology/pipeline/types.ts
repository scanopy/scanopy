import type { LayoutGraph } from '../layout/layout-graph';
import type { TopologyNode, TopologyEdge } from '../types/base';
import type { AggregatedEdge } from '../collapse';
import type { TopologyParentIndex } from '../topology-parent-index';
import type { TopologyView } from '../queries';

export type XY = { x: number; y: number };

/**
 * Mutable state shared across all pipeline phases.
 * Created once by the orchestrator, passed by reference to pipeline functions.
 */
export interface LayoutState {
	layoutGraph: LayoutGraph | null;
	containerSizeCache: Map<string, { collapsed?: XY; expanded?: XY }>;
	viewSizeCache: Map<string, Map<string, XY>>;
	/**
	 * Elements whose cached height has already been corrected against the DOM under the current
	 * structure. Bounds the post-render size self-heal to one corrective re-layout per node, so a
	 * card whose rendered height never matches what was measured for it cannot drive an endless
	 * measure/re-layout cycle. Cleared whenever the structure changes.
	 */
	driftCorrectedIds: Set<string>;
	sessionStructureKey: string;
	sessionBaseKey: string;
	seenAutoCollapseIds: Set<string>;
	collapseLevelInferred: boolean;
	/**
	 * Whether the level indicator has been reconciled with what auto-collapse actually drew.
	 *
	 * Separate from `collapseLevelInferred` because the two are consumed by different steps in
	 * the same run, and sharing one flag meant the later step never ran: seeding sets the level
	 * from the stored default *before* `applyAutoCollapse` scale-collapses the graph, and the
	 * re-infer that was supposed to correct it found the flag already taken. The view then
	 * opened fully collapsed while the indicator read 3.
	 */
	collapseLevelReconciled: boolean;
	lastSeenTopologyId: string;
	fitViewPending: boolean;
	prevExpandedPortIds: Set<string>;
	/** Open manager boxes the last layout was run with; a change re-runs ELK. */
	prevExpandedInlineGroups: Set<string>;
	lastRenderedTopoKey: string;
	lastRenderedView: string;
	layoutGeneration: number;
}

/**
 * Output of the prepare phase, consumed by measure/execute/build phases.
 */
export interface PrepareResult {
	layoutNodes: TopologyNode[];
	collapsed: Set<string>;
	elevatedEdges: TopologyEdge[];
	elementToContainer: Map<string, string>;
	parentIndex: TopologyParentIndex;
	topoKey: string;
	structureKey: string;
	baseKey: string;
	isNewStructure: boolean;
	isNewBaseStructure: boolean;
	viewChanged: boolean;
	topologyChanged: boolean;
	deferCollapse: boolean;
	needsElkForExpand: boolean;
	collapseChanged: boolean;
	visibleNodes: TopologyNode[];
	aggregatedEdges: AggregatedEdge[];
	hiddenEdgeTypes: string[];
	prevExpandedSizes: Map<string, { width: number; height: number }> | undefined;
	prevChildPositions: Map<string, Map<string, { x: number; y: number }>> | undefined;
	currentView: TopologyView;
	topologyId: string;
	needsElk: boolean;
	isViewTransition: boolean;
}

export function createInitialState(): LayoutState {
	return {
		layoutGraph: null,
		containerSizeCache: new Map(),
		viewSizeCache: new Map(),
		driftCorrectedIds: new Set(),
		sessionStructureKey: '',
		sessionBaseKey: '',
		seenAutoCollapseIds: new Set(),
		collapseLevelInferred: false,
		collapseLevelReconciled: false,
		lastSeenTopologyId: '',
		fitViewPending: false,
		prevExpandedPortIds: new Set(),
		prevExpandedInlineGroups: new Set(),
		lastRenderedTopoKey: '',
		lastRenderedView: '',
		layoutGeneration: 0
	};
}
