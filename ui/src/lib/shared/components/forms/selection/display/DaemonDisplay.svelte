<script lang="ts" context="module">
	import { entities } from '#lib/shared/stores/metadata.ts';
	import { entityRef } from '#lib/shared/components/data/types.ts';
	import type { Host } from '#lib/features/hosts/types/base.ts';
	import type { Subnet } from '#lib/features/subnets/types/base.ts';
	import type { Daemon } from '#lib/features/daemons/types/base.ts';

	// Context for daemon display - needs access to hosts and subnets for lookups
	export interface DaemonDisplayContext {
		hosts: Host[];
		subnets: Subnet[];
	}

	export const DaemonDisplay: EntityDisplayComponent<Daemon, DaemonDisplayContext> = {
		getId: (daemon: Daemon) => daemon.id,
		getLabel: (daemon: Daemon) => daemon.name || 'Unknown Daemon',
		getDescription: (daemon: Daemon, context: DaemonDisplayContext) => {
			const hostsData = context?.hosts ?? [];
			const host = hostsData.find((h) => h.id === daemon.host_id);
			const parts = [];
			if (host?.description) parts.push(host.description);
			return parts.join(' · ');
		},
		getIcon: () => entities.getIconComponent('Daemon'),
		getIconColor: () => entities.getColorHelper('Daemon').icon,
		getTags: (daemon: Daemon, context: DaemonDisplayContext) => {
			const subnetsData = context?.subnets ?? [];
			return daemon.interfaced_subnet_ids
				.map((id) => subnetsData.find((sub) => sub.id === id))
				.filter(Boolean)
				.map((subnet) => ({
					label: subnet!.cidr,
					color: entities.getColorHelper('Subnet').color,
					entityRef: entityRef('Subnet', subnet!.id, subnet!)
				}));
		},
		getCategory: () => null
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';

	export let item: Daemon;
	export let context: DaemonDisplayContext = { hosts: [], subnets: [] };
</script>

<ListSelectItem {item} {context} displayComponent={DaemonDisplay} />
