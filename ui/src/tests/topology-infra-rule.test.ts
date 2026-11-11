import { describe, it, expect } from 'vitest';
import {
	findInfraRuleId,
	getInfrastructureRuleIdForTopology
} from '#lib/features/topology/queries.ts';
import type { RenderableTopology } from '#lib/features/topology/types/base.ts';

// Minimal element-rules fixtures (only the fields findInfraRuleId reads).
const infraRule = {
	id: 'infra-rule-id',
	rule: { ByServiceCategory: { is_infra_rule: true, categories: [] } }
};
const nonInfraRule = {
	id: 'other-rule-id',
	rule: { ByServiceCategory: { is_infra_rule: false, categories: [] } }
};
const tagRule = { id: 'tag-rule-id', rule: { ByTag: { tag_ids: [] } } };

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const topoWith = (rules: any[]): RenderableTopology =>
	({ options: { request: { element_rules: rules } } }) as unknown as RenderableTopology;

describe('findInfraRuleId', () => {
	it('returns the id of the ByServiceCategory rule flagged is_infra_rule', () => {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		expect(findInfraRuleId([tagRule, nonInfraRule, infraRule] as any)).toBe('infra-rule-id');
	});

	it('returns null when no infra rule is present', () => {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		expect(findInfraRuleId([tagRule, nonInfraRule] as any)).toBe(null);
		expect(findInfraRuleId([])).toBe(null);
		expect(findInfraRuleId(undefined)).toBe(null);
	});
});

describe('getInfrastructureRuleIdForTopology', () => {
	it('derives the infra rule id from the topology bundle options', () => {
		expect(getInfrastructureRuleIdForTopology(topoWith([nonInfraRule, infraRule]))).toBe(
			'infra-rule-id'
		);
	});

	it('returns null when the bundle has no infra rule', () => {
		expect(getInfrastructureRuleIdForTopology(topoWith([tagRule]))).toBe(null);
	});
});
