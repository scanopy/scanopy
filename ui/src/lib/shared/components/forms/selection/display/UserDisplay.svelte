<script lang="ts" module>
	export type UserTagRole = 'permission';

	export const UserDisplay: EntityDisplayComponent<User, DisplayTagContext<UserTagRole>> = {
		getId: (user) => user.id,
		// The User type carries no name or avatar — email is the identity everywhere in the UI.
		getLabel: (user) => user.email,
		getIcon: () => entities.getIconComponent('User'),
		getIconColor: () => entities.getColorHelper('User').icon,
		getTags: (user) => [
			{
				label: permissions.getName(user.permissions),
				color: permissions.getColorHelper(user.permissions).color,
				role: 'permission' satisfies UserTagRole
			}
		],
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { DisplayTagContext, EntityDisplayComponent } from '../types';
	import type { User } from '$lib/features/users/types';
	import { entities, permissions } from '$lib/shared/stores/metadata';

	interface Props {
		item: User;
		context?: DisplayTagContext<UserTagRole>;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={UserDisplay} />
