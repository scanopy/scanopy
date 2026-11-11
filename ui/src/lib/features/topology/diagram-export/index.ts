/**
 * Editable diagram exports (draw.io, Visio) of the topology as drawn. Loaded on demand from the
 * export modal, so fflate and the serializers stay out of the main bundle.
 */

import type { Edge, InternalNode, Node } from '@xyflow/svelte';
import { cardGlyph } from '../card-glyph';
import type { RenderableTopology, TopologyNode } from '../types/base';
import { collectSourceNodes, loadIcons } from './collect';
import { createDescriber, diagramHeader } from './describe';
import { serializeDrawio } from './drawio';
import { buildDiagramModel } from './model';
import { DIAGRAM_PALETTE, type DiagramTheme } from './style';
import { buildVsdxParts, zipVsdx } from './vsdx';

export type DiagramFormat = 'drawio' | 'vsdx';

export interface DiagramExportInput {
	format: DiagramFormat;
	name: string;
	nodes: Node[];
	edges: Edge[];
	getInternalNode: (id: string) => InternalNode | undefined;
	topology: RenderableTopology;
	hiddenEdgeTypes: readonly string[];
	theme: DiagramTheme;
	header: { title: string; subtitle: string } | null;
}

export async function buildDiagramExport(input: DiagramExportInput): Promise<Blob> {
	const describe = createDescriber({
		topology: input.topology,
		nodeData: new Map(input.nodes.map((n) => [n.id, n.data as unknown as TopologyNode])),
		edges: new Map(input.edges.map((e) => [e.id, e])),
		hiddenEdgeTypes: input.hiddenEdgeTypes,
		theme: input.theme,
		glyph: cardGlyph
	});
	const model = buildDiagramModel({
		name: input.name,
		nodes: collectSourceNodes(input.nodes, input.getInternalNode),
		edges: input.edges,
		background: DIAGRAM_PALETTE[input.theme].canvas,
		header: input.header
			? diagramHeader(input.theme, input.header.title, input.header.subtitle)
			: null,
		describe
	});
	// The descriptions name their icons as they are built, so the images load after the model.
	model.icons = await loadIcons(describe.icons, { png: input.format === 'vsdx' });

	if (input.format === 'drawio') {
		return new Blob([serializeDrawio(model)], { type: 'application/vnd.jgraph.mxfile' });
	}
	const zip = zipVsdx(buildVsdxParts(model));
	const buffer = new ArrayBuffer(zip.byteLength);
	new Uint8Array(buffer).set(zip);
	return new Blob([buffer], { type: 'application/vnd.ms-visio.drawing' });
}
