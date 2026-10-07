<script lang="ts" context="module">
	import { entities, dependencyTypes } from '$lib/shared/stores/metadata';
	import { dependencies_memberCount } from '$lib/paraglide/messages';

	export type DependencyTagRole = 'dependencyType';

	export type DependencyDisplayContext = DisplayTagContext<DependencyTagRole>;

	export const DependencyDisplay: EntityDisplayComponent<Dependency, DependencyDisplayContext> = {
		getId: (dependency: Dependency) => dependency.id,
		getLabel: (dependency: Dependency) => dependency.name,
		getDescription: (dependency: Dependency) => {
			const members = dependency.members;
			const count =
				members?.type === 'Services'
					? members.service_ids.length
					: members?.type === 'Bindings'
						? members.binding_ids.length
						: 0;
			return dependencies_memberCount({ count });
		},
		getIcon: (dependency: Dependency) =>
			dependencyTypes.getIconComponent(dependency.dependency_type),
		getIconColor: () => entities.getColorHelper('Dependency').icon,
		compactHides: ['dependencyType'] satisfies DependencyTagRole[],
		getTags: (dependency: Dependency) => [
			{
				...dependencyTypes.getTag(dependency.dependency_type),
				role: 'dependencyType' satisfies DependencyTagRole
			}
		]
	};
</script>

<script lang="ts">
	import type { DisplayTagContext, EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';
	import type { Dependency } from '$lib/features/dependencies/types/base';

	export let item: Dependency;
	export let context: DependencyDisplayContext = {};
</script>

<ListSelectItem {item} {context} displayComponent={DependencyDisplay} />
