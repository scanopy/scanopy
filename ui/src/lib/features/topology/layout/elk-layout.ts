import type { ElkNode, ElkExtendedEdge } from 'elkjs';
import type { TopologyNode } from '../types/base';
import { affectsLayout, affectsLayoutWithinContainer, getViewConfig } from './edge-classification';
import {
	containerTypes,
	getIrrelevantServiceCategories,
	getServiceDefinitionCategory
} from '#lib/shared/stores/metadata.ts';
import type { LayoutInput, LayoutResult } from './engine';
import { getOrgUseCase } from '../queries';
import { getTopologyIndex } from '../entity-index';
import * as perf from '../perf';
import { resolveCollapsedAncestor } from '../collapse';
import { noteElkRun } from '../diagnostics';
import { connectedGroups, dependencyOrder, pinChildOrder } from './child-order';
import { getViewLayout, type ViewLayoutConfig } from './view-layout';

/**
 * The port-constraint a container's own ports can honour.
 *
 * `FIXED_POS` is a promise that every port on the node carries coordinates, and ELK reads one
 * without them as (0, 0) — stacking it on the container's top-left corner. So a single
 * unpositioned port disqualifies the whole container, no matter how many of its siblings are
 * placed. `FIXED_SIDE` keeps the side constraint that makes edges leave in the right direction and
 * lets ELK space the ports itself.
 */
export function portConstraintFor(ports: { x?: number; y?: number }[]): 'FIXED_POS' | 'FIXED_SIDE' {
	const allPositioned = ports.every(
		(port) => typeof port.x === 'number' && typeof port.y === 'number'
	);
	return allPositioned ? 'FIXED_POS' : 'FIXED_SIDE';
}

/**
 * Resolve any node ID (element, subcontainer, or root container) to its
 * root container ID. Returns undefined if the ID isn't found in any map.
 */
function resolveToRootContainer(
	id: string,
	elementToRoot: Map<string, string>,
	containerIds: Set<string>,
	parentContainerMap: Map<string, string>
): string | undefined {
	const fromElem = elementToRoot.get(id);
	if (fromElem) return fromElem;
	if (!containerIds.has(id)) return undefined;
	let rootId = id;
	while (parentContainerMap.has(rootId)) {
		rootId = parentContainerMap.get(rootId)!;
	}
	return rootId;
}

/** @deprecated Use LayoutInput from engine.ts */
export type ElkLayoutInput = LayoutInput;

export type HandleSide = 'Top' | 'Bottom' | 'Left' | 'Right';

export interface EdgeHandles {
	sourceHandle: HandleSide;
	targetHandle: HandleSide;
}

/** @deprecated Use LayoutResult from engine.ts */
export type ElkLayoutResult = LayoutResult;

// @ts-expect-error -- elkjs module import type works at runtime but svelte-check disagrees
let elkPromise: Promise<import('elkjs')['default']> | null = null;

/**
 * Load ELK in-process.
 *
 * NOTE: elkjs also ships a web-worker build (`elk-worker.min.js` + `elk-api`),
 * and it was tried here — loading ELK is the largest single stage of a cold
 * load (~900ms on a production build of a 440-host L2 view), so moving it off
 * the main thread looked like the obvious win. **It measured worse and was
 * reverted.** On that same view:
 *
 *   TTI              7007ms -> 7982ms
 *   elk.module-load   908ms ->  959ms   (unchanged: ELK still awaits worker
 *                                        readiness before it can lay out, so
 *                                        the load stays on the critical path)
 *   elk.layout        802ms -> 1116ms   (structured-cloning a 1246-node graph
 *                                        twice per layout costs more than
 *                                        running it in-process saves)
 *
 * Pan frame times did not improve either. Don't re-attempt without a way to
 * avoid re-serialising the whole graph per pass.
 */
async function loadBundledElk() {
	const mod = await import('elkjs/lib/elk.bundled.js');
	return new mod.default();
}

// @ts-expect-error -- elkjs module import type works at runtime but svelte-check disagrees
async function getElk(): Promise<import('elkjs/lib/elk-api')['default']> {
	if (!elkPromise) {
		elkPromise = loadBundledElk();
	}
	return elkPromise;
}

/**
 * Begin loading ELK without waiting for it.
 *
 * Started at the top of the pipeline so the import overlaps the measure pass
 * rather than following it. Measured as no better in practice — the parse is
 * main-thread-bound and the measure pass saturates that thread — but it cannot
 * be worse, and it pays off if the measure pass ever stops being the blocker.
 * Safe to call repeatedly; `getElk` memoizes.
 */
export function preloadElk(): void {
	void getElk();
}

/** Root-level ELK layout options for layered compound layout. */
const ROOT_LAYOUT_OPTIONS: Record<string, string> = {
	'elk.algorithm': 'layered',
	'elk.direction': 'DOWN',
	'elk.layered.spacing.nodeNodeBetweenLayers': '75',
	'elk.layered.spacing.edgeNodeBetweenLayers': '50',
	'elk.edgeRouting': 'POLYLINE',
	'elk.layered.spacing.edgeEdgeBetweenLayers': '25',
	'elk.spacing.componentComponent': '75',
	'elk.spacing.nodeNode': '75',
	'elk.layered.nodePlacement.strategy': 'SITE_SIMPLEX',
	'elk.layered.crossingMinimization.strategy': 'LAYER_SWEEP',
	'elk.hierarchyHandling': 'SEPARATE_CHILDREN',
	'elk.layered.layering.strategy': 'SITE_SIMPLEX',
	'elk.layered.compaction.postCompaction.strategy': 'LEFT_RIGHT_CONSTRAINT_LOCKING',
	'elk.layered.compaction.connectedComponents': 'true',
	'elk.aspectRatio': '1.6',
	'elk.padding': '[top=25,left=25,bottom=25,right=25]',
	'elk.randomSeed': '1'
};

/**
 * Root-level ELK options per view layout algorithm. Keyed by the backend's algorithm enum, so an
 * algorithm the backend adds fails to compile here until it has options.
 */
const ROOT_OPTIONS_BY_ALGORITHM: Record<ViewLayoutConfig['algorithm'], Record<string, string>> = {
	layered_right: {
		'elk.algorithm': 'layered',
		'elk.direction': 'RIGHT',
		// Not POLYLINE. Laid out RIGHT with connected-component compaction, POLYLINE routing makes
		// ELK return NaN for every root's x as soon as the graph has a cycle (a ring of switches
		// linked port to port) beside another component, which renders the whole L2 canvas at
		// x = NaN. Only node positions are read back, so the routing style draws nothing.
		'elk.edgeRouting': 'ORTHOGONAL',
		'elk.spacing.nodeNode': '50',
		'elk.spacing.componentComponent': '75',
		// Laid out RIGHT, a layer is a *vertical* stack of switches and the spacing between layers
		// is the *horizontal* gap between those stacks — the axes are inverted relative to every
		// view that runs DOWN. Widened from 75 so the port-to-port edges running between stacks
		// have room to be followed by eye.
		'elk.layered.spacing.nodeNodeBetweenLayers': '150',
		'elk.layered.spacing.edgeNodeBetweenLayers': '25',
		'elk.layered.crossingMinimization.strategy': 'LAYER_SWEEP',
		'elk.layered.crossingMinimization.forceNodeModelOrder': 'true',
		'elk.layered.considerModelOrder.strategy': 'NODES_AND_EDGES',
		'elk.layered.nodePlacement.strategy': 'SIMPLE',
		'elk.layered.compaction.connectedComponents': 'true',
		'elk.hierarchyHandling': 'SEPARATE_CHILDREN',
		'elk.padding': '[top=25,left=25,bottom=25,right=25]'
	},
	workloads: {
		...ROOT_LAYOUT_OPTIONS,
		'elk.layered.crossingMinimization.forceNodeModelOrder': 'true',
		'elk.layered.considerModelOrder.strategy': 'NODES_AND_EDGES',
		'elk.layered.considerModelOrder.components': 'MODEL_ORDER',
		// Hosts with no layout edge between them are each their own component. Compacting
		// components stacks thousands of them into one strip a few hosts wide; without it they
		// fill rows to the root aspect ratio.
		'elk.layered.compaction.connectedComponents': 'false'
	},
	// For mostly independent container groups (Application's Customer Portal, CI/CD, Wiki,
	// Monitoring, Ungrouped, …) with only a few cross-group flow edges. Layered layout strings the
	// connected ones into a diagonal cascade and shelf-packs the rest, leaving large empty gaps.
	// rectpacking instead tiles every group compactly into a landscape rectangle
	// (whitespace-minimizing); the sparse flow edges still render but don't drive placement.
	rect_packing: {
		'elk.algorithm': 'rectpacking',
		'elk.aspectRatio': '1.6',
		'elk.spacing.nodeNode': '40',
		'elk.padding': '[top=25,left=25,bottom=25,right=25]'
	},
	default: ROOT_LAYOUT_OPTIONS
};

/**
 * Build an ELK graph from topology data.
 * Containers become parent nodes; elements become children inside their container.
 * Only layout-affecting edges are included in the ELK graph.
 */
function buildElkGraph(
	input: ElkLayoutInput,
	elementPositions?: Map<
		string,
		{ x: number; y: number; w: number; h: number; containerW: number; containerH: number }
	>,
	subcontainerPositions?: Map<string, { x: number; y: number; h: number }>,
	rootBoxes?: Map<string, { x: number; y: number; w: number; h: number }>
): {
	graph: ElkNode;
	containerIds: Set<string>;
} {
	const containers: Map<string, ElkNode> = new Map();
	const containerIds = new Set<string>();

	// Indexed lookups, built once.
	//
	// Several sites below scanned `topology.interfaces` (16,964 records) or `input.nodes` (19,095)
	// linearly from inside per-child loops, and three of them from inside `sort` comparators, where
	// the scan re-ran on every comparison. Graph construction measured 7.4s and 3.1s across the two
	// passes — more than a quarter of the 33.7s spent in layout — for lookups that are already
	// indexed elsewhere. `getTopologyIndex` is built and cached per topology for the render path.
	const topoIndex = input.topology ? getTopologyIndex(input.topology) : null;
	const nodeById = new Map(input.nodes.map((n) => [n.id, n]));

	const collapsed = input.collapsedContainers ?? new Set<string>();

	// Track parent relationships for nested containers.
	// Pre-seed from parentIndex if available; otherwise built during container loop below.
	const parentContainerMap = input.parentIndex
		? new Map(input.parentIndex.containerParent)
		: new Map<string, string>();

	// A left-to-right layered view lays its children out layered too
	// (crossing minimization for port-to-port edges)
	const layout = getViewLayout(input.view);
	const useLayeredChildren = layout.algorithm === 'layered_right';
	const orderByOperStatus = layout.element_order === 'oper_status';
	// The server has already sorted every container's children; the layout keeps that order.
	const preserveOrder = !!input.preserveChildOrder;

	// Create container (parent) nodes
	for (const node of input.nodes) {
		if (node.node_type === 'Container') {
			containerIds.add(node.id);
			const isCollapsed = collapsed.has(node.id);
			const containerType =
				((node as Record<string, unknown>).container_type as string) ?? 'Subnet';
			const meta = containerTypes.getMetadata(containerType);
			const isSubcontainer = meta.is_subcontainer;
			const parentId = (node as Record<string, unknown>).parent_container_id as string | undefined;
			if (parentId) parentContainerMap.set(node.id, parentId);

			const p = meta.padding;
			const padding = `[top=${p.top},left=${p.left},bottom=${p.bottom},right=${p.right}]`;

			// DOM-measured size for collapsed containers. Metadata fallback
			// is a safety net only — the measurement pipeline should always
			// provide sizes via elementNodeSizes or the container size cache.
			const measured = input.elementNodeSizes?.get(node.id);
			const collapsedWidth = measured?.x ?? meta.collapsed_size.width;
			const collapsedHeight = measured?.y ?? meta.collapsed_size.height;
			const elkCollapsedWidth = collapsedWidth;

			// Layered children: ELK optimizes child ordering for crossing minimization
			// Box children: grid packing by size (default for most views)
			// L2 uses narrow 0.3 for vertical port columns; subcontainers use
			// wider 5.0 to spread children horizontally; root containers use 1.4
			const aspectRatio = useLayeredChildren ? '0.3' : isSubcontainer ? '5.0' : '1.4';
			const childLayoutOptions: Record<string, string> = {
				'elk.algorithm': 'box',
				'elk.box.packingMode': 'SIMPLE',
				'elk.aspectRatio': aspectRatio,
				'elk.padding': padding,
				'elk.nodeSize.constraints': 'MINIMUM_SIZE',
				'elk.spacing.nodeNode': '25',
				'elk.spacing.componentComponent': '25'
			};

			const elkNode: ElkNode = isCollapsed
				? {
						id: node.id,
						width: elkCollapsedWidth,
						height: collapsedHeight,
						children: [],
						layoutOptions: {
							'elk.nodeSize.constraints': 'MINIMUM_SIZE',
							'elk.nodeSize.minimum': `(${elkCollapsedWidth},${collapsedHeight})`
						}
					}
				: {
						id: node.id,
						children: [],
						layoutOptions: {
							...childLayoutOptions,
							// An open container is as wide as its children and padding: its title
							// truncates rather than widening it, so a container holding one card is
							// that card's width in every view. The floor is on height only —
							// subcontainers from metadata, root containers from the DOM-measured
							// collapsed size.
							'elk.nodeSize.minimum': `(0,${isSubcontainer ? meta.collapsed_size.height : collapsedHeight})`
						}
					};
			containers.set(node.id, elkNode);
		}
	}

	// Nest sub-group containers inside their parent containers.
	// For L2: determine edge direction per subcontainer for horizontal priority.
	const subEdgeDirection = new Map<string, 'left' | 'right'>();

	for (const [childId, parentId] of parentContainerMap) {
		const parent = containers.get(parentId);
		const child = containers.get(childId);
		if (parent && child && parent.children) {
			if (useLayeredChildren && child.layoutOptions) {
				// Left-connecting subs get higher priority = packed first = leftmost
				const dir = subEdgeDirection.get(childId);
				child.layoutOptions['elk.priority'] =
					dir === 'left' ? '200' : dir === 'right' ? '50' : '100';
				child.layoutOptions['elk.box.packingMode'] = 'SIMPLE';
			}
			parent.children.push(child);
		}
	}

	// Subcontainer children sorted after elements are added (below)

	// Build dual element→container mappings, using parentIndex when available.
	const elementToImmediateContainer = input.parentIndex
		? new Map([...input.parentIndex.elementToContainer].filter(([, cid]) => containers.has(cid)))
		: new Map<string, string>();
	const elementToRootContainer = input.parentIndex
		? new Map(
				[...input.parentIndex.elementToRootContainer].filter(([id]) =>
					elementToImmediateContainer.has(id)
				)
			)
		: new Map<string, string>();

	if (!input.parentIndex) {
		// Fallback: build from nodes if no parentIndex provided
		for (const node of input.nodes) {
			if (node.node_type === 'Element') {
				const parentId = node.container_id;
				if (typeof parentId === 'string' && containers.has(parentId)) {
					elementToImmediateContainer.set(node.id, parentId);
					let rootId = parentId;
					while (parentContainerMap.has(rootId)) {
						rootId = parentContainerMap.get(rootId)!;
					}
					elementToRootContainer.set(node.id, rootId);
				}
			}
		}
	}

	// Full parent map from ALL topology nodes (for resolving edge endpoints in collapsed containers).
	const fullParentMap = input.parentIndex ? input.parentIndex.parentMap : new Map<string, string>();
	if (!input.parentIndex && input.topology) {
		for (const n of input.topology.nodes) {
			if (n.node_type === 'Element') {
				const pid = (n as Record<string, unknown>).container_id as string | undefined;
				if (pid) fullParentMap.set(n.id, pid);
			} else if (n.node_type === 'Container') {
				const pid = (n as Record<string, unknown>).parent_container_id as string | undefined;
				if (pid) fullParentMap.set(n.id, pid);
			}
		}
	}

	// Add element nodes as children of their containers (skip collapsed)
	// A view ordered by oper status sorts Up ports first.
	// Collect elements per container for sorting
	const elementsPerContainer = new Map<
		string,
		{ node: TopologyNode; size: { x: number; y: number } }[]
	>();
	for (const node of input.nodes) {
		if (node.node_type === 'Element') {
			const parentId = node.container_id ?? '';
			if (!parentId || collapsed.has(parentId)) continue;
			if (!containers.has(parentId)) continue;
			// `node.size` is the server's value, which for elements is `Uxy::default()` — 0x0.
			// A zero-sized ELK node is never legitimate: box packing puts such children a
			// spacing apart and the DOM then renders them at their real size, overlapping.
			// The measure pass is supposed to make this unreachable; counting it keeps a
			// regression loud instead of silent, and the fallback keeps the graph merely
			// imperfect rather than broken.
			let size = input.elementNodeSizes?.get(node.id) ?? node.size;
			if (!size || size.x <= 0 || size.y <= 0) {
				perf.count('elk.element-size-missing');
				size = { x: 250, y: 54 };
			}
			if (!elementsPerContainer.has(parentId)) elementsPerContainer.set(parentId, []);
			elementsPerContainer.get(parentId)!.push({ node, size });
		}
	}

	for (const [parentId, elements] of elementsPerContainer) {
		const parent = containers.get(parentId);
		if (!parent?.children) continue;

		if (orderByOperStatus) {
			// Sort: Up ports first, then Down, then others
			const statusOrder = (n: TopologyNode): number => {
				const status = (n as Record<string, unknown>).oper_status as string | undefined;
				// oper_status isn't on the node directly — look it up from topology
				const ifEntryId = (n as Record<string, unknown>).interface_id as string | undefined;
				const iface = ifEntryId ? topoIndex?.interfacesById.get(ifEntryId) : undefined;
				const s = iface?.oper_status ?? status ?? '';
				if (s === 'Up') return 0;
				if (s === 'Down') return 1;
				return 2;
			};
			// Ranked once per element rather than once per comparison: `sort` calls its comparator
			// O(n log n) times, so computing the key inside it multiplied the lookup by that factor.
			const rankByNode = new Map(elements.map((e) => [e.node, statusOrder(e.node)]));
			elements.sort((a, b) => (rankByNode.get(a.node) ?? 2) - (rankByNode.get(b.node) ?? 2));

			// UP direction: edge targets (subcontainers with connected ports)
			// naturally go to upper layers (top). Down ports with no edges
			// stay in lower layers (bottom).
			for (const { node, size } of elements) {
				parent.children!.push({
					id: node.id,
					width: size.x,
					height: size.y,
					layoutOptions: {
						'elk.nodeSize.constraints': 'MINIMUM_SIZE',
						'elk.nodeSize.minimum': `(${size.x},${size.y})`
					}
				});
			}
		} else {
			for (const { node, size } of elements) {
				parent.children!.push({
					id: node.id,
					width: size.x,
					height: size.y,
					layoutOptions: {
						'elk.nodeSize.constraints': 'MINIMUM_SIZE',
						'elk.nodeSize.minimum': `(${size.x},${size.y})`
					}
				});
			}
		}
	}

	// Helper: resolve an edge endpoint to its root container.
	// Handles hidden elements inside collapsed containers via fullParentMap.
	const resolveRoot = (id: string): string | undefined => {
		const resolved = resolveToRootContainer(
			id,
			elementToRootContainer,
			containerIds,
			parentContainerMap
		);
		if (resolved) return resolved;
		// Fallback: hidden element inside a collapsed container
		const ancestor = resolveCollapsedAncestor(id, collapsed, fullParentMap);
		if (ancestor)
			return resolveToRootContainer(
				ancestor,
				elementToRootContainer,
				containerIds,
				parentContainerMap
			);
		return undefined;
	};

	// L2: determine edge direction per subcontainer for horizontal priority
	if (useLayeredChildren) {
		for (const edge of input.edges) {
			if (!affectsLayout(edge)) continue;
			const srcRoot = resolveRoot(edge.source);
			const tgtRoot = resolveRoot(edge.target);
			if (!srcRoot || !tgtRoot || srcRoot === tgtRoot) continue;
			const srcImm = elementToImmediateContainer.get(edge.source);
			if (srcImm && parentContainerMap.has(srcImm)) subEdgeDirection.set(srcImm, 'right');
			const tgtImm = elementToImmediateContainer.get(edge.target);
			if (tgtImm && parentContainerMap.has(tgtImm)) subEdgeDirection.set(tgtImm, 'left');
		}
		// Update subcontainer priorities based on edge direction
		for (const [childId] of parentContainerMap) {
			const child = containers.get(childId);
			if (child?.layoutOptions) {
				const dir = subEdgeDirection.get(childId);
				if (dir) child.layoutOptions['elk.priority'] = dir === 'left' ? '200' : '50';
			}
		}
	}

	// Build element → target root container(s) mapping for edge-aware sorting.
	// Elements connecting to the same target should be adjacent in the grid so
	// their ports cluster together, giving ELK meaningful crossing information.
	// Only orders children, so an edge that places cards within a container counts too.
	const elementTargets = new Map<string, Set<string>>();
	for (const edge of input.edges) {
		if (!affectsLayoutWithinContainer(edge)) continue;
		const srcRoot = resolveRoot(edge.source);
		const tgtRoot = resolveRoot(edge.target);
		if (!srcRoot || !tgtRoot || srcRoot === tgtRoot) continue;

		// Map source element → target container
		if (elementToRootContainer.has(edge.source)) {
			if (!elementTargets.has(edge.source)) elementTargets.set(edge.source, new Set());
			elementTargets.get(edge.source)!.add(tgtRoot);
		}
		// Map target element → source container (reverse direction)
		if (elementToRootContainer.has(edge.target)) {
			if (!elementTargets.has(edge.target)) elementTargets.set(edge.target, new Set());
			elementTargets.get(edge.target)!.add(srcRoot);
		}
	}

	// Where in its container an element with edges to other root containers belongs, from where those
	// containers ended up after pass 1 (`rootBoxes`): 0 top, 1 left, 2 no such edge, 3 right,
	// 4 bottom. A container packs its children in this order, so an element ends up on the side facing
	// what it links to and its edge leaves without crossing its siblings. Pass 1 has no positions;
	// there every linked element ranks first, as before.
	const rootCentre = (id: string) => {
		const pos = rootBoxes?.get(id);
		return pos ? { x: pos.x + pos.w / 2, y: pos.y + pos.h / 2 } : undefined;
	};
	const sideRank = (elemId: string): number => {
		const targets = elementTargets.get(elemId);
		if (!targets?.size) return 2;
		const own = rootCentre(elementToRootContainer.get(elemId) ?? '');
		if (!own) return 0;
		let dx = 0;
		let dy = 0;
		for (const target of targets) {
			const peer = rootCentre(target);
			if (!peer) continue;
			const length = Math.hypot(peer.x - own.x, peer.y - own.y) || 1;
			dx += (peer.x - own.x) / length;
			dy += (peer.y - own.y) / length;
		}
		if (dx === 0 && dy === 0) return 0;
		if (Math.abs(dy) >= Math.abs(dx)) return dy < 0 ? 0 : 4;
		return dx < 0 ? 1 : 3;
	};

	// Ordered by oper status: count Up ports inside each subcontainer for sorting
	const subcontainerUpCount = new Map<string, number>();
	if (orderByOperStatus) {
		for (const [subId] of parentContainerMap) {
			const sub = containers.get(subId);
			if (!sub?.children) continue;
			let upCount = 0;
			for (const child of sub.children) {
				if (containerIds.has(child.id)) continue;
				const node = topoIndex?.nodesById.get(child.id);
				const ifEntryId = node
					? ((node as Record<string, unknown>).interface_id as string | undefined)
					: undefined;
				const iface = ifEntryId ? topoIndex?.interfacesById.get(ifEntryId) : undefined;
				if (iface?.oper_status === 'Up') upCount++;
			}
			subcontainerUpCount.set(subId, upCount);
		}
	}

	// Under a server-side sort, every container (subcontainers included) takes its children in
	// input order instead.
	const inputRank = new Map(input.nodes.map((node, index) => [node.id, index]));

	// Sort children: ordered by oper status, elements keep their status order and subcontainers
	// rank by Up port count, so edges don't traverse through disconnected Down ports.
	// Otherwise: elements grouped by target, then subcontainers last.
	for (const [containerId, container] of containers) {
		if (!container.children || container.children.length < 2) continue;
		if (preserveOrder) {
			pinChildOrder(container.children, inputRank);
			continue;
		}
		if (parentContainerMap.has(containerId)) continue;

		container.children.sort((a, b) => {
			const aIsSub = containerIds.has(a.id) ? 1 : 0;
			const bIsSub = containerIds.has(b.id) ? 1 : 0;

			if (orderByOperStatus) {
				if (aIsSub !== bIsSub) return aIsSub - bIsSub;
				if (aIsSub && bIsSub) {
					// Sort subcontainers by Up port count ascending —
					// GROUP_DEC reverses large items to top, so ascending input = descending visual
					const aUp = subcontainerUpCount.get(a.id) ?? 0;
					const bUp = subcontainerUpCount.get(b.id) ?? 0;
					return aUp - bUp;
				}
				return 0;
			}

			// Default sort for other views
			if (aIsSub !== bIsSub) return aIsSub - bIsSub;
			if (aIsSub && bIsSub) return a.id.localeCompare(b.id);

			// Both are elements: by the side of the container they link toward, unlinked ones between
			const side = sideRank(a.id) - sideRank(b.id);
			if (side !== 0) return side;
			const aTargets = elementTargets.get(a.id);
			const bTargets = elementTargets.get(b.id);
			if (!aTargets?.size || !bTargets?.size) return a.id.localeCompare(b.id);

			// Both linked the same way: group by target (sorted target IDs as group key)
			const aKey = Array.from(aTargets).sort().join(',');
			const bKey = Array.from(bTargets).sort().join(',');
			if (aKey !== bKey) return aKey.localeCompare(bKey);

			return a.id.localeCompare(b.id);
		});

		// Box packing places children by priority before size, so the side groups hold; within one,
		// it still orders by size. Subcontainers rank with unlinked elements, so a container with no
		// links out packs exactly as it did without priorities.
		if (!orderByOperStatus && rootBoxes) {
			for (const child of container.children) {
				const rank = containerIds.has(child.id) ? 2 : sideRank(child.id);
				child.layoutOptions = { ...child.layoutOptions, 'elk.priority': String(10 - rank) };
			}
		}
	}

	// Create port-based edges for cross-container connections.
	// Ports encode the relative order of edge sources within a container so ELK's
	// crossing minimization can meaningfully order same-layer target containers.
	//
	// Port positions are distributed evenly across the container width, ordered by
	// target group. Box packing internally reorders elements by size, so predicting
	// actual element positions is unreliable. What matters is the RELATIVE order:
	// elements connecting to "left" targets get left-side ports, "right" targets
	// get right-side ports.
	const edges: ElkExtendedEdge[] = [];
	const seenEdges = new Set<string>();
	let edgeIndex = 0;

	// The centre Y within its root container, from pass 1, of an edge endpoint: an element, a
	// subcontainer, or an element hidden in a collapsed subcontainer, which stands at that
	// subcontainer. Undefined when pass 1 placed none of them. Pass-1 positions are relative to the
	// immediate container, so an element inside a subcontainer takes that subcontainer's offset; a
	// root container's own position is not added.
	const centreYInRoot = (id: string): number | undefined => {
		const pos = elementPositions?.get(id);
		if (pos) {
			const imm = elementToImmediateContainer.get(id);
			const offset =
				imm && parentContainerMap.has(imm) ? (subcontainerPositions?.get(imm)?.y ?? 0) : 0;
			return offset + pos.y + pos.h / 2;
		}
		const sub = parentContainerMap.has(id) ? subcontainerPositions?.get(id) : undefined;
		if (sub) return sub.y + sub.h / 2;
		const ancestor = resolveCollapsedAncestor(id, collapsed, fullParentMap);
		return ancestor && ancestor !== id ? centreYInRoot(ancestor) : undefined;
	};

	// Build container → first child element lookup (for container-targeted edges).
	// After edge elevation, will_target_container edges have a container ID as their
	// target. ELK needs element-based ports for proper layer separation, so we
	// substitute the container target with its first child element.
	const containerFirstElement = new Map<string, string>();
	for (const node of input.nodes) {
		if (node.node_type === 'Element') {
			const cid = elementToRootContainer.get(node.id);
			if (cid && !containerFirstElement.has(cid)) {
				containerFirstElement.set(cid, node.id);
			}
		}
	}

	// Collect all cross-container edges grouped by source container
	const edgesBySourceContainer = new Map<
		string,
		{ source: string; target: string; srcRoot: string; tgtRoot: string }[]
	>();
	for (const edge of input.edges) {
		if (!affectsLayout(edge)) continue;
		const key = `${edge.source}->${edge.target}`;
		if (seenEdges.has(key)) continue;
		seenEdges.add(key);

		const srcRoot = resolveRoot(edge.source);
		const tgtRoot = resolveRoot(edge.target);
		if (!srcRoot || !tgtRoot || srcRoot === tgtRoot) continue;

		// For edges targeting containers (after edge elevation), substitute
		// the first element inside the target container as the edge target.
		// ELK creates proper port-based layer constraints for element targets
		// but not for direct container targets.
		let effectiveTarget = edge.target;
		if (containerIds.has(edge.target)) {
			const elem = containerFirstElement.get(tgtRoot);
			if (elem) effectiveTarget = elem;
		}

		if (!edgesBySourceContainer.has(srcRoot)) edgesBySourceContainer.set(srcRoot, []);
		edgesBySourceContainer
			.get(srcRoot)!
			.push({ source: edge.source, target: effectiveTarget, srcRoot, tgtRoot });
	}

	// For each source container, distribute ports evenly ordered by target group
	for (const [srcContainerId, containerEdges] of edgesBySourceContainer) {
		const container = containers.get(srcContainerId);
		if (!container) continue;

		// Group edges by source element, then sort elements by their target group key
		// (same key = same target set → adjacent ports)
		const elementEdges = new Map<string, Set<string>>();
		for (const e of containerEdges) {
			if (!elementEdges.has(e.source)) elementEdges.set(e.source, new Set());
			elementEdges.get(e.source)!.add(e.tgtRoot);
		}
		const sortedElements = Array.from(elementEdges.entries()).sort(([, aTargets], [, bTargets]) => {
			const aKey = Array.from(aTargets).sort().join(',');
			const bKey = Array.from(bTargets).sort().join(',');
			return aKey.localeCompare(bKey);
		});

		if (!container.ports) container.ports = [];
		if (!container.layoutOptions) container.layoutOptions = {};

		// Port side depends on layout direction: DOWN→SOUTH/NORTH, RIGHT→EAST/WEST
		const srcSide = useLayeredChildren ? 'EAST' : 'SOUTH';
		const elementPortIds = new Map<string, string>();
		for (const [elemId] of sortedElements) {
			const portId = `port-${elemId}-${srcSide}`;
			const pos = elementPositions?.get(elemId);
			if (pos) {
				// Pass 2: place port at the element's actual position within the container
				const portPos = useLayeredChildren
					? { x: pos.containerW * 0.9, y: centreYInRoot(elemId)! } // RIGHT: port on east side, y = element centre
					: { x: pos.x + pos.w / 2, y: pos.containerW * 0.7 }; // DOWN: port on south side, x = element center
				container.ports.push({
					id: portId,
					x: portPos.x,
					y: portPos.y,
					width: 1,
					height: 1,
					layoutOptions: { 'elk.port.side': srcSide }
				});
			} else {
				// FIXED_SIDE: ELK decides port positions, constrained to srcSide.
				// At collapse level 2, elements are hidden but EAST/WEST side
				// constraint ensures edges route horizontally.
				container.ports.push({
					id: portId,
					layoutOptions: { 'elk.port.side': srcSide }
				});
			}
			elementPortIds.set(elemId, portId);
		}

		// Pre-create target ports sorted by element position so port order
		// on the target container matches physical layout (for crossing minimization)
		const tgtSide = useLayeredChildren ? 'WEST' : 'NORTH';
		const targetPortIds = new Map<string, string>();

		// Collect unique target elements and sort by position in their container
		const targetElements = new Map<string, string[]>(); // tgtRoot → [target element IDs]
		for (const e of containerEdges) {
			if (!containerIds.has(e.target)) {
				if (!targetElements.has(e.tgtRoot)) targetElements.set(e.tgtRoot, []);
				const list = targetElements.get(e.tgtRoot)!;
				if (!list.includes(e.target)) list.push(e.target);
			}
		}

		for (const [tgtRootId, elemIds] of targetElements) {
			const tgtContainer = containers.get(tgtRootId);
			if (!tgtContainer) continue;

			// Sort target elements by their position within the container
			if (elementPositions && elementPositions.size > 0) {
				elemIds.sort((a, b) => {
					const posA = elementPositions.get(a);
					const posB = elementPositions.get(b);
					return (posA?.x ?? 0) - (posB?.x ?? 0);
				});
			}

			if (!tgtContainer.ports) tgtContainer.ports = [];
			if (!tgtContainer.layoutOptions) tgtContainer.layoutOptions = {};

			// Only the layered (L2) child layout consumes target-port Y for crossing minimization;
			// elsewhere the side constraint alone is what the target end needs.
			const positionTargets = !!elementPositions && useLayeredChildren;

			for (const elemId of elemIds) {
				const tgtPortId = `port-${elemId}-${tgtSide}`;
				if (!tgtContainer.ports.some((p: { id: string }) => p.id === tgtPortId)) {
					const absY = positionTargets ? centreYInRoot(elemId) : undefined;
					if (absY !== undefined) {
						// Y within the root container. Only written for an element pass 1 actually
						// placed — an element inside a collapsed container has no position, and
						// defaulting it to zero used to put every such port on the container's top
						// edge, which is a crossing signal that says the opposite of the truth.
						tgtContainer.ports.push({
							id: tgtPortId,
							x: 0,
							y: absY,
							width: 1,
							height: 1,
							layoutOptions: { 'elk.port.side': tgtSide }
						});
					} else {
						tgtContainer.ports.push({
							id: tgtPortId,
							layoutOptions: { 'elk.port.side': tgtSide }
						});
					}
				}
				targetPortIds.set(elemId, tgtPortId);
			}
		}

		// Create edges from source ports to target ports
		for (const e of containerEdges) {
			const srcPortId = elementPortIds.get(e.source);
			if (!srcPortId) continue;

			const tgtPortId = targetPortIds.get(e.target);
			const tgtEndpoint = tgtPortId ?? e.tgtRoot;

			edges.push({
				id: `elk-edge-${edgeIndex++}`,
				sources: [srcPortId],
				targets: [tgtEndpoint]
			});
		}
	}

	// Decide each container's port constraint from the ports it actually carries.
	//
	// `FIXED_POS` is a promise that every port on the node has coordinates; ELK reads a port
	// without them as (0, 0), stacking it on the container's top-left corner. Ports arrive here
	// from two places — a container is a source for its own edges and a target for other
	// containers' — and only some of them can be positioned: an element inside a collapsed
	// container is skipped from the pass-1 graph, so it has no position, yet it still emits a port
	// because edge endpoints deliberately resolve through the collapsed ancestor.
	//
	// Deciding at each push site made the outcome depend on which loop wrote last, and deciding
	// from a graph-wide "did pass 1 position anything" flag declared FIXED_POS on containers full
	// of coordinate-less ports. Both are only reachable in a *mixed* collapse state, where some
	// containers are open and some closed; the uniform states either position every element or
	// none, which is why the extremes always looked fine. One decision, taken once every port is
	// known, is the only form of this that cannot disagree with itself.
	for (const container of containers.values()) {
		if (!container.ports?.length) continue;
		if (!container.layoutOptions) container.layoutOptions = {};
		container.layoutOptions['elk.portConstraints'] = portConstraintFor(container.ports);
	}

	// Edges between two children of one container: element to element, element to subcontainer, or
	// subcontainer to subcontainer (e.g. a Docker element to its ByStack subcontainer, or two cards on
	// one dependency). Each is added to the lowest container holding both endpoints, between the two
	// children of it that contain them, so that container can lay them out layered and put linked
	// children next to each other in edge order.
	const inGraph = new Set<string>();
	for (const container of containers.values()) {
		for (const child of container.children ?? []) inGraph.add(child.id);
	}
	for (const id of containerIds) {
		if (!parentContainerMap.has(id)) inGraph.add(id);
	}
	const parentOf = (id: string): string | undefined =>
		elementToImmediateContainer.get(id) ?? parentContainerMap.get(id) ?? fullParentMap.get(id);
	// The endpoint as the graph holds it (a hidden element becomes the collapsed container standing
	// in for it), followed by every container enclosing it, innermost first.
	const enclosingChain = (id: string): string[] => {
		let node: string | undefined = id;
		while (node && !inGraph.has(node)) node = parentOf(node);
		const chain: string[] = [];
		while (node) {
			chain.push(node);
			node = parentOf(node);
		}
		return chain;
	};

	const innerEdgesByContainer = new Map<string, { source: string; target: string }[]>();
	const seenInnerEdges = new Set<string>();

	// A container with inner edges switches to layered, which assigns rows from those edges, so no
	// model-order option would keep a reading order. Under a server-side sort containers stay
	// box-packed and connected children are no longer pulled together.
	const innerEdgeCandidates = preserveOrder ? [] : input.edges;
	for (const edge of innerEdgeCandidates) {
		if (!affectsLayoutWithinContainer(edge)) continue;
		const srcChain = enclosingChain(edge.source);
		const tgtChain = enclosingChain(edge.target);
		const srcDepth = srcChain.findIndex((id) => tgtChain.includes(id));
		// Different root containers: a cross-container edge, routed through ports above.
		if (srcDepth === -1) continue;
		const tgtDepth = tgtChain.indexOf(srcChain[srcDepth]);
		// An endpoint that is the shared container itself has no child to place.
		if (srcDepth === 0 || tgtDepth === 0) continue;
		const containerId = srcChain[srcDepth];
		const srcNode = srcChain[srcDepth - 1];
		const tgtNode = tgtChain[tgtDepth - 1];
		if (srcNode === tgtNode || !containers.has(containerId)) continue;

		const key = `${srcNode}->${tgtNode}`;
		if (seenInnerEdges.has(key) || seenInnerEdges.has(`${tgtNode}->${srcNode}`)) continue;
		seenInnerEdges.add(key);
		if (!innerEdgesByContainer.has(containerId)) innerEdgesByContainer.set(containerId, []);
		innerEdgesByContainer.get(containerId)!.push({ source: srcNode, target: tgtNode });
	}

	// Switch containers with inner edges from box to layered, children in edge order
	for (const [containerId, linkedEdges] of innerEdgesByContainer) {
		const container = containers.get(containerId);
		if (!container?.children) continue;
		// The side each child links toward (see `sideRank`), once pass 1 has placed the containers; a
		// view ordered by oper status keeps its status order instead.
		const facing = !orderByOperStatus && rootBoxes;
		const rankOf = (id: string) => (facing && !containerIds.has(id) ? sideRank(id) : 2);
		// Layered puts an edge's source above its target, so an element linking out through the bottom
		// would stay on top of its neighbours. Its edges here are laid out as pointing into it (and an
		// element linking out through the top, out of it); they still draw as they are.
		const innerEdges = linkedEdges.map((e) =>
			(rankOf(e.source) === 4 && rankOf(e.target) !== 4) ||
			(rankOf(e.target) === 0 && rankOf(e.source) !== 0)
				? { source: e.target, target: e.source }
				: e
		);
		container.edges = [
			...(container.edges ?? []),
			...innerEdges.map((e) => ({
				id: `elk-inner-edge-${edgeIndex++}`,
				sources: [e.source],
				targets: [e.target]
			}))
		];
		if (!orderByOperStatus) {
			const childById = new Map(container.children.map((child) => [child.id, child]));
			const ordered = dependencyOrder(
				container.children.map((child) => child.id),
				innerEdges
			);
			// Connected groups are packed in this order, so a group linking out through the bottom goes
			// last and one linking out through the top first: the rank of the member furthest from the
			// middle.
			const groupOf = connectedGroups(ordered, innerEdges);
			const groupRank = new Map<string, number>();
			for (const id of ordered) {
				const group = groupOf.get(id)!;
				const rank = rankOf(id);
				const current = groupRank.get(group) ?? 2;
				groupRank.set(group, Math.abs(rank - 2) > Math.abs(current - 2) ? rank : current);
			}
			const rankById = new Map(ordered.map((id) => [id, groupRank.get(groupOf.get(id)!)!]));
			container.children = ordered
				.map((id, index) => ({ id, index }))
				.sort((a, b) => rankById.get(a.id)! - rankById.get(b.id)! || a.index - b.index)
				.map(({ id }) => childById.get(id)!);
		}
		if (container.layoutOptions) {
			container.layoutOptions['elk.algorithm'] = 'layered';
			container.layoutOptions['elk.direction'] = useLayeredChildren ? 'RIGHT' : 'DOWN';
			container.layoutOptions['elk.hierarchyHandling'] = 'SEPARATE_CHILDREN';
			container.layoutOptions['elk.layered.nodePlacement.strategy'] = 'SITE_SIMPLEX';
			container.layoutOptions['elk.layered.crossingMinimization.strategy'] = 'LAYER_SWEEP';
			container.layoutOptions['elk.layered.layering.strategy'] = 'SITE_SIMPLEX';
			container.layoutOptions['elk.spacing.nodeNode'] = '15';
			// The gap a hop between two linked cards is drawn in, so it matches box packing's spacing
			// rather than hiding the edge between touching cards.
			container.layoutOptions['elk.layered.spacing.nodeNodeBetweenLayers'] = '25';
			container.layoutOptions['elk.layered.spacing.edgeNodeBetweenLayers'] = '5';
			container.layoutOptions['elk.layered.compaction.postCompaction.strategy'] = 'EDGE_LENGTH';
			if (orderByOperStatus) {
				// Force model order so our status-based sort (subcontainers first,
				// Up ports next, Down ports last) is preserved
				container.layoutOptions['elk.layered.crossingMinimization.forceNodeModelOrder'] = 'true';
				container.layoutOptions['elk.layered.considerModelOrder.strategy'] = 'NODES_AND_EDGES';
			} else {
				// Prefer the dependency order above where crossing minimization has a free choice,
				// and lay unconnected groups out in it too.
				container.layoutOptions['elk.layered.considerModelOrder.strategy'] = 'NODES_AND_EDGES';
				container.layoutOptions['elk.layered.considerModelOrder.components'] = 'MODEL_ORDER';
			}
			delete container.layoutOptions['elk.box.packingMode'];
		}
	}

	// Only add root-level containers (not nested sub-groups) to root children
	const rootContainers = Array.from(containers.entries())
		.filter(([id]) => !parentContainerMap.has(id))
		.map(([, node]) => node);

	// L2: sort root containers so hosts match their target port order inside the switch.
	// With forceNodeModelOrder, ELK preserves this order for crossing-free layout. Runs in pass 2
	// even when pass 1 placed no element: with every port group closed, the groups are what is placed.
	if (useLayeredChildren && elementPositions) {
		// Each root container takes the Y of the first linked element on the other end.
		const rootTargetY = new Map<string, number>();
		for (const edge of input.edges) {
			if (!affectsLayout(edge)) continue;
			const srcRoot = resolveRoot(edge.source);
			const tgtRoot = resolveRoot(edge.target);
			if (!srcRoot || !tgtRoot || srcRoot === tgtRoot) continue;

			const tgtY = centreYInRoot(edge.target);
			if (tgtY !== undefined && !rootTargetY.has(srcRoot)) rootTargetY.set(srcRoot, tgtY);
			const srcY = centreYInRoot(edge.source);
			if (srcY !== undefined && !rootTargetY.has(tgtRoot)) rootTargetY.set(tgtRoot, srcY);
		}

		rootContainers.sort((a, b) => {
			const aY = rootTargetY.get(a.id) ?? Infinity;
			const bY = rootTargetY.get(b.id) ?? Infinity;
			return aY - bY;
		});
	}

	// Sort root containers by app-relevant workload count (descending).
	// Uses topology.services and topology.hosts (always full data, independent
	// of collapse level) instead of input.nodes (which only has visible nodes).
	const orderByWorkloadCount = layout.root_order === 'workload_count';
	if (orderByWorkloadCount && input.topology) {
		const irrelevantCategories = getIrrelevantServiceCategories(getOrgUseCase());

		// Count app-relevant services per host_id
		const countByHostId = new Map<string, number>();
		for (const svc of input.topology.services ?? []) {
			const cat = getServiceDefinitionCategory(svc.service_definition);
			if (cat && !irrelevantCategories.has(cat)) {
				countByHostId.set(svc.host_id, (countByHostId.get(svc.host_id) ?? 0) + 1);
			}
		}

		// Count VM hosts as workloads on their hypervisor host.
		// virtualization_service_id points to the virtualizer service on the hypervisor.
		const serviceHostMap = new Map<string, string>();
		for (const svc of input.topology.services ?? []) {
			serviceHostMap.set(svc.id, svc.host_id);
		}
		for (const host of input.topology.hosts ?? []) {
			const serviceId = host.virtualization_service_id;
			if (serviceId) {
				const hypervisorHostId = serviceHostMap.get(serviceId);
				if (hypervisorHostId) {
					countByHostId.set(hypervisorHostId, (countByHostId.get(hypervisorHostId) ?? 0) + 1);
				}
			}
		}

		// A host container's `entity_id` is its host's id, so the count joins on it directly; the
		// header breaks ties. Count and header are resolved once per container: two linear scans per
		// comparison over 19,095 nodes made this the single most expensive thing in graph
		// construction.
		const rankById = new Map(
			rootContainers.map((c) => {
				const node = nodeById.get(c.id);
				const hostId =
					node?.node_type === 'Container'
						? ((node as Record<string, unknown>).entity_id as string | undefined)
						: undefined;
				return [
					c.id,
					{ count: hostId ? (countByHostId.get(hostId) ?? 0) : 0, name: node?.header ?? '' }
				];
			})
		);
		rootContainers.sort((a, b) => {
			const rankA = rankById.get(a.id);
			const rankB = rankById.get(b.id);
			const countA = rankA?.count ?? 0;
			const countB = rankB?.count ?? 0;
			if (countB !== countA) return countB - countA;
			return (rankA?.name ?? '').localeCompare(rankB?.name ?? '');
		});
	}

	// Assign descending priority so box packing places highest-workload containers first
	// (top-left). The sort above put them in descending workload count, so the first container
	// gets the highest priority.
	if (orderByWorkloadCount) {
		for (let i = 0; i < rootContainers.length; i++) {
			const container = rootContainers[i];
			if (!container.layoutOptions) container.layoutOptions = {};
			container.layoutOptions['elk.priority'] = String(rootContainers.length - i);
		}
	}

	const graph: ElkNode = {
		id: 'root',
		layoutOptions: ROOT_OPTIONS_BY_ALGORITHM[layout.algorithm],
		children: rootContainers,
		// rectpacking ignores edges for placement and can choke on cross-hierarchy endpoints;
		// downstream reads only node positions, so a view that packs its roots drops the root
		// edge set. Container-internal edges are unaffected.
		edges: layout.root_edges ? edges : []
	};

	return { graph, containerIds };
}

/** Recompute y-coordinates for a column of nodes based on actual heights. */
function recomputeColumnY(colNodes: ElkNode[], spacing: number): void {
	colNodes.sort((a, b) => (a.y ?? 0) - (b.y ?? 0));
	const startY = colNodes[0].y ?? 0;
	let y = startY;
	for (const node of colNodes) {
		node.y = y;
		y += (node.height ?? 0) + spacing;
	}
}

function mapElkResults(
	layoutResult: ElkNode,
	containerIds: Set<string>,
	input: ElkLayoutInput
): ElkLayoutResult {
	const nodePositions = new Map<string, { x: number; y: number }>();
	const containerSizes = new Map<string, { width: number; height: number }>();

	// Track absolute positions for handle computation (elements need container offset)
	const absolutePositions = new Map<string, { x: number; y: number }>();

	// Recursively map container and child positions
	function processChildren(children: ElkNode[], parentAbsX: number, parentAbsY: number) {
		for (const child of children) {
			const cx = child.x ?? 0;
			const cy = child.y ?? 0;
			const absX = parentAbsX + cx;
			const absY = parentAbsY + cy;

			if (containerIds.has(child.id)) {
				// Container node: position relative to parent, track absolute
				nodePositions.set(child.id, { x: cx, y: cy });
				absolutePositions.set(child.id, { x: absX, y: absY });
				containerSizes.set(child.id, {
					width: child.width ?? 0,
					height: child.height ?? 0
				});
				// Recurse into children (nested containers or elements)
				if (child.children) {
					processChildren(child.children, absX, absY);
				}
			} else {
				// Element node: position relative to parent for SvelteFlow
				nodePositions.set(child.id, { x: cx, y: cy });
				absolutePositions.set(child.id, { x: absX, y: absY });
			}
		}
	}

	if (layoutResult.children) {
		processChildren(layoutResult.children, 0, 0);
	}

	// Snap container positions to the 25px grid so they align with SvelteFlow's snapGrid.
	// Only snap containers — element positions are relative to their parent and snapping
	// them independently would break the inter-node spacing ELK computed.
	const SNAP = 25;
	for (const [id, pos] of nodePositions) {
		if (containerIds.has(id)) {
			nodePositions.set(id, {
				x: Math.round(pos.x / SNAP) * SNAP,
				y: Math.round(pos.y / SNAP) * SNAP
			});
		}
	}

	return {
		nodePositions,
		containerSizes,
		elementNodeSizes: input.elementNodeSizes ?? new Map()
	};
}

/**
 * @deprecated Use LayoutGraph.updateElementSize() instead.
 * Kept temporarily for transition — will be removed.
 */
export function applyLocalSizeAdjustment(
	cachedResult: ElkLayoutResult,
	updatedLeafSizes: Map<string, { x: number; y: number }>,
	nodes: TopologyNode[],
	collapsed: Set<string>
): ElkLayoutResult {
	const nodePositions = new Map(cachedResult.nodePositions);
	const containerSizes = new Map(cachedResult.containerSizes);
	const leafNodeSizes = new Map(cachedResult.elementNodeSizes);

	// Indexed once. The container lookup below runs per container, and scanning `nodes` linearly
	// there is quadratic on a graph this size.
	const nodeById = new Map(nodes.map((n) => [n.id, n]));

	// Build leaf→container mapping and container→children mapping
	const leafToContainer = new Map<string, string>();
	const containerChildren = new Map<string, string[]>();
	for (const node of nodes) {
		if (node.node_type === 'Element') {
			const parentId = node.container_id;
			if (parentId && !collapsed.has(parentId)) {
				leafToContainer.set(node.id, parentId);
				if (!containerChildren.has(parentId)) containerChildren.set(parentId, []);
				containerChildren.get(parentId)!.push(node.id);
			}
		}
	}

	// Build parent container map for nested containers
	const parentContainerMap = new Map<string, string>();
	for (const node of nodes) {
		if (node.node_type === 'Container') {
			const parentId = (node as Record<string, unknown>).parent_container_id as string | undefined;
			if (parentId) parentContainerMap.set(node.id, parentId);
		}
	}

	// Find affected containers
	const affectedContainers = new Set<string>();
	for (const [leafId] of updatedLeafSizes) {
		const containerId = leafToContainer.get(leafId);
		if (containerId) affectedContainers.add(containerId);
	}

	// Update leaf sizes
	for (const [id, size] of updatedLeafSizes) {
		leafNodeSizes.set(id, size);
	}

	// For each affected container, rebuild column layout
	for (const containerId of affectedContainers) {
		const childIds = containerChildren.get(containerId) ?? [];
		if (childIds.length === 0) continue;

		// Group children by x-position (column), using ELK-computed positions
		// (from cachedResult, never mutated) for Y sort order, and updated
		// heights for spacing. recomputeColumnY sorts by y then re-stacks,
		// so using computed Y preserves ELK's original column order.
		const columns = new Map<number, ElkNode[]>();
		for (const childId of childIds) {
			const computedPos = cachedResult.nodePositions.get(childId);
			const size = leafNodeSizes.get(childId);
			if (!computedPos || !size) continue;
			const x = computedPos.x;
			if (!columns.has(x)) columns.set(x, []);
			columns.get(x)!.push({
				id: childId,
				x: computedPos.x,
				y: computedPos.y,
				width: size.x,
				height: size.y
			});
		}

		// Detect container type for correct spacing/padding
		const containerNode = nodeById.get(containerId);
		const containerType = (containerNode as Record<string, unknown>)?.container_type as
			string | undefined;
		const ctMeta = containerTypes.getMetadata(containerType ?? 'Subnet');
		const spacing = 25;
		const bottomPad = ctMeta.padding.bottom;

		// Reuse recomputeColumnY: sorts by y (= computed Y = stable order),
		// then re-stacks with updated heights
		let maxColumnBottom = 0;
		for (const [, colNodes] of columns) {
			recomputeColumnY(colNodes, spacing);
			for (const node of colNodes) {
				nodePositions.set(node.id, { x: node.x ?? 0, y: node.y ?? 0 });
			}
			const lastNode = colNodes[colNodes.length - 1];
			const columnBottom = (lastNode.y ?? 0) + (lastNode.height ?? 0);
			if (columnBottom > maxColumnBottom) maxColumnBottom = columnBottom;
		}

		// Update container height
		const newHeight = maxColumnBottom + bottomPad;
		const prevSize = containerSizes.get(containerId);
		if (prevSize) {
			const heightDelta = newHeight - prevSize.height;
			containerSizes.set(containerId, { width: prevSize.width, height: newHeight });

			// If nested in parent, grow parent and shift sibling containers
			const parentId = parentContainerMap.get(containerId);
			if (parentId && heightDelta !== 0) {
				const siblingIds = nodes
					.filter(
						(n) =>
							n.node_type === 'Container' &&
							(n as Record<string, unknown>).parent_container_id === parentId &&
							n.id !== containerId
					)
					.map((n) => n.id);

				const myPos = nodePositions.get(containerId);
				if (myPos) {
					for (const sibId of siblingIds) {
						const sibPos = nodePositions.get(sibId);
						if (sibPos && sibPos.y > myPos.y) {
							nodePositions.set(sibId, { x: sibPos.x, y: sibPos.y + heightDelta });
						}
					}
				}

				// Grow parent container
				const parentSize = containerSizes.get(parentId);
				if (parentSize) {
					containerSizes.set(parentId, {
						width: parentSize.width,
						height: parentSize.height + heightDelta
					});
				}
			}
		}
	}

	return {
		nodePositions,
		containerSizes,
		elementNodeSizes: leafNodeSizes
	};
}

/**
 * Apply local size adjustment when subgroups collapse/expand.
 * Adjusts subgroup sizes and reflections within their parent containers.
 */
export function applySubgroupCollapseAdjustment(
	cachedResult: ElkLayoutResult,
	nodes: TopologyNode[],
	collapsed: Set<string>,
	prevCollapsed: Set<string>
): ElkLayoutResult {
	const nodePositions = new Map(cachedResult.nodePositions);
	const containerSizes = new Map(cachedResult.containerSizes);

	// Find subgroups whose collapse state changed
	const changedSubgroups = new Set<string>();
	for (const node of nodes) {
		if (node.node_type !== 'Container') continue;
		const parentId = (node as Record<string, unknown>).parent_container_id as string | undefined;
		if (!parentId) continue; // only subgroups
		const wasCollapsed = prevCollapsed.has(node.id);
		const isCollapsed = collapsed.has(node.id);
		if (wasCollapsed !== isCollapsed) changedSubgroups.add(node.id);
	}

	if (changedSubgroups.size === 0) return cachedResult;

	// Find affected parent containers
	const affectedParents = new Set<string>();
	for (const node of nodes) {
		if (!changedSubgroups.has(node.id)) continue;
		const parentId = (node as Record<string, unknown>).parent_container_id as string | undefined;
		if (parentId) affectedParents.add(parentId);
	}

	// For each affected parent, recompute child subgroup positions
	for (const parentId of affectedParents) {
		// Gather all children of this parent (subgroups + elements not in subgroups)
		const childContainers = nodes.filter(
			(n) =>
				n.node_type === 'Container' &&
				(n as Record<string, unknown>).parent_container_id === parentId
		);

		// Group children by x-position (column) and restack
		const columns = new Map<number, { id: string; x: number; y: number; height: number }[]>();
		for (const child of childContainers) {
			const pos = nodePositions.get(child.id);
			if (!pos) continue;
			const isCollapsed = collapsed.has(child.id);
			const existingSize = containerSizes.get(child.id);
			const size = isCollapsed
				? { width: existingSize?.width ?? 250, height: 40 }
				: (existingSize ?? { width: 250, height: 100 });
			const x = pos.x;
			if (!columns.has(x)) columns.set(x, []);
			columns.get(x)!.push({ id: child.id, x: pos.x, y: pos.y, height: size.height });
		}

		// Restack each column
		let maxColumnBottom = 0;
		for (const [, colNodes] of columns) {
			colNodes.sort((a, b) => a.y - b.y);
			const startY = colNodes[0].y;
			let y = startY;
			for (const node of colNodes) {
				nodePositions.set(node.id, { x: node.x, y });
				y += node.height + 25; // spacing between subgroups (grid-aligned)
			}
			const lastNode = colNodes[colNodes.length - 1];
			const columnBottom = y - 25 + lastNode.height; // undo last spacing
			if (columnBottom > maxColumnBottom) maxColumnBottom = columnBottom;
		}

		// Update parent container height
		const parentSize = containerSizes.get(parentId);
		if (parentSize) {
			const newHeight = maxColumnBottom + 25; // bottom padding
			containerSizes.set(parentId, { width: parentSize.width, height: newHeight });
		}
	}

	return {
		nodePositions,
		containerSizes,
		elementNodeSizes: cachedResult.elementNodeSizes
	};
}

/**
 * Repack disconnected root containers beside the connected layout.
 * ELK's layered algorithm places disconnected containers in separate layers,
 * wasting vertical space. This post-processing step moves them to available
 * space beside the connected component, producing a denser layout.
 */
function repackDisconnectedContainers(
	result: ElkLayoutResult,
	input: ElkLayoutInput
): ElkLayoutResult {
	// A view that packs every root container itself (rectpacking) or keeps a deliberate root order
	// would only have its groups scattered back out by this shelf-pack.
	if (!getViewLayout(input.view).repack_disconnected) return result;

	// Build element → root container mapping
	const parentContainerMap = new Map<string, string>();
	const allContainerIds = new Set<string>();
	const rootContainerIds = new Set<string>();
	for (const node of input.nodes) {
		if (node.node_type === 'Container') {
			allContainerIds.add(node.id);
			const parentId = (node as Record<string, unknown>).parent_container_id as string | undefined;
			if (parentId) parentContainerMap.set(node.id, parentId);
			else rootContainerIds.add(node.id);
		}
	}

	const elementToRoot = new Map<string, string>();
	for (const node of input.nodes) {
		if (node.node_type === 'Element' && node.container_id) {
			let rootId: string = node.container_id;
			while (parentContainerMap.has(rootId)) rootId = parentContainerMap.get(rootId)!;
			elementToRoot.set(node.id, rootId);
		}
	}

	// Identify connected root containers (have cross-container layout-affecting edges)
	const connectedIds = new Set<string>();
	for (const edge of input.edges) {
		if (!affectsLayout(edge)) continue;
		const srcRoot = resolveToRootContainer(
			edge.source,
			elementToRoot,
			allContainerIds,
			parentContainerMap
		);
		const tgtRoot = resolveToRootContainer(
			edge.target,
			elementToRoot,
			allContainerIds,
			parentContainerMap
		);
		if (srcRoot && tgtRoot && srcRoot !== tgtRoot) {
			connectedIds.add(srcRoot);
			connectedIds.add(tgtRoot);
		}
	}

	// Find disconnected root containers
	const disconnectedIds: string[] = [];
	for (const id of rootContainerIds) {
		if (!connectedIds.has(id) && result.containerSizes.has(id)) {
			disconnectedIds.push(id);
		}
	}

	if (disconnectedIds.length === 0 || connectedIds.size === 0) return result;

	// The connected containers stay where the layout put them; everything else is placed around them.
	const SPACING = 75;
	const VIEW_ASPECT = Number(ROOT_LAYOUT_OPTIONS['elk.aspectRatio']);
	const placed = new Map<string, { x: number; y: number; width: number; height: number }>();
	for (const id of connectedIds) {
		const pos = result.nodePositions.get(id);
		const size = result.containerSizes.get(id);
		if (pos && size) {
			placed.set(id, { x: pos.x, y: pos.y, width: size.width, height: size.height });
		}
	}

	const nodePositions = new Map(result.nodePositions);

	// Drawn edges between root containers, counted per pair. The layout above only saw edges that
	// affect layout, so a container linked by anything else (a dependency in L3, say) landed in the
	// shelf below with no regard for where its partner is. Read from the view's edge config rather
	// than the user's edge toggles, so hiding an edge type never moves a container.
	const links = new Map<string, Map<string, number>>();
	for (const edge of input.edges) {
		const config = getViewConfig(edge);
		if (config.type !== 'active' || config.default_visibility !== 'visible') continue;
		const a = resolveToRootContainer(
			edge.source,
			elementToRoot,
			allContainerIds,
			parentContainerMap
		);
		const b = resolveToRootContainer(
			edge.target,
			elementToRoot,
			allContainerIds,
			parentContainerMap
		);
		if (!a || !b || a === b) continue;
		for (const [from, to] of [
			[a, b],
			[b, a]
		]) {
			if (!links.has(from)) links.set(from, new Map());
			links.get(from)!.set(to, (links.get(from)!.get(to) ?? 0) + 1);
		}
	}

	const overlapsPlaced = (box: { x: number; y: number; width: number; height: number }) => {
		for (const other of placed.values()) {
			if (
				box.x < other.x + other.width + SPACING &&
				other.x < box.x + box.width + SPACING &&
				box.y < other.y + other.height + SPACING &&
				other.y < box.y + box.height + SPACING
			) {
				return other;
			}
		}
		return undefined;
	};

	// The two free slots flush against a placed box: below it, moved down past anything in the way,
	// and right of it, moved right past anything in the way.
	const slotsBeside = (
		p: { x: number; y: number; width: number; height: number },
		size: { width: number; height: number }
	) => {
		const below = { x: p.x, y: p.y + p.height + SPACING, ...size };
		for (let hit = overlapsPlaced(below); hit; hit = overlapsPlaced(below)) {
			below.y = hit.y + hit.height + SPACING;
		}
		const right = { x: p.x + p.width + SPACING, y: p.y, ...size };
		for (let hit = overlapsPlaced(right); hit; hit = overlapsPlaced(right)) {
			right.x = hit.x + hit.width + SPACING;
		}
		return [below, right];
	};

	const place = (id: string, slot: { x: number; y: number; width: number; height: number }) => {
		nodePositions.set(id, { x: slot.x, y: slot.y });
		placed.set(id, slot);
	};

	// Place each linked disconnected container in the free slot, flush below or right of one of its
	// placed partners, that is closest to all of them. The container with the most links to what is
	// already placed goes first (ties by id), so a chain of linked containers is placed outward from
	// the connected group.
	const unplaced = new Set(disconnectedIds);
	for (;;) {
		let next: { id: string; weight: number } | undefined;
		for (const id of [...unplaced].sort()) {
			let weight = 0;
			for (const [partner, count] of links.get(id) ?? []) {
				if (placed.has(partner)) weight += count;
			}
			if (weight > 0 && (!next || weight > next.weight)) next = { id, weight };
		}
		if (!next) break;

		const size = result.containerSizes.get(next.id)!;
		const partners = [...(links.get(next.id) ?? [])].filter(([partner]) => placed.has(partner));
		const distanceTo = (x: number, y: number) =>
			partners.reduce((sum, [partner, count]) => {
				const p = placed.get(partner)!;
				const dx = x + size.width / 2 - (p.x + p.width / 2);
				const dy = y + size.height / 2 - (p.y + p.height / 2);
				return sum + count * Math.hypot(dx, dy);
			}, 0);

		let best:
			{ slot: { x: number; y: number; width: number; height: number }; score: number } | undefined;
		for (const [partner] of partners.sort(([a], [b]) => a.localeCompare(b))) {
			for (const slot of slotsBeside(placed.get(partner)!, size)) {
				const score = distanceTo(slot.x, slot.y);
				if (!best || score < best.score) best = { slot, score };
			}
		}

		place(next.id, best!.slot);
		unplaced.delete(next.id);
	}

	// Pack the rest, tallest first, into whichever free slot beside a placed container grows the
	// bounding box least, so they fill the gaps the layout left before widening it. Ties go to the
	// higher slot, then the one further left.
	const shelved = disconnectedIds.filter((id) => unplaced.has(id));
	shelved.sort((a, b) => {
		const sA = result.containerSizes.get(a)?.height ?? 0;
		const sB = result.containerSizes.get(b)?.height ?? 0;
		return sB - sA || a.localeCompare(b);
	});

	for (const id of shelved) {
		const size = result.containerSizes.get(id);
		if (!size) continue;

		let minX = Infinity;
		let minY = Infinity;
		let maxX = -Infinity;
		let maxY = -Infinity;
		for (const box of placed.values()) {
			minX = Math.min(minX, box.x);
			minY = Math.min(minY, box.y);
			maxX = Math.max(maxX, box.x + box.width);
			maxY = Math.max(maxY, box.y + box.height);
		}
		// The bounding box as a screen fits it: the smallest rectangle of the root layout's aspect
		// ratio that holds it, so a tall, narrow packing costs what it costs to view.
		const areaWith = (slot: { x: number; y: number; width: number; height: number }) => {
			const width = Math.max(maxX, slot.x + slot.width) - Math.min(minX, slot.x);
			const height = Math.max(maxY, slot.y + slot.height) - Math.min(minY, slot.y);
			return Math.max(width, height * VIEW_ASPECT) * Math.max(height, width / VIEW_ASPECT);
		};

		let best:
			{ slot: { x: number; y: number; width: number; height: number }; area: number } | undefined;
		for (const box of placed.values()) {
			for (const slot of slotsBeside(box, size)) {
				const area = areaWith(slot);
				if (
					!best ||
					area < best.area ||
					(area === best.area &&
						(slot.y < best.slot.y || (slot.y === best.slot.y && slot.x < best.slot.x)))
				) {
					best = { slot, area };
				}
			}
		}
		if (best) place(id, best.slot);
	}

	return { ...result, nodePositions };
}

/**
 * Compute layout positions using elkjs compound layered algorithm.
 * Returns positions for all nodes and computed sizes for containers.
 */
export async function computeElkLayout(input: ElkLayoutInput): Promise<ElkLayoutResult> {
	if (input.nodes.length === 0) {
		return {
			nodePositions: new Map(),
			containerSizes: new Map(),
			elementNodeSizes: new Map()
		};
	}

	const elkLoadDone = perf.stage('elk.module-load');
	const elk = await getElk();
	elkLoadDone();

	// Pass 1: compute layout with FIXED_SIDE ports (no position info).
	// This gives us actual element positions within box-packed containers.
	//
	// `graph1` and `pass1Children` are deliberately `let`, and dropped the moment each is finished
	// with. This function is `async`, so its scope is a heap-allocated context that survives every
	// `await` and V8 does not clear bindings it can no longer reach — holding both passes' graphs
	// live at once. Pass 2 builds a second complete graph of the same size, so the peak was twice a
	// single pass even though only one pass's worth is ever retained afterwards. Nulling lets the
	// first be collected while the second is being built.
	//
	// `no-useless-assignment` fires on both clears, and is wrong here in the way it is wrong for any
	// deliberate release: the assignment has no *dataflow* consumer, which is exactly the point —
	// its effect is on reachability, not on any later read.
	const build1Done = perf.stage('elk.build-graph.pass1');
	let graph1: ElkNode | null;
	const { graph: builtGraph1, containerIds } = buildElkGraph(input);
	graph1 = builtGraph1;
	build1Done();
	const elkPass1Done = perf.stage('elk.layout.pass1');
	noteElkRun();
	// Only the children array is bound, never the root: the root is unreferenced the moment this
	// expression completes, and clearing this one binding after extraction drops the entire pass-1
	// tree. Binding the result itself would keep the tree alive through `.children` anyway.
	let pass1Children: ElkNode[] | null = (await elk.layout(graph1)).children ?? null;
	// eslint-disable-next-line no-useless-assignment -- releases the pass-1 graph for collection
	graph1 = null;
	elkPass1Done();

	// Extract actual element AND subcontainer positions from pass 1
	const elementPositions = new Map<
		string,
		{ x: number; y: number; w: number; h: number; containerW: number; containerH: number }
	>();
	const subcontainerPositions = new Map<string, { x: number; y: number; h: number }>();
	function extractPositions(children: ElkNode[]) {
		for (const child of children) {
			if (containerIds.has(child.id)) {
				// Container: record its position and width, recurse into children
				subcontainerPositions.set(child.id, {
					x: child.x ?? 0,
					y: child.y ?? 0,
					h: child.height ?? 0
				});
				if (child.children) {
					for (const elem of child.children) {
						if (!containerIds.has(elem.id)) {
							elementPositions.set(elem.id, {
								x: elem.x ?? 0,
								y: elem.y ?? 0,
								w: elem.width ?? 0,
								h: elem.height ?? 0,
								containerW: child.width ?? 0,
								containerH: child.height ?? 0
							});
						}
					}
					extractPositions(child.children);
				}
			}
		}
	}
	// Where each root container ends up after pass 1, including the shelf-packing of containers no
	// layout edge connects: an element's side in its container is chosen against these. Pass 1's own
	// positions for an unconnected container say nothing about where it is finally drawn.
	const rootBoxes = new Map<string, { x: number; y: number; w: number; h: number }>();
	if (pass1Children) {
		const children = pass1Children;
		extractPositions(children);
		const placed = repackDisconnectedContainers(
			mapElkResults({ id: 'pass1', children }, containerIds, input),
			input
		);
		for (const { id } of children) {
			const pos = placed.nodePositions.get(id);
			const size = placed.containerSizes.get(id);
			if (pos && size) rootBoxes.set(id, { x: pos.x, y: pos.y, w: size.width, h: size.height });
		}
	}
	// Everything pass 2 needs from pass 1 now lives in the position maps above, which hold plain
	// numbers rather than ELK nodes. Released here so the collector can reclaim the whole first
	// graph before the second one is allocated.
	// eslint-disable-next-line no-useless-assignment -- releases the pass-1 tree for collection
	pass1Children = null;

	await input.onFirstPassDone?.();

	// Pass 2: rebuild graph with FIXED_POS ports at actual element positions
	const build2Done = perf.stage('elk.build-graph.pass2');
	const { graph: graph2, containerIds: cids2 } = buildElkGraph(
		input,
		elementPositions,
		subcontainerPositions,
		rootBoxes
	);
	build2Done();
	const elkPass2Done = perf.stage('elk.layout.pass2');
	noteElkRun();
	const result2 = await elk.layout(graph2);
	elkPass2Done();

	// Top-align layers by shifting each layer's top node to the same Y.
	// ELK centers layers independently, causing vertical misalignment.
	if (getViewLayout(input.view).align_layers_top && result2.children) {
		// Group children by X-band (layer). Merge bands whose containers
		// would horizontally overlap after top-alignment — without merging,
		// containers in nearby but distinct X-bands both get y=TOP and overlap.
		const bandMap = new Map<number, ElkNode[]>();
		for (const child of result2.children) {
			const layerX = Math.round((child.x ?? 0) / 50) * 50;
			if (!bandMap.has(layerX)) bandMap.set(layerX, []);
			bandMap.get(layerX)!.push(child);
		}

		// Merge bands that would overlap: sort by X, merge if any container
		// in band A horizontally overlaps with any container in band B.
		const sortedBands = Array.from(bandMap.entries()).sort(([a], [b]) => a - b);
		const mergedLayers: ElkNode[][] = [];
		for (const [, nodes] of sortedBands) {
			if (mergedLayers.length === 0) {
				mergedLayers.push(nodes);
				continue;
			}
			const prev = mergedLayers[mergedLayers.length - 1];
			// Check if any node in this band overlaps horizontally with prev band
			const prevRight = Math.max(...prev.map((n) => (n.x ?? 0) + (n.width ?? 0)));
			const thisLeft = Math.min(...nodes.map((n) => n.x ?? 0));
			if (thisLeft < prevRight) {
				// Overlapping — merge into previous layer
				prev.push(...nodes);
			} else {
				mergedLayers.push(nodes);
			}
		}

		// Top-align each merged layer and re-stack with consistent gaps
		const SNAP = 25;
		const GAP = 50;
		const TOP = 25;
		for (const layerNodes of mergedLayers) {
			layerNodes.sort((a, b) => (a.y ?? 0) - (b.y ?? 0));
			let y = TOP;
			for (const node of layerNodes) {
				// Snap Y to grid, then compute next from snapped position
				node.y = Math.ceil(y / SNAP) * SNAP;
				y = node.y + (node.height ?? 0) + GAP;
			}
		}
	}

	const layoutResult = mapElkResults(result2, cids2, input);
	return repackDisconnectedContainers(layoutResult, input);
}
