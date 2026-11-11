import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const capture = vi.fn();
vi.mock('posthog-js', () => ({
	default: { __loaded: true, capture: (...args: unknown[]) => capture(...args) }
}));

import { trackEvent } from '#lib/shared/utils/analytics.ts';
import { queryClient, queryKeys } from '#lib/api/query-client.ts';

/**
 * Frontend events reach PostHog scoped to the current organization, the way
 * backend events are, so one query keyed on the org reads both.
 */
describe('trackEvent organization scope', () => {
	const organizationId = '6f1c2a8e-0b7d-4e7a-9f3e-2d4c5b6a7e8f';

	beforeEach(() => capture.mockClear());
	afterEach(() => queryClient.clear());

	it('attaches the current organization and its group', () => {
		queryClient.setQueryData(queryKeys.organizations.current(), { id: organizationId });

		trackEvent('daemon_wizard_step_viewed', { step: 'install' });

		expect(capture).toHaveBeenCalledWith('daemon_wizard_step_viewed', {
			step: 'install',
			organization_id: organizationId,
			$groups: { organization: organizationId }
		});
	});

	it('scopes an event sent with no properties', () => {
		queryClient.setQueryData(queryKeys.organizations.current(), { id: organizationId });

		trackEvent('daemon_wizard_opened');

		expect(capture).toHaveBeenCalledWith('daemon_wizard_opened', {
			organization_id: organizationId,
			$groups: { organization: organizationId }
		});
	});

	it('adds nothing before an organization is loaded', () => {
		trackEvent('onboarding_use_case_selected', { use_case: 'homelab' });

		expect(capture).toHaveBeenCalledWith('onboarding_use_case_selected', {
			use_case: 'homelab'
		});
	});
});
