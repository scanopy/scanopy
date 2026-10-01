<script lang="ts">
	import { SlidersHorizontal } from 'lucide-svelte';
	import type { components } from '$lib/api/schema';
	import type { Network } from '$lib/features/networks/types';
	import type { Host, IPAddress } from '$lib/features/hosts/types/base';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import {
		claimedIntegrationsForHost,
		daemonHostBlockReason
	} from '$lib/features/credentials/utils/daemonHostBlocking';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { NetworkDisplay } from '$lib/shared/components/forms/selection/display/NetworkDisplay.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import {
		IPAddressDisplay,
		type IPAddressDisplayContext
	} from '$lib/shared/components/forms/selection/display/IPAddressDisplay.svelte';
	import {
		common_hosts,
		common_networks,
		credentials_assignNetworkEmpty,
		credentials_assignNetworkPlaceholder,
		credentials_assignHostEmpty,
		credentials_assignHostPlaceholder,
		credentials_assignDaemonHostLabel,
		credentials_assignDaemonHostEmpty,
		credentials_ipTargetAllDefault,
		credentials_ipTargetLabel,
		credentials_ipTargetPlaceholder
	} from '$lib/paraglide/messages';

	type CredentialHostAssignment = components['schemas']['CredentialHostAssignment'];

	interface Props {
		credentialTypeId: string;
		credentialId?: string;
		assignedNetworkIds: string[];
		hostAssignments: CredentialHostAssignment[];
	}

	let {
		credentialTypeId,
		credentialId,
		assignedNetworkIds = $bindable([]),
		hostAssignments = $bindable([])
	}: Props = $props();

	let targets = $derived(credentialTypes.getMetadata(credentialTypeId)?.targets ?? []);
	let supportsBroadcast = $derived(targets.includes('Network'));
	let supportsPerHost = $derived(targets.includes('Hosts'));
	// Daemon-host-only types (e.g. a Docker/Podman socket) are assigned to a daemon's
	// own host. Show a host picker filtered to daemon hosts; proxies (which also support
	// 'Hosts') use the regular host surface, where daemon hosts are already selectable.
	let supportsDaemonHostOnly = $derived(targets.includes('DaemonHost') && !supportsPerHost);

	const networksQuery = useNetworksQuery();
	// Still the full nested query, deliberately. The per-host IP-scoping rows
	// (`hostIpAddresses`, `getScopedInterfaces`) read the ip-addresses cache, which
	// has no fetcher of its own and is populated as a side effect of *this* query.
	// A credential can be assigned to hosts on any network, and
	// `GET /api/v1/ip-addresses` takes only a single `host_id` — no `ids` — so
	// there is no bounded direct replacement today.
	//
	// Fixing that properly means the deferred child-cache re-architecture plus an
	// `ids` param on that endpoint (see
	// planned-work/child-cache-rearchitecture.md). This surface is lazy — it only
	// mounts inside the credential modal's assignments tab — so it no longer costs
	// anything on page load.
	const hostsQuery = useHostsQuery({ limit: 0 });
	const ipAddressesQuery = useIPAddressesQuery();
	const subnetsQuery = useSubnetsQuery();
	const daemonsQuery = useDaemonsQuery();
	const credentialsQuery = useCredentialsQuery();

	let allCredentials = $derived(credentialsQuery.data ?? []);

	// Socket↔proxy exclusion: a daemon host whose single-endpoint integration is already claimed
	// by another credential (the other transport) can't take this one. Shared predicate with the
	// host modal and discovery modal via daemonHostBlocking.
	function hostBlockReason(hostId: string): string | null {
		return daemonHostBlockReason(
			credentialTypeId,
			claimedIntegrationsForHost(hostId, allCredentials, credentialId)
		);
	}

	let allNetworks = $derived(networksQuery.data ?? []);
	let allHosts = $derived(hostsQuery.data?.items ?? []);
	let daemonHostIds = $derived((daemonsQuery.data ?? []).map((d) => d.host_id));
	let availableDaemonHosts = $derived(
		allHosts.filter(
			(h) => daemonHostIds.includes(h.id) && !hostAssignments.some((a) => a.host_id === h.id)
		)
	);
	let allIpAddresses = $derived(ipAddressesQuery.data ?? []);
	let subnets = $derived(subnetsQuery.data ?? []);

	// --- Networks (Broadcast) ---
	let selectedNetworks = $derived(
		assignedNetworkIds
			.map((id) => allNetworks.find((n) => n.id === id))
			.filter((n): n is Network => n != null)
	);

	function addNetwork(id: string) {
		if (!assignedNetworkIds.includes(id)) {
			assignedNetworkIds = [...assignedNetworkIds, id];
		}
	}

	function removeNetwork(index: number) {
		const target = selectedNetworks[index];
		if (target) assignedNetworkIds = assignedNetworkIds.filter((id) => id !== target.id);
	}

	// --- Hosts (PerHost), with per-host IP scoping via row expansion ---
	let selectedHosts = $derived(
		hostAssignments
			.map((a) => allHosts.find((h) => h.id === a.host_id))
			.filter((h): h is Host => h != null)
	);

	let availableHosts = $derived(
		allHosts.filter((h) => !hostAssignments.some((a) => a.host_id === h.id))
	);

	// Which host row is expanded to show its IP-address scope (by host id)
	let expandedHostId = $state<string | null>(null);

	function toggleExpand(hostId: string) {
		expandedHostId = expandedHostId === hostId ? null : hostId;
	}

	function addHost(id: string) {
		if (!hostAssignments.some((a) => a.host_id === id)) {
			hostAssignments = [...hostAssignments, { host_id: id, ip_address_ids: null }];
		}
	}

	function removeHost(index: number) {
		const target = selectedHosts[index];
		if (target) hostAssignments = hostAssignments.filter((a) => a.host_id !== target.id);
	}

	function hostIpAddresses(hostId: string): IPAddress[] {
		return allIpAddresses.filter((ip) => ip.host_id === hostId);
	}

	function getInterfaceContext(): IPAddressDisplayContext {
		return { subnets };
	}

	// Scoped IP addresses for a host assignment (null = all)
	function getScopedInterfaces(hostId: string): IPAddress[] {
		const assignment = hostAssignments.find((a) => a.host_id === hostId);
		if (!assignment || assignment.ip_address_ids === null) return [];
		return assignment.ip_address_ids
			.map((id) => allIpAddresses.find((ip) => ip.id === id))
			.filter((ip): ip is IPAddress => ip != null);
	}

	function addInterfaceToScope(hostId: string, interfaceId: string) {
		hostAssignments = hostAssignments.map((a) => {
			if (a.host_id !== hostId) return a;
			const current = a.ip_address_ids;
			if (current === null) return { ...a, ip_address_ids: [interfaceId] };
			if (current.includes(interfaceId)) return a;
			return { ...a, ip_address_ids: [...current, interfaceId] };
		});
	}

	function removeInterfaceFromScope(hostId: string, interfaceIndex: number) {
		hostAssignments = hostAssignments.map((a) => {
			if (a.host_id !== hostId || a.ip_address_ids === null) return a;
			const next = a.ip_address_ids.filter((_, i) => i !== interfaceIndex);
			// Empty list reverts to "all interfaces" (null)
			return { ...a, ip_address_ids: next.length === 0 ? null : next };
		});
	}
</script>

{#snippet networksSurface()}
	<div class="min-w-0 flex-1">
		<ListManager
			label={`${common_networks()} (${assignedNetworkIds.length})`}
			placeholder={credentials_assignNetworkPlaceholder()}
			emptyMessage={credentials_assignNetworkEmpty()}
			allowReorder={false}
			options={allNetworks}
			items={selectedNetworks}
			optionDisplayComponent={NetworkDisplay}
			itemDisplayComponent={NetworkDisplay}
			onAdd={addNetwork}
			onRemove={removeNetwork}
		/>
	</div>
{/snippet}

{#snippet hostsSurface()}
	<div class="min-w-0 flex-1">
		<ListManager
			label={`${common_hosts()} (${hostAssignments.length})`}
			placeholder={credentials_assignHostPlaceholder()}
			emptyMessage={credentials_assignHostEmpty()}
			allowReorder={false}
			options={availableHosts}
			showSearch={true}
			getOptionContext={(h) => ({ disabledReason: hostBlockReason(h.id) })}
			items={selectedHosts}
			optionDisplayComponent={HostDisplay}
			itemDisplayComponent={HostDisplay}
			itemClickAction="edit"
			editIcon={() => SlidersHorizontal}
			isItemEditing={(host) => host.id === expandedHostId}
			onEdit={(host) => toggleExpand(host.id)}
			onAdd={addHost}
			onRemove={removeHost}
		>
			{#snippet itemExpandedSnippet({ item })}
				{#if item.id === expandedHostId}
					{@const hostIps = hostIpAddresses(item.id)}
					<div
						role="presentation"
						onclick={(e) => e.stopPropagation()}
						onkeydown={(e) => e.stopPropagation()}
						class="mt-2 w-full border-t border-gray-200 pt-3 dark:border-gray-700"
					>
						<ListManager
							label={credentials_ipTargetLabel()}
							emptyMessage={credentials_ipTargetAllDefault()}
							placeholder={credentials_ipTargetPlaceholder()}
							allowReorder={false}
							options={hostIps}
							items={getScopedInterfaces(item.id)}
							optionDisplayComponent={IPAddressDisplay}
							itemDisplayComponent={IPAddressDisplay}
							getOptionContext={() => getInterfaceContext()}
							getItemContext={() => getInterfaceContext()}
							onAdd={(id) => addInterfaceToScope(item.id, id)}
							onRemove={(i) => removeInterfaceFromScope(item.id, i)}
						/>
					</div>
				{/if}
			{/snippet}
		</ListManager>
	</div>
{/snippet}

{#snippet daemonHostsSurface()}
	<div class="min-w-0 flex-1">
		<ListManager
			label={`${credentials_assignDaemonHostLabel()} (${hostAssignments.length})`}
			placeholder={credentials_assignHostPlaceholder()}
			emptyMessage={credentials_assignDaemonHostEmpty()}
			allowReorder={false}
			options={availableDaemonHosts}
			showSearch={true}
			getOptionContext={(h) => ({ disabledReason: hostBlockReason(h.id) })}
			items={selectedHosts}
			optionDisplayComponent={HostDisplay}
			itemDisplayComponent={HostDisplay}
			onAdd={addHost}
			onRemove={removeHost}
		/>
	</div>
{/snippet}

<div class="flex min-h-[18rem] flex-1 gap-6">
	{#if supportsBroadcast}
		{@render networksSurface()}
	{/if}
	{#if supportsPerHost}
		{@render hostsSurface()}
	{/if}
	{#if supportsDaemonHostOnly}
		{@render daemonHostsSurface()}
	{/if}
</div>
