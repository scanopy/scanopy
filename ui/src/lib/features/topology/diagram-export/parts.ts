/**
 * Layout of what a card or a container draws, as format-neutral parts.
 *
 * This is the export's counterpart of the markup in `ElementNode.svelte` and
 * `ContainerHeader.svelte`: where the tile, the badges, the title pill and the rows sit. Both
 * serializers draw the parts as given, so draw.io and Visio cannot disagree with each other, and
 * a change to the canvas layout is made here once.
 */

import type { DiagramBox, DiagramLabel, DiagramPart, DiagramPartRole, DiagramText } from './model';
import { EXTERNAL_TITLE_OFFSET } from '../container-render-data';
import { STYLE, textWidth, type DiagramPalette } from './style';

/** An icon ready to draw: its key in the model's icons, its drawn size, the colour behind it. */
export interface GlyphDraw {
	iconKey?: string;
	size: number;
	/** A logo's own background colour, which fills its frame. */
	fill?: string | null;
}

/** A `Tag` as the canvas draws it: a filled chip with an icon, a label, or both. */
export interface TagDraw {
	label?: string;
	fill: string;
	text: string;
	icon?: GlyphDraw;
	pill?: boolean;
}

export interface GroupMemberDraw {
	/** A device sits in a small tile, software in a ring, a port bare. */
	frame: 'tile' | 'ring' | 'none';
	glyph: GlyphDraw;
	label: string;
	statusColor?: string;
}

export interface GroupDraw {
	headingGlyph?: GlyphDraw;
	headingName: string;
	collapsed: boolean;
	members: GroupMemberDraw[];
}

export interface IconCardDraw {
	tile: { outline: 'rect' | 'ellipse'; fill: string; stroke: string; glyph: GlyphDraw };
	statusColor?: string;
	os?: { fill: string; glyph: GlyphDraw };
	/** Top badge first, as the canvas stacks them. */
	badges: { fill: string; glyph: GlyphDraw }[];
	freshness?: TagDraw;
	label: string;
	sublabel?: string | null;
	groups: GroupDraw[];
}

export interface TextCardDraw {
	header?: string | null;
	body?: string | null;
	freshness?: TagDraw;
}

export interface ContainerDraw {
	isSubcontainer: boolean;
	hasBorder: boolean;
	isCollapsed: boolean;
	/** The title sits in a pill above the box; otherwise inside its top edge. */
	externalTitle: boolean;
	accent: string;
	title: string;
	/** The chip of the entity the container stands for; absent for a grouping. */
	entity?: { fill: string; text: string; icon?: GlyphDraw };
	/** A grouping's icon, a device tile or a logo, drawn before a plain title. */
	titleGlyph?: GlyphDraw & { frame: 'tile' | 'none' };
	freshness?: TagDraw;
	tags: TagDraw[];
	/** A collapsed container's element summary, e.g. "5 services, 2 hosts". */
	summary: string;
	/** A collapsed subcontainer's trailing count, already in parentheses; empty when hidden. */
	count: string;
	/** A collapsed root container's "Ungrouped: …" line. */
	ungrouped?: string;
	/** A collapsed root container's subgroup rows, each as its text. */
	subgroups: string[];
}

interface Size {
	width: number;
	height: number;
}

const rect = (
	role: DiagramPartRole,
	x: number,
	y: number,
	width: number,
	height: number,
	box: Omit<DiagramBox, 'outline'> = {}
): DiagramPart => ({ role, outline: 'rect', x, y, width, height, ...box });

const ellipse = (
	role: DiagramPartRole,
	x: number,
	y: number,
	size: number,
	box: Omit<DiagramBox, 'outline'> = {}
): DiagramPart => ({ role, outline: 'ellipse', x, y, width: size, height: size, ...box });

/** An icon centred on a point; nothing when the icon did not resolve. */
function glyphAt(glyph: GlyphDraw | undefined, cx: number, cy: number): DiagramPart[] {
	if (!glyph?.iconKey) return [];
	return [
		{
			role: 'glyph',
			outline: 'rect',
			x: cx - glyph.size / 2,
			y: cy - glyph.size / 2,
			width: glyph.size,
			height: glyph.size,
			iconKey: glyph.iconKey
		}
	];
}

const textPart = (
	x: number,
	y: number,
	width: number,
	height: number,
	text: DiagramText
): DiagramPart => ({ role: 'text', outline: 'rect', x, y, width, height, text });

/** A `Tag` at a point, no wider than `maxWidth`. Returns its parts and the width it took. */
function tagParts(
	tag: TagDraw,
	role: DiagramPartRole,
	x: number,
	y: number,
	maxWidth: number
): { parts: DiagramPart[]; width: number } {
	const { height, icon, gap, paddingX, iconOnlyPaddingX, radius } = STYLE.tag;
	const hasIcon = !!tag.icon?.iconKey;
	const padding = tag.label ? paddingX : iconOnlyPaddingX;
	const iconSpace = hasIcon ? icon + (tag.label ? gap : 0) : 0;
	const natural = padding * 2 + iconSpace + (tag.label ? textWidth(tag.label, STYLE.text) : 0);
	const width = Math.max(Math.min(natural, maxWidth), 0);
	const parts: DiagramPart[] = [
		{
			...rect(role, x, y, width, height, {
				radius: tag.pill ? height / 2 : radius,
				fill: tag.fill
			}),
			text: tag.label
				? {
						value: tag.label,
						color: tag.text,
						size: STYLE.text,
						align: 'left',
						inset: padding + iconSpace
					}
				: undefined
		}
	];
	if (hasIcon) {
		parts.push(...glyphAt({ ...tag.icon!, size: icon }, x + padding + icon / 2, y + height / 2));
	}
	return { parts, width };
}

/** A frame with its glyph centred in it: a tile, a ring or a bare icon. */
function framedGlyph(
	role: DiagramPartRole,
	frame: 'tile' | 'ring' | 'none',
	glyph: GlyphDraw,
	x: number,
	y: number,
	size: number,
	palette: DiagramPalette
): DiagramPart[] {
	const parts: DiagramPart[] = [];
	if (frame === 'tile') {
		parts.push(rect(role, x, y, size, size, { radius: 2, fill: palette.tile }));
	} else if (frame === 'ring') {
		parts.push(
			ellipse(role, x, y, size, {
				fill: glyph.fill ?? palette.node,
				stroke: palette.ring,
				strokeWidth: 1
			})
		);
	}
	parts.push(...glyphAt(glyph, x + size / 2, y + size / 2));
	return parts;
}

function groupHeight(group: GroupDraw): number {
	const { padding, rowGap, glyph, headingGlyph } = STYLE.group;
	const rows = group.collapsed ? 0 : group.members.length;
	return 2 + padding * 2 + headingGlyph + rows * (glyph + rowGap);
}

/** A manager group box on a guest's card: its heading, then a row per member while open. */
function groupParts(
	group: GroupDraw,
	x: number,
	y: number,
	width: number,
	palette: DiagramPalette
): DiagramPart[] {
	const g = STYLE.group;
	const parts: DiagramPart[] = [
		rect('group', x, y, width, groupHeight(group), {
			radius: g.radius,
			stroke: palette.ring,
			strokeWidth: 1,
			dashed: true
		})
	];
	const left = x + 1 + g.padding;
	const nameX = left + g.glyph + g.columnGap;
	const trailingX = x + width - 1 - g.padding - g.trailing;
	const nameWidth = trailingX - g.columnGap - nameX;
	let rowY = y + 1 + g.padding;

	parts.push(...glyphAt(group.headingGlyph, left + g.glyph / 2, rowY + g.headingGlyph / 2));
	parts.push(
		textPart(nameX, rowY, nameWidth, g.headingGlyph, {
			value: group.headingName,
			color: palette.textTertiary,
			size: STYLE.rowText,
			align: 'left'
		}),
		textPart(trailingX, rowY, g.trailing, g.headingGlyph, {
			value: String(group.members.length),
			color: palette.textTertiary,
			size: STYLE.rowText - 1,
			align: 'center'
		})
	);
	rowY += g.headingGlyph + g.rowGap;

	if (group.collapsed) return parts;
	for (const member of group.members) {
		parts.push(...framedGlyph('row', member.frame, member.glyph, left, rowY, g.glyph, palette));
		parts.push(
			textPart(nameX, rowY, nameWidth, g.glyph, {
				value: member.label,
				color: palette.textSecondary,
				size: STYLE.rowText,
				align: 'left'
			})
		);
		if (member.statusColor) {
			const dot = STYLE.status.rowDot;
			parts.push(
				ellipse('status', trailingX + (g.trailing - dot) / 2, rowY + (g.glyph - dot) / 2, dot, {
					fill: member.statusColor
				})
			);
		}
		rowY += g.glyph + g.rowGap;
	}
	return parts;
}

/**
 * An icon card: the tile with its glyph, the status dot and badges on its corners, the status
 * pill, and the group boxes at the bottom. The label is the shape's own text, under the tile.
 */
export function iconCardShape(
	card: IconCardDraw,
	size: Size,
	palette: DiagramPalette
): { label: DiagramLabel; parts: DiagramPart[] } {
	const { padding, gap } = STYLE.card;
	const tile = STYLE.tile;
	const badge = STYLE.badge.size;
	const tileX = (size.width - tile.size) / 2;
	const tileY = padding + tile.margin;
	const parts: DiagramPart[] = [];

	const frame = {
		fill: card.tile.glyph.fill ?? card.tile.fill,
		stroke: card.tile.stroke,
		strokeWidth: tile.borderWidth
	};
	parts.push(
		card.tile.outline === 'ellipse'
			? ellipse('tile', tileX, tileY, tile.size, frame)
			: rect('tile', tileX, tileY, tile.size, tile.size, { ...frame, radius: tile.radius })
	);
	parts.push(...glyphAt(card.tile.glyph, tileX + tile.size / 2, tileY + tile.size / 2));

	if (card.statusColor) {
		// Centred on the tile's top-left corner: a ring in the card colour around the state's dot.
		const { ring, dot } = STYLE.status;
		parts.push(
			ellipse('status', tileX - ring / 2, tileY - ring / 2, ring, {
				fill: palette.node,
				stroke: palette.ring,
				strokeWidth: 1
			}),
			ellipse('status', tileX - dot / 2, tileY - dot / 2, dot, { fill: card.statusColor })
		);
	}

	const corner = (x: number, y: number, fill: string, glyph: GlyphDraw): DiagramPart[] => [
		ellipse('badge', x, y, badge, {
			fill: glyph.fill ?? fill,
			stroke: palette.ring,
			strokeWidth: 1
		}),
		...glyphAt(glyph, x + badge / 2, y + badge / 2)
	];
	if (card.os) {
		// Centred on the tile's bottom-left corner.
		parts.push(
			...corner(tileX - badge / 2, tileY + tile.size - badge / 2, card.os.fill, card.os.glyph)
		);
	}
	// Centred on the bottom-right corner; each one behind the top steps up and left. Drawn back
	// to front, so the first badge ends on top as it does on the canvas.
	for (let i = card.badges.length - 1; i >= 0; i--) {
		const layer = Math.min(i, 2) * STYLE.badge.layerStep;
		const at = tile.size - badge / 2 - layer;
		parts.push(...corner(tileX + at, tileY + at, card.badges[i].fill, card.badges[i].glyph));
	}

	let top = tileY + tile.size + tile.margin + gap;
	if (card.freshness) {
		const width = tagParts(card.freshness, 'freshness', 0, 0, size.width).width;
		parts.push(
			...tagParts(card.freshness, 'freshness', (size.width - width) / 2, top, size.width).parts
		);
		top += STYLE.tag.height + gap;
	}

	// Group boxes stack up from the card's bottom padding.
	let groupY = size.height - padding;
	for (const group of [...card.groups].reverse()) {
		groupY -= groupHeight(group);
		parts.push(...groupParts(group, padding, groupY, size.width - padding * 2, palette));
		groupY -= gap;
	}

	const lines: DiagramText[] = [{ value: card.label, color: palette.text, size: STYLE.text }];
	if (card.sublabel) {
		lines.push({ value: card.sublabel, color: palette.textTertiary, size: STYLE.text });
	}
	return { label: { top, lines }, parts };
}

/** An element that did not resolve to an icon card: its status pill, header and body line. */
export function textCardShape(
	card: TextCardDraw,
	size: Size,
	palette: DiagramPalette
): { label: DiagramLabel; parts: DiagramPart[] } {
	const parts: DiagramPart[] = [];
	let top = STYLE.card.padding;
	if (card.freshness) {
		const width = tagParts(card.freshness, 'freshness', 0, 0, size.width).width;
		parts.push(
			...tagParts(card.freshness, 'freshness', (size.width - width) / 2, top, size.width).parts
		);
		top += STYLE.tag.height;
	}
	const lines: DiagramText[] = [];
	if (card.header) {
		lines.push({ value: card.header, color: palette.textTertiary, size: STYLE.text });
	}
	if (card.body) lines.push({ value: card.body, color: palette.textSecondary, size: STYLE.text });
	return { label: { top, lines }, parts };
}

/**
 * A container's title, then its status badge and tags, laid left to right from a point. Used
 * inside a subcontainer's top edge and in a collapsed subcontainer's row.
 */
function titleRow(
	container: ContainerDraw,
	x: number,
	y: number,
	maxRight: number,
	palette: DiagramPalette,
	options: { tagLimit?: number } = {}
): DiagramPart[] {
	const { height, gap } = STYLE.tag;
	const parts: DiagramPart[] = [];
	let cursor = x;
	const suffix = container.tags.length > 0 ? ':' : '';

	if (container.entity) {
		const title = tagParts(
			{ ...container.entity, label: `${container.title}${suffix}`, pill: true },
			'title',
			cursor,
			y,
			maxRight - cursor
		);
		parts.push(...title.parts);
		cursor += title.width + gap;
	} else {
		if (container.titleGlyph) {
			const size =
				container.titleGlyph.frame === 'tile' ? STYLE.group.glyph : container.titleGlyph.size;
			parts.push(
				...framedGlyph(
					'glyph',
					container.titleGlyph.frame,
					container.titleGlyph,
					cursor,
					y + (height - size) / 2,
					size,
					palette
				)
			);
			cursor += size + gap;
		}
		if (container.title) {
			const value = `${container.title}${suffix}`;
			const width = Math.min(textWidth(value, STYLE.text), Math.max(maxRight - cursor, 0));
			parts.push({
				...textPart(cursor, y, width, height, {
					value,
					color: palette.textTertiary,
					size: STYLE.text,
					align: 'left'
				}),
				role: 'title'
			});
			cursor += width + gap;
		}
	}

	if (container.freshness) {
		const badge = tagParts(container.freshness, 'freshness', cursor, y, maxRight - cursor);
		parts.push(...badge.parts);
		cursor += badge.width + gap;
	}
	// A tag that does not fit is left out, where the canvas folds it behind its overflow chip.
	for (const tag of container.tags.slice(0, options.tagLimit)) {
		const drawn = tagParts(tag, 'tag', cursor, y, Infinity);
		if (cursor + drawn.width > maxRight) break;
		parts.push(...drawn.parts);
		cursor += drawn.width + gap;
	}
	return parts;
}

/** The title pill above a root container: the entity's chip, or a grouping's card pill. */
function externalTitle(
	container: ContainerDraw,
	size: Size,
	palette: DiagramPalette
): DiagramPart[] {
	const c = STYLE.container;
	const { height, gap } = STYLE.tag;
	if (container.entity) {
		const y = -(height + c.entityTitleGap);
		const title = tagParts(
			{ ...container.entity, label: container.title, pill: true },
			'title',
			0,
			y,
			size.width
		);
		const parts = title.parts;
		if (container.freshness) {
			parts.push(
				...tagParts(container.freshness, 'freshness', title.width + gap, y, size.width).parts
			);
		}
		return parts;
	}
	if (!container.title && !container.titleGlyph) return [];

	const icon = container.titleGlyph;
	const iconSpace = icon ? c.groupTitleIcon + gap : 0;
	const padding = STYLE.tag.paddingX;
	const width = Math.min(
		padding * 2 + iconSpace + textWidth(container.title, STYLE.text),
		size.width
	);
	const y = -EXTERNAL_TITLE_OFFSET;
	const parts: DiagramPart[] = [
		{
			...rect('title', 0, y, width, c.groupTitleHeight, {
				radius: STYLE.card.radius,
				fill: palette.node,
				stroke: palette.border,
				strokeWidth: 1
			}),
			text: {
				value: container.title,
				color: palette.textSecondary,
				size: STYLE.text,
				align: 'left',
				inset: padding + iconSpace
			}
		}
	];
	if (icon) {
		parts.push(
			...framedGlyph(
				'glyph',
				icon.frame,
				{ ...icon, size: icon.frame === 'tile' ? STYLE.group.memberGlyph : icon.size },
				padding,
				y + (c.groupTitleHeight - c.groupTitleIcon) / 2,
				c.groupTitleIcon,
				palette
			)
		);
	}
	return parts;
}

/** A collapsed root container's block: its summary, the ungrouped line, a row per subgroup. */
function collapsedSummary(
	container: ContainerDraw,
	size: Size,
	palette: DiagramPalette
): DiagramPart[] {
	const c = STYLE.container;
	const parts: DiagramPart[] = [];
	let y = c.summaryPaddingY;
	parts.push(
		textPart(0, y, size.width, c.summaryLine, {
			value: container.summary,
			color: palette.textSecondary,
			size: STYLE.summaryText,
			align: 'center'
		})
	);
	y += c.summaryLine + c.summaryGap;
	if (container.ungrouped && container.subgroups.length > 0) {
		parts.push(
			textPart(0, y, size.width, STYLE.lineHeight, {
				value: container.ungrouped,
				color: palette.textTertiary,
				size: STYLE.text,
				align: 'center'
			})
		);
		y += STYLE.lineHeight + c.summaryGap;
	}
	for (const subgroup of container.subgroups) {
		const width = Math.min(textWidth(subgroup, STYLE.text) + c.subgroupRowPaddingX * 2, size.width);
		parts.push({
			...rect('row', (size.width - width) / 2, y, width, c.subgroupRowHeight, {
				radius: STYLE.group.radius,
				fill: palette.subgroup,
				stroke: palette.ring,
				strokeWidth: 1,
				dashed: true
			}),
			text: { value: subgroup, color: palette.textTertiary, size: STYLE.text, align: 'center' }
		});
		y += c.subgroupRowHeight + c.summaryGap;
	}
	return parts;
}

/**
 * A container: its box, and the title, status badge, tags and collapsed summary the canvas
 * draws on it.
 */
export function containerShape(
	container: ContainerDraw,
	size: Size,
	palette: DiagramPalette
): { box: DiagramBox; parts: DiagramPart[] } {
	const c = STYLE.container;
	const parts: DiagramPart[] = [];
	let box: DiagramBox;

	if (container.isSubcontainer) {
		box =
			container.hasBorder || container.isCollapsed
				? {
						outline: 'rect',
						radius: c.subRadius,
						fill: palette.subgroup,
						stroke: palette.ring,
						strokeWidth: 1,
						dashed: true
					}
				: { outline: 'rect' };
	} else {
		// The canvas separates a root container from the page with a drop shadow, which neither
		// format carries; the border colour stands in for it.
		box = {
			outline: 'rect',
			radius: c.radius,
			fill: palette.node,
			stroke: palette.border,
			strokeWidth: 1
		};
		// The accent is the box's top border, which follows its rounded corners; a strip between
		// the corners is the nearest shape both formats draw.
		parts.push(
			rect('accent', c.radius, 0, Math.max(size.width - c.radius * 2, 0), c.accentHeight, {
				fill: container.accent
			})
		);
	}

	if (container.externalTitle) {
		parts.push(...externalTitle(container, size, palette));
	}

	if (container.isCollapsed && container.isSubcontainer) {
		const y = (size.height - STYLE.tag.height) / 2;
		const right = size.width - c.collapsedPaddingX;
		const countWidth = container.count ? textWidth(container.count, STYLE.text) : 0;
		parts.push(
			...titleRow(container, c.collapsedPaddingX, y, right - countWidth, palette, { tagLimit: 2 })
		);
		if (container.count) {
			parts.push(
				textPart(right - countWidth, y, countWidth, STYLE.tag.height, {
					value: container.count,
					color: palette.textTertiary,
					size: STYLE.text,
					align: 'right'
				})
			);
		}
	} else if (container.isCollapsed) {
		parts.push(...collapsedSummary(container, size, palette));
	} else if (!container.externalTitle) {
		parts.push(
			...titleRow(container, c.inlineInset, c.inlineInset, size.width - c.inlineInset, palette)
		);
	}

	return { box, parts };
}
