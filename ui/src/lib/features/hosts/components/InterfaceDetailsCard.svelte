<script lang="ts">
	import type { Interface, IPAddress } from '$lib/features/hosts/types/base';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import type { Host } from '$lib/features/hosts/types/base';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import { interfaceDisplayName } from '$lib/features/hosts/interface-display-name';
	import { getAdminStatusLabels, getOperStatusLabels } from '$lib/features/credentials/types/base';
	import CollapsibleCard from '$lib/shared/components/data/CollapsibleCard.svelte';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import { entities } from '$lib/shared/stores/metadata';
	import type { Color } from '$lib/shared/utils/styling';
	import {
		common_details,
		common_ipAddress,
		common_macAddress,
		common_speed,
		common_status,
		common_index,
		common_unknown,
		hosts_interfaces_adminStatus,
		hosts_interfaces_aliasDescription,
		hosts_interfaces_nativeVlan,
		hosts_interfaces_neighbor,
		hosts_interfaces_operStatus,
		hosts_interfaces_taggedVlans
	} from '$lib/paraglide/messages';

	interface VlanInfo {
		id: string;
		vlan_number: number;
		name: string;
	}

	/**
	 * One resolved adjacency, ready to render — GH #701 replaced the old single-valued
	 * `Interface.neighbor` with a `Vec` of rows, so a port can now show several of these instead of
	 * at most one. `id` is the backing `InterfaceNeighborRow.id`, kept for a stable `{#each}` key.
	 */
	interface NeighborEntry {
		id: string;
		neighborHost: Host | null;
		neighborInterface: Interface | null;
	}

	interface Props {
		iface: Interface;
		linkedIpAddress?: IPAddress | null;
		linkedSubnet?: Subnet | null;
		neighbours?: NeighborEntry[];
		nativeVlan?: VlanInfo | null;
		taggedVlans?: VlanInfo[];
		showStatus?: boolean;
	}

	let {
		iface,
		linkedIpAddress = null,
		linkedSubnet = null,
		neighbours = [],
		nativeVlan = null,
		taggedVlans = [],
		showStatus = true
	}: Props = $props();

	function formatSpeed(speed: number | null | undefined): string {
		if (!speed) return common_unknown();
		if (speed >= 1_000_000_000) return `${(speed / 1_000_000_000).toFixed(1)} Gbps`;
		if (speed >= 1_000_000) return `${(speed / 1_000_000).toFixed(1)} Mbps`;
		if (speed >= 1_000) return `${(speed / 1_000).toFixed(1)} Kbps`;
		return `${speed} bps`;
	}

	// A status nothing read is unknown, the same as one this build has no label for.
	let adminStatusLabel = $derived(
		(iface.admin_status && getAdminStatusLabels()[iface.admin_status]) || common_unknown()
	);
	let operStatusLabel = $derived(
		(iface.oper_status && getOperStatusLabels()[iface.oper_status]) || common_unknown()
	);

	let operStatusColor: Color = $derived.by(() => {
		switch (iface.oper_status) {
			case 'Up':
				return 'Green';
			case 'Down':
				return 'Red';
			case 'Dormant':
				return 'Yellow';
			default:
				return 'Gray';
		}
	});

	let statusExpanded = $state(true);
	let detailsExpanded = $state(true);
</script>

{#if showStatus}
	<CollapsibleCard title={common_status()} bind:expanded={statusExpanded}>
		<InfoRow label={hosts_interfaces_adminStatus()}>{adminStatusLabel}</InfoRow>
		<InfoRow label={hosts_interfaces_operStatus()}>
			<Tag label={operStatusLabel} color={operStatusColor} />
		</InfoRow>
	</CollapsibleCard>
{/if}

<CollapsibleCard title={common_details()} bind:expanded={detailsExpanded}>
	<InfoRow label="ifName">{iface.if_name || '-'}</InfoRow>
	<InfoRow label="ifType">{iface.if_type || '-'}</InfoRow>
	<InfoRow label={common_macAddress()} mono>{iface.mac_address || '-'}</InfoRow>
	<InfoRow label={common_speed()}>{formatSpeed(iface.speed_bps)}</InfoRow>
	<InfoRow label={hosts_interfaces_aliasDescription()}>{iface.if_alias || '-'}</InfoRow>
	<InfoRow label={common_index()}>{iface.if_index ?? '-'}</InfoRow>

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
					<span class="text-tertiary text-xs">on</span>
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

	{#if nativeVlan}
		<InfoRow label={hosts_interfaces_nativeVlan()}>
			<Tag
				label="VLAN {nativeVlan.vlan_number} ({nativeVlan.name})"
				color={entities.getColorHelper('Vlan').color}
			/>
		</InfoRow>
	{/if}

	{#if taggedVlans.length > 0}
		<InfoRow label={hosts_interfaces_taggedVlans()}>
			<div class="flex flex-wrap gap-1">
				{#each taggedVlans as vlan (vlan.id)}
					<Tag label="VLAN {vlan.vlan_number}" color={entities.getColorHelper('Vlan').color} />
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
							<span class="text-tertiary text-xs">on</span>
						{/if}
						{#if neighbour.neighborHost}
							<EntityTag
								entityRef={entityRef('Host', neighbour.neighborHost.id, neighbour.neighborHost)}
								label={hostDisplayName(neighbour.neighborHost)}
								icon={entities.getIconComponent('Host')}
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
</CollapsibleCard>
