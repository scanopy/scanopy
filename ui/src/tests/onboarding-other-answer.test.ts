import { describe, it, expect } from 'vitest';
import { otherAnswer } from '#lib/features/auth/components/onboarding/otherAnswer.ts';
import { recommendationFor } from '#lib/features/billing/types.ts';

/** The server refuses what this lets through, so the bounds have to match: 3 to 200, trimmed. */
describe('otherAnswer', () => {
	it('refuses an answer that is empty or too short once trimmed', () => {
		expect(otherAnswer({ value: '' })).toBeDefined();
		expect(otherAnswer({ value: '   ' })).toBeDefined();
		expect(otherAnswer({ value: '  ab  ' })).toBeDefined();
	});

	it('takes an answer within the bounds, counting it trimmed', () => {
		expect(otherAnswer({ value: '  a school project  ' })).toBeUndefined();
		expect(otherAnswer({ value: ` ${'x'.repeat(200)} ` })).toBeUndefined();
		expect(otherAnswer({ value: 'x'.repeat(201) })).toBeDefined();
	});
});

describe('recommendationFor', () => {
	const recommendations = {
		cloud: { plan: 'Business', reasons: [] },
		self_hosted: { plan: 'SelfHostedStandard', reasons: [] }
	};

	it('finds the recommendation on whichever hosting tab names the plan', () => {
		expect(recommendationFor(recommendations, 'Business')?.plan).toBe('Business');
		expect(recommendationFor(recommendations, 'SelfHostedStandard')?.plan).toBe(
			'SelfHostedStandard'
		);
	});

	it('marks no other plan, and nothing when the org has no recommendation', () => {
		expect(recommendationFor(recommendations, 'Pro')).toBeNull();
		expect(recommendationFor({ cloud: null, self_hosted: null }, 'Free')).toBeNull();
		expect(recommendationFor(null, 'Free')).toBeNull();
	});
});
