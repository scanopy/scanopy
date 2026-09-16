import { describe, expect, it } from 'vitest';
import attributeMethodsJson from '$lib/data/attribute-methods.json';
import { attributeSourceLabel, type AttributeSource } from '$lib/shared/utils/attribute-source';
import { clientProbes } from '$lib/shared/stores/metadata';

/**
 * Every source the backend can produce renders as a readable label.
 *
 * The source list is the backend's own, from the tier fixture, so a source added on the server is
 * covered here without anyone listing it.
 */
const EVERY_SOURCE: AttributeSource[] = attributeMethodsJson.flatMap(
	(method) => (method.metadata as { sources?: AttributeSource[] } | null)?.sources ?? []
);

describe('attribute source labels', () => {
	it('has sources to check', () => {
		expect(EVERY_SOURCE.length).toBeGreaterThan(0);
	});

	it('labels every source, Unspecified included, with no unfilled slot', () => {
		for (const source of EVERY_SOURCE) {
			const label = attributeSourceLabel(source);
			expect(label.trim(), JSON.stringify(source)).not.toBe('');
			expect(label, JSON.stringify(source)).not.toMatch(/[{}]/);
		}
	});

	it('names the probe a probe-carrying source rode in on', () => {
		for (const source of EVERY_SOURCE) {
			if (typeof source === 'string') continue;
			const [probe] = Object.values(source);
			expect(attributeSourceLabel(source)).toContain(clientProbes.getName(probe));
		}
	});
});
