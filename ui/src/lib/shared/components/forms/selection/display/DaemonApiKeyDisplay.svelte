<script lang="ts" module>
	export const DaemonApiKeyDisplay: EntityDisplayComponent<DaemonApiKey, object> = {
		getId: (key) => key.id,
		getLabel: (key) => key.name,
		getDescription: (key) =>
			key.last_used
				? common_lastUsedAgo({ time: formatTimestamp(key.last_used) })
				: common_neverUsed(),
		getIcon: () => entities.getIconComponent('DaemonApiKey'),
		getIconColor: () => entities.getColorHelper('DaemonApiKey').icon,
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
	import type { DaemonApiKey } from '$lib/features/daemon_api_keys/types/base';
	import { entities } from '$lib/shared/stores/metadata';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import {
		common_disabled,
		common_enabled,
		common_lastUsedAgo,
		common_neverUsed
	} from '$lib/paraglide/messages';
	import { toColor } from '$lib/shared/utils/styling';

	interface Props {
		item: DaemonApiKey;
		context?: object;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={DaemonApiKeyDisplay} />
