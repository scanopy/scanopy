/**
 * Link speed, for views that colour links by it.
 *
 * A physical link joins two ports, and each reports its own speed (ifSpeed or ifHighSpeed). The
 * bands a speed is drawn in come from the backend (`LinkSpeedTier`); this file files each end's
 * speed under a band and works out what a link, or an edge standing for several, shows.
 */

import { getContext, setContext } from 'svelte';
import type { components } from '#lib/api/schema';
import edgeColoringsJson from '#lib/data/edge-colorings.json';
import linkSpeedTiersJson from '#lib/data/link-speed-tiers.json';
import type { TopologyEdge, TopologyOptions } from './types/base';

export type EdgeColoring = components['schemas']['EdgeColoring'];

/** The speed bands, slowest first, as the backend declares them. */
export const LINK_SPEED_TIERS = linkSpeedTiersJson;

/** Stroke width of a link in each band, slowest first. Width carries the order as well as colour. */
export const TIER_STROKE_WIDTHS = [1.5, 2, 3, 4] as const;

/** Stroke width of a link whose speed is unknown. */
export const UNKNOWN_STROKE_WIDTH = 1.5;

/**
 * The band a speed falls in, as an index into `LINK_SPEED_TIERS`: the fastest band whose lowest
 * speed it reaches. Null for a speed the port did not report.
 */
export function tierOf(bps: number | null | undefined): number | null {
	if (bps == null || bps <= 0) return null;
	let tier = 0;
	LINK_SPEED_TIERS.forEach((band, index) => {
		if (bps >= band.metadata.min_bps) tier = index;
	});
	return tier;
}

/** Each end's band. Null at an end whose port reported no speed. */
export interface LinkSpeed {
	source: number | null;
	target: number | null;
}

/**
 * The bands at the two ends of a physical link. Null for any other edge: a neighbour link names a
 * device, not a port, so it has no speed.
 */
export function linkSpeedOf(
	edge: Pick<TopologyEdge, 'edge_type'> & {
		source_entity_id?: string | null;
		target_entity_id?: string | null;
	},
	speedOf: (interfaceId: string) => number | null | undefined
): LinkSpeed | null {
	if (edge.edge_type !== 'PhysicalLink') return null;
	return {
		source: edge.source_entity_id ? tierOf(speedOf(edge.source_entity_id)) : null,
		target: edge.target_entity_id ? tierOf(speedOf(edge.target_entity_id)) : null
	};
}

/** The band a link runs at: the slower end's, or the only end that reported one. */
export function effectiveTier(speed: LinkSpeed | null): number | null {
	if (!speed) return null;
	const known = [speed.source, speed.target].filter((t): t is number => t !== null);
	return known.length ? Math.min(...known) : null;
}

/** Whether both ends reported a speed and the two fall in different bands. */
export function endsDiffer(speed: LinkSpeed | null): boolean {
	return !!speed && speed.source !== null && speed.target !== null && speed.source !== speed.target;
}

/**
 * The band an edge standing for several links shows: the fastest link among them, so a collapsed
 * uplink group reads at the speed of its best path. Null when none reported a speed.
 */
export function fastestTier(speeds: (LinkSpeed | null)[]): number | null {
	const tiers = speeds.map(effectiveTier).filter((t): t is number => t !== null);
	return tiers.length ? Math.max(...tiers) : null;
}

/** The colourings a view offers, in the order the backend declares them. */
export function edgeColoringsFor(view: string): (typeof edgeColoringsJson)[number][] {
	return edgeColoringsJson.filter((coloring) => coloring.metadata.views.includes(view));
}

/**
 * The colouring a view draws with: `chosen` if the view offers it, otherwise `Type`. A share's
 * own colouring, when it has one, comes before the topology's stored choice.
 */
export function effectiveEdgeColoring(
	local: TopologyOptions['local'] | undefined,
	view: string,
	shareColoring?: EdgeColoring | null
): EdgeColoring {
	const chosen = shareColoring ?? local?.edge_coloring?.[view];
	if (chosen && edgeColoringsFor(view).some((coloring) => coloring.id === chosen)) return chosen;
	return 'Type';
}

const SHARE_EDGE_COLORING = 'topology:shareEdgeColoring';

/** Gives the canvas a share's own colouring. Call while the share's viewer initialises. */
export function provideShareEdgeColoring(coloring: EdgeColoring | null | undefined): void {
	setContext(SHARE_EDGE_COLORING, coloring ?? null);
}

/** The colouring a share sets, or null outside a share. Call while a component initialises. */
export function useShareEdgeColoring(): EdgeColoring | null {
	return getContext<EdgeColoring | null | undefined>(SHARE_EDGE_COLORING) ?? null;
}

/** What a link coloured by speed shows: its band, and each end's band where the two differ. */
export interface SpeedStroke {
	tier: number | null;
	ends: LinkSpeed | null;
}

/**
 * The speed stroke of an edge drawn by speed. `members` are the links an edge stands for when it
 * stands for several (a bundle, or links between collapsed containers); it then shows the fastest
 * of them, and no two-tone split.
 */
export function speedStrokeOf(
	edge: Parameters<typeof linkSpeedOf>[0],
	members: Parameters<typeof linkSpeedOf>[0][] | undefined,
	speedOf: (interfaceId: string) => number | null | undefined
): SpeedStroke {
	if (members && members.length > 1) {
		return { tier: fastestTier(members.map((m) => linkSpeedOf(m, speedOf))), ends: null };
	}
	const link = linkSpeedOf(edge, speedOf);
	return { tier: effectiveTier(link), ends: endsDiffer(link) ? link : null };
}

/** Stroke width of a speed stroke at rest. */
export function speedStrokeWidth(stroke: SpeedStroke): number {
	return stroke.tier === null ? UNKNOWN_STROKE_WIDTH : TIER_STROKE_WIDTHS[stroke.tier];
}

/**
 * The colours of a speed stroke from `colors` (one colour per band, slowest first): one colour, or
 * the two ends' colours where they differ. `unknown` is the colour of a link with no speed.
 */
export function speedStrokeColors(
	stroke: SpeedStroke,
	colors: readonly string[],
	unknown: string
): { color: string; ends: [string, string] | null } {
	const color = stroke.tier === null ? unknown : colors[stroke.tier];
	const { ends } = stroke;
	return {
		color,
		ends:
			ends && ends.source !== null && ends.target !== null
				? [colors[ends.source], colors[ends.target]]
				: null
	};
}
