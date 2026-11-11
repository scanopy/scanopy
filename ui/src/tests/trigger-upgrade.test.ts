import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';

vi.mock('#lib/shared/utils/analytics.ts', () => ({ trackEvent: vi.fn() }));

import { queryClient, queryKeys } from '#lib/api/query-client.ts';
import { closeModal, modalState, openModal } from '#lib/shared/stores/modal-registry.ts';
import { billingOverModal } from '#lib/features/billing/stores.ts';
import { triggerUpgrade } from '#lib/features/billing/trigger-upgrade.ts';
import { startSetupPayment } from '#lib/shared/billing/setup-payment.ts';

describe('billing modals opened from inside another modal', () => {
	beforeEach(() => {
		queryClient.setQueryData(queryKeys.config.all, { deployment_type: 'cloud' });
		billingOverModal.set(null);
		closeModal();
	});

	it('stacks the plan picker over an open form, leaving the form open', () => {
		openModal('discovery-editor');
		const beforeModal = vi.fn();

		triggerUpgrade({ source: 'test', surface: 'discovery_form', beforeModal });

		expect(get(modalState).name).toBe('discovery-editor');
		expect(get(billingOverModal)?.modal).toBe('billing-plan');
		expect(beforeModal).not.toHaveBeenCalled();
	});

	it('stacks the payment dialog over an open form too', () => {
		openModal('discovery-editor');

		startSetupPayment({ org: undefined, source: 'trial_banner', trialDaysLeft: null });

		expect(get(modalState).name).toBe('discovery-editor');
		expect(get(billingOverModal)?.modal).toBe('payment-method');
	});

	it('opens the plan picker through the registry with no modal open', () => {
		triggerUpgrade({ source: 'test', surface: 'sidebar' });

		expect(get(modalState).name).toBe('billing-plan');
		expect(get(billingOverModal)).toBeNull();
	});

	it('replaces Settings, which reopens after billing closes', () => {
		openModal('settings', { tab: 'billing' });
		const beforeModal = vi.fn();

		triggerUpgrade({ source: 'test', surface: 'billing_tab', reopenSettings: true, beforeModal });

		expect(get(modalState).name).toBe('billing-plan');
		expect(get(billingOverModal)).toBeNull();
		expect(beforeModal).toHaveBeenCalled();
	});
});
