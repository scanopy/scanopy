import { describe, expect, it } from 'vitest';
import type { components } from '$lib/api/schema';
import {
	formatDateTime,
	formatRelative,
	formatTimeOfDay,
	weekdayOrder
} from '$lib/shared/utils/date-format';
import { DEFAULT_DISPLAY_SETTINGS } from '$lib/shared/stores/display-settings.svelte';

type DisplaySettings = components['schemas']['DisplaySettings'];

const settings = (overrides: Partial<DisplaySettings> = {}): DisplaySettings => ({
	...DEFAULT_DISPLAY_SETTINGS,
	...overrides
});

// 2026-10-08 15:45 in UTC. Every case pins a zone so results don't depend on the test machine.
const INSTANT = '2026-10-08T15:45:00Z';
const UTC = { time_zone: 'UTC' };

describe('formatDateTime: date order', () => {
	it.each([
		['iso', 'short', '2026-10-08'],
		['iso', 'long', '2026-10-08'],
		['day_first', 'short', '8 Oct 2026'],
		['day_first', 'long', '8 October 2026'],
		['month_first', 'short', 'Oct 8, 2026'],
		['month_first', 'long', 'October 8, 2026']
	] as const)('%s / %s', (date_order, style, expected) => {
		expect(formatDateTime(INSTANT, { date: style }, settings({ date_order, ...UTC }), 'en')).toBe(
			expected
		);
	});

	it('browser default matches the browser locale for every style, with and without time', () => {
		const tz = 'UTC';
		const date = new Date(INSTANT);
		const s = settings({ time_zone: tz });
		const styles = {
			short: { year: 'numeric', month: 'short', day: 'numeric' },
			long: { year: 'numeric', month: 'long', day: 'numeric' }
		} as const;
		for (const [style, opts] of Object.entries(styles)) {
			const dateStyle = style as keyof typeof styles;
			expect(formatDateTime(date, { date: dateStyle }, s)).toBe(
				date.toLocaleDateString(undefined, { ...opts, timeZone: tz })
			);
			expect(formatDateTime(date, { date: dateStyle, time: true }, s)).toBe(
				date.toLocaleString(undefined, {
					...opts,
					hour: '2-digit',
					minute: '2-digit',
					timeZone: tz
				})
			);
		}
	});
});

describe('formatDateTime: clock', () => {
	it('12-hour and 24-hour override the locale clock', () => {
		const twelve = formatDateTime(
			INSTANT,
			{ date: 'short', time: true },
			settings({ date_order: 'iso', clock: 'twelve_hour', ...UTC }),
			'en'
		);
		const twentyFour = formatDateTime(
			INSTANT,
			{ date: 'short', time: true },
			settings({ date_order: 'iso', clock: 'twenty_four_hour', ...UTC }),
			'en'
		);
		expect(twelve).toMatch(/^2026-10-08 03:45\s?PM$/i);
		expect(twentyFour).toBe('2026-10-08 15:45');
	});

	it('applies to the browser-default date order too', () => {
		const twentyFour = formatDateTime(
			INSTANT,
			{ date: 'short', time: true },
			settings({ clock: 'twenty_four_hour', ...UTC })
		);
		const twelve = formatDateTime(
			INSTANT,
			{ date: 'short', time: true },
			settings({ clock: 'twelve_hour', ...UTC })
		);
		expect(twentyFour).toContain('15:45');
		expect(twelve).not.toContain('15:45');
		expect(twelve).toContain('03:45');
	});

	it('midnight reads 00 on a 24-hour clock, not 24', () => {
		const midnight = formatDateTime(
			'2026-10-08T00:05:00Z',
			{ date: 'short', time: true },
			settings({ date_order: 'iso', clock: 'twenty_four_hour', ...UTC }),
			'en'
		);
		expect(midnight).toBe('2026-10-08 00:05');
	});
});

describe('formatDateTime: time zone', () => {
	it('renders in the chosen zone, crossing midnight', () => {
		const s = settings({ date_order: 'iso', clock: 'twenty_four_hour', time_zone: 'Asia/Tokyo' });
		expect(formatDateTime(INSTANT, { date: 'short', time: true }, s, 'en')).toBe(
			'2026-10-09 00:45'
		);
	});

	it('an explicit zone overrides the user zone, so date-only values keep their day', () => {
		const s = settings({ date_order: 'iso', time_zone: 'America/Los_Angeles' });
		expect(formatDateTime('2026-12-01', { date: 'long' }, s, 'en')).toBe('2026-11-30');
		expect(formatDateTime('2026-12-01', { date: 'long', timeZone: 'UTC' }, s, 'en')).toBe(
			'2026-12-01'
		);
	});

	it('ignores a stored zone this browser does not know', () => {
		const s = settings({ date_order: 'iso', time_zone: 'Mars/Olympus_Mons' });
		expect(() => formatDateTime(INSTANT, { date: 'short' }, s, 'en')).not.toThrow();
	});
});

describe('formatDateTime: invalid input', () => {
	it('returns unparseable input unchanged', () => {
		expect(formatDateTime('not a date', { date: 'short' }, settings())).toBe('not a date');
		expect(formatDateTime('', { date: 'short' }, settings())).toBe('');
	});
});

describe('formatRelative', () => {
	const now = new Date(INSTANT).getTime();
	const ago = (ms: number) => new Date(now - ms).toISOString();
	const MIN = 60_000;

	it('buckets into minutes, hours and days', () => {
		const s = settings();
		expect(formatRelative(ago(30_000), s, now, 'en')).toBe('just now');
		expect(formatRelative(ago(5 * MIN), s, now, 'en')).toBe('5m ago');
		expect(formatRelative(ago(3 * 60 * MIN), s, now, 'en')).toBe('3h ago');
		expect(formatRelative(ago(2 * 24 * 60 * MIN), s, now, 'en')).toBe('2d ago');
	});

	it('treats future times as just now', () => {
		expect(formatRelative(ago(-10 * MIN), settings(), now, 'en')).toBe('just now');
	});

	it('shows the full timestamp when relative times are off', () => {
		const s = settings({
			timestamps: 'absolute',
			date_order: 'iso',
			clock: 'twenty_four_hour',
			...UTC
		});
		expect(formatRelative(ago(5 * MIN), s, now, 'en')).toBe(
			formatDateTime(ago(5 * MIN), { date: 'short', time: true }, s, 'en')
		);
	});
});

describe('formatTimeOfDay', () => {
	it('follows the clock setting and ignores the user time zone', () => {
		const zoned = { time_zone: 'Asia/Tokyo' };
		expect(formatTimeOfDay(15, 5, settings({ clock: 'twenty_four_hour', ...zoned }))).toBe('15:05');
		expect(formatTimeOfDay(15, 5, settings({ clock: 'twelve_hour', ...zoned }))).toMatch(
			/^03:05\s?PM$/i
		);
		expect(formatTimeOfDay(0, 0, settings({ clock: 'twenty_four_hour' }))).toBe('00:00');
	});
});

describe('weekdayOrder', () => {
	it('starts on the chosen day and lists each weekday once', () => {
		for (const week_start of ['monday', 'sunday'] as const) {
			const order = weekdayOrder(settings({ week_start }));
			expect(order[0]).toBe(week_start === 'monday' ? 1 : 0);
			expect([...order].sort()).toEqual([0, 1, 2, 3, 4, 5, 6]);
		}
	});
});
