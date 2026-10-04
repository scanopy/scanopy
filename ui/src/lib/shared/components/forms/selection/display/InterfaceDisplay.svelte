<script lang="ts" context="module">
	import type { Interface } from '$lib/features/hosts/types/base';
	import type { DisplayTagContext, EntityDisplayComponent } from '../types';
	import { entities } from '$lib/shared/stores/metadata';
	import { getOperStatusLabels } from '$lib/features/credentials/types/base';
	import { common_status, common_unknown, hosts_noMacAddress } from '$lib/paraglide/messages';
	import { interfaceDisplayName } from '$lib/features/hosts/interface-display-name';

	/** `linkFault` is a link reported Down; `linkStatus` is any other status, Up or unknown. */
	export type InterfaceTagRole = 'linkStatus' | 'linkFault';

	export type InterfaceDisplayContext = DisplayTagContext<InterfaceTagRole> | undefined;

	export const InterfaceDisplay: EntityDisplayComponent<Interface, InterfaceDisplayContext> = {
		getId: (entry: Interface) => entry.id,
		getLabel: (entry: Interface) => interfaceDisplayName(entry),
		getDescription: (entry: Interface) => {
			// A port named after its MAC (no ifAlias or ifDescr) already shows it as the label.
			const label = interfaceDisplayName(entry);
			if (entry.mac_address === label) return '';
			return entry.mac_address ?? hosts_noMacAddress();
		},
		getIcon: () => entities.getIconComponent('Interface'),
		getIconColor: () => entities.getColorHelper('Interface').icon,
		// A compact row (the topology inspector) keeps the status only when the link is down: on
		// every other row "Status: Up" repeats what the link being drawn already says.
		compactHides: ['linkStatus'] satisfies InterfaceTagRole[],
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
					color: statusColor,
					role: (entry.oper_status === 'Down'
						? 'linkFault'
						: 'linkStatus') satisfies InterfaceTagRole
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
	export let context: InterfaceDisplayContext = undefined;
</script>

<ListSelectItem {item} {context} displayComponent={InterfaceDisplay} />
