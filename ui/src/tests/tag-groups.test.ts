import { describe, expect, it } from 'vitest';
import { groupNames, tagTooltip, withTagAdded } from '$lib/features/tags/groups';
import { createDefaultTag, type Tag } from '$lib/features/tags/types/base';

function tag(id: string, tag_group: Tag['tag_group'] = null): Tag {
	return { ...createDefaultTag('org'), id, name: id, tag_group };
}

const status = (name: string) => tag(name, { type: 'Named', name: 'Status' });
const environment = (name: string) => tag(name, { type: 'Named', name: 'Environment' });
const application = (name: string) => tag(name, { type: 'Application' });

describe('withTagAdded', () => {
	const tags = [
		status('planned'),
		status('decommissioned'),
		environment('production'),
		application('web'),
		application('db'),
		tag('critical')
	];

	it('replaces the held tag of the same group and keeps the rest', () => {
		expect(withTagAdded(['planned', 'production', 'critical'], 'decommissioned', tags)).toEqual([
			'production',
			'critical',
			'decommissioned'
		]);
	});

	it('treats application tags as one group', () => {
		expect(withTagAdded(['web', 'planned'], 'db', tags)).toEqual(['planned', 'db']);
	});

	it('leaves every held tag alone when the added tag is in no group', () => {
		expect(withTagAdded(['planned', 'web'], 'critical', tags)).toEqual([
			'planned',
			'web',
			'critical'
		]);
	});
});

describe('groupNames', () => {
	it('lists each named group once, without the application group', () => {
		expect(
			groupNames([status('a'), environment('b'), status('c'), application('d'), tag('e')])
		).toEqual(['Environment', 'Status']);
	});
});

describe('tagTooltip', () => {
	it('puts the group first and the description under it', () => {
		const lines = tagTooltip({ ...status('planned'), description: '  Not deployed yet ' }).split(
			'\n'
		);
		expect(lines).toHaveLength(2);
		expect(lines[0]).toContain('Status');
		expect(lines[1]).toBe('Not deployed yet');
	});

	it('shows whichever of the two a tag has', () => {
		expect(tagTooltip({ ...tag('critical'), description: 'Pages on-call' })).toBe('Pages on-call');
		expect(tagTooltip(status('planned'))).toContain('Status');
	});

	it('is empty for a tag with neither, so the chip shows no tooltip', () => {
		expect(tagTooltip({ ...tag('critical'), description: '   ' })).toBe('');
		expect(tagTooltip(null)).toBe('');
	});
});
