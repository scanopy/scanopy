<script lang="ts" context="module">
	import { entities } from '$lib/shared/stores/metadata';
	import { formatRelativeTime } from '$lib/shared/utils/formatting';
	import { getDaemonStatusTag } from '$lib/features/daemons/utils';
	import type { Daemon } from '$lib/features/daemons/types/base';

	export const HomeDaemonDisplay: EntityDisplayComponent<Daemon, Record<string, never>> = {
		getId: (daemon) => daemon.id,
		getLabel: (daemon) => daemon.name,
		getIcon: () => entities.getIconComponent('Daemon'),
		getIconColor: () => entities.getColorHelper('Daemon').icon,
		getDescription: (daemon) =>
			daemon.last_seen ? `Last seen ${formatRelativeTime(daemon.last_seen)}` : '',
		getTags: (daemon) => [getDaemonStatusTag(daemon)]
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '$lib/shared/components/forms/selection/types';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';

	export let item: Daemon;
	export let context: Record<string, never> = {} as Record<string, never>;
</script>

<ListSelectItem {item} {context} displayComponent={HomeDaemonDisplay} />
