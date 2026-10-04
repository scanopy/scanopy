<script lang="ts">
	import { credentialItems } from '$lib/features/credentials/columns';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import type { Network } from '../types';
	import NetworkEditModal from './NetworkEditModal.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import { tagNames } from '$lib/features/tags/columns';
	import { entityRef, type CardAction } from '$lib/shared/components/data/types';
	import { entities } from '$lib/shared/stores/metadata';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import type { Credential } from '$lib/features/credentials/types/base';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import { Plus, Trash2, Edit } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { permissions } from '$lib/shared/stores/metadata';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import type { TabProps } from '$lib/shared/types';
	import {
		common_confirmBulkDelete,
		common_create,
		common_created,
		common_credentials,
		common_daemons,
		common_delete,
		common_edit,
		common_subnets,
		common_vlans,
		common_name,
		common_networks,
		common_noEntityYet,
		common_tags,
		common_hoursCount,
		common_updated,
		networks_confirmDelete,
		networks_staleAfter,
		networks_staleAfterDefault
	} from '$lib/paraglide/messages';

	let { isReadOnly = false }: TabProps = $props();
	import {
		useNetworksQuery,
		useCreateNetworkMutation,
		useUpdateNetworkMutation,
		useDeleteNetworkMutation,
		useBulkDeleteNetworksMutation
	} from '../queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { isUserManagedSubnet, useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useVlansQuery } from '$lib/features/vlans/queries';
	import type { Vlan } from '$lib/features/vlans/types/base';
	import { useDependenciesQuery } from '$lib/features/dependencies/queries';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';

	// Queries
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	let networkLimit = $derived(org?.plan?.included_networks ?? null);
	let canBuyMore = $derived(
		org?.plan?.network_cents !== undefined && org?.plan?.network_cents !== null
	);

	const tagsQuery = useTagsQuery();
	const networksQuery = useNetworksQuery();
	// What each network contains, resolved here so card and table share it.
	const daemonsQuery = useDaemonsQuery();
	const subnetsQuery = useSubnetsQuery();
	const vlansQuery = useVlansQuery();
	const credentialsQuery = useCredentialsQuery();
	useDependenciesQuery();

	// Only the hosts the daemons run on. Each card needs one host name per daemon
	// chip; fetching per card meant every card subscribing to an unpaginated
	// org-wide hosts query (~1.9MB), shared by key with every other consumer.
	let daemonHostIds = $derived([
		...new Set((daemonsQuery.data ?? []).map((d) => d.host_id).filter((id): id is string => !!id))
	]);
	const daemonHostsQuery = useHostsByIds(() => daemonHostIds);
	let daemonHosts = $derived(daemonHostsQuery.data ?? []);

	// Mutations
	const createNetworkMutation = useCreateNetworkMutation();
	const updateNetworkMutation = useUpdateNetworkMutation();
	const deleteNetworkMutation = useDeleteNetworkMutation();
	const bulkDeleteNetworksMutation = useBulkDeleteNetworksMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let daemonsData = $derived(daemonsQuery.data ?? []);
	// Narrowed once, here: both the per-network subnet column and the daemon chips
	// below must show the same list the Subnets and Daemons tabs do.
	let subnetsData = $derived((subnetsQuery.data ?? []).filter(isUserManagedSubnet));
	let vlansData = $derived(vlansQuery.data ?? []);
	let credentialsData = $derived(credentialsQuery.data ?? []);
	let isLoading = $derived(networksQuery.isPending);
	let isAtNetworkLimit = $derived(
		networkLimit !== null && networksData.length >= networkLimit && !canBuyMore
	);
	let isNearNetworkLimit = $derived(
		networkLimit !== null &&
			networksData.length >= networkLimit - 2 &&
			!isAtNetworkLimit &&
			!canBuyMore
	);

	let showCreateNetworkModal = $state(false);
	let editingNetwork = $state<Network | null>(null);

	// Deep-link: open network editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'network-editor',
			networksData,
			showCreateNetworkModal,
			editingNetwork?.id
		);
		if (result !== undefined) {
			editingNetwork = result;
			showCreateNetworkModal = true;
		}
	});

	let allowBulkDelete = $derived(
		!isReadOnly && currentUser
			? permissions.getMetadata(currentUser.permissions).manage_org_entities
			: false
	);

	let canManageNetworks = $derived(
		!isReadOnly &&
			currentUser &&
			permissions.getMetadata(currentUser.permissions).manage_org_entities
	);

	/** Row actions for table mode, matching what the card offers. */
	function networkActions(network: Network): CardAction[] {
		if (!allowBulkDelete) return [];

		return [
			{ label: common_edit(), icon: Edit, onClick: () => handleEditNetwork(network) },
			{
				label: common_delete(),
				icon: Trash2,
				class: 'btn-icon-danger',
				onClick: () => handleDeleteNetwork(network)
			}
		];
	}

	function handleDeleteNetwork(network: Network) {
		if (confirm(networks_confirmDelete({ name: network.name }))) {
			deleteNetworkMutation.mutate(network.id);
		}
	}

	function handleCreateNetwork() {
		editingNetwork = null;
		showCreateNetworkModal = true;
	}

	function handleEditNetwork(network: Network) {
		editingNetwork = network;
		showCreateNetworkModal = true;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_networks() }))) {
			await bulkDeleteNetworksMutation.mutateAsync(ids);
		}
	}

	function getNetworkTags(network: Network): string[] {
		return network.tags;
	}

	async function handleNetworkCreate(data: Network) {
		try {
			await createNetworkMutation.mutateAsync(data);
			showCreateNetworkModal = false;
			editingNetwork = null;
		} catch {
			// Error handled by mutation
		}
	}

	async function handleNetworkUpdate(id: string, data: Network) {
		try {
			await updateNetworkMutation.mutateAsync(data);
			showCreateNetworkModal = false;
			editingNetwork = null;
		} catch {
			// Error handled by mutation
		}
	}

	function handleCloseNetworkEditor() {
		showCreateNetworkModal = false;
		editingNetwork = null;
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Network', {});
	}

	// What a network contains. These were computed inside the card, so the table
	// had no way to show them; resolving here is what gives both views the same
	// columns.
	function networkDaemons(network: Network): Daemon[] {
		return daemonsData.filter((daemon) => daemon.network_id === network.id);
	}

	function networkSubnets(network: Network): Subnet[] {
		return subnetsData.filter((subnet) => subnet.network_id === network.id);
	}

	function networkVlans(network: Network): Vlan[] {
		return vlansData.filter((vlan) => vlan.network_id === network.id);
	}

	function networkCredentials(network: Network): Credential[] {
		return (network.credential_ids ?? [])
			.map((id) => credentialsData.find((c) => c.id === id))
			.filter((c): c is Credential => Boolean(c));
	}

	// Derived, not a plain const: it closes over `tagsData` and references the
	// `tagsCell` snippet, neither of which exists yet when the script body runs.
	let networkFields = $derived<FieldConfig<Network>[]>([
		{
			key: 'name',
			label: common_name(),
			type: 'string',
			searchable: true,
			sortable: true,
			display: { primary: true, width: 220, order: 0 }
		},
		{
			key: 'vlans',
			label: common_vlans(),
			type: 'array',
			searchable: true,
			getValue: (network) => networkVlans(network).map((v) => v.name),
			display: {
				order: 1,
				getItems: (network) =>
					networkVlans(network).map((vlan) => ({
						id: vlan.id,
						label: vlan.name,
						color: entities.getColorHelper('Vlan').color,
						entityRef: entityRef('Vlan', vlan.id, vlan)
					}))
			}
		},
		{
			key: 'tags',
			label: common_tags(),
			type: 'array',
			searchable: true,
			filterable: true,
			getValue: (entity) => tagNames(entity.tags, tagsData)
		},
		{
			key: 'daemons',
			label: common_daemons(),
			type: 'array',
			searchable: true,
			getValue: (network) => networkDaemons(network).map((d) => d.name),
			display: {
				order: 3,
				getItems: (network) =>
					networkDaemons(network).map((daemon) => ({
						id: daemon.id,
						label: daemon.name,
						color: entities.getColorHelper('Daemon').color,
						entityRef: entityRef('Daemon', daemon.id, daemon, {
							hosts: daemonHosts,
							subnets: subnetsData
						})
					}))
			}
		},
		{
			key: 'credentials',
			label: common_credentials(),
			type: 'array',
			searchable: true,
			// Credentials are shared across networks, so this filters. The other arrays here hold
			// members of one network each, which search already finds.
			filterable: true,
			getValue: (network) => networkCredentials(network).map((c) => c.name),
			display: {
				order: 4,
				getItems: (network) => credentialItems(networkCredentials(network))
			}
		},
		{
			key: 'subnets',
			label: common_subnets(),
			type: 'array',
			searchable: true,
			getValue: (network) => networkSubnets(network).map((s) => s.name),
			display: {
				order: 2,
				getItems: (network) =>
					networkSubnets(network).map((subnet) => ({
						id: subnet.id,
						label: subnet.name,
						color: entities.getColorHelper('Subnet').color,
						entityRef: entityRef('Subnet', subnet.id, subnet)
					}))
			}
		},
		{
			key: 'created_at',
			label: common_created(),
			type: 'date',
			sortable: true,
			display: { hiddenByDefault: true }
		},
		{
			key: 'updated_at',
			label: common_updated(),
			type: 'date',
			sortable: true,
			display: { hiddenByDefault: true }
		},
		{
			// The effective window, with the server default applied, so a network that
			// never set its own still shows the number staleness is judged by.
			key: 'effective_stale_after_hours',
			label: networks_staleAfter(),
			type: 'string',
			// A handful of values across networks; numeric collation orders 24 before 168.
			sortable: true,
			groupable: true,
			filterable: true,
			// Marked when no override is set, so an override equal to the default still reads as one.
			getValue: (network) => {
				if (network.effective_stale_after_hours == null) return null;
				const hours = common_hoursCount({ hours: network.effective_stale_after_hours });
				return network.stale_after_hours == null ? networks_staleAfterDefault({ hours }) : hours;
			},
			display: { hiddenByDefault: true }
		}
	]);
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_networks()}>
		<svelte:fragment slot="actions">
			<div class="flex items-center gap-3">
				{#if networkLimit !== null && !canBuyMore}
					<span
						class="text-sm {isAtNetworkLimit
							? 'text-amber-400'
							: isNearNetworkLimit
								? 'text-yellow-400'
								: 'text-tertiary'}"
					>
						{networksData.length} / {networkLimit}
					</span>
				{/if}
				{#if canManageNetworks}
					{#if isAtNetworkLimit}
						<UpgradeButton feature="networks" surface="networks_tab" gate_type="limit_hit" />
					{:else}
						{#if isNearNetworkLimit}
							<UpgradeButton feature="networks" surface="networks_tab" gate_type="limit_hit" />
						{/if}
						<button class="btn-primary flex items-center" onclick={handleCreateNetwork}
							><Plus class="h-5 w-5" />{common_create()}</button
						>
					{/if}
				{/if}
			</div>
		</svelte:fragment>
	</TabHeader>

	<!-- Loading state -->
	{#if isLoading}
		<Loading />
	{:else if networksData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={common_noEntityYet({ entity: common_networks() })}
			subtitle=""
			onClick={handleCreateNetwork}
			cta={common_create()}
		/>
	{:else}
		<DataControls
			items={networksData}
			fields={networkFields}
			onBulkDelete={handleBulkDelete}
			entityType={allowBulkDelete ? 'Network' : undefined}
			getItemTags={getNetworkTags}
			{allowBulkDelete}
			storageKey="scanopy-networks-table-state"
			getItemId={(item) => item.id}
			getIcon={() => ({
				icon: entities.getIconComponent('Network'),
				color: entities.getColorHelper('Network').icon
			})}
			onCsvExport={handleCsvExport}
			getActions={networkActions}
			entityLabel={common_networks()}
		></DataControls>
	{/if}
</div>

<NetworkEditModal
	name="network-editor"
	isOpen={showCreateNetworkModal}
	network={editingNetwork}
	onCreate={handleNetworkCreate}
	onUpdate={handleNetworkUpdate}
	onClose={handleCloseNetworkEditor}
	onDelete={editingNetwork
		? () => {
				handleDeleteNetwork(editingNetwork!);
				handleCloseNetworkEditor();
			}
		: null}
/>
