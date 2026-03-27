<script lang="ts" context="module">
	import type { IfEntry } from '$lib/features/hosts/types/base';
	import type { EntityDisplayComponent } from '../types';
	import { entities } from '$lib/shared/stores/metadata';
	import { getOperStatusLabels } from '$lib/features/credentials/types/base';
	import { common_unknown } from '$lib/paraglide/messages';

	export const IfEntryDisplay: EntityDisplayComponent<IfEntry, void> = {
		getId: (entry: IfEntry) => entry.id,
		getLabel: (entry: IfEntry) => entry.if_descr || `Interface ${entry.if_index}`,
		getDescription: (entry: IfEntry) => {
			return entry.mac_address ?? 'No MAC Address';
		},
		getIcon: () => entities.getIconComponent('IfEntry'),
		getIconColor: () => entities.getColorHelper('IfEntry').icon,
		getTags: (entry: IfEntry) => {
			// Show neighbor info as tag if present
			const operStatusLabels = getOperStatusLabels();
			const status = entry.oper_status ? operStatusLabels[entry.oper_status] : common_unknown();
			const statusColor =
				entry.oper_status === 'Up' ? 'Green' : entry.oper_status === 'Down' ? 'Red' : 'Yellow';

			const tags: TagProps[] = [
				{
					label: status,
					color: statusColor
				}
			];

			return tags;
		},
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '../ListSelectItem.svelte';
	import type { TagProps } from '$lib/shared/components/data/types';

	export let item: IfEntry;
	export let context: void = undefined;
</script>

<ListSelectItem {item} {context} displayComponent={IfEntryDisplay} />
