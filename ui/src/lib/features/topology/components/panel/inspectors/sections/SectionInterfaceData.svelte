<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import type { RenderableTopology, TopologyNode } from '$lib/features/topology/types/base';
	import type { ElementRenderContext } from '$lib/features/topology/resolvers';
	import InterfaceDetailsCard from '$lib/features/hosts/components/InterfaceDetailsCard.svelte';

	let {
		node,
		topology,
		elementContext
	}: {
		node: Node;
		topology: RenderableTopology;
		elementContext?: ElementRenderContext;
	} = $props();

	// Find the SNMP Interface: from node data or element context
	let iface = $derived.by(() => {
		const nodeData = node.data as TopologyNode;
		const interfaceId = 'interface_id' in nodeData ? (nodeData.interface_id as string) : undefined;
		if (interfaceId) {
			return topology.interfaces.find((e) => e.id === interfaceId) ?? null;
		}
		if (!elementContext?.interfaceId) return null;
		return topology.interfaces.find((e) => e.id === elementContext.interfaceId) ?? null;
	});

	// Resolve linked IPAddress from the SNMP interface's ip_address_id FK
	let linkedIpAddress = $derived.by(() => {
		if (!iface?.ip_address_id) return null;
		return topology.ip_addresses.find((i) => i.id === iface!.ip_address_id) ?? null;
	});

	let linkedSubnet = $derived.by(() => {
		if (!linkedIpAddress) return null;
		return topology.subnets.find((s) => s.id === linkedIpAddress!.subnet_id) ?? null;
	});

	// GH #701: `iface.neighbor` was a single value; a port's resolved adjacencies are now a `Vec`
	// on the topology bundle (`neighbours`), filtered here by this interface's id.
	let neighbours = $derived.by(() => {
		if (!iface) return [];
		const rows = (topology.neighbours ?? []).filter((n) => n.interface_id === iface!.id);
		return rows.map((row) => {
			if (row.neighbor.type === 'Host') {
				return {
					id: row.id,
					neighborHost: topology.hosts.find((h) => h.id === row.neighbor.id) ?? null,
					neighborInterface: null
				};
			}
			const remoteEntry = topology.interfaces.find((e) => e.id === row.neighbor.id) ?? null;
			return {
				id: row.id,
				neighborInterface: remoteEntry,
				neighborHost: remoteEntry
					? (topology.hosts.find((h) => h.id === remoteEntry.host_id) ?? null)
					: null
			};
		});
	});

	type VlanShape = { id: string; vlan_number: number; name: string };

	let nativeVlan = $derived.by(() => {
		if (!iface?.native_vlan_id) return null;
		return (topology.vlans as VlanShape[]).find((v) => v.id === iface!.native_vlan_id) ?? null;
	});

	let taggedVlans = $derived.by(() => {
		if (!iface?.vlan_ids?.length) return [];
		const vlans = topology.vlans as VlanShape[];
		return iface!
			.vlan_ids!.map((id) => vlans.find((v) => v.id === id))
			.filter(Boolean) as VlanShape[];
	});
</script>

{#if iface}
	<InterfaceDetailsCard
		{iface}
		{linkedIpAddress}
		{linkedSubnet}
		{neighbours}
		{nativeVlan}
		{taggedVlans}
	/>
{/if}
