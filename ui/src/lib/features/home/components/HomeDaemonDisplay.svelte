<script lang="ts" context="module">
	import { entities } from '$lib/shared/stores/metadata';
	import { toColor } from '$lib/shared/utils/styling';
	import { formatRelativeTime } from '$lib/shared/utils/formatting';
	import { getDaemonStatusTag } from '$lib/features/daemons/utils';
	import type { Daemon } from '$lib/features/daemons/types/base';

	export const HomeDaemonDisplay: EntityDisplayComponent<Daemon, Record<string, never>> = {
		getId: (daemon) => daemon.id,
		getLabel: (daemon) => daemon.name,
		getIcon: () => entities.getIconComponent('Daemon'),
		getIconColor: () => entities.getColorHelper('Daemon').icon,
		getTags: (daemon) => {
			const tags = [];

			if (daemon.last_seen) {
				tags.push({
					label: `Last seen ${formatRelativeTime(daemon.last_seen)}`,
					color: toColor('green')
				});
			}

			const statusTag = getDaemonStatusTag(daemon);
			if (statusTag) {
				tags.push(statusTag);
			}

			return tags;
		}
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '$lib/shared/components/forms/selection/types';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';

	export let item: Daemon;
	export let context: Record<string, never> = {} as Record<string, never>;
</script>

<ListSelectItem {item} {context} displayComponent={HomeDaemonDisplay} />
