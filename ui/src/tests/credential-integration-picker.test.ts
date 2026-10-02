import { describe, it, expect } from 'vitest';
import {
	groupIntegrationsByCategory,
	groupTypesByIntegration,
	isIntegrationExpanded,
	selectedIntegrationCount,
	selectedTypeCount,
	type PickerType
} from '$lib/features/credentials/utils/integrationPicker';
import credentialTypesJson from '../lib/data/credential-types.json';
import credentialIntegrationsJson from '../lib/data/credential-integrations.json';

const types = credentialTypesJson as PickerType[];
const integrations = credentialIntegrationsJson as { id: string; category: string }[];
const groups = groupTypesByIntegration(types, integrations);

const multiGroup = groups.find((g) => g.types.length > 1)!;
const singleGroup = groups.find((g) => g.types.length === 1)!;

describe('groupTypesByIntegration', () => {
	it('places every credential type in exactly one integration group', () => {
		const grouped = groups.flatMap((g) => g.types.map((t) => t.id));
		expect(grouped.sort()).toEqual(types.map((t) => t.id).sort());
	});

	it('gives each fixture integration one row', () => {
		expect(groups.map((g) => g.integration.id).sort()).toEqual(
			integrations.map((i) => i.id).sort()
		);
	});

	it('includes both single-type and multi-type integrations', () => {
		expect(multiGroup).toBeDefined();
		expect(singleGroup).toBeDefined();
	});

	it('orders daemon-only integrations before network-applicable ones', () => {
		const reach = (t: PickerType) => {
			const targets = t.metadata?.targets ?? [];
			return targets.includes('Network') ? 2 : targets.includes('Hosts') ? 1 : 0;
		};
		const groupReach = groups.map((g) => Math.min(...g.types.map(reach)));
		expect(groupReach).toEqual([...groupReach].sort((a, b) => a - b));
		for (const g of groups) {
			const typeReach = g.types.map(reach);
			expect(typeReach).toEqual([...typeReach].sort((a, b) => a - b));
		}
	});

	it('leaves out a type whose integration has no fixture entry', () => {
		const orphan: PickerType = { id: 'Orphan', metadata: { integration: 'Missing' } };
		const result = groupTypesByIntegration([...types, orphan], integrations);
		expect(result.flatMap((g) => g.types).some((t) => t.id === 'Orphan')).toBe(false);
	});
});

describe('selectedTypeCount', () => {
	it('counts only the group’s own checked types', () => {
		const [first] = multiGroup.types;
		expect(selectedTypeCount(multiGroup, [first.id, singleGroup.types[0].id])).toBe(1);
		expect(
			selectedTypeCount(
				multiGroup,
				multiGroup.types.map((t) => t.id)
			)
		).toBe(multiGroup.types.length);
		expect(selectedTypeCount(multiGroup, [])).toBe(0);
	});
});

describe('isIntegrationExpanded', () => {
	const selected = [multiGroup.types[0].id];

	it('expands an untouched multi-type integration holding a selection', () => {
		expect(isIntegrationExpanded(multiGroup, selected, undefined)).toBe(true);
	});

	it('keeps an untouched integration with no selection collapsed', () => {
		expect(isIntegrationExpanded(multiGroup, [], undefined)).toBe(false);
	});

	it('lets the user collapse a row holding a selection, and expand an empty one', () => {
		expect(isIntegrationExpanded(multiGroup, selected, false)).toBe(false);
		expect(isIntegrationExpanded(multiGroup, [], true)).toBe(true);
	});

	it('never expands a single-type integration', () => {
		expect(isIntegrationExpanded(singleGroup, [singleGroup.types[0].id], true)).toBe(false);
	});
});

describe('selectedIntegrationCount', () => {
	it('counts integrations, not types', () => {
		const allOfOne = multiGroup.types.map((t) => t.id);
		expect(selectedIntegrationCount(groups, allOfOne)).toBe(1);
		expect(selectedIntegrationCount(groups, [...allOfOne, singleGroup.types[0].id])).toBe(2);
		expect(selectedIntegrationCount(groups, [])).toBe(0);
	});
});

describe('groupIntegrationsByCategory', () => {
	const sections = groupIntegrationsByCategory(groups, integrations);

	it('places every integration in exactly one section, under its own category', () => {
		const placed = sections.flatMap((s) => s.groups.map((g) => g.integration.id));
		expect(placed.sort()).toEqual(groups.map((g) => g.integration.id).sort());
		for (const section of sections) {
			for (const g of section.groups) expect(g.integration.category).toBe(section.category);
		}
	});

	it('orders sections by the categories’ order in the fixture and leaves none empty', () => {
		const fixtureOrder = [...new Set(integrations.map((i) => i.category))];
		expect(sections.map((s) => s.category)).toEqual(fixtureOrder);
		expect(sections.every((s) => s.groups.length > 0)).toBe(true);
	});

	it('drops a category with no integrations among the groups', () => {
		const withoutFirst = groups.filter((g) => g.integration.category !== sections[0].category);
		const result = groupIntegrationsByCategory(withoutFirst, integrations);
		expect(result.map((s) => s.category)).not.toContain(sections[0].category);
	});
});
