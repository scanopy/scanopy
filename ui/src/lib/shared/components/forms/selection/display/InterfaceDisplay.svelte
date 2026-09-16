<script lang="ts" context="module">
	import type { Interface } from '$lib/features/hosts/types/base';
	import type { EntityDisplayComponent } from '../types';
	import { entities } from '$lib/shared/stores/metadata';
	import { getOperStatusLabels } from '$lib/features/credentials/types/base';
	import { common_status, common_unknown } from '$lib/paraglide/messages';
	import { interfaceDisplayName } from '$lib/features/hosts/interface-display-name';

	export const InterfaceDisplay: EntityDisplayComponent<Interface, void> = {
		getId: (entry: Interface) => entry.id,
		getLabel: (entry: Interface) => interfaceDisplayName(entry),
		getDescription: (entry: Interface) => {
			return entry.mac_address ?? 'No MAC Address';
		},
		getIcon: () => entities.getIconComponent('Interface'),
		getIconColor: () => entities.getColorHelper('Interface').icon,
		getTags: (entry: Interface) => {
			// Operational (link) status — prefixed with "Status" so a bare "Unknown" badge next to
			// the interface name doesn't read as "we don't know anything about this interface".
			// oper_status genuinely is unset for e.g. a PROFINET DCP identify, which reports a MAC
			// and nothing else; that's real information, not a rendering gap.
			const operStatusLabels = getOperStatusLabels();
			const status = entry.oper_status ? operStatusLabels[entry.oper_status] : common_unknown();
			const statusColor =
				entry.oper_status === 'Up' ? 'Green' : entry.oper_status === 'Down' ? 'Red' : 'Yellow';

			const tags: TagProps[] = [
				{
					label: `${common_status()}: ${status}`,
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

	export let item: Interface;
	export let context: void = undefined;
</script>

<ListSelectItem {item} {context} displayComponent={InterfaceDisplay} />
