import type { components } from '$lib/api/schema';
import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';

export type Tag = components['schemas']['Tag'];

export function createDefaultTag(organization_id: string): Tag {
	return {
		name: '',
		description: null,
		color: 'Yellow',
		exclusive_set: null,
		icon: null,
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		organization_id
	};
}
