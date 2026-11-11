<script lang="ts" module>
	import { entities } from '#lib/shared/stores/metadata.ts';
	import { formatDate } from '#lib/shared/utils/formatting.ts';
	import type { Snapshot } from '#lib/features/snapshots/queries.ts';

	export const SnapshotDisplay: EntityDisplayComponent<Snapshot, object> = {
		getId: (snapshot: Snapshot) => snapshot.id,
		getLabel: (snapshot: Snapshot) => formatDate(snapshot.taken_at),
		getIcon: () => entities.getIconComponent('Snapshot'),
		getIconColor: () => entities.getColorHelper('Snapshot').icon
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';

	let {
		item,
		context = {}
	}: {
		item: Snapshot;
		context: object;
	} = $props();

	$effect(() => {
		void entities;
	});
</script>

<ListSelectItem {item} {context} displayComponent={SnapshotDisplay} />
