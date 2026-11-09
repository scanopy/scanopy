import { describe, expect, it, vi } from 'vitest';
import { QueryClient, QueryObserver } from '@tanstack/svelte-query';
import type { components } from '$lib/api/schema';

vi.mock('$env/dynamic/public', () => ({
	env: { PUBLIC_SERVER_HOSTNAME: 'scanopy.test', PUBLIC_SERVER_PROTOCOL: 'http' }
}));

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

describe('afterDaemonDeleted', () => {
	it("re-reads the wizard's eligibility, so deleting the unconnected daemon lifts the refusal", async () => {
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
		expect(observer.getCurrentResult().data?.plan_required).toBe(true);

		await afterDaemonDeleted(queryClient, unconnected.id);

		expect(queryFn).toHaveBeenCalledTimes(2);
		const after = observer.getCurrentResult().data;
		expect(after?.plan_required || after?.email_verification_required).toBe(false);
		unsubscribe();
	});
});
