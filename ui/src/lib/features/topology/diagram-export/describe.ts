/**
 * What each node and edge of a diagram export draws, read from the same render data the canvas
 * components render from (`buildElementRender` for `ElementNode`, `buildContainerRender` for
 * `ContainerNode`, the edge classification for `CustomEdge`), plus each shape's entity fields
 * from the backend registry as Shape Data.
 *
 * Nothing here decides what a card shows. A card or header that changes on the canvas changes in
 * those render functions, and the export follows; where it sits is `parts.ts`.
 */

import type { Edge } from '@xyflow/svelte';
import {
	containerTypes,
	deviceTypes,
	edgeTypes,
	entities,
	serviceDefinitions
} from '$lib/shared/stores/metadata';
import {
	common_type,
	topology_elementCount,
	topology_ungroupedCount
} from '$lib/paraglide/messages';
import {
	edgeFieldLabel,
	edgeFields,
	entityFieldLabel,
	entityFields,
	fieldText,
	type EntityLookup
} from '$lib/shared/entity-fields';
import type { TagProps } from '$lib/shared/components/data/types';
import { createColorHelper } from '$lib/shared/utils/styling';
import type { IconComponent } from '$lib/shared/utils/types';
import { hostOsIcon } from '$lib/features/hosts/host-os';
import type { CardGlyph } from '../card-glyph';
import { buildContainerRender, type SubgroupRow } from '../container-render-data';
import { buildElementRender, currentElementRenderContext } from '../element-render-data';
import type { IconCardGroup } from '../element-render-data';
import type { EntityGlyph } from '../entity-glyph';
import {
	getLinkEvidenceTag,
	isDottedEdge,
	isEdgeHidden,
	isOverlayEdge,
	restingEdgeStrokeWidth
} from '../layout/edge-classification';
import { nodeEntity, topologyLookup } from '../resolvers';
import type { RenderableTopology, TopologyEdge, TopologyNode } from '../types/base';
import type {
	DiagramDescriber,
	DiagramEdgeDescription,
	DiagramField,
	DiagramHeader,
	DiagramNodeDescription,
	DiagramSourceNode
} from './model';
import {
	containerShape,
	iconCardShape,
	textCardShape,
	type ContainerDraw,
	type GlyphDraw,
	type GroupDraw,
	type TagDraw
} from './parts';
import { classColor, DIAGRAM_PALETTE, STYLE, tagColors, type DiagramTheme } from './style';

export function diagramHeader(theme: DiagramTheme, title: string, subtitle: string): DiagramHeader {
	const palette = DIAGRAM_PALETTE[theme];
	return { title, subtitle, color: palette.text, subtitleColor: palette.textTertiary };
}

/** Where an icon's image comes from: inline SVG markup, or a URL to fetch. */
export type DiagramIconSource = { svg: string } | { src: string };

export interface DescriberInput {
	topology: RenderableTopology;
	/** Flow node data (the backend node, plus `isCollapsed` when collapsed), by node id. */
	nodeData: Map<string, TopologyNode & { isCollapsed?: boolean }>;
	/** Flow edges by id, for their data. */
	edges: Map<string, Edge>;
	hiddenEdgeTypes: readonly string[];
	theme: DiagramTheme;
	/**
	 * What an icon component draws, under the key the canvas caches it by: `cardGlyph` in the
	 * app, so a card's icon in the file is the one on the card.
	 */
	glyph: (key: string, icon: IconComponent) => CardGlyph | undefined;
}

/** A describer, and the source of every icon its descriptions name by key. */
export type TopologyDescriber = DiagramDescriber & { icons: Map<string, DiagramIconSource> };

/**
 * A cache key for an icon that has no name of its own (a tag's icon, an entity's chip icon). Kept
 * per component for the page's lifetime, since the glyph cache it keys outlives one export.
 */
const componentKeys = new WeakMap<object, string>();
let componentKeyCount = 0;
function componentKey(icon: IconComponent): string {
	let key = componentKeys.get(icon);
	if (!key) {
		key = `component:${++componentKeyCount}`;
		componentKeys.set(icon, key);
	}
	return key;
}

/**
 * A shape's Shape Data: its type, then every registry field of the entity it stands for, with
 * empty fields left out. Which fields an entity has, and where related values come from, is the
 * backend registry's call (`shared/entity-fields`).
 */
function shapeData(
	typeName: string,
	node: TopologyNode,
	topology: RenderableTopology,
	lookup: EntityLookup
): DiagramField[] {
	const data: DiagramField[] = [{ label: common_type(), value: typeName }];
	const resolved = nodeEntity(node, topology);
	if (!resolved) return data;
	for (const field of entityFields(resolved.entityType)) {
		const value = fieldText(field, resolved.entity, lookup);
		if (value) data.push({ label: entityFieldLabel(resolved.entityType, field), value });
	}
	return data;
}

function edgeData(edge: TopologyEdge, lookup: EntityLookup): DiagramField[] {
	const data: DiagramField[] = [{ label: common_type(), value: edgeTypes.getName(edge.edge_type) }];
	for (const field of edgeFields(edge.edge_type)) {
		const value = fieldText(field, edge, lookup);
		if (value) data.push({ label: edgeFieldLabel(edge.edge_type, field), value });
	}
	return data;
}

/** A collapsed root container's subgroup row, as one line of text. */
function subgroupText(row: SubgroupRow): string {
	const labels = row.labels
		.slice(0, 2)
		.map((l) => l.label)
		.join(', ');
	const count = row.hideCount
		? ''
		: row.countOnly
			? ` (${row.childCount})`
			: ` (${topology_elementCount({ summary: row.childSummary })})`;
	return `${row.headerText}${labels ? `: ${labels}` : ''}${count}`;
}

export function createDescriber(input: DescriberInput): TopologyDescriber {
	const { topology, theme } = input;
	const palette = DIAGRAM_PALETTE[theme];
	const context = currentElementRenderContext();
	const lookup = topologyLookup(topology);
	const icons = new Map<string, DiagramIconSource>();

	/**
	 * An icon at the size the canvas draws it: `size` for a glyph, and for a logo the size its
	 * frame gives it (`ElementNode`'s `glyph` snippet). A glyph drawn in the text colour is tinted
	 * `color`; a `Paired` logo gives the file for the export's theme.
	 */
	function draw(
		key: string,
		icon: IconComponent,
		size: number,
		color: string,
		frame = size
	): GlyphDraw {
		const glyph = input.glyph(key, icon);
		if (!glyph) return { size };
		if (glyph.type === 'mask') {
			if (!glyph.svg) return { size };
			const iconKey = `${key}|${color}`;
			icons.set(iconKey, { svg: glyph.svg.replaceAll('currentColor', color) });
			return { iconKey, size };
		}
		const src = theme === 'dark' && glyph.darkSrc ? glyph.darkSrc : glyph.src;
		if (!src) return { size };
		icons.set(src, { src });
		return {
			iconKey: src,
			size: glyph.round ? frame : glyph.fill ? size : size - 2,
			fill: glyph.fill
		};
	}

	const serviceGlyph = (definition: string, size: number, frame = size) =>
		draw(
			`service:${definition}`,
			serviceDefinitions.getIconComponent(definition),
			size,
			palette.text,
			frame
		);
	const deviceGlyph = (deviceType: string, size: number) =>
		draw(`device:${deviceType}`, deviceTypes.getIconComponent(deviceType), size, palette.text);
	const portGlyph = (size: number) =>
		draw('entity:Interface', entities.getIconComponent('Interface'), size, palette.textSecondary);

	/** A status tag (Stale, Unconfirmed) as the icon badge the canvas draws: no label. */
	function statusBadge(tag: TagProps | null | undefined): TagDraw | undefined {
		if (!tag) return undefined;
		const colors = tagColors(tag.color ?? 'Gray', theme, palette.node);
		return {
			...colors,
			pill: true,
			icon: tag.icon
				? draw(componentKey(tag.icon), tag.icon, STYLE.tag.icon, colors.text)
				: undefined
		};
	}

	function tile(glyph: EntityGlyph) {
		const t = STYLE.tile;
		if (glyph.type === 'software') {
			return {
				outline: 'ellipse' as const,
				fill: palette.node,
				stroke: palette.ring,
				glyph: serviceGlyph(glyph.definition, t.glyph, t.frameInner)
			};
		}
		return {
			outline: 'rect' as const,
			fill: palette.tile,
			stroke: palette.tile,
			glyph: glyph.type === 'device' ? deviceGlyph(glyph.deviceType, t.glyph) : portGlyph(t.glyph)
		};
	}

	function group(group: IconCardGroup): GroupDraw {
		const g = STYLE.group;
		return {
			headingGlyph: group.headerDefinition
				? serviceGlyph(group.headerDefinition, g.headingGlyph)
				: undefined,
			headingName: group.headerName,
			collapsed: group.collapsed,
			members: group.members.map((member) => ({
				frame:
					member.glyph.type === 'device'
						? 'tile'
						: member.glyph.type === 'software'
							? 'ring'
							: 'none',
				glyph:
					member.glyph.type === 'device'
						? deviceGlyph(member.glyph.deviceType, g.memberGlyph)
						: member.glyph.type === 'software'
							? serviceGlyph(member.glyph.definition, g.memberGlyph, g.glyph - 2)
							: portGlyph(g.headingGlyph),
				label: member.label,
				statusColor: member.status ? createColorHelper(member.status.color).rgb : undefined
			}))
		};
	}

	function element(
		source: DiagramSourceNode,
		data: Extract<TopologyNode, { node_type: 'Element' }>
	): DiagramNodeDescription {
		const render = buildElementRender({ nodeId: source.id, node: data, topology, ...context });
		const freshness = statusBadge(render.staleTag);
		const card = render.iconCard;
		const b = STYLE.badge;
		const shape = card
			? iconCardShape(
					{
						tile: tile(card.glyph),
						statusColor: render.marks.Status
							? createColorHelper(render.marks.Status.color).rgb
							: undefined,
						os:
							card.os && card.glyph.type === 'device'
								? {
										fill: palette.node,
										glyph: draw(
											`os:${card.os.family}`,
											hostOsIcon(card.os.family),
											b.osGlyph,
											palette.text,
											b.frameInner
										)
									}
								: undefined,
						badges: card.badgeDefinitions.map((definition) => ({
							fill: palette.node,
							glyph: serviceGlyph(definition, b.serviceGlyph, b.frameInner)
						})),
						freshness,
						label: card.label,
						sublabel: card.sublabel,
						groups: card.groups.map(group)
					},
					source,
					palette
				)
			: textCardShape(
					{
						header: render.data ? render.data.headerText : data.header,
						body: render.data?.bodyText,
						freshness
					},
					source,
					palette
				);
		return {
			box: {
				outline: 'rect',
				radius: STYLE.card.radius,
				fill: palette.node,
				stroke: palette.cardBorder,
				strokeWidth: STYLE.card.borderWidth
			},
			...shape,
			data: shapeData(entities.getName(data.element_type ?? null), data, topology, lookup)
		};
	}

	function container(
		source: DiagramSourceNode,
		data: TopologyNode & { isCollapsed?: boolean; childCount?: number }
	): DiagramNodeDescription {
		const isCollapsed = !!data.isCollapsed;
		const render = buildContainerRender({
			nodeId: source.id,
			node: data,
			topology,
			activeView: context.activeView,
			options: context.options,
			sites: context.sites,
			isCollapsed
		});
		const meta = containerTypes.getMetadata(render.containerType);
		const externalTitle = meta?.title_style === 'External';
		const c = STYLE.container;

		const entityColors = render.entityColor
			? tagColors(render.entityColor, theme, palette.node)
			: undefined;

		// A grouping's glyph: a device tile, else its service's bare logo, else its type icon. An
		// expanded subcontainer leaves the type icon out, as its header does.
		let titleGlyph: ContainerDraw['titleGlyph'];
		if (render.deviceType) {
			titleGlyph = { frame: 'tile', ...deviceGlyph(render.deviceType, STYLE.group.memberGlyph) };
		} else if (render.serviceDefinition) {
			titleGlyph = { frame: 'none', ...serviceGlyph(render.serviceDefinition, STYLE.tag.icon) };
		} else if (render.iconComponent && (externalTitle || isCollapsed)) {
			const tint =
				classColor(render.colorHelper.icon, 'text', theme, palette.node) ?? render.colorHelper.rgb;
			titleGlyph = {
				frame: 'none',
				...draw(
					componentKey(render.iconComponent),
					render.iconComponent,
					externalTitle ? c.groupTitleIcon : c.inlineIcon,
					tint
				)
			};
		}

		const childCount = data.childCount ?? 0;
		const ungroupedCount = childCount - render.subgroups.reduce((sum, s) => sum + s.childCount, 0);
		const shape = containerShape(
			{
				isSubcontainer: render.isSubcontainer,
				hasBorder: meta?.has_border ?? true,
				isCollapsed,
				externalTitle,
				accent: render.colorHelper.rgb,
				title: render.headerText,
				entity: entityColors && {
					...entityColors,
					icon: render.chipIcon
						? draw(
								componentKey(render.chipIcon),
								render.chipIcon,
								STYLE.tag.icon,
								entityColors.text
							)
						: undefined
				},
				titleGlyph,
				freshness: statusBadge(render.staleTag),
				tags: render.groupLabels.map((pill) => ({
					label: pill.label,
					...tagColors(pill.color, theme, palette.node)
				})),
				summary: topology_elementCount({ summary: render.childSummary }),
				count: render.isInfraRule
					? `(${childCount})`
					: `(${topology_elementCount({ summary: render.childSummary })})`,
				ungrouped:
					ungroupedCount > 0
						? topology_ungroupedCount({ summary: render.ungroupedSummary })
						: undefined,
				subgroups: render.subgroups.map(subgroupText)
			},
			source,
			palette
		);
		return {
			...shape,
			data: shapeData(containerTypes.getName(render.containerType), data, topology, lookup)
		};
	}

	return {
		icons,

		node(source: DiagramSourceNode): DiagramNodeDescription {
			const data = input.nodeData.get(source.id);
			if (!data) {
				return {
					box: { outline: 'rect', radius: STYLE.card.radius, fill: palette.node },
					parts: []
				};
			}
			return data.node_type === 'Container' ? container(source, data) : element(source, data);
		},

		edge(source): DiagramEdgeDescription | null {
			const flowEdge = input.edges.get(source.id);
			const data = flowEdge?.data as
				(TopologyEdge & Record<string, unknown> & { is_preview?: boolean }) | undefined;
			// A preview edge is an unsaved dependency draft in the editor, not part of the topology.
			if (data?.is_preview) return null;
			if (
				isEdgeHidden(data, {
					isPreview: false,
					simplified: false,
					hiddenEdgeTypes: input.hiddenEdgeTypes
				})
			) {
				return null;
			}

			const isBundle = !!data?.isBundle;
			const isOverlay = isBundle ? !!data?.bundleIsOverlay : data ? isOverlayEdge(data) : false;
			const isDotted = !isBundle && data ? isDottedEdge(data) : false;
			const typeMeta = data ? edgeTypes.getMetadata(data.edge_type) : null;
			const isDependencyEdge = typeMeta?.is_dependency_edge ?? false;
			const dependency =
				data && 'dependency_id' in data
					? topology.dependencies?.find((d) => d.id === data.dependency_id)
					: undefined;
			const color = dependency?.color
				? createColorHelper(dependency.color).rgb
				: data
					? edgeTypes.getColorHelper(data.edge_type).rgb
					: createColorHelper('Gray').rgb;
			const edgeStyle = dependency?.edge_style ?? typeMeta?.edge_style;

			// A bundle draws no label on the canvas. A stale link keeps its mark beside the label,
			// and keeps a label for it where the label itself was stripped.
			const stale =
				!isBundle && data
					? getLinkEvidenceTag(data, topology.interfaces ?? [], topology.neighbours ?? [])
					: null;
			const text = !isBundle && typeof flowEdge?.label === 'string' ? flowEdge.label : '';
			const label = [text, stale?.label].filter(Boolean).join(' · ');

			return {
				label: label || undefined,
				labelStyle: {
					color: palette.textSecondary,
					size: STYLE.edge.labelText,
					fill: palette.node,
					stroke: palette.border
				},
				color,
				width: restingEdgeStrokeWidth({
					isAggregated: !!data?.isAggregated,
					aggregatedCount: (data?.aggregatedCount as number) ?? 1,
					isBundle,
					bundleStrokeWidth: (data?.bundleStrokeWidth as number) ?? 2,
					isOverlay
				}),
				dash: !isDependencyEdge && isOverlay ? (isDotted ? 'dotted' : 'dashed') : 'solid',
				curved: edgeStyle === 'Bezier' || edgeStyle === 'Simplebezier',
				data: data ? edgeData(data, lookup) : []
			};
		}
	};
}
