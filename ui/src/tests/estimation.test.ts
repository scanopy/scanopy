import { describe, it, expect } from 'vitest';
import { formatEstimatedRemaining } from '#lib/features/discovery/utils/estimation.ts';

describe('formatEstimatedRemaining', () => {
	it('returns "less than a minute" for under 60 seconds', () => {
		expect(formatEstimatedRemaining(0)).toBe('less than a minute');
		expect(formatEstimatedRemaining(30)).toBe('less than a minute');
		expect(formatEstimatedRemaining(59)).toBe('less than a minute');
	});

	it('returns "about 1 minute" for around 60 seconds', () => {
		expect(formatEstimatedRemaining(60)).toBe('about 1 minute');
		expect(formatEstimatedRemaining(89)).toBe('about 1 minute');
	});

	it('returns "about N minutes" for multiple minutes', () => {
		expect(formatEstimatedRemaining(120)).toBe('about 2 minutes');
		expect(formatEstimatedRemaining(300)).toBe('about 5 minutes');
		expect(formatEstimatedRemaining(600)).toBe('about 10 minutes');
	});

	it('returns hours for 60+ minutes', () => {
		expect(formatEstimatedRemaining(3600)).toBe('about 1 hour');
		expect(formatEstimatedRemaining(7200)).toBe('about 2 hours');
	});

	it('returns hours and minutes for non-round hours', () => {
		expect(formatEstimatedRemaining(5400)).toBe('about 1 hour 30 minutes');
		expect(formatEstimatedRemaining(9000)).toBe('about 2 hours 30 minutes');
	});

	it('rounds seconds to nearest minute', () => {
		expect(formatEstimatedRemaining(150)).toBe('about 3 minutes');
		expect(formatEstimatedRemaining(90)).toBe('about 2 minutes');
	});
});
