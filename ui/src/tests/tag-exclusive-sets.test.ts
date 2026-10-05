import { describe, expect, it } from 'vitest';
import { groupNames, withTagAdded } from '$lib/features/tags/sets';
import { createDefaultTag, type Tag } from '$lib/features/tags/types/base';

function tag(id: string, exclusive_set: Tag['exclusive_set'] = null): Tag {
	return { ...createDefaultTag('org'), id, name: id, exclusive_set };
}

const lifecycle = (name: string) => tag(name, { type: 'Group', name: 'Lifecycle' });
const environment = (name: string) => tag(name, { type: 'Group', name: 'Environment' });
const application = (name: string) => tag(name, { type: 'Application' });

describe('withTagAdded', () => {
	const tags = [
		lifecycle('planned'),
		lifecycle('decommissioned'),
		environment('production'),
		application('web'),
		application('db'),
		tag('critical')
	];

	it('replaces the held tag of the same set and keeps the rest', () => {
		expect(withTagAdded(['planned', 'production', 'critical'], 'decommissioned', tags)).toEqual([
			'production',
			'critical',
			'decommissioned'
		]);
	});

	it('treats application tags as one set', () => {
		expect(withTagAdded(['web', 'planned'], 'db', tags)).toEqual(['planned', 'db']);
	});

	it('leaves every held tag alone when the added tag is in no set', () => {
		expect(withTagAdded(['planned', 'web'], 'critical', tags)).toEqual([
			'planned',
			'web',
			'critical'
		]);
	});
});

describe('groupNames', () => {
	it('lists each named set once, without the application set', () => {
		expect(
			groupNames([lifecycle('a'), environment('b'), lifecycle('c'), application('d'), tag('e')])
		).toEqual(['Environment', 'Lifecycle']);
	});
});
