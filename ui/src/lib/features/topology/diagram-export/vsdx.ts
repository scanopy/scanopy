/**
 * Visio (VSDX) serializer for a `DiagramModel`.
 *
 * A VSDX file is an OPC zip package (MS-VSDX). Containers become group shapes holding their
 * children, so moving one moves its contents. Every shape carries four connection points (one per
 * side), and each connector is a 1-D shape whose ends are glued to those points twice over: by
 * `PAR(PNT(...))` formulas on BeginX/EndX, which keep the ends on the points when a shape moves,
 * and by `Connect` rows, which is how Visio records glue.
 */

import { zipSync, strToU8 } from 'fflate';
import {
	DIAGRAM_PADDING,
	DIAGRAM_SIDES,
	sideAnchor,
	type DiagramBox,
	type DiagramConnector,
	type DiagramField,
	type DiagramModel,
	type DiagramPart,
	type DiagramShape,
	type DiagramSide,
	type DiagramText
} from './model';
import { STYLE } from './style';
import { escapeXml, fieldKeys, hexColor } from './xml';

/** Diagram units per inch: one flow pixel is one screen pixel at 96 DPI. */
const UNITS_PER_INCH = 96;
const POINT = 1 / 72;

const NS_MAIN = 'http://schemas.microsoft.com/office/visio/2012/main';
const NS_REL = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships';
const NS_PKG_REL = 'http://schemas.openxmlformats.org/package/2006/relationships';
const REL_DOCUMENT = 'http://schemas.microsoft.com/visio/2010/relationships/document';
const REL_PAGES = 'http://schemas.microsoft.com/visio/2010/relationships/pages';
const REL_PAGE = 'http://schemas.microsoft.com/visio/2010/relationships/page';
const REL_WINDOWS = 'http://schemas.microsoft.com/visio/2010/relationships/windows';
const REL_IMAGE = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships/image';
const REL_CORE =
	'http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties';
const REL_APP =
	'http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties';

const XML_DECL = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>';

/** Package parts by path inside the zip. */
export type VsdxParts = Record<string, string | Uint8Array>;

const inches = (units: number) => units / UNITS_PER_INCH;
const fmt = (value: number) => String(Math.round(value * 1e6) / 1e6);

const cell = (name: string, value: string | number, formula?: string, unit?: string) =>
	`<Cell N="${name}" V="${typeof value === 'number' ? fmt(value) : escapeXml(value)}"${
		unit ? ` U="${unit}"` : ''
	}${formula ? ` F="${escapeXml(formula)}"` : ''}/>`;

/** A box in diagram units, y down, used as the coordinate frame for a shape's children. */
interface Frame {
	x: number;
	y: number;
	width: number;
	height: number;
}

/** Transform cells placing a box inside a frame (Visio local coords: inches, y up). */
function transformCells(box: Frame, frame: Frame): string {
	const w = inches(box.width);
	const h = inches(box.height);
	return [
		cell('PinX', inches(box.x + box.width / 2 - frame.x)),
		cell('PinY', inches(frame.y + frame.height - (box.y + box.height / 2))),
		cell('Width', w),
		cell('Height', h),
		cell('LocPinX', w / 2, 'Width*0.5'),
		cell('LocPinY', h / 2, 'Height*0.5'),
		cell('Angle', 0),
		cell('FlipX', 0),
		cell('FlipY', 0),
		cell('ResizeMode', 0)
	].join('');
}

function rectangleGeometry(noFill: boolean, noLine: boolean): string {
	const corners: [number, number][] = [
		[0, 0],
		[1, 0],
		[1, 1],
		[0, 1],
		[0, 0]
	];
	const rows = corners
		.map(
			([x, y], i) =>
				`<Row T="${i === 0 ? 'RelMoveTo' : 'RelLineTo'}" IX="${i + 1}">${cell('X', x)}${cell('Y', y)}</Row>`
		)
		.join('');
	return `<Section N="Geometry" IX="0">${cell('NoFill', noFill ? 1 : 0)}${cell('NoLine', noLine ? 1 : 0)}${cell('NoShow', 0)}${cell('NoSnap', 0)}${rows}</Section>`;
}

/** Connection point row index for each side (Visio rows are bottom-up in y). */
const CONNECTION_ROW: Record<DiagramSide, number> = { Top: 0, Right: 1, Bottom: 2, Left: 3 };

function connectionSection(width: number, height: number): string {
	const rows = DIAGRAM_SIDES.map((side) => {
		const a = sideAnchor(side);
		const fx = a.x;
		const fy = 1 - a.y;
		return `<Row IX="${CONNECTION_ROW[side]}">${cell('X', inches(width) * fx, `Width*${fx}`)}${cell('Y', inches(height) * fy, `Height*${fy}`)}${cell('DirX', 0)}${cell('DirY', 0)}${cell('Type', 0)}</Row>`;
	}).join('');
	return `<Section N="Connection">${rows}</Section>`;
}

/** Shape Data rows: Visio lists them in the Shape Data pane, and reports and data graphics read them. */
function propertySection(fields: DiagramField[]): string {
	if (fields.length === 0) return '';
	const keys = fieldKeys(fields.map((f) => f.label));
	const rows = fields
		.map(
			(f, i) =>
				`<Row N="${keys[i]}">${cell('Label', f.label)}${cell('Type', 0)}<Cell N="Value" V="${escapeXml(f.value)}" U="STR"/></Row>`
		)
		.join('');
	return `<Section N="Property">${rows}</Section>`;
}

/** Points per diagram unit: a 12px label is 9pt. */
const POINTS_PER_UNIT = 72 / UNITS_PER_INCH;

/** A shape's text as one run: its character row and its `Text` element. */
function textBlock(text: DiagramText | undefined): { character: string; text: string } {
	if (!text) return { character: '', text: '' };
	return {
		character: `<Section N="Character"><Row IX="0">${cell('Color', hexColor(text.color))}${cell('Size', text.size * POINTS_PER_UNIT * POINT, undefined, 'PT')}${cell('Style', text.bold ? 1 : 0)}</Row></Section>`,
		text: `<Text>${escapeXml(text.value)}</Text>`
	};
}

/**
 * An ellipse filling a box of `width` by `height` diagram units: its centre, then a point on each
 * axis. Each cell carries its value beside its formula, since a reader that does not evaluate
 * formulas draws from the values.
 */
function ellipseGeometry(width: number, height: number, noFill: boolean, noLine: boolean): string {
	const w = inches(width);
	const h = inches(height);
	return `<Section N="Geometry" IX="0">${cell('NoFill', noFill ? 1 : 0)}${cell('NoLine', noLine ? 1 : 0)}${cell('NoShow', 0)}${cell('NoSnap', 0)}<Row T="Ellipse" IX="1">${cell('X', w / 2, 'Width*0.5')}${cell('Y', h / 2, 'Height*0.5')}${cell('A', w, 'Width*1')}${cell('B', h / 2, 'Height*0.5')}${cell('C', w / 2, 'Width*0.5')}${cell('D', h, 'Height*1')}</Row></Section>`;
}

/** Fill, line and corner cells for an outline; an undrawn fill or line is pattern 0. */
function boxCells(box: DiagramBox): string {
	return [
		cell('FillForegnd', hexColor(box.fill ?? '#ffffff')),
		cell('FillBkgnd', hexColor(box.fill ?? '#ffffff')),
		cell('FillPattern', box.fill ? 1 : 0),
		cell('LineColor', hexColor(box.stroke ?? '#000000')),
		cell('LineWeight', (box.strokeWidth ?? 1) * POINTS_PER_UNIT * POINT, undefined, 'PT'),
		cell('LinePattern', box.stroke ? (box.dashed ? 2 : 1) : 0),
		cell('Rounding', inches(box.outline === 'rect' ? (box.radius ?? 0) : 0))
	].join('');
}

const boxGeometry = (box: DiagramBox, size: { width: number; height: number }) =>
	box.outline === 'ellipse'
		? ellipseGeometry(size.width, size.height, !box.fill, !box.stroke)
		: rectangleGeometry(!box.fill, !box.stroke);

interface Ids {
	next: number;
	byShape: Map<string, number>;
}

interface Media {
	relIdByIcon: Map<string, string>;
	files: { path: string; relId: string; bytes: Uint8Array }[];
}

function iconRelId(media: Media, model: DiagramModel, iconKey: string | undefined): string | null {
	if (!iconKey) return null;
	const existing = media.relIdByIcon.get(iconKey);
	if (existing) return existing;
	const png = model.icons.get(iconKey)?.png;
	if (!png) return null;
	const relId = `rId${media.files.length + 1}`;
	media.files.push({ path: `visio/media/image${media.files.length + 1}.png`, relId, bytes: png });
	media.relIdByIcon.set(iconKey, relId);
	return relId;
}

function boxShapeXml(
	shape: DiagramShape,
	frame: Frame,
	model: DiagramModel,
	children: Map<string, DiagramShape[]>,
	ids: Ids,
	media: Media
): string {
	const id = ids.next++;
	ids.byShape.set(shape.id, id);
	const isContainer = shape.type === 'container';
	const kids = children.get(shape.id) ?? [];
	const childFrame: Frame = shape;
	// A shape's parts first, then the shapes it holds, as draw.io orders them.
	const childXml = [
		...shape.parts.map((part) => partShapeXml(shape, part, childFrame, model, ids, media)),
		...kids.map((kid) => boxShapeXml(kid, childFrame, model, children, ids, media))
	].filter(Boolean);
	const isGroup = childXml.length > 0;

	// The shape is its box alone. Everything drawn on it, its name included, is a part.
	const cells = [
		transformCells(shape, frame),
		boxCells(shape.box),
		// 1 draws the group's own box behind its members. 2 draws it in front and, filled, hides a
		// container's cards and a card's icon in Visio (draw.io's importer ignores the cell).
		isGroup ? cell('SelectMode', 1) : '',
		isGroup ? cell('DisplayMode', 1) : ''
	].join('');

	const sections =
		boxGeometry(shape.box, shape) +
		connectionSection(shape.width, shape.height) +
		propertySection(shape.data);

	const name = `${isContainer ? 'Container' : 'Element'}.${id}`;
	return `<Shape ID="${id}" NameU="${name}" Name="${name}" Type="${isGroup ? 'Group' : 'Shape'}" LineStyle="0" FillStyle="0" TextStyle="0">${cells}${sections}${
		isGroup ? `<Shapes>${childXml.join('')}</Shapes>` : ''
	}</Shape>`;
}

const HORZ_ALIGN = { left: 0, center: 1, right: 2 } as const;

/**
 * A part as a member of its shape's group: a bitmap for an icon, otherwise an outline with its
 * text. A part whose icon did not load is not drawn.
 */
function partShapeXml(
	owner: DiagramShape,
	part: DiagramPart,
	frame: Frame,
	model: DiagramModel,
	ids: Ids,
	media: Media
): string {
	const box: Frame = {
		x: owner.x + part.x,
		y: owner.y + part.y,
		width: part.width,
		height: part.height
	};
	if (part.iconKey) {
		const relId = iconRelId(media, model, part.iconKey);
		if (!relId) return '';
		const id = ids.next++;
		return `<Shape ID="${id}" NameU="Icon.${id}" Name="Icon.${id}" Type="Foreign" LineStyle="0" FillStyle="0" TextStyle="0">${transformCells(box, frame)}${cell('ImgOffsetX', 0, 'ImgWidth*0')}${cell('ImgOffsetY', 0, 'ImgHeight*0')}${cell('ImgWidth', inches(part.width), 'Width*1')}${cell('ImgHeight', inches(part.height), 'Height*1')}${rectangleGeometry(true, true)}<ForeignData ForeignType="Bitmap" CompressionType="PNG"><Rel r:id="${relId}"/></ForeignData></Shape>`;
	}

	const id = ids.next++;
	const text = part.text;
	const align = text?.align ?? 'center';
	const { character, text: textXml } = textBlock(text);
	const cells = [
		transformCells(box, frame),
		boxCells(part),
		// Wrapped text starts at the top of its box; one line sits in the middle of it.
		cell('VerticalAlign', text?.wrap ? 0 : 1),
		cell('LeftMargin', inches(align === 'left' ? (text?.inset ?? 0) : 0)),
		cell('RightMargin', inches(align === 'right' ? (text?.inset ?? 0) : 0)),
		cell('TopMargin', 0),
		cell('BottomMargin', 0)
	].join('');
	const paragraph = text
		? `<Section N="Paragraph"><Row IX="0">${cell('HorzAlign', HORZ_ALIGN[align])}</Row></Section>`
		: '';
	const name = `${part.role}.${id}`;
	return `<Shape ID="${id}" NameU="${name}" Name="${name}" Type="Shape" LineStyle="0" FillStyle="0" TextStyle="0">${cells}${boxGeometry(part, part)}${character}${paragraph}${textXml}</Shape>`;
}

/** A side's connection point in page inches (y up). */
function pagePoint(shape: DiagramShape, side: DiagramSide, pageHeight: number) {
	const a = sideAnchor(side);
	return {
		x: inches(shape.x + shape.width * a.x),
		y: inches(pageHeight - (shape.y + shape.height * a.y))
	};
}

function connectorXml(
	connector: DiagramConnector,
	model: DiagramModel,
	shapesById: Map<string, DiagramShape>,
	ids: Ids
): { shape: string; connects: string } | null {
	const source = shapesById.get(connector.sourceId);
	const target = shapesById.get(connector.targetId);
	const sourceSheet = ids.byShape.get(connector.sourceId);
	const targetSheet = ids.byShape.get(connector.targetId);
	if (!source || !target || sourceSheet === undefined || targetSheet === undefined) return null;

	const id = ids.next++;
	const begin = pagePoint(source, connector.sourceSide, model.height);
	const end = pagePoint(target, connector.targetSide, model.height);
	const length = Math.hypot(end.x - begin.x, end.y - begin.y);
	const beginRow = CONNECTION_ROW[connector.sourceSide] + 1;
	const endRow = CONNECTION_ROW[connector.targetSide] + 1;
	const glue = (sheet: number, row: number) =>
		`PAR(PNT(Sheet.${sheet}!Connections.X${row},Sheet.${sheet}!Connections.Y${row}))`;
	const pattern = connector.dash === 'dotted' ? 3 : connector.dash === 'dashed' ? 2 : 1;

	const cells = [
		cell('PinX', (begin.x + end.x) / 2, '(BeginX+EndX)/2'),
		cell('PinY', (begin.y + end.y) / 2, '(BeginY+EndY)/2'),
		cell('Width', length, 'SQRT((EndX-BeginX)^2+(EndY-BeginY)^2)'),
		cell('Height', 0),
		cell('LocPinX', length / 2, 'Width*0.5'),
		cell('LocPinY', 0, 'Height*0.5'),
		cell('Angle', Math.atan2(end.y - begin.y, end.x - begin.x), 'ATAN2(EndY-BeginY,EndX-BeginX)'),
		cell('FlipX', 0),
		cell('FlipY', 0),
		cell('BeginX', begin.x, glue(sourceSheet, beginRow)),
		cell('BeginY', begin.y, glue(sourceSheet, beginRow)),
		cell('EndX', end.x, glue(targetSheet, endRow)),
		cell('EndY', end.y, glue(targetSheet, endRow)),
		cell('BegTrigger', 2, `_XFTRIGGER(Sheet.${sourceSheet}!EventXFMod)`),
		cell('EndTrigger', 2, `_XFTRIGGER(Sheet.${targetSheet}!EventXFMod)`),
		cell('ObjType', 2),
		cell('LineColor', hexColor(connector.color)),
		cell('LineWeight', connector.width * POINTS_PER_UNIT * POINT, undefined, 'PT'),
		cell('LinePattern', pattern),
		cell('BeginArrow', 0),
		cell('EndArrow', 0)
	].join('');

	const geometry = `<Section N="Geometry" IX="0">${cell('NoFill', 1)}${cell('NoLine', 0)}${cell('NoShow', 0)}${cell('NoSnap', 0)}<Row T="MoveTo" IX="1">${cell('X', 0, 'Width*0')}${cell('Y', 0, 'Height*0.5')}</Row><Row T="LineTo" IX="2">${cell('X', length, 'Width*1')}${cell('Y', 0, 'Height*0.5')}</Row></Section>`;
	const { character, text } = textBlock(
		connector.label
			? {
					value: connector.label,
					color: connector.labelStyle?.color ?? connector.color,
					size: connector.labelStyle?.size ?? STYLE.edge.labelText
				}
			: undefined
	);
	const name = `Dynamic connector.${id}`;

	return {
		shape: `<Shape ID="${id}" NameU="${name}" Name="${name}" Type="Shape" LineStyle="0" FillStyle="0" TextStyle="0">${cells}${geometry}${character}${propertySection(connector.data)}${text}</Shape>`,
		connects:
			`<Connect FromSheet="${id}" FromCell="BeginX" FromPart="9" ToSheet="${sourceSheet}" ToCell="Connections.X${beginRow}" ToPart="${100 + beginRow - 1}"/>` +
			`<Connect FromSheet="${id}" FromCell="EndX" FromPart="12" ToSheet="${targetSheet}" ToCell="Connections.X${endRow}" ToPart="${100 + endRow - 1}"/>`
	};
}

function headerXml(model: DiagramModel, ids: Ids, page: Frame): string {
	if (!model.header) return '';
	const width = Math.max(model.width - DIAGRAM_PADDING * 2, 200);
	const line = (
		value: string,
		color: string,
		sizePt: number,
		bold: boolean,
		y: number,
		h: number
	) => {
		const id = ids.next++;
		const box: Frame = { x: DIAGRAM_PADDING, y, width, height: h };
		return `<Shape ID="${id}" NameU="Header.${id}" Name="Header.${id}" Type="Shape" LineStyle="0" FillStyle="0" TextStyle="0">${transformCells(box, page)}${cell('VerticalAlign', 0)}${rectangleGeometry(true, true)}<Section N="Character"><Row IX="0">${cell('Color', hexColor(color))}${cell('Size', sizePt * POINT, undefined, 'PT')}${cell('Style', bold ? 1 : 0)}</Row></Section><Section N="Paragraph"><Row IX="0">${cell('HorzAlign', 0)}</Row></Section><Text>${escapeXml(value)}</Text></Shape>`;
	};
	const { title, subtitle, color, subtitleColor } = model.header;
	return (
		line(title, color, STYLE.header.titleText * POINTS_PER_UNIT, true, DIAGRAM_PADDING - 8, 26) +
		line(
			subtitle,
			subtitleColor,
			STYLE.header.subtitleText * POINTS_PER_UNIT,
			false,
			DIAGRAM_PADDING + 18,
			20
		)
	);
}

function pageXml(model: DiagramModel): { page: string; media: Media } {
	const ids: Ids = { next: 1, byShape: new Map() };
	const media: Media = { relIdByIcon: new Map(), files: [] };
	const page: Frame = { x: 0, y: 0, width: model.width, height: model.height };
	const shapesById = new Map(model.shapes.map((s) => [s.id, s]));
	const children = new Map<string, DiagramShape[]>();
	const roots: DiagramShape[] = [];
	for (const shape of model.shapes) {
		if (shape.parentId && shapesById.has(shape.parentId)) {
			const list = children.get(shape.parentId) ?? [];
			list.push(shape);
			children.set(shape.parentId, list);
		} else {
			roots.push(shape);
		}
	}

	const shapes = [headerXml(model, ids, page)];
	for (const root of roots) shapes.push(boxShapeXml(root, page, model, children, ids, media));
	const connects: string[] = [];
	for (const connector of model.connectors) {
		const built = connectorXml(connector, model, shapesById, ids);
		if (!built) continue;
		shapes.push(built.shape);
		connects.push(built.connects);
	}

	const page1 = `${XML_DECL}<PageContents xmlns="${NS_MAIN}" xmlns:r="${NS_REL}" xml:space="preserve"><Shapes>${shapes.join('')}</Shapes>${
		connects.length ? `<Connects>${connects.join('')}</Connects>` : ''
	}</PageContents>`;
	return { page: page1, media };
}

function relationships(rels: { id: string; type: string; target: string }[]): string {
	return `${XML_DECL}<Relationships xmlns="${NS_PKG_REL}">${rels
		.map((r) => `<Relationship Id="${r.id}" Type="${r.type}" Target="${escapeXml(r.target)}"/>`)
		.join('')}</Relationships>`;
}

function contentTypes(hasMedia: boolean): string {
	const override = (part: string, type: string) =>
		`<Override PartName="${part}" ContentType="${type}"/>`;
	return `${XML_DECL}<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/>${
		hasMedia ? '<Default Extension="png" ContentType="image/png"/>' : ''
	}${override('/visio/document.xml', 'application/vnd.ms-visio.drawing.main+xml')}${override('/visio/pages/pages.xml', 'application/vnd.ms-visio.pages+xml')}${override('/visio/pages/page1.xml', 'application/vnd.ms-visio.page+xml')}${override('/visio/windows.xml', 'application/vnd.ms-visio.windows+xml')}${override('/docProps/core.xml', 'application/vnd.openxmlformats-package.core-properties+xml')}${override('/docProps/app.xml', 'application/vnd.openxmlformats-officedocument.extended-properties+xml')}</Types>`;
}

function documentXml(): string {
	const styleCells = [
		cell('EnableLineProps', 1),
		cell('EnableFillProps', 1),
		cell('EnableTextProps', 1),
		cell('HideForApply', 0),
		cell('LineWeight', 0.75 * POINT, undefined, 'PT'),
		cell('LineColor', '#000000'),
		cell('LinePattern', 1),
		cell('Rounding', 0),
		cell('LineCap', 0),
		cell('BeginArrow', 0),
		cell('EndArrow', 0),
		cell('FillForegnd', '#FFFFFF'),
		cell('FillBkgnd', '#FFFFFF'),
		cell('FillPattern', 1),
		cell('ShdwPattern', 0),
		cell('LeftMargin', 4 * POINT, undefined, 'PT'),
		cell('RightMargin', 4 * POINT, undefined, 'PT'),
		cell('TopMargin', 4 * POINT, undefined, 'PT'),
		cell('BottomMargin', 4 * POINT, undefined, 'PT'),
		cell('VerticalAlign', 1),
		cell('TextBkgnd', 0)
	].join('');
	const styleSections = `<Section N="Character"><Row IX="0">${cell('Font', 'Calibri')}${cell('Color', '#000000')}${cell('Style', 0)}${cell('Size', 9 * POINT, undefined, 'PT')}</Row></Section><Section N="Paragraph"><Row IX="0">${cell('HorzAlign', 1)}</Row></Section>`;
	return `${XML_DECL}<VisioDocument xmlns="${NS_MAIN}" xmlns:r="${NS_REL}" xml:space="preserve"><DocumentSettings TopPage="0" DefaultTextStyle="0" DefaultLineStyle="0" DefaultFillStyle="0" DefaultGuideStyle="0"><GlueSettings>9</GlueSettings><SnapSettings>65847</SnapSettings><SnapExtensions>34</SnapExtensions><DynamicGridEnabled>0</DynamicGridEnabled><ProtectStyles>0</ProtectStyles><ProtectShapes>0</ProtectShapes><ProtectMasters>0</ProtectMasters><ProtectBkgnds>0</ProtectBkgnds></DocumentSettings><Colors><ColorEntry IX="0" RGB="#000000"/><ColorEntry IX="1" RGB="#FFFFFF"/></Colors><FaceNames><FaceName NameU="Calibri" Name="Calibri"/></FaceNames><StyleSheets><StyleSheet ID="0" NameU="No Style" Name="No Style">${styleCells}${styleSections}</StyleSheet></StyleSheets></VisioDocument>`;
}

function pagesXml(model: DiagramModel): string {
	const w = inches(model.width);
	const h = inches(model.height);
	return `${XML_DECL}<Pages xmlns="${NS_MAIN}" xmlns:r="${NS_REL}" xml:space="preserve"><Page ID="0" NameU="Page-1" Name="${escapeXml(model.name)}"><PageSheet LineStyle="0" FillStyle="0" TextStyle="0">${cell('PageWidth', w)}${cell('PageHeight', h)}${cell('PageScale', 1, undefined, 'IN_F')}${cell('DrawingScale', 1, undefined, 'IN_F')}${cell('DrawingSizeType', 3)}${cell('DrawingScaleType', 0)}${cell('FillForegnd', hexColor(model.background))}</PageSheet><Rel r:id="rId1"/></Page></Pages>`;
}

function windowsXml(model: DiagramModel): string {
	return `${XML_DECL}<Windows ClientWidth="1600" ClientHeight="900" xmlns="${NS_MAIN}" xmlns:r="${NS_REL}" xml:space="preserve"><Window ID="0" WindowType="Drawing" WindowState="1073741824" ContainerType="Page" Page="0" ViewScale="-1" ViewCenterX="${fmt(inches(model.width) / 2)}" ViewCenterY="${fmt(inches(model.height) / 2)}"><ShowRulers>1</ShowRulers><ShowGrid>0</ShowGrid><ShowPageBreaks>0</ShowPageBreaks><ShowGuides>1</ShowGuides><ShowConnectionPoints>1</ShowConnectionPoints><GlueSettings>9</GlueSettings><SnapSettings>65847</SnapSettings><SnapExtensions>34</SnapExtensions><DynamicGridEnabled>0</DynamicGridEnabled></Window></Windows>`;
}

function corePropsXml(model: DiagramModel, created: Date): string {
	return `${XML_DECL}<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title>${escapeXml(model.name)}</dc:title><dc:creator>Scanopy</dc:creator><dcterms:created xsi:type="dcterms:W3CDTF">${created.toISOString()}</dcterms:created></cp:coreProperties>`;
}

function appPropsXml(): string {
	return `${XML_DECL}<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>Scanopy</Application></Properties>`;
}

/** Every part of the package, keyed by its path in the zip. */
export function buildVsdxParts(model: DiagramModel, created = new Date()): VsdxParts {
	const { page, media } = pageXml(model);
	const parts: VsdxParts = {
		'[Content_Types].xml': contentTypes(media.files.length > 0),
		'_rels/.rels': relationships([
			{ id: 'rId1', type: REL_DOCUMENT, target: 'visio/document.xml' },
			{ id: 'rId2', type: REL_CORE, target: 'docProps/core.xml' },
			{ id: 'rId3', type: REL_APP, target: 'docProps/app.xml' }
		]),
		'docProps/core.xml': corePropsXml(model, created),
		'docProps/app.xml': appPropsXml(),
		'visio/document.xml': documentXml(),
		'visio/_rels/document.xml.rels': relationships([
			{ id: 'rId1', type: REL_PAGES, target: 'pages/pages.xml' },
			{ id: 'rId2', type: REL_WINDOWS, target: 'windows.xml' }
		]),
		'visio/windows.xml': windowsXml(model),
		'visio/pages/pages.xml': pagesXml(model),
		'visio/pages/_rels/pages.xml.rels': relationships([
			{ id: 'rId1', type: REL_PAGE, target: 'page1.xml' }
		]),
		'visio/pages/page1.xml': page
	};
	if (media.files.length > 0) {
		parts['visio/pages/_rels/page1.xml.rels'] = relationships(
			media.files.map((f) => ({
				id: f.relId,
				type: REL_IMAGE,
				target: `../${f.path.slice('visio/'.length)}`
			}))
		);
		for (const file of media.files) parts[file.path] = file.bytes;
	}
	return parts;
}

/** Zip the parts, `[Content_Types].xml` first. */
export function zipVsdx(parts: VsdxParts): Uint8Array {
	const entries: Record<string, Uint8Array> = {};
	for (const [path, content] of Object.entries(parts)) {
		entries[path] = typeof content === 'string' ? strToU8(content) : content;
	}
	return zipSync(entries, { level: 6 });
}
