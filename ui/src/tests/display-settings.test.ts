import { describe, expect, it } from 'vitest';
import type { components } from '$lib/api/schema';
import { displaySettings } from '$lib/shared/stores/display-settings.svelte';

type DisplaySettings = components['schemas']['DisplaySettings'];

describe('displaySettings.set', () => {
	it('reads a user saved before table density existed as comfortable', () => {
		// An older server writes the JSONB back from its own struct and drops the key.
		const stored = {
			date_order: 'iso',
			clock: 'twelve_hour',
			time_zone: null,
			week_start: 'sunday',
			timestamps: 'absolute'
		} as Omit<DisplaySettings, 'table_density'> as DisplaySettings;

		displaySettings.set(stored);

		expect(displaySettings.current.table_density).toBe('comfortable');
		expect(displaySettings.current.date_order).toBe('iso');
	});
});
