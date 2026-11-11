import { describe, expect, it } from 'vitest';
import { opaqueOn } from '#lib/shared/billing/stripe-appearance.ts';

describe('opaqueOn', () => {
	it('blends a translucent colour onto its backdrop', () => {
		expect(opaqueOn('rgba(55, 65, 81, 0.5)', '#111827')).toBe('rgb(36, 45, 60)');
	});

	it('leaves an opaque colour as it is', () => {
		expect(opaqueOn('#ffffff', '#111827')).toBe('rgb(255, 255, 255)');
		expect(opaqueOn('#fff', '#000')).toBe('rgb(255, 255, 255)');
	});

	it('passes through a colour it cannot read', () => {
		expect(opaqueOn('color-mix(in srgb, red, blue)', '#111827')).toBe(
			'color-mix(in srgb, red, blue)'
		);
	});
});
