<script lang="ts">
	import { useQueryClient } from '@tanstack/svelte-query';
	import { queryKeys } from '$lib/api/query-client';
	import type { IfEntry, Interface } from '$lib/features/hosts/types/base';
	import { getHostByIdFromCache } from '$lib/features/hosts/queries';
	import { getSubnetByIdFromCache } from '$lib/features/subnets/queries';
	import { getAdminStatusLabels, getOperStatusLabels } from '$lib/features/credentials/types/base';
	import ConfigHeader from '$lib/shared/components/forms/config/ConfigHeader.svelte';
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
		common_unknown,
		hosts_ifEntries_adminStatus,
		hosts_ifEntries_aliasDescription,
		hosts_ifEntries_cdpNeighbor,
		hosts_ifEntries_chassisId,
		hosts_ifEntries_index,
		hosts_ifEntries_lldpNeighbor,
		hosts_ifEntries_lldpSysDescr,
		hosts_ifEntries_managementAddress,
		hosts_ifEntries_neighbor,
		hosts_ifEntries_operStatus,
		hosts_ifEntries_portId,
		hosts_ifEntries_remoteAddress,
		hosts_ifEntries_remoteDevice,
		hosts_ifEntries_remotePlatform,
		hosts_ifEntries_remotePort,
		hosts_ifEntries_remoteSystemName
	} from '$lib/paraglide/messages';

	interface Props {
		ifEntry: IfEntry;
	}

	let { ifEntry }: Props = $props();

	const queryClient = useQueryClient();

	function formatSpeed(speed: number | null | undefined): string {
		if (!speed) return common_unknown();
		if (speed >= 1_000_000_000) return `${(speed / 1_000_000_000).toFixed(1)} Gbps`;
		if (speed >= 1_000_000) return `${(speed / 1_000_000).toFixed(1)} Mbps`;
		if (speed >= 1_000) return `${(speed / 1_000).toFixed(1)} Kbps`;
		return `${speed} bps`;
	}

	let adminStatusLabel = $derived(getAdminStatusLabels()[ifEntry.admin_status] ?? common_unknown());
	let operStatusLabel = $derived(getOperStatusLabels()[ifEntry.oper_status] ?? common_unknown());

	let operStatusColor: Color = $derived.by(() => {
		switch (ifEntry.oper_status) {
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

	// Linked Interface + Subnet resolution
	let linkedInterface = $derived.by(() => {
		if (!ifEntry.interface_id) return null;
		const allInterfaces = queryClient.getQueryData<Interface[]>(queryKeys.interfaces.all) ?? [];
		return allInterfaces.find((i) => i.id === ifEntry.interface_id) ?? null;
	});

	let linkedSubnet = $derived.by(() => {
		if (!linkedInterface) return null;
		return getSubnetByIdFromCache(queryClient, linkedInterface.subnet_id);
	});

	// Neighbor resolution
	let neighborHost = $derived.by(() => {
		if (!ifEntry.neighbor) return null;
		if (ifEntry.neighbor.type === 'Host') {
			return getHostByIdFromCache(queryClient, ifEntry.neighbor.id);
		}
		// IfEntry type — resolve through the remote ifEntry's host_id
		const allIfEntries = queryClient.getQueryData<IfEntry[]>(queryKeys.ifEntries.all) ?? [];
		const remoteEntry = allIfEntries.find((e) => e.id === ifEntry.neighbor!.id);
		if (remoteEntry) {
			return getHostByIdFromCache(queryClient, remoteEntry.host_id);
		}
		return null;
	});

	let neighborIfEntry = $derived.by(() => {
		if (!ifEntry.neighbor || ifEntry.neighbor.type !== 'IfEntry') return null;
		const allIfEntries = queryClient.getQueryData<IfEntry[]>(queryKeys.ifEntries.all) ?? [];
		return allIfEntries.find((e) => e.id === ifEntry.neighbor!.id) ?? null;
	});

	// Section expand state
	let statusExpanded = $state(true);
	let detailsExpanded = $state(true);
	let cdpExpanded = $state(false);
	let lldpExpanded = $state(false);
</script>

<div class="space-y-6">
	<ConfigHeader
		title={ifEntry.if_name || ifEntry.if_descr || `Interface ${ifEntry.if_index}`}
		subtitle={hosts_ifEntries_index({ index: ifEntry.if_index })}
	/>

	<!-- Status Section -->
	<CollapsibleCard title={common_status()} bind:expanded={statusExpanded}>
		<InfoRow label={hosts_ifEntries_adminStatus()}>{adminStatusLabel}</InfoRow>
		<InfoRow label={hosts_ifEntries_operStatus()}>
			<Tag label={operStatusLabel} color={operStatusColor} />
		</InfoRow>
	</CollapsibleCard>

	<!-- Details Section -->
	<CollapsibleCard title={common_details()} bind:expanded={detailsExpanded}>
		<InfoRow label="ifName">{ifEntry.if_name || '-'}</InfoRow>
		<InfoRow label="ifType">{ifEntry.if_type || '-'}</InfoRow>
		<InfoRow label={common_macAddress()} mono>{ifEntry.mac_address || '-'}</InfoRow>
		<InfoRow label={common_speed()}>{formatSpeed(ifEntry.speed_bps)}</InfoRow>
		<InfoRow label={hosts_ifEntries_aliasDescription()}>{ifEntry.if_alias || '-'}</InfoRow>

		<!-- IP Address (linked Interface + Subnet as tags) -->
		<InfoRow label={common_ipAddress()}>
			{#if linkedInterface}
				<div class="flex flex-wrap items-center gap-1">
					<EntityTag
						entityRef={entityRef('Interface', linkedInterface.id, linkedInterface)}
						label={linkedInterface.ip_address}
						icon={entities.getIconComponent('Interface')}
						color={entities.getColorHelper('Interface').color}
					/>
					<span class="text-tertiary text-xs">on</span>
					{#if linkedSubnet}
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

		<!-- Neighbor -->
		<InfoRow label={hosts_ifEntries_neighbor()}>
			{#if ifEntry.neighbor}
				<div class="flex flex-wrap items-center gap-1">
					{#if neighborIfEntry}
						<EntityTag
							entityRef={entityRef('IfEntry', neighborIfEntry.id, neighborIfEntry)}
							label={neighborIfEntry.if_name ||
								neighborIfEntry.if_descr ||
								`Index ${neighborIfEntry.if_index}`}
							icon={entities.getIconComponent('IfEntry')}
							color={entities.getColorHelper('IfEntry').color}
						/>
						<span class="text-tertiary text-xs">on</span>
					{/if}
					{#if neighborHost}
						<EntityTag
							entityRef={entityRef('Host', neighborHost.id, neighborHost)}
							label={neighborHost.name}
							icon={entities.getIconComponent('Host')}
							color={entities.getColorHelper('Host').color}
						/>
					{/if}
				</div>
			{:else}
				-
			{/if}
		</InfoRow>
	</CollapsibleCard>

	<!-- CDP Neighbor Info Section -->
	<CollapsibleCard title={hosts_ifEntries_cdpNeighbor()} bind:expanded={cdpExpanded}>
		<InfoRow label={hosts_ifEntries_remoteDevice()}>{ifEntry.cdp_device_id || '-'}</InfoRow>
		<InfoRow label={hosts_ifEntries_remotePort()}>{ifEntry.cdp_port_id || '-'}</InfoRow>
		<InfoRow label={hosts_ifEntries_remoteAddress()} mono>{ifEntry.cdp_address || '-'}</InfoRow>
		<InfoRow label={hosts_ifEntries_remotePlatform()}>{ifEntry.cdp_platform || '-'}</InfoRow>
	</CollapsibleCard>

	<!-- LLDP Neighbor Info Section -->
	<CollapsibleCard title={hosts_ifEntries_lldpNeighbor()} bind:expanded={lldpExpanded}>
		<InfoRow label={hosts_ifEntries_chassisId()} mono
			>{ifEntry.lldp_chassis_id?.value || '-'}</InfoRow
		>
		<InfoRow label={hosts_ifEntries_portId()} mono>{ifEntry.lldp_port_id?.value || '-'}</InfoRow>
		<InfoRow label={hosts_ifEntries_remoteSystemName()}>{ifEntry.lldp_sys_name || '-'}</InfoRow>
		<InfoRow label={hosts_ifEntries_remotePort()}>{ifEntry.lldp_port_desc || '-'}</InfoRow>
		<InfoRow label={hosts_ifEntries_managementAddress()} mono
			>{ifEntry.lldp_mgmt_addr || '-'}</InfoRow
		>
		<InfoRow label={hosts_ifEntries_lldpSysDescr()}>{ifEntry.lldp_sys_desc || '-'}</InfoRow>
	</CollapsibleCard>
</div>
