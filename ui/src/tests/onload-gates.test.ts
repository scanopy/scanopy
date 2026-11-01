import { describe, it, expect } from 'vitest';
import {
	isShellReady,
	shouldCloseBillingPlanLink,
	shouldOpenLicenseSettings,
	shouldShowTrialExpiryModal
} from '$lib/features/organizations/onload-gates';

const pending = { data: undefined, isError: false };
const loaded = { data: {}, isError: false };
const failed = { data: undefined, isError: true };

describe('isShellReady', () => {
	it('holds the shell until both the org and config have answered', () => {
		expect(isShellReady(pending, pending)).toBe(false);
		expect(isShellReady(pending, loaded)).toBe(false);
		expect(isShellReady(loaded, pending)).toBe(false);
		expect(isShellReady(loaded, loaded)).toBe(true);
	});

	it('paints the shell when a query fails, so an error never holds Loading', () => {
		expect(isShellReady(failed, loaded)).toBe(true);
		expect(isShellReady(loaded, failed)).toBe(true);
	});

	it('still holds for a pending config when the org failed', () => {
		expect(isShellReady(failed, pending)).toBe(false);
	});
});

describe('shouldOpenLicenseSettings', () => {
	const open = {
		locked: true,
		planType: 'Standard',
		openedFor: null,
		appInitialized: true,
		planPickerShowing: false,
		modalName: null
	};

	it('opens for a locked org once the app is up', () => {
		expect(shouldOpenLicenseSettings(open)).toBe(true);
	});

	it('leaves an org alone when it is not locked, including one whose trial never started', () => {
		expect(shouldOpenLicenseSettings({ ...open, locked: false })).toBe(false);
	});

	it('waits for the org to name a plan', () => {
		expect(shouldOpenLicenseSettings({ ...open, planType: null })).toBe(false);
	});

	it('opens once per plan, and again on a different plan', () => {
		expect(shouldOpenLicenseSettings({ ...open, openedFor: 'Standard' })).toBe(false);
		expect(shouldOpenLicenseSettings({ ...open, openedFor: 'Plus' })).toBe(true);
	});

	it('waits for the plan picker', () => {
		expect(shouldOpenLicenseSettings({ ...open, planPickerShowing: true })).toBe(false);
	});

	it('waits for another modal, including the payment-method dialog', () => {
		expect(shouldOpenLicenseSettings({ ...open, modalName: 'payment-method' })).toBe(false);
	});

	it('replaces a Settings deep link, so the License tab wins', () => {
		expect(shouldOpenLicenseSettings({ ...open, modalName: 'settings' })).toBe(true);
	});

	it('waits for app initialization', () => {
		expect(shouldOpenLicenseSettings({ ...open, appInitialized: false })).toBe(false);
	});
});

describe('shouldCloseBillingPlanLink', () => {
	const link = { modalName: 'billing-plan', configLoaded: true, isCloudDeployment: false };

	it('closes the link on a self-hosted deployment', () => {
		expect(shouldCloseBillingPlanLink(link)).toBe(true);
	});

	it('keeps the link while config is loading, so Cloud links survive a slow config', () => {
		expect(shouldCloseBillingPlanLink({ ...link, configLoaded: false })).toBe(false);
	});

	it('keeps the link on Cloud', () => {
		expect(shouldCloseBillingPlanLink({ ...link, isCloudDeployment: true })).toBe(false);
	});

	it('ignores other modals', () => {
		expect(shouldCloseBillingPlanLink({ ...link, modalName: 'settings' })).toBe(false);
	});
});

describe('shouldShowTrialExpiryModal', () => {
	const expiring = {
		trialingWithoutPayment: true,
		hasPendingQuote: false,
		pendingQuoteLoading: false,
		trialDaysLeft: 1,
		dismissedToday: false,
		blocked: false
	};

	it('shows for a trial ending within a day', () => {
		expect(shouldShowTrialExpiryModal(expiring)).toBe(true);
		expect(shouldShowTrialExpiryModal({ ...expiring, trialDaysLeft: 0 })).toBe(true);
	});

	it('holds while the quote query loads, so a pending quote never flashes the modal', () => {
		expect(shouldShowTrialExpiryModal({ ...expiring, pendingQuoteLoading: true })).toBe(false);
	});

	it('stands down for a pending quote', () => {
		expect(shouldShowTrialExpiryModal({ ...expiring, hasPendingQuote: true })).toBe(false);
	});

	it('stays closed with more than a day left, or no trial end', () => {
		expect(shouldShowTrialExpiryModal({ ...expiring, trialDaysLeft: 2 })).toBe(false);
		expect(shouldShowTrialExpiryModal({ ...expiring, trialDaysLeft: null })).toBe(false);
	});

	it('stays closed when dismissed today or blocked by another modal', () => {
		expect(shouldShowTrialExpiryModal({ ...expiring, dismissedToday: true })).toBe(false);
		expect(shouldShowTrialExpiryModal({ ...expiring, blocked: true })).toBe(false);
	});
});
