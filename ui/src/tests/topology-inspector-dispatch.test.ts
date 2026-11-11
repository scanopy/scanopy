import { describe, it, expect } from 'vitest';
import viewsJson from '#lib/data/views.json';
import { getSectionComponent } from '#lib/features/topology/components/panel/inspectors/view-config.ts';
import { aggregatedDisplay } from '#lib/features/topology/components/panel/inspectors/edges/aggregated-display.ts';
import type { RenderableTopology, TopologyEdge } from '#lib/features/topology/types/base.ts';

describe('inspector dispatch', () => {
	it('has a component for every section a view lists', () => {
		for (const view of viewsJson) {
			const config = (
				view.metadata as {
					inspector_config: { element_sections: string[]; container_sections: string[] };
				}
			).inspector_config;
			for (const section of [...config.element_sections, ...config.container_sections]) {
				expect(getSectionComponent(section as never), `${view.id}: ${section}`).toBeDefined();
			}
		}
	});

	// Regression: the aggregated inspector matched a removed `Interface` edge type, so a
	// SameHost aggregate fell back to a card with only the type name.
	it('draws each edge of an aggregated SameHost connection with its own card', () => {
		const display = aggregatedDisplay('SameHost');
		expect(display).not.toBeNull();
		const edge = {
			edge_type: 'SameHost',
			host_id: 'h1',
			source: 'a',
			target: 'b'
		} as unknown as TopologyEdge;
		const topology = {
			hosts: [{ id: 'h1', display_name: 'web-01' }],
			ip_addresses: []
		} as unknown as RenderableTopology;
		expect(display!.getLabel(edge, { topology })).toBe('web-01');
	});
});
