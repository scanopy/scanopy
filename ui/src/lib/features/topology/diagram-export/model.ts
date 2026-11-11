/**
 * Format-neutral model of the drawn topology, which the draw.io and Visio serializers both read.
 *
 * It is built from the flow graph as the export capture draws it: the nodes in the store (view,
 * filters and collapse are already applied upstream) and only the edges the canvas draws. Text,
 * colours and icons come from a `DiagramDescriber`, so this module stays free of stores and the
 * structural rules (containment, edge endpoints, what is left out) are testable on their own.
 */

/** A side of a shape, matching the flow handle ids. */
export type DiagramSide = 'Top' | 'Right' | 'Bottom' | 'Left';

export const DIAGRAM_SIDES: readonly DiagramSide[] = ['Top', 'Right', 'Bottom', 'Left'];

/** Space around the content, in flow units, matching the raster exports' crop. */
export const DIAGRAM_PADDING = 40;
/** Height reserved above the content for the header block. */
export const DIAGRAM_HEADER_HEIGHT = 56;

/** A rendered flow node, with its absolute position and measured size. */
export interface DiagramSourceNode {
	id: string;
	type: 'Container' | 'Element';
	parentId?: string;
	x: number;
	y: number;
	width: number;
	height: number;
}

/** A flow edge, as handed to SvelteFlow. */
export interface DiagramSourceEdge {
	id: string;
	source: string;
	target: string;
	sourceHandle?: string | null;
	targetHandle?: string | null;
}

/** The text in a part: one run, in one colour and size. */
export interface DiagramText {
	value: string;
	color: string;
	/** Font size in diagram units. */
	size: number;
	bold?: boolean;
	align?: 'left' | 'center' | 'right';
	/** Space before the text on its aligned side, e.g. past a pill's icon. */
	inset?: number;
	/** Break onto further lines at the part's width. Otherwise the text is one line. */
	wrap?: boolean;
}

/** How a shape's or a part's outline is drawn. A missing `fill` or `stroke` leaves it undrawn. */
export interface DiagramBox {
	outline: 'rect' | 'ellipse';
	/** Corner radius of a `rect`, in diagram units. */
	radius?: number;
	fill?: string;
	stroke?: string;
	strokeWidth?: number;
	dashed?: boolean;
}

/** What a part is, so a reader of the file (and a test) can tell a status dot from a badge. */
export type DiagramPartRole =
	| 'accent'
	| 'tile'
	| 'glyph'
	| 'status'
	| 'badge'
	| 'os'
	| 'freshness'
	| 'title'
	| 'label'
	| 'tag'
	| 'text'
	| 'group'
	| 'row';

/**
 * Something a shape draws besides its own box and label: a tile, a badge, a title pill. Placed
 * relative to the shape's top-left, and free to sit outside it (a container's title pill sits
 * above its box).
 */
export interface DiagramPart extends DiagramBox {
	role: DiagramPartRole;
	x: number;
	y: number;
	width: number;
	height: number;
	/** Key into `DiagramModel.icons`: the part is that image, drawn to fill its box. */
	iconKey?: string;
	text?: DiagramText;
}

/** A named fact about the entity a shape stands for, written as Visio Shape Data and draw.io properties. */
export interface DiagramField {
	label: string;
	value: string;
}

export interface DiagramNodeDescription {
	box: DiagramBox;
	parts: DiagramPart[];
	data?: DiagramField[];
}

export interface DiagramEdgeDescription {
	label?: string;
	/** The label's text colour and the card it sits on. */
	labelStyle?: { color: string; size: number; fill: string; stroke: string };
	color: string;
	width: number;
	dash: 'solid' | 'dashed' | 'dotted';
	/** The canvas draws the edge as a curve, not as right-angled segments. */
	curved?: boolean;
	data?: DiagramField[];
}

export interface DiagramDescriber {
	node(node: DiagramSourceNode): DiagramNodeDescription;
	/** `null` when the canvas leaves the edge undrawn. */
	edge(edge: DiagramSourceEdge): DiagramEdgeDescription | null;
}

export interface DiagramIcon {
	/** Image as a data URI (SVG or raster), for formats that embed it directly. */
	dataUri: string;
	/** PNG bytes, for formats that only take bitmaps. */
	png?: Uint8Array;
}

export interface DiagramShape {
	id: string;
	parentId: string | null;
	type: 'container' | 'element';
	/** Absolute, in diagram units, origin at the diagram's top-left. */
	x: number;
	y: number;
	width: number;
	height: number;
	box: DiagramBox;
	parts: DiagramPart[];
	data: DiagramField[];
}

export interface DiagramConnector {
	id: string;
	sourceId: string;
	targetId: string;
	sourceSide: DiagramSide;
	targetSide: DiagramSide;
	label?: string;
	labelStyle?: DiagramEdgeDescription['labelStyle'];
	color: string;
	width: number;
	dash: 'solid' | 'dashed' | 'dotted';
	curved: boolean;
	data: DiagramField[];
}

export interface DiagramHeader {
	title: string;
	subtitle: string;
	color: string;
	subtitleColor: string;
}

export interface DiagramModel {
	name: string;
	width: number;
	height: number;
	background: string;
	header: DiagramHeader | null;
	/** Parents before their children. */
	shapes: DiagramShape[];
	connectors: DiagramConnector[];
	icons: Map<string, DiagramIcon>;
}

export interface DiagramModelInput {
	name: string;
	nodes: DiagramSourceNode[];
	edges: DiagramSourceEdge[];
	describe: DiagramDescriber;
	background: string;
	header?: DiagramHeader | null;
	icons?: Map<string, DiagramIcon>;
}

function toSide(handle: string | null | undefined, fallback: DiagramSide): DiagramSide {
	return DIAGRAM_SIDES.includes(handle as DiagramSide) ? (handle as DiagramSide) : fallback;
}

/** Order nodes so every parent precedes its children; nodes whose parent is absent become roots. */
function orderParentsFirst(nodes: DiagramSourceNode[]): DiagramSourceNode[] {
	const byId = new Map(nodes.map((n) => [n.id, n]));
	const ordered: DiagramSourceNode[] = [];
	const placed = new Set<string>();
	const place = (node: DiagramSourceNode, trail: Set<string>) => {
		if (placed.has(node.id) || trail.has(node.id)) return;
		trail.add(node.id);
		const parent = node.parentId ? byId.get(node.parentId) : undefined;
		if (parent) place(parent, trail);
		placed.add(node.id);
		ordered.push(node);
	};
	for (const node of nodes) place(node, new Set());
	return ordered;
}

export function buildDiagramModel(input: DiagramModelInput): DiagramModel {
	const nodes = orderParentsFirst(input.nodes);
	const nodeIds = new Set(nodes.map((n) => n.id));
	const header = input.header ?? null;

	// A loop, not Math.min(...): a spread of ~20k arguments is near the engine limits.
	let minX = Infinity;
	let minY = Infinity;
	let maxX = -Infinity;
	let maxY = -Infinity;
	const descriptions = nodes.map((node) => input.describe.node(node));
	for (const [i, n] of nodes.entries()) {
		minX = Math.min(minX, n.x);
		minY = Math.min(minY, n.y);
		maxX = Math.max(maxX, n.x + n.width);
		maxY = Math.max(maxY, n.y + n.height);
		// A part can sit outside its shape (a title pill above a container), so it counts too.
		for (const part of descriptions[i].parts) {
			minX = Math.min(minX, n.x + part.x);
			minY = Math.min(minY, n.y + part.y);
			maxX = Math.max(maxX, n.x + part.x + part.width);
			maxY = Math.max(maxY, n.y + part.y + part.height);
		}
	}
	if (nodes.length === 0) minX = minY = maxX = maxY = 0;
	const offsetX = DIAGRAM_PADDING - minX;
	const offsetY = DIAGRAM_PADDING + (header ? DIAGRAM_HEADER_HEIGHT : 0) - minY;

	const shapes: DiagramShape[] = nodes.map((node, i) => {
		const description = descriptions[i];
		const parentId = node.parentId && nodeIds.has(node.parentId) ? node.parentId : null;
		return {
			id: node.id,
			parentId,
			type: node.type === 'Container' ? 'container' : 'element',
			x: node.x + offsetX,
			y: node.y + offsetY,
			width: node.width,
			height: node.height,
			box: description.box,
			parts: description.parts,
			data: description.data ?? []
		};
	});

	const connectors: DiagramConnector[] = [];
	for (const edge of input.edges) {
		if (!nodeIds.has(edge.source) || !nodeIds.has(edge.target)) continue;
		if (edge.source === edge.target) continue;
		const description = input.describe.edge(edge);
		if (!description) continue;
		connectors.push({
			id: edge.id,
			sourceId: edge.source,
			targetId: edge.target,
			sourceSide: toSide(edge.sourceHandle, 'Bottom'),
			targetSide: toSide(edge.targetHandle, 'Top'),
			label: description.label || undefined,
			labelStyle: description.labelStyle,
			color: description.color,
			width: description.width,
			dash: description.dash,
			curved: description.curved ?? false,
			data: description.data ?? []
		});
	}

	return {
		name: input.name,
		width: maxX - minX + DIAGRAM_PADDING * 2,
		height: maxY - minY + DIAGRAM_PADDING * 2 + (header ? DIAGRAM_HEADER_HEIGHT : 0),
		background: input.background,
		header,
		shapes,
		connectors,
		icons: input.icons ?? new Map()
	};
}

/** Where a side's midpoint sits, as a fraction of the shape's width and height. */
export function sideAnchor(side: DiagramSide): { x: number; y: number } {
	switch (side) {
		case 'Top':
			return { x: 0.5, y: 0 };
		case 'Right':
			return { x: 1, y: 0.5 };
		case 'Bottom':
			return { x: 0.5, y: 1 };
		case 'Left':
			return { x: 0, y: 0.5 };
	}
}
