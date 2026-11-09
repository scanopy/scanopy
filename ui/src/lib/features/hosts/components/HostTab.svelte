<script lang="ts">
	import { rowStatusBadges } from '$lib/shared/utils/row-badges';
	import CreateButton from '$lib/shared/components/data/controls/CreateButton.svelte';
	import type {
		Host,
		CreateHostWithServicesRequest,
		UpdateHostWithServicesRequest
	} from '../types/base';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import OsTag from './OsTag.svelte';
	import HostEditor from './HostEditModal/HostEditor.svelte';
	import HostMergeModal from './HostMergeModal.svelte';
	import UnconfirmedHostModal from './UnconfirmedHostModal.svelte';
	import { unconfirmedEvidence } from '../unconfirmed-evidence';
	import { isUnconfirmed } from '$lib/shared/utils/confirmation';
	import HostExportModal from './HostExportModal.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import { disabledWhen, hiddenWhen, type RowAction } from '$lib/shared/components/data/types';
	import { hostColumns } from './HostTab.columns';
	import { collectionLookup } from '$lib/shared/entity-fields';
	import { Trash2, RefreshCw, Replace, Edit, CircleDashedCheck } from '@lucide/svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import type { TabProps } from '$lib/shared/types';
	import {
		common_confirmDeleteName,
		common_confirmDeleteNameWithGuests,
		common_merge,
		common_create,
		common_delete,
		common_edit,
		common_hosts,
		common_confirmBulkDelete,
		common_confirmBulkDeleteWithGuests,
		common_noEntityYet,
		common_rescan,
		daemons_installPromptHosts,
		hosts_alreadyConfirmed,
		hosts_resolveFirst,
		hosts_resolveFirstBulk,
		hosts_rescanNoAddresses,
		hosts_rescanNoAddressesBulk,
		hosts_resolveUnconfirmed
	} from '$lib/paraglide/messages';

	import { refusedUnless, useCan } from '$lib/features/auth/access';

	let { isActive = false }: TabProps = $props();
	const can = useCan();
	import { missingRootIds } from '../virtualization-tree';
	import {
		useHostsQuery,
		useHostsByIds,
		useCreateHostMutation,
		useUpdateHostMutation,
		useDeleteHostMutation,
		useBulkDeleteHostsMutation,
		useMergeHostsMutation,
		useBulkRescanHostsMutation,
		useRescanHostMutation,
		useSettleUnconfirmedHostMutation,
		type HostQueryOptions
	} from '../queries';
	import { useServicesByIds, useServicesCacheQuery } from '$lib/features/services/queries';
	import { virtualizedSummaryOnHosts } from '$lib/features/services/virtualized-count';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
	import { usePortsQuery } from '$lib/features/ports/queries';
	import { useInterfacesByIds, useInterfacesQuery } from '$lib/features/interfaces/queries';
	import { useDiscoveriesByIds } from '$lib/features/discovery/queries';
	import { discoveryRunIds } from '$lib/features/discovery/columns';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import type { components } from '$lib/api/schema';
	import { useFieldValuesQuery } from '$lib/shared/api/field-values';

	type HostOrderField = components['schemas']['HostOrderField'];

	const HOST_FIELD_VALUES = '/api/v1/hosts/field-values/{field}';
	const SERVICE_FIELD_VALUES = '/api/v1/services/field-values/{field}';

	// Pagination state
	let pageSize = $state(20);
	let currentPage = $state(1);

	// The grouping and sort the table opens with until the user picks their own.
	const tableDefaults: TableDefaults<HostOrderField> = {
		group: 'virtualized_by',
		sort: { field: 'name', direction: 'asc' }
	};

	// The list's view as the endpoint's params: search, tags, staleness, field filters and ordering,
	// all server-side because the client holds one page of hosts, so filtering here would narrow
	// that page while the total count kept describing every match. Null until DataControls has
	// restored the view. The query is not held for it: the table only mounts once a first page
	// has loaded.
	let listQuery = $state<URLSearchParams | null>(null);

	// Queries
	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	let hostLimit = $derived(org?.plan?.included_hosts ?? null);
	let canBuyMoreHosts = $derived(
		org?.plan?.host_cents !== undefined && org?.plan?.host_cents !== null
	);
	let daemonConnected = $derived(org?.daemon_connected ?? false);

	const tagsQuery = useTagsQuery();
	// Paginated hosts with server-side pagination, ordering, and tag filtering.
	//
	// Deliberately NOT gated on `isActive`, unlike the other tabs' list queries.
	// `useHostsQuery` is the last remaining writer of the ip-addresses / ports /
	// services / interfaces caches (it populates them from its nested response),
	// and those caches have no fetcher of their own. Gating this would leave the
	// services tab's binding chips and the host editor's interface lists empty for
	// anyone who never opens the hosts tab. Un-gate it only once those caches have
	// real queries — see planned-work/child-cache-rearchitecture.md.
	const hostsQuery = useHostsQuery((): HostQueryOptions => ({
		limit: pageSize,
		offset: (currentPage - 1) * pageSize,
		list: listQuery ?? undefined
	}));
	const sitesQuery = useSitesQuery();
	useDaemonsQuery();
	const ipAddressesQuery = useIPAddressesQuery();
	const portsQuery = usePortsQuery();
	const interfacesQuery = useInterfacesQuery();
	const credentialsQuery = useCredentialsQuery();
	const subnetsQuery = useSubnetsQuery();
	// Filter options: the values the caller's hosts actually hold, counted by the server. The
	// loaded page would only offer the values on it, and the fixtures and caches would offer values
	// no host holds. None of these takes the tab's active filters, so the options do not shrink as
	// the user filters.
	const siteValuesQuery = useFieldValuesQuery(
		HOST_FIELD_VALUES,
		'site_id',
		undefined,
		() => isActive
	);
	const virtualizedByValuesQuery = useFieldValuesQuery(
		HOST_FIELD_VALUES,
		'virtualized_by',
		undefined,
		() => isActive
	);
	const sourceValuesQuery = useFieldValuesQuery(
		HOST_FIELD_VALUES,
		'source',
		undefined,
		() => isActive
	);
	const osFamilyValuesQuery = useFieldValuesQuery(
		HOST_FIELD_VALUES,
		'os_family',
		undefined,
		() => isActive
	);
	const manufacturerValuesQuery = useFieldValuesQuery(
		HOST_FIELD_VALUES,
		'manufacturer',
		undefined,
		() => isActive
	);
	const modelValuesQuery = useFieldValuesQuery(
		HOST_FIELD_VALUES,
		'model',
		undefined,
		() => isActive
	);
	const sysLocationValuesQuery = useFieldValuesQuery(
		HOST_FIELD_VALUES,
		'sys_location',
		undefined,
		() => isActive
	);
	// Every service belongs to a host, so the services' names are the names some host runs.
	const serviceNameValuesQuery = useFieldValuesQuery(
		SERVICE_FIELD_VALUES,
		'name',
		undefined,
		() => isActive
	);

	// Selective service lookup - only fetches services needed for virtualization display
	// Extract service IDs from visible hosts for "Virtualized By" field
	const servicesQuery = useServicesByIds(() => {
		return (hostsQuery.data?.items ?? [])
			.map((h) => h.virtualization_service_id)
			.filter((id): id is string => id != null)
			.filter((id, idx, arr) => arr.indexOf(id) === idx);
	});
	// The interface a virtualizing host presents each network identity from. It belongs to the
	// virtualizing host, which is rarely on this page, so fetched by id.
	const presentingInterfacesQuery = useInterfacesByIds(() => [
		...new Set(
			(hostsQuery.data?.items ?? [])
				.map((h) => h.virtualization_interface_id)
				.filter((id): id is string => id != null)
		)
	]);
	const discoveryRunsQuery = useDiscoveriesByIds(() =>
		discoveryRunIds(hostsQuery.data?.items ?? [])
	);
	// The hosts that head a virtualization tree on this page but sit on another, so the group
	// header can name them.
	const virtualizationRootsQuery = useHostsByIds(() =>
		missingRootIds(hostsQuery.data?.items ?? [])
	);

	// Mutations
	const createHostMutation = useCreateHostMutation();
	const updateHostMutation = useUpdateHostMutation();
	const deleteHostMutation = useDeleteHostMutation();
	const bulkDeleteHostsMutation = useBulkDeleteHostsMutation();
	const mergeHostsMutation = useMergeHostsMutation();
	const rescanHostMutation = useRescanHostMutation();
	const bulkRescanHostsMutation = useBulkRescanHostsMutation();
	const settleUnconfirmedMutation = useSettleUnconfirmedHostMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let hostsData = $derived(hostsQuery.data?.items ?? []);
	let hostsPagination = $derived(hostsQuery.data?.pagination ?? null);
	let servicesData = $derived(servicesQuery.data ?? []);
	let presentingInterfacesData = $derived(presentingInterfacesQuery.data ?? []);
	let discoveryRunsData = $derived(discoveryRunsQuery.data ?? []);
	let virtualizationRoots = $derived(
		new Map([...hostsData, ...(virtualizationRootsQuery.data ?? [])].map((h) => [h.id, h]))
	);
	const servicesCacheQuery = useServicesCacheQuery();
	let allServicesData = $derived(servicesCacheQuery.data ?? []);
	let sitesData = $derived(sitesQuery.data ?? []);
	let ipAddressesData = $derived(ipAddressesQuery.data ?? []);
	let portsData = $derived(portsQuery.data ?? []);
	let interfacesData = $derived(interfacesQuery.data ?? []);
	let credentialsData = $derived(credentialsQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	// Only show full loading on initial load (no data yet)
	let isInitialLoading = $derived(hostsQuery.isPending && !hostsQuery.data);
	let pageLoad = $derived(loadFailure(organizationQuery, hostsQuery));

	// Host limit tracking
	let totalHostCount = $derived(hostsPagination?.total_count ?? hostsData.length);
	let isAtHostLimit = $derived(
		hostLimit !== null && totalHostCount >= hostLimit && !canBuyMoreHosts
	);
	let isNearHostLimit = $derived(
		hostLimit !== null &&
			totalHostCount >= hostLimit - 5 &&
			totalHostCount < hostLimit &&
			!canBuyMoreHosts
	);

	// Page change handler for server-side pagination
	function handlePageChange(page: number, newPageSize: number) {
		currentPage = page;
		pageSize = newPageSize;
	}

	// Export modal state
	let showExportModal = $state(false);
	// The list's own query: the export endpoint takes the same params, so an export mirrors what
	// the user was looking at rather than the whole network.
	let exportParams = $derived(listQuery ?? new URLSearchParams());

	let showHostEditor = $state(false);
	// Track the host being edited by id + a snapshot, and resolve `editingHost` from the
	// live query cache. So when an external change (e.g. a credential assignment removed
	// elsewhere) invalidates and refetches hosts, the open editor reflects it without a
	// page reload. The snapshot is the fallback for hosts not in the current page.
	let editingHostId = $state<string | null>(null);
	let editingHostSnapshot = $state<Host | null>(null);
	let editingHost = $derived(
		editingHostId ? (hostsData.find((h) => h.id === editingHostId) ?? editingHostSnapshot) : null
	);
	function setEditingHost(host: Host | null) {
		editingHostId = host?.id ?? null;
		editingHostSnapshot = host;
	}

	let otherHost = $state<Host | null>(null);
	let showHostMergeModal = $state(false);
	let resolvingHost = $state<Host | null>(null);
	let showUnconfirmedHost = $state(false);

	// Deep-link: resolve an unconfirmed host from anywhere that names one, such as a scan warning.
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'unconfirmed-host',
			hostsData,
			showUnconfirmedHost,
			resolvingHost?.id,
			isUnconfirmed
		);
		if (result) {
			resolvingHost = result;
			showUnconfirmedHost = true;
		}
	});

	// Deep-link: open host editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'host-editor',
			hostsData,
			showHostEditor,
			editingHost?.id
		);
		if (result !== undefined) {
			setEditingHost(result);
			showHostEditor = true;
		}
	});

	function hostIPAddresses(host: Host) {
		return ipAddressesData.filter((i) => i.host_id === host.id);
	}

	// Related entities the registry's host fields read, from the collections this page already
	// loads. Keyed by id, since a virtualizing service or presenting interface can sit in both the
	// page's cache and the by-id fetch, and a child lookup would otherwise list it twice.
	let lookup = $derived(
		collectionLookup({
			IPAddress: ipAddressesData,
			Port: portsData,
			Interface: [
				...new Map([...interfacesData, ...presentingInterfacesData].map((i) => [i.id, i])).values()
			],
			Service: [...new Map([...allServicesData, ...servicesData].map((s) => [s.id, s])).values()],
			Site: sitesData
		})
	);

	let hostFields = $derived(
		hostColumns({
			lookup: () => lookup,
			services: () => servicesData,
			allServices: () => allServicesData,
			ipAddresses: () => ipAddressesData,
			ports: () => portsData,
			interfaces: () => interfacesData,
			credentials: () => credentialsData,
			subnets: () => subnetsData,
			sites: () => sitesData,
			tags: () => tagsData,
			discoveryRuns: () => discoveryRunsData,
			virtualizationRoots: () => virtualizationRoots,
			hostIPAddresses,
			siteValues: () => siteValuesQuery.data,
			virtualizedByValues: () => virtualizedByValuesQuery.data,
			sourceValues: () => sourceValuesQuery.data,
			osFamilyValues: () => osFamilyValuesQuery.data,
			manufacturerValues: () => manufacturerValuesQuery.data,
			modelValues: () => modelValuesQuery.data,
			sysLocationValues: () => sysLocationValuesQuery.data,
			serviceNameValues: () => serviceNameValuesQuery.data,
			osCell
		})
	);

	function handleCreateHost() {
		setEditingHost(null);
		showHostEditor = true;
	}

	/** Row actions. */
	function hostActions(host: Host): RowAction[] {
		if (!can('Host', 'Write')) return [];

		const writeRefusal = refusedUnless(can('Host', 'Write', host.site_id));
		// An unconfirmed host is settled first, through Resolve: confirm, merge or dismiss it.
		const unresolved =
			writeRefusal ??
			disabledWhen(isUnconfirmed(host), hosts_resolveFirst(), hosts_resolveFirstBulk());

		return [
			// Only where there is something to resolve, as on the Subnets tab.
			{
				label: hosts_resolveUnconfirmed(),
				icon: CircleDashedCheck,
				onClick: () => handleResolveHost(host),
				unavailable: hiddenWhen(!isUnconfirmed(host), hosts_alreadyConfirmed()) ?? writeRefusal
			},
			{
				label: common_edit(),
				icon: Edit,
				isOpen: true,
				onClick: () => handleEditHost(host),
				unavailable: unresolved
			},
			// A rescan targets the host's known addresses, so a host with none (a device identified
			// purely by a MAC) has nothing to scan, and the endpoint answers 400.
			{
				label: common_rescan(),
				icon: RefreshCw,
				onClick: () => handleRescanHost(host),
				onBulk: handleBulkRescan,
				unavailable:
					unresolved ??
					disabledWhen(
						hostIPAddresses(host).length === 0,
						hosts_rescanNoAddresses(),
						hosts_rescanNoAddressesBulk()
					)
			},
			{
				label: common_merge(),
				icon: Replace,
				onClick: () => handleStartMerge(host),
				unavailable: unresolved
			},
			{
				label: common_delete(),
				icon: Trash2,
				tone: 'danger',
				isDelete: true,
				onBulk: handleBulkDelete,
				onClick: () => handleDeleteHost(host),
				unavailable: refusedUnless(can('Host', 'Delete', host.site_id)) ?? unresolved
			}
		];
	}

	function handleEditHost(host: Host) {
		setEditingHost(host);
		showHostEditor = true;
	}

	function handleResolveHost(host: Host) {
		resolvingHost = host;
		showUnconfirmedHost = true;
	}

	function handleCloseUnconfirmedHost() {
		showUnconfirmedHost = false;
		resolvingHost = null;
	}

	/** Confirm first, so the labels a person types land on a host they have vouched for. */
	async function handleConfirmAndLabel(host: Host) {
		try {
			const confirmed = await settleUnconfirmedMutation.mutateAsync({
				hostId: host.id,
				action: 'confirm'
			});
			handleCloseUnconfirmedHost();
			handleEditHost({ ...host, source: confirmed.source, updated_at: confirmed.updated_at });
		} catch {
			// Error handled by mutation
		}
	}

	function handleMergeUnconfirmed(host: Host) {
		handleCloseUnconfirmedHost();
		handleStartMerge(host);
	}

	async function handleDismissUnconfirmed(host: Host) {
		try {
			await settleUnconfirmedMutation.mutateAsync({ hostId: host.id, action: 'dismiss' });
			handleCloseUnconfirmedHost();
		} catch {
			// Error handled by mutation
		}
	}

	function handleStartMerge(host: Host) {
		otherHost = host;
		showHostMergeModal = true;
	}

	function handleRescanHost(host: Host) {
		rescanHostMutation.mutate({ id: host.id, name: hostDisplayName(host) });
	}

	async function handleBulkRescan(ids: string[]) {
		await bulkRescanHostsMutation.mutateAsync(ids);
	}

	async function handleDeleteHost(host: Host) {
		// Deleting a hypervisor's or runtime's host deletes what it runs, so the question says how much.
		const guests = await virtualizedSummaryOnHosts([host.id]);
		const name = hostDisplayName(host);
		const question = guests
			? common_confirmDeleteNameWithGuests({ name, guests })
			: common_confirmDeleteName({ name });
		if (confirm(question)) {
			deleteHostMutation.mutate(host.id);
		}
	}

	async function handleHostCreate(data: CreateHostWithServicesRequest) {
		try {
			await createHostMutation.mutateAsync(data);
			showHostEditor = false;
			setEditingHost(null);
		} catch {
			// Error handled by mutation
		}
	}

	async function handleHostUpdate(data: UpdateHostWithServicesRequest) {
		try {
			await updateHostMutation.mutateAsync(data);
			showHostEditor = false;
			setEditingHost(null);
		} catch {
			// Error handled by mutation
		}
	}

	async function handleMergeHosts(destinationHostId: string, otherHostId: string) {
		try {
			await mergeHostsMutation.mutateAsync({
				destinationHostId,
				otherHostId,
				otherHostName: otherHost ? hostDisplayName(otherHost) : undefined
			});
			showHostMergeModal = false;
			otherHost = null;
		} catch {
			// Error handled by mutation
		}
	}

	async function handleBulkDelete(ids: string[]) {
		const guests = await virtualizedSummaryOnHosts(ids);
		const question = guests
			? common_confirmBulkDeleteWithGuests({ count: ids.length, entity: common_hosts(), guests })
			: common_confirmBulkDelete({ count: ids.length, entity: common_hosts() });
		if (confirm(question)) {
			await bulkDeleteHostsMutation.mutateAsync(ids);
		}
	}

	function getHostTags(host: Host): string[] {
		return host.tags;
	}

	function handleCloseHostEditor() {
		showHostEditor = false;
		setEditingHost(null);
	}
</script>

{#snippet osCell(host: Host)}
	{#if host.os}
		<OsTag os={host.os} source={host.os_source} />
	{:else}
		<span class="text-sm text-tertiary">—</span>
	{/if}
{/snippet}

<!-- The page's own actions, last in the table toolbar beside the filter and columns. -->
{#snippet toolbarActions()}
	{#if daemonConnected}
		<div class="flex items-center gap-3">
			{#if hostLimit !== null && !canBuyMoreHosts}
				<span
					class="text-sm {isAtHostLimit
						? 'text-amber-700 dark:text-amber-400'
						: isNearHostLimit
							? 'text-yellow-700 dark:text-yellow-400'
							: 'text-tertiary'}"
				>
					{totalHostCount} / {hostLimit}
				</span>
			{/if}
			{#if can('Host', 'Write')}
				<CreateButton
					onclick={handleCreateHost}
					upgrade={isAtHostLimit
						? { feature: 'hosts', surface: 'hosts_tab', gate_type: 'limit_hit' }
						: null}
				/>
			{/if}
		</div>
	{/if}
{/snippet}

<div class="space-y-6">
	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if !daemonConnected}
		<PreDaemonEmptyState title={daemons_installPromptHosts()} />
	{:else if isInitialLoading}
		<!-- Loading state (only on initial load) -->
		<Loading />
	{:else if hostsData.length === 0 && !hostsPagination}
		<!-- Empty state -->
		<EmptyState
			title={common_noEntityYet({ entity: common_hosts() })}
			subtitle=""
			onClick={can('Host', 'Write') ? handleCreateHost : undefined}
			cta={can('Host', 'Write') ? common_create() : ''}
		/>
	{:else}
		<DataControls
			title={common_hosts()}
			{toolbarActions}
			items={hostsData}
			fields={hostFields}
			storageKey="scanopy-hosts-table-state"
			defaults={tableDefaults}
			entityType="Host"
			canEditTags={can('Tag', 'Write')}
			getItemTags={getHostTags}
			getItemId={(item) => item.id}
			serverPagination={hostsPagination}
			onPageChange={handlePageChange}
			onQueryChange={(query) => (listQuery = query)}
			onExportClick={() => {
				showExportModal = true;
			}}
			getActions={hostActions}
			getRowBadges={rowStatusBadges(() => sitesData, 'Host')}
			entityLabel={common_hosts()}
		></DataControls>
	{/if}
</div>

<HostEditor
	isOpen={showHostEditor}
	name="host-editor"
	host={editingHost}
	onCreate={handleHostCreate}
	onDelete={handleDeleteHost}
	onUpdate={handleHostUpdate}
	onClose={handleCloseHostEditor}
/>

<HostMergeModal
	isOpen={showHostMergeModal}
	{otherHost}
	onMerge={handleMergeHosts}
	onClose={() => (showHostMergeModal = false)}
/>

<UnconfirmedHostModal
	isOpen={showUnconfirmedHost}
	host={resolvingHost}
	known={resolvingHost ? unconfirmedEvidence(resolvingHost, interfacesQuery.data ?? []) : null}
	onConfirmAndLabel={handleConfirmAndLabel}
	onMerge={handleMergeUnconfirmed}
	onDismiss={handleDismissUnconfirmed}
	onClose={handleCloseUnconfirmedHost}
/>

<HostExportModal
	isOpen={showExportModal}
	onClose={() => (showExportModal = false)}
	{exportParams}
/>
