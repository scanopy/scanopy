import { describe, expect, it } from 'vitest';
import {
	compareTagsByGroup,
	groupNamed,
	groupsFor,
	tagAllowedOn,
	tagsDisplacedBy,
	tagTooltip,
	withTagAdded
} from '#lib/features/tags/groups.ts';
import { tagGroupFields, tagGroupIdOf } from '#lib/features/tags/groupFields.ts';
import { blankFilterState, toggleValue } from '#lib/shared/components/data/controls/filtering.ts';
import { decodeFilters, listParams } from '#lib/shared/components/data/controls/listQuery.ts';
import {
	createDefaultTag,
	createDefaultTagGroup,
	type Tag,
	type TagGroup
} from '#lib/features/tags/types/base.ts';
import { getFieldKey, isOrderableField } from '#lib/shared/components/data/types.ts';

function group(
	id: string,
	group_type: TagGroup['group_type'] = 'Custom',
	entity_types: TagGroup['entity_types'] = null
): TagGroup {
	return { ...createDefaultTagGroup('org'), id, name: id, group_type, entity_types };
}

const statusGroup = group('Status', 'Status');
const environmentGroup = group('Environment');
const applicationGroup = { ...group('Application', 'Application'), ordered: false };
const backupGroup = group('Backup', 'Custom', ['Host']);
const groups = [applicationGroup, statusGroup, backupGroup, environmentGroup];

function tag(id: string, tagGroup: TagGroup | null = null, position: number | null = null): Tag {
	return {
		...createDefaultTag('org'),
		id,
		name: id,
		tag_group_id: tagGroup?.id ?? null,
		position
	};
}

const planned = tag('planned', statusGroup, 1);
const active = tag('active', statusGroup, 2);
const decommissioned = tag('decommissioned', statusGroup, 3);
const production = tag('production', environmentGroup, 1);
// Application is unordered: its tags carry no position and list by name.
const web = tag('web', applicationGroup);
const db = tag('db', applicationGroup);
const nightly = tag('nightly', backupGroup, 1);
const critical = tag('critical');
const tags = [decommissioned, planned, active, production, web, db, nightly, critical];

describe('withTagAdded', () => {
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

describe('tagsDisplacedBy', () => {
	// GH #841: the application wizard's picker lists the Application group's tags, not one entity's.
	// Picking suggestions, going Next and Back, then creating a custom tag there must keep them all.
	it('displaces nothing when the selection is a group member list', () => {
		const media = tag('media', applicationGroup);
		const custom = tag('custom', applicationGroup);
		const wizardTags = [web, db, media, custom];
		expect(tagsDisplacedBy(['web', 'db', 'media'], 'custom', wizardTags, false)).toEqual([]);
	});

	it("displaces the same group's tags on an entity", () => {
		expect(tagsDisplacedBy(['web', 'planned'], 'db', tags, true)).toEqual(['web']);
	});
});

describe('entity-type scope', () => {
	it('offers a limited group only to the types it names', () => {
		expect(groupsFor(groups, 'Host')).toContain(backupGroup);
		expect(groupsFor(groups, 'Service')).not.toContain(backupGroup);
		expect(groupsFor(groups, 'Service')).toContain(statusGroup);
	});

	it('allows a tag in no group anywhere', () => {
		expect(tagAllowedOn(critical, groups, 'Vlan')).toBe(true);
		expect(tagAllowedOn(nightly, groups, 'Vlan')).toBe(false);
		expect(tagAllowedOn(nightly, groups, 'Host')).toBe(true);
	});
});

describe('compareTagsByGroup', () => {
	it('lists groups in the server order, each group in its own order, ungrouped tags last', () => {
		expect([...tags].sort(compareTagsByGroup(groups)).map((t) => t.id)).toEqual([
			'db',
			'web',
			'planned',
			'active',
			'decommissioned',
			'nightly',
			'production',
			'critical'
		]);
	});
});

describe('groupNamed', () => {
	it('finds a group by name or by a system group label, in any case', () => {
		expect(groupNamed(groups, ' environment ')).toBe(environmentGroup);
		expect(groupNamed(groups, 'STATUS')).toBe(statusGroup);
		expect(groupNamed(groups, 'Tier')).toBeNull();
	});
});

describe('tagTooltip', () => {
	it('puts the group first and the trimmed description under it', () => {
		const content = tagTooltip({ ...planned, description: '  Not deployed yet ' }, groups);
		expect(content?.sections).toHaveLength(2);
		expect(content?.sections[0].rows?.[0].value).toContain('Status');
		expect(content?.sections[1].text).toBe('Not deployed yet');
	});

	it('is null for a tag with neither, so the chip shows no tooltip', () => {
		expect(tagTooltip({ ...critical, description: '   ' }, groups)).toBeNull();
		expect(tagTooltip(null, groups)).toBeNull();
	});
});

describe('tag group table columns', () => {
	type Row = { id: string; tags: string[] };
	const rows: Row[] = [
		{ id: 'retired', tags: ['decommissioned'] },
		{ id: 'untagged', tags: [] },
		{ id: 'new', tags: ['planned', 'production'] },
		{ id: 'live', tags: ['active'] }
	];
	const fields = tagGroupFields<Row>({
		groups,
		tags,
		entityType: 'Service',
		getItemTags: (row) => row.tags
	});

	it('adds one hidden column per group that applies to the entity type', () => {
		expect(fields.map((f) => tagGroupIdOf(getFieldKey(f)))).toEqual([
			'Application',
			'Status',
			'Environment'
		]);
		expect(fields.every((f) => f.display?.hiddenByDefault)).toBe(true);
	});

	it('is server-orderable, keyed the way the server groups', () => {
		const status = fields[1];
		expect(isOrderableField(status)).toBe(true);
		expect(status.serverFiltered).toBe(true);
		expect(status.getGroupValue!(rows[2])).toBe('0000000001:planned:planned');
		expect(status.getGroupValue!(rows[1])).toBeNull();
		// An unordered group's tags key at position 0, so the name orders them.
		const application = fields[0];
		expect(application.getGroupValue!({ id: 'x', tags: ['web'] })).toBe('0000000000:web:web');
	});
});

describe('tag-group filters as tag_sets', () => {
	type Row = { id: string; tags: string[] };
	const fields = tagGroupFields<Row>({
		groups,
		tags,
		entityType: 'Service',
		getItemTags: (row) => row.tags
	});
	const [, status, environment] = fields;

	it('sends one set of tag ids per filtered group, and reads each back to its own group', () => {
		let filterState = blankFilterState(fields, false);
		filterState = toggleValue(filterState, getFieldKey(status), 'planned')!;
		filterState = toggleValue(filterState, getFieldKey(status), 'active')!;
		filterState = toggleValue(filterState, getFieldKey(environment), 'production')!;

		const { request } = listParams(
			{
				sort: { field: null, direction: 'asc' },
				group: null,
				filterState,
				staleOnly: false,
				search: ''
			},
			fields,
			{ tags: true, stale: true, search: true }
		);
		expect(request.getAll('tag_sets')).toEqual(['planned,active', 'production']);

		const decoded = decodeFilters(request, fields);
		expect(decoded[getFieldKey(status)].values).toEqual(['planned', 'active']);
		expect(decoded[getFieldKey(environment)].values).toEqual(['production']);
	});
});
