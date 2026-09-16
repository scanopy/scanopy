<script lang="ts">
	import { useQueryClient } from '@tanstack/svelte-query';
	import { queryKeys } from '$lib/api/query-client';
	import type { Interface, IPAddress } from '$lib/features/hosts/types/base';
	import { getHostByIdFromCache } from '$lib/features/hosts/queries';
	import { getSubnetByIdFromCache } from '$lib/features/subnets/queries';
	import { useTopologyDataQuery } from '$lib/features/topology/queries';
	import ConfigHeader from '$lib/shared/components/forms/config/ConfigHeader.svelte';
	import CollapsibleCard from '$lib/shared/components/data/CollapsibleCard.svelte';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import InterfaceDetailsCard from '$lib/features/hosts/components/InterfaceDetailsCard.svelte';
	import { interfaceDisplayName } from '$lib/features/hosts/interface-display-name';
	import {
		hosts_interfaces_cdpNeighbor,
		hosts_interfaces_cdpNeighborNumbered,
		hosts_interfaces_index,
		hosts_interfaces_lldpNeighbor,
		hosts_interfaces_lldpNeighborNumbered,
		hosts_interfaces_managementAddress,
		hosts_interfaces_portId,
		hosts_interfaces_remoteAddress,
		hosts_interfaces_remoteDevice,
		hosts_interfaces_remotePlatform,
		hosts_interfaces_remotePort,
		hosts_interfaces_remoteSystemName,
		hosts_snmp_chassisId,
		hosts_snmp_sysDescr
	} from '$lib/paraglide/messages';

	interface Props {
		iface: Interface;
	}

	let { iface }: Props = $props();

	const queryClient = useQueryClient();

	// Linked IPAddress + Subnet resolution
	let linkedIpAddress = $derived.by(() => {
		if (!iface.ip_address_id) return null;
		const allIpAddresses = queryClient.getQueryData<IPAddress[]>(queryKeys.ipAddresses.all) ?? [];
		return allIpAddresses.find((i) => i.id === iface.ip_address_id) ?? null;
	});

	let linkedSubnet = $derived.by(() => {
		if (!linkedIpAddress) return null;
		return getSubnetByIdFromCache(queryClient, linkedIpAddress.subnet_id);
	});

	// GH #701: neighbour resolution and raw LLDP/CDP evidence both moved off `Interface` onto the
	// topology bundle (`neighbours`/`candidates` — see `InterfaceNeighborRow`/
	// `InterfaceNeighborCandidate`). The plain host/interface CRUD responses this modal otherwise
	// reads from cache no longer carry either, so this admin/debug panel fetches the interface's
	// network's topology bundle directly. Usually a cache hit: opening this modal from the
	// Topology tab's own host editor means the same query is already populated.
	const topologyDataQuery = useTopologyDataQuery(
		() => iface.network_id,
		() => undefined
	);

	let candidates = $derived(
		(topologyDataQuery.data?.candidates ?? []).filter((c) => c.base.interface_id === iface.id)
	);

	// Neighbor resolution — a port can resolve to several adjacencies now, so this is a list rather
	// than a single optional pair.
	let neighbours = $derived.by(() => {
		const rows = (topologyDataQuery.data?.neighbours ?? []).filter(
			(n) => n.interface_id === iface.id
		);
		const allInterfaces = queryClient.getQueryData<Interface[]>(queryKeys.interfaces.all) ?? [];
		return rows.map((row) => {
			if (row.neighbor.type === 'Host') {
				return {
					id: row.id,
					neighborHost: getHostByIdFromCache(queryClient, row.neighbor.id),
					neighborInterface: null
				};
			}
			const remoteEntry = allInterfaces.find((e) => e.id === row.neighbor.id) ?? null;
			return {
				id: row.id,
				neighborInterface: remoteEntry,
				neighborHost: remoteEntry ? getHostByIdFromCache(queryClient, remoteEntry.host_id) : null
			};
		});
	});
</script>

<div class="space-y-6">
	<ConfigHeader
		title={interfaceDisplayName(iface)}
		subtitle={iface.if_index == null ? null : hosts_interfaces_index({ index: iface.if_index })}
	/>

	<InterfaceDetailsCard {iface} {linkedIpAddress} {linkedSubnet} {neighbours} />

	<!-- Raw LLDP/CDP evidence: one candidate per distinct record heard on this port (GH #701),
	     including still-unresolved ones. Each gets its own CDP + LLDP pair of sections, numbered
	     once there is more than one to tell apart. -->
	{#each candidates as candidate, i (candidate.id)}
		{@const evidence = candidate.base.evidence}
		{@const number = i + 1}
		<CollapsibleCard
			title={candidates.length > 1
				? hosts_interfaces_cdpNeighborNumbered({ number })
				: hosts_interfaces_cdpNeighbor()}
			expanded={false}
		>
			<InfoRow label={hosts_interfaces_remoteDevice()}>{evidence.cdp_device_id || '-'}</InfoRow>
			<InfoRow label={hosts_interfaces_remotePort()}>{evidence.cdp_port_id || '-'}</InfoRow>
			<InfoRow label={hosts_interfaces_remoteAddress()} mono>{evidence.cdp_address || '-'}</InfoRow>
			<InfoRow label={hosts_interfaces_remotePlatform()}>{evidence.cdp_platform || '-'}</InfoRow>
		</CollapsibleCard>

		<CollapsibleCard
			title={candidates.length > 1
				? hosts_interfaces_lldpNeighborNumbered({ number })
				: hosts_interfaces_lldpNeighbor()}
			expanded={false}
		>
			<InfoRow label={hosts_snmp_chassisId()} mono>{evidence.lldp_chassis_id?.value || '-'}</InfoRow
			>
			<InfoRow label={hosts_interfaces_portId()} mono>{evidence.lldp_port_id?.value || '-'}</InfoRow
			>
			<InfoRow label={hosts_interfaces_remoteSystemName()}>{evidence.lldp_sys_name || '-'}</InfoRow>
			<InfoRow label={hosts_interfaces_remotePort()}>{evidence.lldp_port_desc || '-'}</InfoRow>
			<InfoRow label={hosts_interfaces_managementAddress()} mono
				>{evidence.lldp_mgmt_addr || '-'}</InfoRow
			>
			<InfoRow label={hosts_snmp_sysDescr()}>{evidence.lldp_sys_desc || '-'}</InfoRow>
		</CollapsibleCard>
	{:else}
		<CollapsibleCard title={hosts_interfaces_cdpNeighbor()} expanded={false}>
			<InfoRow label={hosts_interfaces_remoteDevice()}>-</InfoRow>
			<InfoRow label={hosts_interfaces_remotePort()}>-</InfoRow>
			<InfoRow label={hosts_interfaces_remoteAddress()} mono>-</InfoRow>
			<InfoRow label={hosts_interfaces_remotePlatform()}>-</InfoRow>
		</CollapsibleCard>

		<CollapsibleCard title={hosts_interfaces_lldpNeighbor()} expanded={false}>
			<InfoRow label={hosts_snmp_chassisId()} mono>-</InfoRow>
			<InfoRow label={hosts_interfaces_portId()} mono>-</InfoRow>
			<InfoRow label={hosts_interfaces_remoteSystemName()}>-</InfoRow>
			<InfoRow label={hosts_interfaces_remotePort()}>-</InfoRow>
			<InfoRow label={hosts_interfaces_managementAddress()} mono>-</InfoRow>
			<InfoRow label={hosts_snmp_sysDescr()}>-</InfoRow>
		</CollapsibleCard>
	{/each}
</div>
