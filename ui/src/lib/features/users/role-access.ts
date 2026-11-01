import { Eye, Pencil, Plus, Trash2 } from '@lucide/svelte';
import type { components } from '$lib/api/schema';
import { common_create, common_delete, common_edit, common_view } from '$lib/paraglide/messages';
import { productAreas, resources } from '$lib/shared/stores/metadata';
import type { IconComponent } from '$lib/shared/utils/types';

type Resource = components['schemas']['Resource'];
type DisplayAction = components['schemas']['DisplayAction'];
type UserOrgPermissions = components['schemas']['UserOrgPermissions'];

const ACTION_DISPLAY: Record<DisplayAction, { icon: IconComponent; label: () => string }> = {
	View: { icon: Eye, label: () => common_view() },
	Create: { icon: Plus, label: () => common_create() },
	Edit: { icon: Pencil, label: () => common_edit() },
	Delete: { icon: Trash2, label: () => common_delete() }
};

/** The actions in the fixed order every row shows them. */
export function actionColumns() {
	return (Object.keys(ACTION_DISPLAY) as DisplayAction[]).map((id) => ({
		id,
		icon: ACTION_DISPLAY[id].icon,
		label: ACTION_DISPLAY[id].label()
	}));
}

/** What `role` can do to `resource`. Empty for no access. */
export function roleActions(resource: Resource, role: string): DisplayAction[] {
	return resources.getMetadata(resource).display_actions[role as UserOrgPermissions] ?? [];
}

/** The resources `role` can do anything to, ordered by product area. */
export function roleResources(role: string): Resource[] {
	return productAreas
		.getItems()
		.flatMap((area) => resources.getItems().filter((resource) => resource.category === area.id))
		.map((resource) => resource.id as Resource)
		.filter((resource) => roleActions(resource, role).length > 0);
}
