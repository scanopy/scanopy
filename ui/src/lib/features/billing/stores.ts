import { writable } from 'svelte/store';

// When true, closing the billing modal should reopen the settings modal
export const reopenSettingsAfterBilling = writable(false);

/**
 * Settings tab the payment-method modal returns to once it closes, or null when
 * it wasn't opened from Settings. A locked org never actually closes the Settings
 * modal (its close handler is a no-op while non-dismissible), so without this the
 * modal registry and the URL stop naming what is on screen.
 */
export const reopenSettingsTabAfterPayment = writable<string | null>(null);
