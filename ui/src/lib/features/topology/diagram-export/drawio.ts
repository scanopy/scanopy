/**
 * draw.io (mxGraph XML) serializer for a `DiagramModel`.
 *
 * Containers are container cells that hold their children, so moving one in draw.io moves its
 * contents. Each part of a shape (a tile, a badge, a title pill) is a child cell of that shape,
 * so it moves with it and stays editable. Edges name their source and target cells, so they stay
 * attached when either moves.
 *
 * Every size and colour comes from the model; this file decides only how draw.io spells them.
 */

import {
	DIAGRAM_PADDING,
	sideAnchor,
	type DiagramBox,
	type DiagramConnector,
	type DiagramField,
	type DiagramModel,
	type DiagramPart,
	type DiagramShape
} from './model';
import { overlayDashArray } from '../layout/edge-classification';
import { STYLE } from './style';
import { escapeXml, fieldKeys, hexColor, num } from './xml';

const ROOT_ID = '0';
const LAYER_ID = '1';

export interface DrawioCell {
	id: string;
	parent: string;
	value: string;
	style: string;
	vertex?: boolean;
	edge?: boolean;
	source?: string;
	target?: string;
	/** Relative to the parent cell for vertices; unused for edges. */
	geometry?: { x: number; y: number; width: number; height: number };
	/** Custom properties, shown in draw.io's Edit Data dialog. */
	data?: { key: string; value: string }[];
}

const shapeCellId = (id: string) => `n-${id}`;
const partCellId = (id: string, index: number) => `p-${id}-${index}`;
const connectorCellId = (id: string) => `e-${id}`;

type StyleEntries = Record<string, string | number | undefined>;

function style(entries: StyleEntries): string {
	return Object.entries(entries)
		.filter(([, v]) => v !== undefined)
		.map(([k, v]) => `${k}=${v}`)
		.join(';');
}

/** draw.io styles use `;` as a separator, so embedded images drop the `;base64` marker. */
function styleImage(dataUri: string): string {
	return dataUri.replace(';base64,', ',');
}

function cellData(fields: DiagramField[]): DrawioCell['data'] {
	if (fields.length === 0) return undefined;
	const keys = fieldKeys(fields.map((f) => f.label));
	return fields.map((f, i) => ({ key: keys[i], value: f.value }));
}

/** An outline's style: draw.io measures `arcSize` across the corner, twice the radius. */
function boxStyle(box: DiagramBox): StyleEntries {
	return {
		shape: box.outline === 'ellipse' ? 'ellipse' : undefined,
		perimeter: box.outline === 'ellipse' ? 'ellipsePerimeter' : undefined,
		rounded: box.outline === 'rect' && box.radius ? 1 : 0,
		absoluteArcSize: box.radius ? 1 : undefined,
		arcSize: box.radius ? box.radius * 2 : undefined,
		fillColor: box.fill ? hexColor(box.fill) : 'none',
		strokeColor: box.stroke ? hexColor(box.stroke) : 'none',
		strokeWidth: box.stroke ? (box.strokeWidth ?? 1) : undefined,
		dashed: box.dashed ? 1 : undefined
	};
}

/** A shape's own box. Everything it draws on it, its name included, is a part. */
function shapeCell(shape: DiagramShape, byId: Map<string, DiagramShape>): DrawioCell {
	const parent = shape.parentId ? byId.get(shape.parentId) : undefined;
	const isContainer = shape.type === 'container';

	return {
		id: shapeCellId(shape.id),
		parent: parent ? shapeCellId(parent.id) : LAYER_ID,
		value: '',
		vertex: true,
		data: cellData(shape.data),
		style: style({
			...boxStyle(shape.box),
			html: 1,
			container: isContainer ? 1 : undefined,
			collapsible: isContainer ? 0 : undefined,
			recursiveResize: isContainer ? 0 : undefined
		}),
		geometry: {
			x: shape.x - (parent?.x ?? 0),
			y: shape.y - (parent?.y ?? 0),
			width: shape.width,
			height: shape.height
		}
	};
}

/** A part as a child cell of its shape. A part whose icon did not load is not drawn. */
function partCell(
	shape: DiagramShape,
	part: DiagramPart,
	index: number,
	model: DiagramModel
): DrawioCell | null {
	const base = {
		id: partCellId(shape.id, index),
		parent: shapeCellId(shape.id),
		vertex: true,
		geometry: { x: part.x, y: part.y, width: part.width, height: part.height }
	};
	if (part.iconKey) {
		const icon = model.icons.get(part.iconKey);
		if (!icon) return null;
		return {
			...base,
			value: '',
			style: style({
				shape: 'image',
				image: styleImage(icon.dataUri),
				imageAspect: 1,
				aspect: 'fixed',
				connectable: 0
			})
		};
	}
	const text = part.text;
	const align = text?.align ?? 'center';
	return {
		...base,
		value: text ? escapeXml(text.value) : '',
		style: style({
			...boxStyle(part),
			html: 1,
			connectable: 0,
			// One line clipped to the part, or wrapped at its width from the top.
			whiteSpace: text?.wrap ? 'wrap' : undefined,
			overflow: text && !text.wrap ? 'hidden' : undefined,
			fontColor: text ? hexColor(text.color) : undefined,
			fontSize: text?.size,
			fontStyle: text ? (text.bold ? 1 : 0) : undefined,
			align: text ? align : undefined,
			verticalAlign: text ? (text.wrap ? 'top' : 'middle') : undefined,
			spacing: text ? 0 : undefined,
			spacingLeft: text && align === 'left' ? (text.inset ?? 0) : undefined,
			spacingRight: text && align === 'right' ? (text.inset ?? 0) : undefined
		})
	};
}

function connectorCell(connector: DiagramConnector, background: string): DrawioCell {
	const exit = sideAnchor(connector.sourceSide);
	const entry = sideAnchor(connector.targetSide);
	const label = connector.labelStyle;
	return {
		id: connectorCellId(connector.id),
		parent: LAYER_ID,
		value: connector.label ? escapeXml(connector.label) : '',
		edge: true,
		data: cellData(connector.data),
		source: shapeCellId(connector.sourceId),
		target: shapeCellId(connector.targetId),
		style: style({
			edgeStyle: 'orthogonalEdgeStyle',
			curved: connector.curved ? 1 : undefined,
			rounded: 1,
			orthogonalLoop: 1,
			html: 1,
			startArrow: 'none',
			endArrow: 'none',
			strokeColor: hexColor(connector.color),
			strokeWidth: connector.width,
			dashed: connector.dash === 'solid' ? undefined : 1,
			dashPattern:
				connector.dash === 'solid' ? undefined : overlayDashArray(connector.dash === 'dotted'),
			exitX: exit.x,
			exitY: exit.y,
			exitDx: 0,
			exitDy: 0,
			entryX: entry.x,
			entryY: entry.y,
			entryDx: 0,
			entryDy: 0,
			fontSize: label?.size,
			fontColor: label ? hexColor(label.color) : undefined,
			labelBackgroundColor: hexColor(label?.fill ?? background),
			labelBorderColor: label ? hexColor(label.stroke) : undefined
		})
	};
}

function headerCells(model: DiagramModel): DrawioCell[] {
	if (!model.header) return [];
	const { title, subtitle, color, subtitleColor } = model.header;
	const text = (fontColor: string, fontSize: number, bold: boolean) =>
		style({
			text: '',
			html: 1,
			align: 'left',
			verticalAlign: 'top',
			fontColor: hexColor(fontColor),
			fontSize,
			fontStyle: bold ? 1 : 0,
			strokeColor: 'none',
			fillColor: 'none'
		});
	const width = Math.max(model.width - DIAGRAM_PADDING * 2, 200);
	return [
		{
			id: 'header-title',
			parent: LAYER_ID,
			value: escapeXml(title),
			vertex: true,
			style: text(color, STYLE.header.titleText, true),
			geometry: { x: DIAGRAM_PADDING, y: DIAGRAM_PADDING - 8, width, height: 26 }
		},
		{
			id: 'header-subtitle',
			parent: LAYER_ID,
			value: escapeXml(subtitle),
			vertex: true,
			style: text(subtitleColor, STYLE.header.subtitleText, false),
			geometry: { x: DIAGRAM_PADDING, y: DIAGRAM_PADDING + 18, width, height: 20 }
		}
	];
}

/** The cells of the diagram, root and layer first, each shape before its parts and children. */
export function toDrawioCells(model: DiagramModel): DrawioCell[] {
	const byId = new Map(model.shapes.map((s) => [s.id, s]));
	const shapes = model.shapes.flatMap((shape) => [
		shapeCell(shape, byId),
		...shape.parts.flatMap((part, i) => partCell(shape, part, i, model) ?? [])
	]);
	return [
		{ id: ROOT_ID, parent: '', value: '', style: '' },
		{ id: LAYER_ID, parent: ROOT_ID, value: '', style: '' },
		...headerCells(model),
		...shapes,
		...model.connectors.map((c) => connectorCell(c, model.background))
	];
}

function serializeCell(cell: DrawioCell): string {
	// A cell with properties is wrapped in <object>, which then owns the id and the label.
	const wrapped = !!cell.data?.length;
	const attrs = wrapped ? [] : [`id="${escapeXml(cell.id)}"`];
	if (cell.parent) attrs.push(`parent="${escapeXml(cell.parent)}"`);
	if (cell.id === ROOT_ID || cell.id === LAYER_ID) return `<mxCell ${attrs.join(' ')}/>`;
	if (!wrapped) attrs.push(`value="${escapeXml(cell.value)}"`);
	attrs.push(`style="${escapeXml(cell.style)}"`);
	if (cell.vertex) attrs.push('vertex="1"');
	if (cell.edge) {
		attrs.push('edge="1"');
		if (cell.source) attrs.push(`source="${escapeXml(cell.source)}"`);
		if (cell.target) attrs.push(`target="${escapeXml(cell.target)}"`);
	}
	const geometry = cell.geometry
		? `<mxGeometry x="${num(cell.geometry.x)}" y="${num(cell.geometry.y)}" width="${num(cell.geometry.width)}" height="${num(cell.geometry.height)}" as="geometry"/>`
		: '<mxGeometry relative="1" as="geometry"/>';
	const mxCell = `<mxCell ${attrs.join(' ')}>${geometry}</mxCell>`;
	if (!wrapped) return mxCell;
	const properties = cell.data!.map((d) => `${d.key}="${escapeXml(d.value)}"`).join(' ');
	return `<object id="${escapeXml(cell.id)}" label="${escapeXml(cell.value)}" ${properties}>${mxCell}</object>`;
}

export function serializeDrawio(model: DiagramModel): string {
	const cells = toDrawioCells(model);
	const body = cells.map(serializeCell).join('\n');
	return [
		'<?xml version="1.0" encoding="UTF-8"?>',
		'<mxfile host="Scanopy" type="device">',
		`<diagram id="scanopy-topology" name="${escapeXml(model.name)}">`,
		// `adaptiveColors="none"`: draw.io otherwise recolours a diagram for its own light or dark
		// mode, which shows a dark export in light colours in a dark editor.
		`<mxGraphModel adaptiveColors="none" dx="0" dy="0" grid="0" gridSize="10" guides="1" tooltips="1" connect="1" arrows="1" fold="1" page="1" pageScale="1" pageWidth="${num(model.width)}" pageHeight="${num(model.height)}" background="${hexColor(model.background)}" math="0" shadow="0">`,
		'<root>',
		body,
		'</root>',
		'</mxGraphModel>',
		'</diagram>',
		'</mxfile>',
		''
	].join('\n');
}
