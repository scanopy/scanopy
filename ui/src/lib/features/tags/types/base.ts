import type { components } from '$lib/api/schema';
import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';

export type Tag = components['schemas']['Tag'];

export function createDefaultTag(organization_id: string): Tag {
	return {
		name: '',
		description: null,
		color: 'Yellow',
		tag_group_id: null,
		position: null,
		icon: null,
		// Read-only: the server sets it to Manual on create.
		source: { type: 'Manual' },
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		organization_id
	};
}

export type TagGroup = components['schemas']['TagGroup'];
export type TagGroupType = components['schemas']['TagGroupType'];

export function createDefaultTagGroup(organization_id: string): TagGroup {
	return {
		name: '',
		description: null,
		group_type: 'Custom',
		entity_types: null,
		ordered: true,
		scope_computed: false,
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		organization_id
	};
}
