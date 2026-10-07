<script lang="ts" module>
	export const UserApiKeyDisplay: EntityDisplayComponent<UserApiKey, object> = {
		getId: (key) => key.id,
		getLabel: (key) => key.name,
		getDescription: (key) =>
			key.last_used
				? common_lastUsedAgo({ time: formatRelativeTime(key.last_used) })
				: common_neverUsed(),
		getIcon: () => entities.getIconComponent('UserApiKey'),
		getIconColor: () => entities.getColorHelper('UserApiKey').icon,
		getTags: (key) => [
			key.is_enabled
				? { label: common_enabled(), color: toColor('green') }
				: { label: common_disabled(), color: toColor('red') }
		],
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import type { UserApiKey } from '$lib/features/user_api_keys/queries';
	import { entities } from '$lib/shared/stores/metadata';
	import { formatRelativeTime } from '$lib/shared/utils/formatting';
	import {
		common_disabled,
		common_enabled,
		common_lastUsedAgo,
		common_neverUsed
	} from '$lib/paraglide/messages';
	import { toColor } from '$lib/shared/utils/styling';

	interface Props {
		item: UserApiKey;
		context?: object;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={UserApiKeyDisplay} />
