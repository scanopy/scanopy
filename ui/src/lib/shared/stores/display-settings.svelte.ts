import type { components } from '$lib/api/schema';

type DisplaySettings = components['schemas']['DisplaySettings'];

// Matches DisplaySettings::default() on the backend. Used until the current user loads, and on
// pages with no user (login, public shares).
export const DEFAULT_DISPLAY_SETTINGS: DisplaySettings = {
	date_order: 'browser_default',
	clock: 'browser_default',
	time_zone: null,
	week_start: 'monday',
	timestamps: 'relative'
};

let current = $state<DisplaySettings>(DEFAULT_DISPLAY_SETTINGS);

/**
 * The signed-in user's date and time preferences. Module state, so every formatter call made
 * from a template re-renders when the user changes a setting.
 */
export const displaySettings = {
	get current(): DisplaySettings {
		return current;
	},
	set(settings: DisplaySettings | undefined) {
		current = { ...DEFAULT_DISPLAY_SETTINGS, ...settings };
	}
};

/** The browser's own IANA zone. */
export function browserTimeZone(): string {
	return Intl.DateTimeFormat().resolvedOptions().timeZone;
}

/** Every IANA zone this browser supports, as select options. */
export function timeZoneOptions(): { value: string; label: string }[] {
	return Intl.supportedValuesOf('timeZone').map((tz) => ({ value: tz, label: tz }));
}

/** The zone dates render in: the user's choice, otherwise the browser's. */
export function effectiveTimeZone(): string {
	return current.time_zone ?? browserTimeZone();
}
