<script lang="ts" module>
	import type { Interface, IPAddress, Host } from '#lib/features/hosts/types/base.ts';
	import type { Subnet } from '#lib/features/subnets/types/base.ts';

	export interface VlanInfo {
		id: string;
		vlan_number: number;
		name: string;
	}

	/**
	 * One resolved adjacency, ready to render — GH #701 replaced the old single-valued
	 * `Interface.neighbor` with a `Vec` of rows, so a port can now show several of these instead of
	 * at most one. `id` is the backing `InterfaceNeighborRow.id`, kept for a stable `{#each}` key.
	 */
	export interface NeighborEntry {
		id: string;
		neighborHost: Host | null;
		neighborInterface: Interface | null;
	}

	export interface InterfaceRelations {
		linkedIpAddress?: IPAddress | null;
		linkedSubnet?: Subnet | null;
		neighbours?: NeighborEntry[];
		taggedVlans?: VlanInfo[];
	}
</script>

<script lang="ts">
	import { hostDisplayName } from '#lib/features/hosts/host-display-name.ts';
	import { interfaceDisplayName } from '#lib/features/hosts/interface-display-name.ts';
	import InfoRow from '#lib/shared/components/data/InfoRow.svelte';
	import EntityTag from '#lib/shared/components/data/EntityTag.svelte';
	import { entityRef } from '#lib/shared/components/data/types.ts';
	import { entities } from '#lib/shared/stores/metadata.ts';
	import {
		common_ipAddress,
		hosts_interfaces_neighbor,
		hosts_interfaces_taggedVlans,
		vlans_vlanLabel
	} from '#lib/paraglide/messages.js';

	/** An interface's rows that resolve other entities: its IP and subnet, tagged VLANs, neighbours. */
	let {
		linkedIpAddress = null,
		linkedSubnet = null,
		neighbours = [],
		taggedVlans = []
	}: InterfaceRelations = $props();
</script>

<InfoRow label={common_ipAddress()}>
	{#if linkedIpAddress}
		<div class="flex flex-wrap items-center gap-1">
			<EntityTag
				entityRef={entityRef('IPAddress', linkedIpAddress.id, linkedIpAddress)}
				label={linkedIpAddress.ip_address}
				icon={entities.getIconComponent('IPAddress')}
				color={entities.getColorHelper('IPAddress').color}
			/>
			{#if linkedSubnet}
				<span class="text-xs text-tertiary">on</span>
				<EntityTag
					entityRef={entityRef('Subnet', linkedSubnet.id, linkedSubnet)}
					label={linkedSubnet.name && linkedSubnet.name !== linkedSubnet.cidr
						? `${linkedSubnet.name} (${linkedSubnet.cidr})`
						: linkedSubnet.cidr}
					icon={entities.getIconComponent('Subnet')}
					color={entities.getColorHelper('Subnet').color}
				/>
			{/if}
		</div>
	{:else}
		-
	{/if}
</InfoRow>

{#if taggedVlans.length > 0}
	<InfoRow label={hosts_interfaces_taggedVlans()}>
		<div class="flex flex-wrap gap-1">
			{#each taggedVlans as vlan (vlan.id)}
				<EntityTag
					entityRef={entityRef('Vlan', vlan.id, vlan)}
					label={vlans_vlanLabel({ number: vlan.vlan_number })}
					color={entities.getColorHelper('Vlan').color}
				/>
			{/each}
		</div>
	</InfoRow>
{/if}

<InfoRow label={hosts_interfaces_neighbor()}>
	{#if neighbours.length > 0}
		<div class="flex flex-col gap-1.5">
			{#each neighbours as neighbour (neighbour.id)}
				<div class="flex flex-wrap items-center gap-1">
					{#if neighbour.neighborInterface}
						<EntityTag
							entityRef={entityRef(
								'Interface',
								neighbour.neighborInterface.id,
								neighbour.neighborInterface
							)}
							label={interfaceDisplayName(neighbour.neighborInterface)}
							icon={entities.getIconComponent('Interface')}
							color={entities.getColorHelper('Interface').color}
						/>
						<span class="text-xs text-tertiary">on</span>
					{/if}
					{#if neighbour.neighborHost}
						<EntityTag
							entityRef={entityRef('Host', neighbour.neighborHost.id, neighbour.neighborHost)}
							label={hostDisplayName(neighbour.neighborHost)}
							color={entities.getColorHelper('Host').color}
						/>
					{/if}
				</div>
			{/each}
		</div>
	{:else}
		-
	{/if}
</InfoRow>
