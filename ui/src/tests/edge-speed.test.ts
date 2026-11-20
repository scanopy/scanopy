import { describe, it, expect } from 'vitest';
import {
	edgeColoringsFor,
	effectiveEdgeColoring,
	LINK_SPEED_TIERS,
	speedStrokeColors,
	speedStrokeOf,
	speedStrokeWidth,
	tierOf,
	TIER_STROKE_WIDTHS,
	UNKNOWN_STROKE_WIDTH
} from '#lib/features/topology/edge-speed.ts';

const MBPS = 1_000_000;
const GBPS = 1_000_000_000;

/** Port speeds by interface id. */
const speeds: Record<string, number | null> = {
	ring1: 10 * GBPS,
	ring2: 10 * GBPS,
	cam: GBPS,
	plcSwitch: 100 * MBPS,
	plc: 100 * MBPS,
	slowEnd: 100 * MBPS,
	fastEnd: GBPS,
	silent: null
};
const speedOf = (id: string) => speeds[id];

const link = (source: string, target: string) => ({
	edge_type: 'PhysicalLink' as const,
	source_entity_id: source,
	target_entity_id: target
});

const ramp = ['slowest', 'slow', 'fast', 'fastest'];

describe('tierOf', () => {
	it('files each band from its lowest speed, and the speed below it in the band beneath', () => {
		LINK_SPEED_TIERS.forEach((band, index) => {
			const floor = band.metadata.min_bps;
			if (floor > 0) {
				expect(tierOf(floor)).toBe(index);
				expect(tierOf(floor - 1)).toBe(index - 1);
			}
		});
	});

	it('has no band for a speed the port did not report', () => {
		expect(tierOf(null)).toBeNull();
		expect(tierOf(undefined)).toBeNull();
		expect(tierOf(0)).toBeNull();
	});
});

describe('speedStrokeOf', () => {
	it('draws a link whose ends agree in their band, as one colour', () => {
		const stroke = speedStrokeOf(link('ring1', 'ring2'), undefined, speedOf);
		expect(stroke).toEqual({ tier: tierOf(10 * GBPS), ends: null });
		expect(speedStrokeColors(stroke, ramp, 'grey').ends).toBeNull();
	});

	it('draws a link whose ends differ at the slower end’s width, each half in its end’s colour', () => {
		const stroke = speedStrokeOf(link('fastEnd', 'slowEnd'), undefined, speedOf);
		const slow = tierOf(100 * MBPS)!;
		const fast = tierOf(GBPS)!;
		expect(stroke.tier).toBe(slow);
		expect(speedStrokeWidth(stroke)).toBe(TIER_STROKE_WIDTHS[slow]);
		expect(speedStrokeColors(stroke, ramp, 'grey')).toEqual({
			color: ramp[slow],
			ends: [ramp[fast], ramp[slow]]
		});
	});

	it('takes the one end that reported a speed for the whole link', () => {
		const stroke = speedStrokeOf(link('silent', 'cam'), undefined, speedOf);
		expect(stroke).toEqual({ tier: tierOf(GBPS), ends: null });
	});

	it('draws a neighbour link, which names no port, as unknown', () => {
		const neighbour = { ...link('cam', 'ring1'), edge_type: 'NeighborLink' as const };
		const stroke = speedStrokeOf(neighbour, undefined, speedOf);
		expect(stroke).toEqual({ tier: null, ends: null });
		expect(speedStrokeWidth(stroke)).toBe(UNKNOWN_STROKE_WIDTH);
		expect(speedStrokeColors(stroke, ramp, 'grey').color).toBe('grey');
	});

	it('draws an edge standing for several links at its fastest link, with no split', () => {
		const members = [link('plcSwitch', 'plc'), link('ring1', 'ring2'), link('fastEnd', 'slowEnd')];
		const stroke = speedStrokeOf(members[0], members, speedOf);
		expect(stroke).toEqual({ tier: tierOf(10 * GBPS), ends: null });
	});
});

describe('effectiveEdgeColoring', () => {
	it('colours by type unless the view offers the stored colouring', () => {
		const local = { edge_coloring: { L2Physical: 'Speed', L3Logical: 'Speed' } } as never;
		expect(effectiveEdgeColoring(local, 'L2Physical')).toBe('Speed');
		// L3 has no ports to read speeds from, so it does not offer speed.
		expect(edgeColoringsFor('L3Logical').map((c) => c.id)).not.toContain('Speed');
		expect(effectiveEdgeColoring(local, 'L3Logical')).toBe('Type');
		expect(effectiveEdgeColoring(undefined, 'L2Physical')).toBe('Type');
	});

	it('puts a share’s own colouring ahead of the stored one', () => {
		expect(effectiveEdgeColoring(undefined, 'L2Physical', 'Speed')).toBe('Speed');
	});
});
