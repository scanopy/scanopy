import { describe, it, expect } from 'vitest';
import { displayTags, fitTags } from '$lib/shared/components/forms/selection/display-tags';
import type { EntityDisplayComponent } from '$lib/shared/components/forms/selection/types';
import type { TagProps } from '$lib/shared/components/data/types';
import { InterfaceDisplay } from '$lib/shared/components/forms/selection/display/InterfaceDisplay.svelte';
import type { Interface } from '$lib/features/hosts/types/base';

const TAGS: TagProps[] = [
	{ label: 'VM', role: 'guest' },
	{ label: 'nginx', role: 'service' },
	{ label: 'Stale' }
];

const display: EntityDisplayComponent<object, object> = {
	getId: () => 'x',
	getLabel: () => 'x',
	getTags: () => TAGS,
	compactHides: ['service']
};

const labels = (tags: TagProps[]) => tags.map((t) => t.label);

describe('displayTags', () => {
	it('shows every tag when the context hides nothing', () => {
		expect(labels(displayTags(display, {}, {}))).toEqual(['VM', 'nginx', 'Stale']);
		expect(labels(displayTags(display, {}, undefined as unknown as object))).toEqual([
			'VM',
			'nginx',
			'Stale'
		]);
	});

	it('drops the roles a compact context hides, and only those', () => {
		expect(labels(displayTags(display, {}, { compact: true }))).toEqual(['VM', 'Stale']);
	});

	it('drops the roles named in hideTags, on top of compact', () => {
		expect(labels(displayTags(display, {}, { hideTags: ['guest'] }))).toEqual(['nginx', 'Stale']);
		expect(labels(displayTags(display, {}, { compact: true, hideTags: ['guest'] }))).toEqual([
			'Stale'
		]);
	});
});

describe('fitTags', () => {
	const spacing = { gap: 8, tagGap: 4, moreWidth: 50 };

	it('shows every tag when they fit beside the full label', () => {
		expect(fitTags(400, 100, [40, 40, 40], spacing)).toBe(3);
	});

	it('shows no tag when the label alone fills the row', () => {
		// The old rule always kept one tag and squeezed the label to 60px to make room for it.
		expect(fitTags(200, 200, [40], spacing)).toBe(0);
		expect(fitTags(200, 260, [40], spacing)).toBe(0);
	});

	it('never takes width the label needs', () => {
		const container = 300;
		const label = 180;
		const tagWidths = [60, 60, 60];
		const count = fitTags(container, label, tagWidths, spacing);
		const used =
			tagWidths.slice(0, count).reduce((sum, w) => sum + w, 0) + Math.max(0, count - 1) * 4;
		const chip = count < tagWidths.length ? 4 + 50 : 0;
		expect(label + spacing.gap + used + chip).toBeLessThanOrEqual(container);
	});

	it('keeps room for the "+N more" chip when tags are left over', () => {
		// 112px is free: one 60px tag fits, but not with the 54px the chip and its gap take.
		expect(fitTags(220, 100, [60, 60], spacing)).toBe(0);
		// 60 + 4 + 50 = 114px: the tag and the chip both fit.
		expect(fitTags(222, 100, [60, 60], spacing)).toBe(1);
	});
});

describe('InterfaceDisplay status in compact rows', () => {
	const iface = (oper_status: Interface['oper_status']) =>
		({ id: 'i1', display_name: 'ten-gigabitEthernet1/1/1', oper_status }) as Interface;

	it('drops a routine status and keeps a down link', () => {
		expect(displayTags(InterfaceDisplay, iface('Up'), { compact: true })).toHaveLength(0);
		expect(displayTags(InterfaceDisplay, iface('Down'), { compact: true })).toHaveLength(1);
	});

	it('keeps the status in full rows', () => {
		expect(displayTags(InterfaceDisplay, iface('Up'), undefined)).toHaveLength(1);
	});
});
