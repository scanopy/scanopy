/**
 * Browser-side inputs for a diagram export: node geometry from SvelteFlow's internal nodes, and
 * the image behind each icon the descriptions name.
 */

import type { InternalNode, Node } from '@xyflow/svelte';
import type { DiagramIcon, DiagramSourceNode } from './model';
import type { DiagramIconSource } from './describe';

/**
 * Absolute position and measured size of every node. `getNodes()` carries neither: positions are
 * parent-relative and measured sizes live on the internal node.
 */
export function collectSourceNodes(
	nodes: Node[],
	getInternalNode: (id: string) => InternalNode | undefined
): DiagramSourceNode[] {
	const sources: DiagramSourceNode[] = [];
	for (const node of nodes) {
		if (node.hidden) continue;
		const internal = getInternalNode(node.id);
		const position = internal?.internals.positionAbsolute ?? node.position;
		const width = internal?.measured?.width ?? node.width ?? node.measured?.width ?? 0;
		const height = internal?.measured?.height ?? node.height ?? node.measured?.height ?? 0;
		sources.push({
			id: node.id,
			type: node.type === 'Container' ? 'Container' : 'Element',
			parentId: node.parentId,
			x: position.x,
			y: position.y,
			width,
			height
		});
	}
	return sources;
}

function bytesToBase64(bytes: Uint8Array): string {
	let binary = '';
	for (let i = 0; i < bytes.length; i += 0x8000) {
		binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
	}
	return btoa(binary);
}

function svgDataUri(svg: string): string {
	return `data:image/svg+xml;base64,${bytesToBase64(new TextEncoder().encode(svg))}`;
}

/**
 * A data URI in the base64 form. A draw.io style separates its entries with `;`, so the URI an
 * image is styled with can hold only the one `;base64` marker that `drawio.ts` strips; a
 * `;charset=utf-8,` SVG URI would split the style.
 */
export function base64DataUri(dataUri: string): string {
	const comma = dataUri.indexOf(',');
	const header = dataUri.slice(0, comma);
	if (header.endsWith(';base64')) return dataUri;
	const mime = header.slice('data:'.length).split(';')[0] || 'image/svg+xml';
	const bytes = new TextEncoder().encode(decodeURIComponent(dataUri.slice(comma + 1)));
	return `data:${mime};base64,${bytesToBase64(bytes)}`;
}

async function fetchImageDataUri(src: string): Promise<string | null> {
	if (src.startsWith('data:')) return base64DataUri(src);
	try {
		const response = await fetch(src);
		if (!response.ok) return null;
		const blob = await response.blob();
		if (blob.type.includes('svg')) return svgDataUri(await blob.text());
		const bytes = new Uint8Array(await blob.arrayBuffer());
		return `data:${blob.type || 'image/png'};base64,${bytesToBase64(bytes)}`;
	} catch {
		return null;
	}
}

/** Edge of the square bitmap an icon is drawn into: sharp for a 48px tile logo at 2x zoom. */
const RASTER_SIZE = 128;

/**
 * Where an image of `width` by `height` sits in a square of `edge`: scaled to fit, centred, its
 * proportions kept. The part that shows the bitmap is square, so a bitmap filled edge to edge
 * would stretch a wide or tall logo. An image with no size of its own fills the square.
 */
export function fitInside(
	width: number,
	height: number,
	edge: number
): { x: number; y: number; width: number; height: number } {
	if (!(width > 0) || !(height > 0)) return { x: 0, y: 0, width: edge, height: edge };
	const scale = edge / Math.max(width, height);
	const fitted = { width: width * scale, height: height * scale };
	return { x: (edge - fitted.width) / 2, y: (edge - fitted.height) / 2, ...fitted };
}

async function rasterizeToPng(dataUri: string): Promise<Uint8Array | undefined> {
	try {
		const image = new Image();
		image.src = dataUri;
		await image.decode();
		const canvas = document.createElement('canvas');
		canvas.width = RASTER_SIZE;
		canvas.height = RASTER_SIZE;
		const box = fitInside(image.naturalWidth, image.naturalHeight, RASTER_SIZE);
		canvas.getContext('2d')?.drawImage(image, box.x, box.y, box.width, box.height);
		const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'));
		return blob ? new Uint8Array(await blob.arrayBuffer()) : undefined;
	} catch {
		return undefined;
	}
}

/**
 * The image for every icon the descriptions name, by the same key. `png` also rasterizes each
 * one, for formats that need bitmaps. An icon that fails to load is left out, and the part that
 * names it is not drawn.
 */
export async function loadIcons(
	sources: Map<string, DiagramIconSource>,
	options: { png: boolean }
): Promise<Map<string, DiagramIcon>> {
	const icons = new Map<string, DiagramIcon>();
	await Promise.all(
		[...sources].map(async ([key, source]) => {
			const dataUri =
				'svg' in source ? svgDataUri(source.svg) : await fetchImageDataUri(source.src);
			if (!dataUri) return;
			icons.set(key, { dataUri, png: options.png ? await rasterizeToPng(dataUri) : undefined });
		})
	);
	return icons;
}
