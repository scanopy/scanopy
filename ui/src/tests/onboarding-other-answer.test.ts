import { describe, it, expect } from 'vitest';
import { otherAnswer } from '#lib/features/auth/components/onboarding/otherAnswer.ts';
import { recommendationFor } from '#lib/features/billing/types.ts';

/** The server refuses what this lets through, so the limit has to match: 200, trimmed. */
describe('otherAnswer', () => {
	it('takes a blank or short answer, since the field is optional', () => {
		expect(otherAnswer({ value: '' })).toBeUndefined();
		expect(otherAnswer({ value: '   ' })).toBeUndefined();
		expect(otherAnswer({ value: '  ab  ' })).toBeUndefined();
	});

	it('caps an answer at 200 characters, counting it trimmed', () => {
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
