import type { components } from '#lib/api/schema';
import containerRuleTypes from '#lib/data/container-rule-types.json';
import elementRuleTypes from '#lib/data/element-rule-types.json';
import { v4 as uuidv4 } from 'uuid';

export type GraphRule<T> = { id: string; rule: T };
export type ContainerRule = components['schemas']['IdentifiedRule_ContainerRule']['rule'];
export type ElementRule = components['schemas']['IdentifiedRule_ElementRule']['rule'];
export type ContainerGraphRule = components['schemas']['IdentifiedRule_ContainerRule'];
export type ElementGraphRule = components['schemas']['IdentifiedRule_ElementRule'];

export type ContainerRuleType = (typeof containerRuleTypes)[number]['id'];

export function getContainerRuleDiscriminant(rule: ContainerRule): ContainerRuleType {
	if (typeof rule === 'string') return rule;
	return Object.keys(rule)[0] as ContainerRuleType;
}

export type ElementRuleType = (typeof elementRuleTypes)[number]['id'];

export function getElementRuleType(rule: ElementRule): ElementRuleType {
	if (typeof rule === 'string') return rule;
	if ('ByServiceCategory' in rule) return 'ByServiceCategory';
	return 'ByTag';
}

export function getElementRuleTitle(rule: ElementRule): string | null | undefined {
	if (typeof rule === 'string') return null;
	if ('ByServiceCategory' in rule) return rule.ByServiceCategory.title;
	return rule.ByTag.title;
}

export function setElementRuleTitle(rule: ElementRule, title: string | null): ElementRule {
	if (typeof rule === 'string') return rule;
	if ('ByServiceCategory' in rule)
		return { ByServiceCategory: { ...rule.ByServiceCategory, title } };
	if ('ByTag' in rule) return { ByTag: { ...rule.ByTag, title } };
	return rule;
}

export function makeGraphRule<T>(rule: T): GraphRule<T> {
	return { id: uuidv4(), rule };
}
