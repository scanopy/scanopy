import { Eye, Pencil, Plus, Trash2 } from '@lucide/svelte';
import type { components } from '$lib/api/schema';
import type {
	ComparisonColumn,
	ComparisonSection
} from '$lib/shared/components/data/ComparisonGrid.svelte';
import { common_create, common_delete, common_edit, common_view } from '$lib/paraglide/messages';
import { permissions, productAreas, resources } from '$lib/shared/stores/metadata';
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

/** The four roles, lowest first, as the plan comparison lists plans. */
export function roleColumns(): ComparisonColumn[] {
	return [...permissions.getItems()]
		.reverse()
		.map((role) => ({ id: role.id, label: permissions.getName(role.id) }));
}

/** What `role` can do to `resource`. Empty for no access. */
export function roleActions(resource: Resource, role: string): DisplayAction[] {
	return resources.getMetadata(resource).display_actions[role as UserOrgPermissions] ?? [];
}

/** Whether any role has a row for `resource`. */
function isShown(resource: Resource): boolean {
	return roleColumns().some((role) => roleActions(resource, role.id).length > 0);
}

/** Every shown resource under its product area, in display order. Empty areas are left out. */
export function roleSections(): ComparisonSection<Resource>[] {
	return productAreas.getItems().flatMap((area) => {
		const rows = resources
			.getItems()
			.filter((resource) => resource.category === area.id)
			.map((resource) => resource.id as Resource)
			.filter(isShown);
		if (rows.length === 0) return [];
		return [
			{
				id: area.id,
				label: productAreas.getName(area.id),
				icon: productAreas.getIconComponent(area.id),
				rows
			}
		];
	});
}

/** The resources `role` can do anything to, in the grid's order. */
export function roleResources(role: string): Resource[] {
	return roleSections()
		.flatMap((section) => section.rows)
		.filter((resource) => roleActions(resource, role).length > 0);
}
