import type { components } from '$lib/api/schema';
import type {
	ComparisonColumn,
	ComparisonSection
} from '$lib/shared/components/data/ComparisonGrid.svelte';
import { common_edit, common_full, common_view } from '$lib/paraglide/messages';
import { permissions, productAreas, resources } from '$lib/shared/stores/metadata';

type Resource = components['schemas']['Resource'];
type AccessLevel = components['schemas']['AccessLevel'];
type UserOrgPermissions = components['schemas']['UserOrgPermissions'];

const LEVEL_NAMES: Record<AccessLevel, () => string> = {
	View: () => common_view(),
	Edit: () => common_edit(),
	Full: () => common_full()
};

export function accessLevelName(level: AccessLevel): string {
	return LEVEL_NAMES[level]();
}

/** The four roles, lowest first, as the plan comparison lists plans. */
export function roleColumns(): ComparisonColumn[] {
	return [...permissions.getItems()]
		.reverse()
		.map((role) => ({ id: role.id, label: permissions.getName(role.id) }));
}

/** Every resource under its product area, in display order. Areas with no resources are left out. */
export function roleSections(): ComparisonSection<Resource>[] {
	return productAreas.getItems().flatMap((area) => {
		const rows = resources
			.getItems()
			.filter((resource) => resource.category === area.id)
			.map((resource) => resource.id as Resource);
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

/** What `role` holds on `resource`, or null for no access. */
export function roleLevel(resource: Resource, role: string): AccessLevel | null {
	return resources.getMetadata(resource).levels[role as UserOrgPermissions];
}
