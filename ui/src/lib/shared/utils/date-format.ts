import type { components } from '$lib/api/schema';
import { getLocale } from '$lib/paraglide/runtime';
import { common_justNow } from '$lib/paraglide/messages';

type DisplaySettings = components['schemas']['DisplaySettings'];

/** How much room a date gets: `8/3/26`, `Aug 3, 2026` or `August 3, 2026` in en-US. */
export type DateStyle = 'numeric' | 'short' | 'long';

export interface DateFormatOptions {
	date: DateStyle;
	/** Append hours and minutes. */
	time?: boolean;
	/**
	 * Zone to render in, overriding the user's choice. Pass `'UTC'` for a date-only value
	 * (`2026-10-08`), which `Date` parses as UTC midnight and would otherwise show a day early
	 * west of UTC.
	 */
	timeZone?: string;
}

const BROWSER_DATE_OPTIONS: Record<DateStyle, Intl.DateTimeFormatOptions> = {
	numeric: { year: '2-digit', month: 'numeric', day: 'numeric' },
	short: { year: 'numeric', month: 'short', day: 'numeric' },
	long: { year: 'numeric', month: 'long', day: 'numeric' }
};

function toDate(value: string | Date): Date | null {
	const date = value instanceof Date ? value : new Date(value);
	return Number.isNaN(date.getTime()) ? null : date;
}

function hourCycle(settings: DisplaySettings): Intl.DateTimeFormatOptions['hourCycle'] {
	switch (settings.clock) {
		case 'twelve_hour':
			return 'h12';
		case 'twenty_four_hour':
			return 'h23';
		case 'browser_default':
			return undefined;
	}
}

/** The zone to render in, dropping a stored zone this browser's Intl doesn't know. */
function resolveTimeZone(opts: DateFormatOptions, settings: DisplaySettings): string | undefined {
	const zone = opts.timeZone ?? settings.time_zone ?? undefined;
	if (!zone) return undefined;
	try {
		new Intl.DateTimeFormat(undefined, { timeZone: zone });
		return zone;
	} catch {
		return undefined;
	}
}

function datePart(
	date: Date,
	style: DateStyle,
	settings: DisplaySettings,
	timeZone: string | undefined,
	locale: string
): string {
	if (settings.date_order === 'browser_default') {
		return date.toLocaleDateString(undefined, { ...BROWSER_DATE_OPTIONS[style], timeZone });
	}

	const numeric = settings.date_order === 'iso' || style === 'numeric';
	const parts = new Intl.DateTimeFormat(locale, {
		year: 'numeric',
		month: numeric ? '2-digit' : style,
		day: numeric ? '2-digit' : 'numeric',
		timeZone
	}).formatToParts(date);
	const part = (type: Intl.DateTimeFormatPartTypes) =>
		parts.find((p) => p.type === type)?.value ?? '';
	const [year, month, day] = [part('year'), part('month'), part('day')];

	switch (settings.date_order) {
		case 'iso':
			return `${year}-${month}-${day}`;
		case 'day_first':
			return numeric ? `${day}/${month}/${year}` : `${day} ${month} ${year}`;
		case 'month_first':
			return numeric ? `${month}/${day}/${year}` : `${month} ${day}, ${year}`;
	}
}

function timePart(date: Date, settings: DisplaySettings, timeZone: string | undefined): string {
	return date.toLocaleTimeString(undefined, {
		hour: '2-digit',
		minute: '2-digit',
		hourCycle: hourCycle(settings),
		timeZone
	});
}

/**
 * Format a date for display under the user's display settings. Every user-visible date in the UI
 * goes through here, via the wrappers in `formatting.ts`. Unparseable input comes back unchanged.
 */
export function formatDateTime(
	value: string | Date,
	opts: DateFormatOptions,
	settings: DisplaySettings,
	locale: string = getLocale()
): string {
	const date = toDate(value);
	if (!date) return String(value);

	const timeZone = resolveTimeZone(opts, settings);

	if (settings.date_order === 'browser_default' && opts.time) {
		return date.toLocaleString(undefined, {
			...BROWSER_DATE_OPTIONS[opts.date],
			hour: '2-digit',
			minute: '2-digit',
			hourCycle: hourCycle(settings),
			timeZone
		});
	}

	const day = datePart(date, opts.date, settings, timeZone, locale);
	if (!opts.time) return day;
	const separator = settings.date_order === 'iso' ? ' ' : ', ';
	return `${day}${separator}${timePart(date, settings, timeZone)}`;
}

/**
 * How long ago `value` was, e.g. `5m ago`, or the absolute timestamp when the user turned
 * relative times off. `now` is a parameter for tests.
 */
export function formatRelative(
	value: string | Date,
	settings: DisplaySettings,
	now: number = Date.now(),
	locale: string = getLocale()
): string {
	if (settings.timestamps === 'absolute') {
		return formatDateTime(value, { date: 'short', time: true }, settings, locale);
	}
	const date = toDate(value);
	if (!date) return String(value);

	const minutes = Math.floor(Math.max(0, now - date.getTime()) / 60000);
	if (minutes < 1) return common_justNow();

	const rtf = new Intl.RelativeTimeFormat(locale, { style: 'narrow' });
	if (minutes < 60) return rtf.format(-minutes, 'minute');
	const hours = Math.floor(minutes / 60);
	if (hours < 24) return rtf.format(-hours, 'hour');
	return rtf.format(-Math.floor(hours / 24), 'day');
}

/** Weekday indices (0 = Sunday, as `Date.getDay()`) in display order for the user's week start. */
export function weekdayOrder(settings: DisplaySettings): number[] {
	switch (settings.week_start) {
		case 'monday':
			return [1, 2, 3, 4, 5, 6, 0];
		case 'sunday':
			return [0, 1, 2, 3, 4, 5, 6];
	}
}
