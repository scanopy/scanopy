import { describe, expect, it, vi } from 'vitest';
import { QueryClient, QueryObserver } from '@tanstack/svelte-query';
import type { components } from '$lib/api/schema';

vi.mock('$env/dynamic/public', () => ({
	env: { PUBLIC_SERVER_HOSTNAME: 'scanopy.test', PUBLIC_SERVER_PROTOCOL: 'http' }
}));

const { provisionGate } = await import('$lib/features/daemons/utils');
const { afterDaemonDeleted } = await import('$lib/features/daemons/queries');
const { queryKeys } = await import('$lib/api/query-client');

type ProvisionEligibility = components['schemas']['ProvisionEligibility'];

const unconnected = {
	id: 'd1',
	name: 'scanopy-daemon-home',
	site_id: 's1',
	created_at: '2026-11-01T00:00:00Z'
};

function eligibility(overrides: Partial<ProvisionEligibility>): ProvisionEligibility {
	return {
		plan_required: false,
		email_verification_required: false,
		unconnected_daemons: [],
		...overrides
	};
}

describe('provisionGate', () => {
	it('leads with the unconnected daemon when either limit refuses', () => {
		for (const refused of [
			{ plan_required: true },
			{ email_verification_required: true },
			{ plan_required: true, email_verification_required: true }
		]) {
			expect(provisionGate(eligibility({ ...refused, unconnected_daemons: [unconnected] }))).toBe(
				'unconnected'
			);
		}
	});

	it("shows today's banner when every daemon has connected", () => {
		expect(provisionGate(eligibility({ plan_required: true }))).toBe('plan');
		expect(provisionGate(eligibility({ email_verification_required: true }))).toBe('verification');
		// Both limits used to arrive one at a time, plan first. The banner keeps that order.
		expect(
			provisionGate(eligibility({ plan_required: true, email_verification_required: true }))
		).toBe('plan');
	});

	it('lets the wizard through when nothing refuses', () => {
		expect(provisionGate(eligibility({}))).toBeNull();
		expect(provisionGate(undefined)).toBeNull();
	});
});

describe('afterDaemonDeleted', () => {
	it("re-reads the wizard's eligibility, so deleting the unconnected daemon clears the notice", async () => {
		const queryClient = new QueryClient();
		const responses = [
			eligibility({ plan_required: true, unconnected_daemons: [unconnected] }),
			eligibility({})
		];
		const queryFn = vi.fn(async () => responses.shift()!);
		const observer = new QueryObserver(queryClient, {
			queryKey: queryKeys.daemons.provisionEligibility(),
			queryFn
		});
		const unsubscribe = observer.subscribe(() => {});
		await vi.waitFor(() => expect(observer.getCurrentResult().data).toBeDefined());
		expect(provisionGate(observer.getCurrentResult().data)).toBe('unconnected');

		await afterDaemonDeleted(queryClient, unconnected.id);

		expect(queryFn).toHaveBeenCalledTimes(2);
		expect(provisionGate(observer.getCurrentResult().data)).toBeNull();
		unsubscribe();
	});
});
