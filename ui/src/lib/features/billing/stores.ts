import { get, writable } from 'svelte/store';
import { modalState, openModal } from '#lib/shared/stores/modal-registry.ts';

// When true, closing the billing modal should reopen the settings modal
export const reopenSettingsAfterBilling = writable(false);

/**
 * Settings tab the payment-method modal returns to once it closes, or null when
 * it wasn't opened from Settings. A locked org never actually closes the Settings
 * modal (its close handler is a no-op while non-dismissible), so without this the
 * modal registry and the URL stop naming what is on screen.
 */
export const reopenSettingsTabAfterPayment = writable<string | null>(null);

type BillingModal = 'billing-plan' | 'payment-method';

/**
 * A billing modal opened over another modal: the plan picker from an upgrade badge in a form, and
 * the payment dialog it can lead to. Opened like any dialog stacked on a modal, without a registry
 * name, so the modal underneath stays mounted with what the user had entered. Null when no billing
 * modal is open this way.
 */
export const billingOverModal = writable<{
	modal: BillingModal;
	entityData?: Record<string, unknown>;
} | null>(null);

/**
 * Modals a billing modal replaces in the registry, as before. Settings keeps its own return path
 * (`reopenSettingsAfterBilling`, `reopenSettingsTabAfterPayment`), which the locked-org flows
 * depend on.
 */
const REPLACED_BY_BILLING = ['settings', 'billing-plan', 'payment-method'];

/** Whether a billing modal opened now would sit over another modal rather than replace it. */
export function billingOpensOverModal(): boolean {
	if (get(billingOverModal) != null) return true;
	const open = get(modalState).name;
	return open != null && !REPLACED_BY_BILLING.includes(open);
}

/** Open a billing modal over the open modal, or through the registry when there is none. */
export function openBillingModal(modal: BillingModal, entityData?: Record<string, unknown>): void {
	if (billingOpensOverModal()) {
		billingOverModal.set({ modal, entityData });
	} else {
		openModal(modal, entityData ? { entityData } : undefined);
	}
}
