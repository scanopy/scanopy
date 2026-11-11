/**
 * Text and colours for each node and edge of a diagram export, read from the same sources the
 * canvas components use (`ElementNode`, `ContainerNode`, `CustomEdge`), plus each shape's entity
 * fields from the backend registry as Shape Data.
 */

import type { Edge } from '@xyflow/svelte';
import { containerTypes, edgeTypes, entities } from '#lib/shared/stores/metadata.ts';
import { common_type } from '#lib/paraglide/messages.js';
import {
	edgeFieldLabel,
	edgeFields,
	entityFieldLabel,
	entityFields,
	fieldText,
	type EntityLookup
} from '#lib/shared/entity-fields/index.ts';
import { nodeEntity, topologyLookup } from '../resolvers';
import { createColorHelper, type Color } from '#lib/shared/utils/styling.ts';
import { buildElementRender, currentElementRenderContext } from '../element-render-data';
import { formatElementSummary, tallyContainerElements } from '../labels';
import { getInfrastructureRuleIdForTopology } from '../queries';
import {
	isDottedEdge,
	isEdgeHidden,
	isOverlayEdge,
	restingEdgeStrokeWidth
} from '../layout/edge-classification';
import type { RenderableTopology, TopologyEdge, TopologyNode } from '../types/base';
import type {
	DiagramDescriber,
	DiagramEdgeDescription,
	DiagramField,
	DiagramHeader,
	DiagramNodeDescription,
	DiagramSourceNode
} from './model';

export type DiagramTheme = 'light' | 'dark';

/**
 * Opaque equivalents of the canvas colours (`app.css` topology variables and the export-only
 * card borders in `topology-viewer.css`). Visio has no alpha on fills, so the translucent
 * subgroup background is pre-blended over the node background.
 */
export const DIAGRAM_PALETTE: Record<
	DiagramTheme,
	{
		canvas: string;
		node: string;
		subgroup: string;
		border: string;
		text: string;
		mutedText: string;
	}
> = {
	light: {
		canvas: '#f8fafc',
		node: '#ffffff',
		subgroup: '#f7f7f7',
		border: '#94a3b8',
		text: '#1f2937',
		mutedText: '#64748b'
	},
	dark: {
		canvas: '#15131e',
		node: '#1a1d29',
		subgroup: '#232633',
		border: '#6b7280',
		text: '#f3f4f6',
		mutedText: '#9ca3af'
	}
};

export function diagramHeader(theme: DiagramTheme, title: string, subtitle: string): DiagramHeader {
	const palette = DIAGRAM_PALETTE[theme];
	return { title, subtitle, color: palette.text, subtitleColor: palette.mutedText };
}

export interface DescriberInput {
	topology: RenderableTopology;
	/** Flow node data (the backend node, plus `isCollapsed` when collapsed), by node id. */
	nodeData: Map<string, TopologyNode & { isCollapsed?: boolean }>;
	/** Flow edges by id, for their data. */
	edges: Map<string, Edge>;
	hiddenEdgeTypes: readonly string[];
	theme: DiagramTheme;
	/** Icon key per node id, for nodes whose icon was captured. */
	iconKeys: Map<string, string>;
}

function elementDescription(
	id: string,
	node: TopologyNode,
	topology: RenderableTopology
): { title: string; lines: string[] } {
	const { data, iconCard } = buildElementRender({
		nodeId: id,
		node,
		topology,
		...currentElementRenderContext()
	});
	if (!data) return { title: node.header ?? '', lines: [] };
	// An icon card shows only its label, so its shape does too; the rest stays in Shape Data.
	if (iconCard) return { title: iconCard.label, lines: [] };
	return { title: data.headerText ?? '', lines: data.bodyText ? [data.bodyText] : [] };
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

export function createDescriber(input: DescriberInput): DiagramDescriber {
	const palette = DIAGRAM_PALETTE[input.theme];
	const infraRuleId = getInfrastructureRuleIdForTopology(input.topology);
	const activeView = currentElementRenderContext().activeView;
	const lookup = topologyLookup(input.topology);

	return {
		node(source: DiagramSourceNode): DiagramNodeDescription {
			const data = input.nodeData.get(source.id);
			const iconKey = input.iconKeys.get(source.id);
			if (!data || data.node_type !== 'Container') {
				const { title, lines } = data
					? elementDescription(source.id, data, input.topology)
					: { title: '', lines: [] };
				return {
					title,
					lines,
					iconKey,
					data: data
						? shapeData(entities.getName(data.element_type ?? null), data, input.topology, lookup)
						: [],
					style: { fill: palette.node, stroke: palette.border, text: palette.text }
				};
			}

			const containerType = data.container_type ?? 'Subnet';
			const isSubcontainer = containerTypes.getMetadata(containerType)?.is_subcontainer ?? false;
			const accent = data.color
				? createColorHelper(data.color as Color).rgb
				: containerTypes.getColorHelper(containerType).rgb;
			const lines = data.isCollapsed
				? [
						formatElementSummary(
							tallyContainerElements(source.id, input.topology, infraRuleId),
							activeView
						)
					].filter(Boolean)
				: [];
			return {
				title: data.header ?? '',
				lines,
				iconKey,
				data: shapeData(containerTypes.getName(containerType), data, input.topology, lookup),
				style: {
					fill: isSubcontainer ? palette.subgroup : palette.node,
					stroke: palette.border,
					text: palette.text,
					accent: isSubcontainer ? undefined : accent
				}
			};
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
			const isDependencyEdge = data
				? (edgeTypes.getMetadata(data.edge_type)?.is_dependency_edge ?? false)
				: false;
			const dependency =
				data && 'dependency_id' in data
					? input.topology.dependencies?.find((d) => d.id === data.dependency_id)
					: undefined;
			const color = dependency?.color
				? createColorHelper(dependency.color).rgb
				: data
					? edgeTypes.getColorHelper(data.edge_type).rgb
					: createColorHelper('Gray').rgb;

			return {
				label: typeof flowEdge?.label === 'string' ? flowEdge.label : undefined,
				color,
				width: restingEdgeStrokeWidth({
					isAggregated: !!data?.isAggregated,
					aggregatedCount: (data?.aggregatedCount as number) ?? 1,
					isBundle,
					bundleStrokeWidth: (data?.bundleStrokeWidth as number) ?? 2,
					isOverlay
				}),
				dash: !isDependencyEdge && isOverlay ? (isDotted ? 'dotted' : 'dashed') : 'solid',
				data: data ? edgeData(data, lookup) : []
			};
		}
	};
}
