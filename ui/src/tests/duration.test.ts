import { describe, it, expect } from 'vitest';
import { splitHours, combineToHours } from '$lib/shared/utils/duration';

describe('duration split/combine', () => {
	// The failure that matters: opening a site's settings and re-saving
	// without meaning to change anything must not silently rewrite its window.
	it('round-trips any stored duration unchanged', () => {
		for (const total of [1, 23, 24, 25, 36, 48, 168, 672, 8760]) {
			const { days, hours } = splitHours(total);
			expect(combineToHours(days, hours)).toBe(total);
		}
	});

	it('leaves a component box empty rather than showing a redundant zero', () => {
		expect(splitHours(48)).toEqual({ days: 2, hours: null });
		expect(splitHours(6)).toEqual({ days: null, hours: 6 });
		expect(splitHours(30)).toEqual({ days: 1, hours: 6 });
	});

	// Empty must mean "unset, use the server default" — reading it as zero
	// would make every entity on the site instantly stale.
	it('treats an empty duration as unset rather than zero', () => {
		expect(combineToHours(null, null)).toBeNull();
		expect(combineToHours(0, 0)).toBeNull();
		expect(splitHours(null)).toEqual({ days: null, hours: null });
		expect(splitHours(0)).toEqual({ days: null, hours: null });
	});

	// An hours-only duration must be expressible: 6 hours is 6 hours, not
	// "the 28-day default plus 6 hours".
	it('supports an hours-only duration with no days component', () => {
		expect(combineToHours(null, 6)).toBe(6);
		expect(splitHours(6)).toEqual({ days: null, hours: 6 });
	});

	it('ignores negative input rather than shortening the window', () => {
		expect(combineToHours(-5, 3)).toBe(3);
		expect(combineToHours(2, -10)).toBe(48);
		expect(splitHours(-1)).toEqual({ days: null, hours: null });
	});
});
