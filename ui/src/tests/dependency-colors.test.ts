import { describe, it, expect } from 'vitest';
import { dependencyColors } from '#lib/features/dependencies/utils.ts';
import { edgeTypes } from '#lib/shared/stores/metadata.ts';
import type { Color } from '#lib/shared/utils/styling.ts';

describe('dependencyColors', () => {
	const offered = new Set(dependencyColors());
	const items = edgeTypes.getItems();

	it('offers no colour that a non-dependency edge type draws in', () => {
		const claimed = items.filter((t) => !t.metadata.is_dependency_edge);
		expect(claimed.length).toBeGreaterThan(0);
		for (const type of claimed) {
			expect(offered.has(type.color as Color), `${type.id} (${type.color})`).toBe(false);
		}
	});

	it("still offers the dependency edge types' own colour", () => {
		for (const type of items.filter((t) => t.metadata.is_dependency_edge)) {
			expect(offered.has(type.color as Color), `${type.id} (${type.color})`).toBe(true);
		}
	});
});
