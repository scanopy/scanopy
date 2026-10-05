<script lang="ts" context="module">
	import type { Interface } from '$lib/features/hosts/types/base';
	import type { DisplayTagContext, EntityDisplayComponent } from '../types';
	import { entities, ifOperStatuses } from '$lib/shared/stores/metadata';
	import {
		common_unknown,
		hosts_interfaces_operStatusNotReported,
		hosts_noMacAddress
	} from '$lib/paraglide/messages';
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
			// Operational (link) status, named and coloured by the IfOperStatus metadata the L2
			// OperStatus filter uses; its tooltip says what the status means. oper_status is unset
			// for e.g. a PROFINET DCP identify, which reports a MAC and nothing else: the tag then
			// reads Unknown, and its tooltip says the device didn't report it.
			const status: TagProps = entry.oper_status
				? ifOperStatuses.getTag(entry.oper_status)
				: {
						label: common_unknown(),
						color: ifOperStatuses.getColorHelper('Unknown').color,
						title: hosts_interfaces_operStatusNotReported()
					};

			const tags: TagProps[] = [
				{
					...status,
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
