/**
 * LayoutGraph: class-based layout state for topology visualization.
 *
 * Replaces the scattered Maps (nodePositions, containerSizes, elementNodeSizes, etc.)
 * with a proper object graph. Each container/element stores its own position, size,
 * and collapse state — eliminating the "lost original size" bugs and map-juggling.
 */

import type { TopologyNode } from '../types/base';
import { containerTypes } from '$lib/shared/stores/metadata';

const CHILD_SPACING = 30;

export class LayoutElement {
	id: string;
	node: TopologyNode;
	container: LayoutContainer | null = null;
	position: { x: number; y: number } = { x: 0, y: 0 };
	/** Current measured size from DOM */
	size: { x: number; y: number };
	portsExpanded = false;

	constructor(node: TopologyNode) {
		this.id = node.id;
		this.node = node;
		this.size = { x: node.size?.x ?? 250, y: node.size?.y ?? 100 };
	}

	get width(): number {
		return this.size.x;
	}
	get height(): number {
		return this.size.y;
	}
}

export class LayoutContainer {
	id: string;
	node: TopologyNode;
	parent: LayoutContainer | null = null;
	childContainers: LayoutContainer[] = [];
	childElements: LayoutElement[] = [];
	position: { x: number; y: number } = { x: 0, y: 0 };
	/** Size computed by ELK when expanded */
	expandedSize: { width: number; height: number } = { width: 0, height: 0 };
	collapsed = false;
	/** DOM-measured size when collapsed, set after first layout pass */
	measuredCollapsedSize: { width: number; height: number } | null = null;
	isSubcontainer: boolean;
	containerType: string;

	constructor(node: TopologyNode) {
		this.id = node.id;
		this.node = node;
		this.containerType = ((node as Record<string, unknown>).container_type as string) ?? 'Subnet';
		this.isSubcontainer = containerTypes.getMetadata(this.containerType).is_subcontainer;
	}

	get collapsedSize(): { width: number; height: number } {
		if (this.measuredCollapsedSize) return { ...this.measuredCollapsedSize };
		const meta = containerTypes.getMetadata(this.containerType);
		return { ...meta.collapsed_size };
	}

	get size(): { width: number; height: number } {
		return this.collapsed ? this.collapsedSize : this.expandedSize;
	}

	/** All children (containers + elements) for layout purposes */
	get allChildren(): (LayoutContainer | LayoutElement)[] {
		return [...this.childContainers, ...this.childElements];
	}

	/** Recursive count of element nodes (for collapsed badge) */
	get childCount(): number {
		let count = this.childElements.length;
		for (const child of this.childContainers) {
			count += child.childCount;
		}
		return count;
	}

	/**
	 * Reflow children within this container after a size change.
	 * If changedChildId is provided, that child keeps its position and only
	 * siblings below it shift by the height delta (stable local adjustment).
	 * Otherwise, restacks all children from their current positions.
	 * Returns the height delta so callers can propagate.
	 */
	reflowChildren(changedChildId?: string): number {
		const children = this.allChildren;
		if (children.length === 0) return 0;

		// Group all children (elements + subgroups) by x-position
		const columns = new Map<
			number,
			{ child: LayoutContainer | LayoutElement; y: number; height: number }[]
		>();
		for (const child of children) {
			const pos = child.position;
			const x = pos.x;
			const height = child instanceof LayoutContainer ? child.size.height : child.height;
			if (!columns.has(x)) columns.set(x, []);
			columns.get(x)!.push({ child, y: pos.y, height });
		}

		if (changedChildId) {
			// Stable reflow: find the changed child, shift only siblings below it
			for (const [, colNodes] of columns) {
				colNodes.sort((a, b) => a.y - b.y);
				const changedIdx = colNodes.findIndex((n) => n.child.id === changedChildId);
				if (changedIdx === -1) continue;

				// Restack from the changed node downward
				let y = colNodes[changedIdx].child.position.y + colNodes[changedIdx].height + CHILD_SPACING;
				for (let i = changedIdx + 1; i < colNodes.length; i++) {
					colNodes[i].child.position = { x: colNodes[i].child.position.x, y };
					y += colNodes[i].height + CHILD_SPACING;
				}
			}
		} else {
			// Full reflow: restack each column from the first node's position
			for (const [, colNodes] of columns) {
				colNodes.sort((a, b) => a.y - b.y);
				const startY = colNodes[0].y;
				let y = startY;
				for (const entry of colNodes) {
					entry.child.position = { x: entry.child.position.x, y };
					y += entry.height + CHILD_SPACING;
				}
			}
		}

		// Recompute container height from all columns.
		// Width is NOT recomputed here — ELK handles width via box
		// packing on every collapse change.
		let maxColumnBottom = 0;
		for (const [, colNodes] of columns) {
			const last = colNodes[colNodes.length - 1];
			const bottom = last.child.position.y + last.height;
			if (bottom > maxColumnBottom) maxColumnBottom = bottom;
		}

		const bottomPad = containerTypes.getMetadata(this.containerType).padding.bottom;
		const newHeight = maxColumnBottom + bottomPad;
		const oldHeight = this.expandedSize.height;
		this.expandedSize = { width: this.expandedSize.width, height: newHeight };
		return newHeight - oldHeight;
	}
}

export class LayoutGraph {
	containers = new Map<string, LayoutContainer>();
	elements = new Map<string, LayoutElement>();

	// Lazy memo of getAbsolutePosition. Invalidated whenever a mutation path
	// (applyElkResult / applyForceResult / updateElementSize / collapse / expand)
	// touches container.position or element.position. Populated on first read.
	private absolutePositionsCache = new Map<string, { x: number; y: number }>();

	private invalidateAbsoluteCache(): void {
		this.absolutePositionsCache.clear();
	}

	/** Build graph from topology nodes */
	static fromTopology(nodes: TopologyNode[]): LayoutGraph {
		const graph = new LayoutGraph();

		// Create all containers
		for (const node of nodes) {
			if (node.node_type === 'Container') {
				graph.containers.set(node.id, new LayoutContainer(node));
			}
		}

		// Create all elements and link to containers
		for (const node of nodes) {
			if (node.node_type === 'Element') {
				const element = new LayoutElement(node);
				const parentId =
					(node as Record<string, unknown>).container_id ??
					(node as Record<string, unknown>).subnet_id;
				if (typeof parentId === 'string') {
					const container = graph.containers.get(parentId);
					if (container) {
						element.container = container;
						container.childElements.push(element);
					}
				}
				graph.elements.set(node.id, element);
			}
		}

		// Link parent-child container relationships
		for (const container of graph.containers.values()) {
			const parentId = (container.node as Record<string, unknown>).parent_container_id as
				string | undefined;
			if (parentId) {
				const parent = graph.containers.get(parentId);
				if (parent) {
					container.parent = parent;
					parent.childContainers.push(container);
				}
			}
		}

		return graph;
	}

	/** Apply positions and sizes from ELK layout result */
	applyElkResult(
		nodePositions: Map<string, { x: number; y: number }>,
		containerSizes: Map<string, { width: number; height: number }>,
		elementNodeSizes: Map<string, { x: number; y: number }>
	): void {
		for (const [id, pos] of nodePositions) {
			const container = this.containers.get(id);
			if (container) {
				container.position = { ...pos };
				if (container.collapsed) {
					// Store the ELK-assigned collapsed size (from DOM measurement)
					const size = containerSizes.get(id);
					if (size) container.measuredCollapsedSize = { ...size };
				} else {
					const size = containerSizes.get(id);
					if (size) container.expandedSize = { ...size };
				}
			}
			const element = this.elements.get(id);
			if (element) {
				element.position = { ...pos };
				const size = elementNodeSizes.get(id);
				if (size) element.size = { ...size };
			}
		}
		this.invalidateAbsoluteCache();
	}

	/** Get node position (works for both containers and elements) */
	getPosition(nodeId: string): { x: number; y: number } | undefined {
		return this.containers.get(nodeId)?.position ?? this.elements.get(nodeId)?.position;
	}

	/** Get absolute position by accumulating parent offsets (ELK stores positions relative to parent) */
	getAbsolutePosition(nodeId: string): { x: number; y: number } | undefined {
		const cached = this.absolutePositionsCache.get(nodeId);
		if (cached) return { ...cached };

		const container = this.containers.get(nodeId);
		if (container) {
			let x = container.position.x;
			let y = container.position.y;
			let parent = container.parent;
			while (parent) {
				x += parent.position.x;
				y += parent.position.y;
				parent = parent.parent;
			}
			const result = { x, y };
			this.absolutePositionsCache.set(nodeId, result);
			return { ...result };
		}
		const element = this.elements.get(nodeId);
		if (element) {
			let x = element.position.x;
			let y = element.position.y;
			let parent = element.container;
			while (parent) {
				x += parent.position.x;
				y += parent.position.y;
				parent = parent.parent;
			}
			const result = { x, y };
			this.absolutePositionsCache.set(nodeId, result);
			return { ...result };
		}
		return undefined;
	}

	/**
	 * Ancestor container IDs for an element or container (excluding `id` itself).
	 * Walks LayoutElement.container / LayoutContainer.parent to the root.
	 */
	ancestorIdsOf(id: string): Set<string> {
		const set = new Set<string>();
		const element = this.elements.get(id);
		const container = this.containers.get(id);
		let parent: LayoutContainer | null = element?.container ?? container?.parent ?? null;
		while (parent) {
			set.add(parent.id);
			parent = parent.parent;
		}
		return set;
	}

	/**
	 * Flat list of absolute-positioned rects for every container and element
	 * in the graph. Used by the handle picker to score candidates against
	 * potential node crossings.
	 */
	getAllNodeRects(): Array<{ id: string; x: number; y: number; w: number; h: number }> {
		const out: Array<{ id: string; x: number; y: number; w: number; h: number }> = [];
		for (const [id, container] of this.containers) {
			const pos = this.getAbsolutePosition(id);
			if (!pos) continue;
			const size = container.size;
			out.push({ id, x: pos.x, y: pos.y, w: size.width, h: size.height });
		}
		for (const [id, element] of this.elements) {
			const pos = this.getAbsolutePosition(id);
			if (!pos) continue;
			out.push({ id, x: pos.x, y: pos.y, w: element.size.x, h: element.size.y });
		}
		return out;
	}

	/** Get container size (respects collapsed state), or undefined if it has none yet.
	 *
	 * `LayoutContainer.expandedSize` starts at `{0, 0}` and is only filled in once ELK has sized
	 * the container, so an expanded container that has not been through a layout returns a
	 * real-looking zero. Callers treat the result as a size — `build-flow-nodes` puts it straight
	 * on the node — and a 0×0 container renders as nothing with its contents clipped out of it,
	 * which is indistinguishable from a blank canvas. `undefined` is the honest answer and the one
	 * every caller already handles: it falls back to a measured hint or leaves the node unsized so
	 * the DOM measures it for real.
	 */
	getContainerSize(containerId: string): { width: number; height: number } | undefined {
		const size = this.containers.get(containerId)?.size;
		if (!size || size.width <= 0 || size.height <= 0) return undefined;
		return size;
	}

	/** Get container expanded size (ignores collapsed state) */
	getExpandedSize(containerId: string): { width: number; height: number } | undefined {
		const container = this.containers.get(containerId);
		return container && container.expandedSize.width > 0 ? container.expandedSize : undefined;
	}

	/** Get expanded sizes for all containers (for preserving across rebuilds) */
	getExpandedContainerSizes(): Map<string, { width: number; height: number }> {
		const sizes = new Map<string, { width: number; height: number }>();
		for (const [id, container] of this.containers) {
			if (container.expandedSize.width > 0) {
				sizes.set(id, { ...container.expandedSize });
			}
		}
		return sizes;
	}

	/**
	 * Restore expanded sizes from a previous layout, so a rebuild does not lose them.
	 *
	 * Applies to expanded containers as well as collapsed ones. It used to skip anything not
	 * collapsed, which quietly made a rebuild destructive at exactly the moment it hurts most: a
	 * rebuild recreates every `LayoutContainer` with `expandedSize` back at `{0, 0}`, and at
	 * collapse level 4 nothing is collapsed, so nothing was restored. Any run that rebuilt the
	 * graph without also re-running ELK then left `getContainerSize` returning zero, the nodes
	 * built with `width: 0, height: 0`, and the containers rendering as 2px slivers with their
	 * contents outside — persistently, until something forced a fresh layout. A customer's report
	 * caught 258 such containers appearing on a single mid-session pipeline run, 33 seconds after
	 * the same graph had rendered correctly.
	 *
	 * A restored size can be stale if the container's children changed, but ELK overwrites it the
	 * moment it runs; the value only has to be better than zero, and zero is never right.
	 */
	restoreExpandedSizes(sizes: Map<string, { width: number; height: number }>): void {
		for (const [id, size] of sizes) {
			const container = this.containers.get(id);
			if (container) {
				container.expandedSize = { ...size };
			}
		}
	}

	/** Get child positions for all containers with valid expanded sizes (for preserving across rebuilds) */
	getContainerChildPositions(): Map<string, Map<string, { x: number; y: number }>> {
		const positions = new Map<string, Map<string, { x: number; y: number }>>();
		for (const [id, container] of this.containers) {
			if (container.expandedSize.width > 0) {
				const childPos = new Map<string, { x: number; y: number }>();
				for (const child of container.childElements) {
					childPos.set(child.id, { ...child.position });
				}
				for (const child of container.childContainers) {
					childPos.set(child.id, { ...child.position });
				}
				if (childPos.size > 0) {
					positions.set(id, childPos);
				}
			}
		}
		return positions;
	}

	/** Restore child positions for collapsed containers (after graph rebuild where ELK skipped them) */
	restoreContainerChildPositions(
		positions: Map<string, Map<string, { x: number; y: number }>>
	): void {
		for (const [containerId, childPositions] of positions) {
			const container = this.containers.get(containerId);
			if (container && container.collapsed) {
				for (const [childId, pos] of childPositions) {
					const element = this.elements.get(childId);
					if (element) {
						element.position = { ...pos };
					}
					const childContainer = this.containers.get(childId);
					if (childContainer) {
						childContainer.position = { ...pos };
					}
				}
			}
		}
	}

	/** Get element size */
	getElementSize(elementId: string): { x: number; y: number } | undefined {
		return this.elements.get(elementId)?.size;
	}

	/** Get child count for a container (recursive) */
	getChildCount(containerId: string): number {
		return this.containers.get(containerId)?.childCount ?? 0;
	}

	/**
	 * Collapse a container. If it has child containers, collapse them too.
	 * Returns the set of all collapsed container IDs.
	 */
	collapse(containerId: string): Set<string> {
		const affected = new Set<string>();
		const container = this.containers.get(containerId);
		if (!container || container.collapsed) return affected;

		container.collapsed = true;
		affected.add(containerId);

		// Cascade: collapse child containers
		for (const child of container.childContainers) {
			if (!child.collapsed) {
				child.collapsed = true;
				affected.add(child.id);
			}
		}

		// Reflow parent if this is a subgroup — keep this container in place, shift siblings
		if (container.parent && !container.parent.collapsed) {
			const delta = container.parent.reflowChildren(containerId);
			if (delta !== 0 && container.parent.parent) {
				this.propagateResize(container.parent);
			}
		}

		this.invalidateAbsoluteCache();
		return affected;
	}

	/**
	 * Expand a container. Also expands child containers.
	 * Returns the set of all expanded container IDs.
	 */
	expand(containerId: string): Set<string> {
		const affected = new Set<string>();
		const container = this.containers.get(containerId);
		if (!container || !container.collapsed) return affected;

		container.collapsed = false;
		affected.add(containerId);

		// Children stay collapsed when parent expands — no cascade needed

		// Recompute this container's expandedSize based on current child states,
		// since children may have been collapsed during the earlier collapse cascade
		container.reflowChildren();

		// Reflow parent if this is a subgroup — keep this container in place, shift siblings
		if (container.parent && !container.parent.collapsed) {
			const delta = container.parent.reflowChildren(containerId);
			if (delta !== 0 && container.parent.parent) {
				this.propagateResize(container.parent);
			}
		}

		this.invalidateAbsoluteCache();
		return affected;
	}

	/**
	 * Update an element's size (e.g., after port expansion).
	 * Reflows its container and propagates upward.
	 */
	updateElementSize(elementId: string, newSize: { x: number; y: number }): void {
		const element = this.elements.get(elementId);
		if (!element) return;
		element.size = { ...newSize };

		// Reflow the element's container — keep this element in place, shift siblings below
		const container = element.container;
		if (container && !container.collapsed) {
			const delta = container.reflowChildren(elementId);
			if (delta !== 0) {
				this.propagateResize(container);
			}
		}

		this.invalidateAbsoluteCache();
	}

	/**
	 * Propagate a container's size change to its parent.
	 * Shifts siblings below and grows the parent.
	 */
	private propagateResize(container: LayoutContainer): void {
		const parent = container.parent;
		if (!parent || parent.collapsed) return;

		const delta = parent.reflowChildren();
		if (delta !== 0 && parent.parent) {
			this.propagateResize(parent);
		}
	}

	/**
	 * Get visible nodes (filtering out children of collapsed containers).
	 */
	getVisibleNodes(allNodes: TopologyNode[]): TopologyNode[] {
		const collapsedIds = new Set<string>();
		for (const c of this.containers.values()) {
			if (c.collapsed) collapsedIds.add(c.id);
		}

		if (collapsedIds.size === 0) return allNodes;

		return allNodes.filter((node) => {
			// Hidden if ANY ancestor container is collapsed — not just the direct
			// parent. A collapsed root must hide grandchildren even when the
			// intermediate subcontainer isn't itself in the collapsed set (e.g.
			// level 3 / auto-collapse add only the root). Mirrors the transitive
			// logic edge aggregation already uses. A collapsed container's own id
			// isn't in its ancestor set, so it still renders (as collapsed).
			for (const ancestorId of this.ancestorIdsOf(node.id)) {
				if (collapsedIds.has(ancestorId)) return false;
			}
			return true;
		});
	}

	/**
	 * Get subgroup summaries for a container (for collapsed subnet display).
	 */
	getSubgroupSummaries(containerId: string): { groupId: string; childCount: number }[] {
		const container = this.containers.get(containerId);
		if (!container) return [];
		return container.childContainers.map((child) => ({
			groupId: child.id,
			childCount: child.childCount
		}));
	}

	/**
	 * Get the set of all collapsed container IDs.
	 */
	getCollapsedIds(): Set<string> {
		const ids = new Set<string>();
		for (const c of this.containers.values()) {
			if (c.collapsed) ids.add(c.id);
		}
		return ids;
	}

	/**
	 * Sync collapse state from an external Set (e.g., the collapsedContainers store).
	 * Returns true if anything changed.
	 */
	syncCollapseState(externalCollapsed: Set<string>): boolean {
		let changed = false;
		for (const container of this.containers.values()) {
			const shouldBeCollapsed = externalCollapsed.has(container.id);
			if (container.collapsed !== shouldBeCollapsed) {
				if (shouldBeCollapsed) {
					this.collapse(container.id);
				} else {
					this.expand(container.id);
				}
				changed = true;
			}
		}
		return changed;
	}

	/**
	 * Check if a node ID belongs to a subgroup container.
	 */
	isSubcontainer(nodeId: string): boolean {
		return this.containers.get(nodeId)?.isSubcontainer ?? false;
	}

	/** Apply positions from force layout (collapsed containers only) */
	applyForceResult(
		nodePositions: Map<string, { x: number; y: number }>,
		measuredSizes?: Map<string, { x: number; y: number }>
	): void {
		for (const [id, pos] of nodePositions) {
			const container = this.containers.get(id);
			if (container) {
				container.position = { ...pos };
				const measured = measuredSizes?.get(id);
				if (measured && container.collapsed) {
					container.measuredCollapsedSize = { width: measured.x, height: measured.y };
				}
			}
		}
		this.invalidateAbsoluteCache();
	}
}
