<script lang="ts">
	import CreateButton from '$lib/shared/components/data/controls/CreateButton.svelte';
	import { formatLongDate } from '$lib/shared/utils/formatting';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import OsTag from '$lib/features/hosts/components/OsTag.svelte';
	import { hostOsOfDaemonOs } from '$lib/features/hosts/host-os';
	import CreateDaemonModal from './CreateDaemonModal/CreateDaemonModal.svelte';
	import {
		disabledWhen,
		hiddenWhen,
		type ActionTone,
		type RowAction
	} from '$lib/shared/components/data/types';
	import { daemonColumns } from './DaemonTab.columns';
	import { collectionLookup } from '$lib/shared/entity-fields';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import { isUserManagedSubnet, useSubnetsQuery } from '$lib/features/subnets/queries';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import { Trash2, Edit, ArrowBigUp, RefreshCw } from '@lucide/svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useDaemonsQuery,
		useDaemonsListQuery,
		useDeleteDaemonMutation,
		useBulkDeleteDaemonsMutation,
		useBulkRetryDaemonConnectionMutation,
		useRetryDaemonConnectionMutation
	} from '$lib/features/daemons/queries';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { useUsersByIds } from '$lib/features/users/queries';
	import {
		modalState,
		openModal,
		closeModal,
		resolveModalDeepLink
	} from '$lib/shared/stores/modal-registry';
	import DaemonUpgradeModal from './DaemonUpgradeModal.svelte';
	import DaemonEditModal from './DaemonEditModal.svelte';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { listPageState } from '$lib/shared/components/data/listPage.svelte';
	import { fieldValueOptions, useFieldValuesQuery } from '$lib/shared/api/field-values';
	import {
		common_create,
		common_confirmBulkDelete,
		common_confirmDeleteName,
		common_daemons,
		common_delete,
		common_edit,
		common_noEntityYet,
		common_upgrade,
		daemons_retryConnection,
		daemons_retryingConnection,
		daemons_notServerPolled,
		daemons_notServerPolledBulk,
		daemons_reachable,
		daemons_reachableBulk,
		daemons_upToDate,
		daemons_upgradeUnreachable,
		daemons_sunsetBannerTitle,
		daemons_sunsetBannerBody
	} from '$lib/paraglide/messages';

	type DaemonOrderField = components['schemas']['DaemonOrderField'];

	import { refusedUnless, useCan } from '$lib/features/auth/access';

	let { isActive = false }: TabProps = $props();
	const can = useCan();

	// Queries
	const tagsQuery = useTagsQuery();
	// Every daemon, for the sunset banner, the empty state and deep links, which concern the whole
	// organization rather than the page on screen.
	const daemonsQuery = useDaemonsQuery();
	// The page the table shows, paged, filtered and ordered on the server.
	const page = listPageState(() => isActive);
	const daemonsPageQuery = useDaemonsListQuery(() => page.params());
	// Filter options: the values the caller's daemons hold, counted by the server.
	const fieldValuesQuery = useFieldValuesQuery(
		'/api/v1/daemons/field-values',
		['site_id', 'mode', 'maintainer', 'version'],
		undefined,
		() => isActive
	);
	const sitesQuery = useSitesQuery();
	// Shared subnets cache, to resolve each daemon's interfaced subnet ids.
	const subnetsQuery = useSubnetsQuery();

	// Mutations
	const deleteDaemonMutation = useDeleteDaemonMutation();
	const bulkDeleteDaemonsMutation = useBulkDeleteDaemonsMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let pageDaemons = $derived(daemonsPageQuery.data?.items ?? []);
	let daemonsPagination = $derived(daemonsPageQuery.data?.pagination ?? null);

	// Only the hosts the daemons actually run on. This was an unpaginated
	// org-wide hosts query (~1.9MB on a few hundred hosts) issued to resolve one
	// name per daemon row — and because TanStack dedupes by key, it was shared
	// with every other consumer, so it loaded on pages that never showed a
	// daemon. Scoped to the ids in hand and read by the host column.
	let daemonHostIds = $derived([
		...new Set(pageDaemons.map((d) => d.host_id).filter((id): id is string => !!id))
	]);
	const daemonHostsQuery = useHostsByIds(() => daemonHostIds);
	let daemonHosts = $derived(daemonHostsQuery.data ?? []);

	// The users who maintain the daemons on the page and the ones the maintainer filter offers, by
	// id: the users list needs Admin, and a daemon's maintainer is often an admin or the owner,
	// whom it leaves out.
	const maintainersQuery = useUsersByIds(() =>
		can('Member', 'Read')
			? [
					...new Set([
						...pageDaemons.map((d) => d.user_id).filter((id): id is string => !!id),
						...fieldValueOptions(fieldValuesQuery.data?.maintainer)
					])
				]
			: []
	);
	let maintainers = $derived(maintainersQuery.data ?? []);

	// Any daemon with a scheduled/active sunset. Drives a non-dismissable banner
	// so the warning re-arms as long as an affected daemon exists.
	const hasSunset = (daemon: Daemon) =>
		daemon.health === 'deprecated' || daemon.health === 'unsupported';
	let hasSunsetDaemons = $derived(daemonsData.some(hasSunset));
	// The shared sunset date across affected daemons (server-published, same value
	// the email uses), formatted for display. All below-floor daemons share the
	// one announced date.
	let sunsetDateDisplay = $derived.by(() => {
		const iso = daemonsData.find(hasSunset)?.version_status.sunset_date;
		if (!iso) return null;
		return formatLongDate(iso, 'UTC');
	});
	let sitesData = $derived(sitesQuery.data ?? []);
	let subnetsData = $derived((subnetsQuery.data ?? []).filter(isUserManagedSubnet));
	let isLoading = $derived(
		(daemonsPageQuery.isPending && !daemonsPageQuery.data) ||
			daemonsQuery.isPending ||
			sitesQuery.isPending
	);
	let pageLoad = $derived(loadFailure(daemonsPageQuery, daemonsQuery, sitesQuery));

	let showCreateDaemonModal = $state(false);
	/** The site a deep link opened the wizard on (`sites` prefill), for a site with no daemon. */
	let createDaemonSiteId = $state<string | null>(null);
	let showDaemonEditor = $state(false);
	let editingDaemon = $state<Daemon | null>(null);

	// Auto-open modal when deep-linked via ?modal=create-daemon
	$effect(() => {
		if ($modalState.name === 'create-daemon' && !showCreateDaemonModal && can('Daemon', 'Write')) {
			createDaemonSiteId = $modalState.prefill?.sites ?? null;
			showCreateDaemonModal = true;
		}
	});

	// Deep-link the daemon editor via ?modal=daemon-editor&id=<daemon-id>
	$effect(() => {
		const resolved = resolveModalDeepLink(
			$modalState,
			'daemon-editor',
			daemonsData,
			showDaemonEditor,
			editingDaemon?.id
		);
		if (resolved !== undefined) {
			editingDaemon = resolved;
			showDaemonEditor = true;
		}
	});

	let upgradingDaemon = $state<Daemon | null>(null);

	// Deep link: ?modal=upgrade-daemon&id=<daemon-id>
	$effect(() => {
		if ($modalState.name === 'upgrade-daemon' && $modalState.id && !upgradingDaemon) {
			upgradingDaemon = daemonsData.find((d) => d.id === $modalState.id) ?? null;
		}
	});

	const retryConnectionMutation = useRetryDaemonConnectionMutation();
	const bulkRetryConnectionMutation = useBulkRetryDaemonConnectionMutation();

	/**
	 * Escalate the upgrade affordance by lifecycle stage: a scheduled or active
	 * sunset is a warning/danger action, a plain newer release is neutral-info.
	 */
	function upgradeTone(daemon: Daemon): ActionTone | undefined {
		switch (daemon.health) {
			case 'unsupported':
				return 'danger';
			case 'deprecated':
				return 'warning';
			case 'outdated':
				return 'info';
			default:
				return undefined;
		}
	}

	function handleOpenUpgrade(daemon: Daemon) {
		upgradingDaemon = daemon;
		openModal('upgrade-daemon', { id: daemon.id });
	}

	function handleCloseUpgrade() {
		upgradingDaemon = null;
		closeModal();
	}

	/**
	 * Row actions: Retry Connection and Upgrade (each only when the daemon's state calls for it,
	 * the rarer leftmost), then Edit and Delete on every row.
	 */
	function daemonActions(daemon: Daemon): RowAction[] {
		// A Member sees the upgrade command and changes nothing. A Viewer has no key access, so no row.
		if (!can('Daemon', 'Write') && !can('DaemonApiKey', 'Read')) return [];

		const writeRefusal = refusedUnless(can('Daemon', 'Write', daemon.site_id));
		const hasUpdateAvailable =
			daemon.health === 'outdated' ||
			daemon.health === 'deprecated' ||
			daemon.health === 'unsupported';
		// The scheduled sunset date is server-published — the UI never computes it.
		const sunsetDate = daemon.version_status.sunset_date ?? null;
		const serverPolled = daemon.mode === 'server_poll';

		return [
			// An unreachable ServerPoll daemon is the one case the server can be asked to try
			// again on demand.
			{
				label: daemons_retryConnection(),
				icon: RefreshCw,
				tone: 'info',
				onClick: () => retryConnectionMutation.mutate(daemon.id),
				onBulk: (ids) => bulkRetryConnectionMutation.mutateAsync(ids),
				unavailable:
					hiddenWhen(!serverPolled, daemons_notServerPolled(), daemons_notServerPolledBulk()) ??
					hiddenWhen(
						daemon.is_unreachable !== true,
						daemons_reachable(),
						daemons_reachableBulk()
					) ??
					writeRefusal ??
					disabledWhen(
						retryConnectionMutation.isPending || bulkRetryConnectionMutation.isPending,
						daemons_retryingConnection()
					)
			},
			{
				label: common_upgrade(),
				icon: ArrowBigUp,
				tone: upgradeTone(daemon),
				onClick: () => handleOpenUpgrade(daemon),
				unavailable:
					hiddenWhen(!hasUpdateAvailable, daemons_upToDate()) ??
					refusedUnless(can('DaemonApiKey', 'Read', daemon.site_id)) ??
					// Without a sunset to warn about, an unreachable daemon has nothing to upgrade
					// from here.
					disabledWhen(
						daemon.is_unreachable === true && sunsetDate === null,
						daemons_upgradeUnreachable()
					)
			},
			// It opens the daemon management modal, which is also where the daemon's 1:1 key is
			// managed — and where a legacy daemon can be given one.
			{
				label: common_edit(),
				icon: Edit,
				isOpen: true,
				onClick: () => handleEditDaemon(daemon),
				unavailable: writeRefusal
			},
			{
				label: common_delete(),
				icon: Trash2,
				tone: 'danger',
				isDelete: true,
				onBulk: handleBulkDelete,
				onClick: () => handleDeleteDaemon(daemon),
				unavailable: refusedUnless(can('Daemon', 'Delete', daemon.site_id))
			}
		];
	}

	function handleEditDaemon(daemon: Daemon) {
		editingDaemon = daemon;
		showDaemonEditor = true;
	}

	function handleCloseDaemonEditor() {
		showDaemonEditor = false;
		editingDaemon = null;
	}

	function handleDeleteDaemon(daemon: Daemon) {
		if (confirm(common_confirmDeleteName({ name: daemon.name }))) {
			deleteDaemonMutation.mutate(daemon.id);
		}
	}

	function handleCreateDaemon() {
		createDaemonSiteId = null;
		showCreateDaemonModal = true;
	}

	function handleCloseCreateDaemon() {
		showCreateDaemonModal = false;
		// Only redirect to Setup if no daemons AND we're still on the daemons tab
		// (user may have navigated away via "View Topology" button)
		if (daemonsData.length === 0 && window.location.hash === '#daemons') {
			window.location.hash = 'setup';
		}
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_daemons() }))) {
			await bulkDeleteDaemonsMutation.mutateAsync(ids);
		}
	}

	function getDaemonTags(daemon: Daemon): string[] {
		return daemon.tags;
	}

	/** Subnets this daemon has an interface on; `subnetsData` is already narrowed to the
	 * ones the management lists show. */
	function interfacedSubnets(daemon: Daemon): Subnet[] {
		return daemon.interfaced_subnet_ids
			.map((id) => subnetsData.find((subnet) => subnet.id === id))
			.filter((subnet) => subnet !== undefined);
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Daemon', page.listQuery);
	}

	const tableDefaults: TableDefaults<DaemonOrderField> = {
		sort: { field: 'name', direction: 'asc' }
	};

	// Related entities the registry's daemon fields read: the host, the maintainer, the site.
	let lookup = $derived(
		collectionLookup({ Host: daemonHosts, User: maintainers, Site: sitesData })
	);

	let daemonFields = $derived(
		daemonColumns({
			lookup: () => lookup,
			sites: () => sitesData,
			tags: () => tagsData,
			daemonHosts: () => daemonHosts,
			maintainers: () => maintainers,
			interfacedSubnets,
			subnets: () => subnetsData,
			osCell: () => osCell,
			siteValues: () => fieldValuesQuery.data?.site_id,
			modeValues: () => fieldValuesQuery.data?.mode,
			maintainerValues: () => fieldValuesQuery.data?.maintainer,
			versionValues: () => fieldValuesQuery.data?.version
		})
	);
</script>

{#snippet osCell(daemon: Daemon)}
	{@const os = daemon.os ? hostOsOfDaemonOs(daemon.os) : null}
	{#if os}
		<OsTag {os} />
	{:else}
		<span class="text-sm text-tertiary">—</span>
	{/if}
{/snippet}

<!-- The page's own actions, last in the table toolbar beside the filter and columns. -->
{#snippet toolbarActions()}
	{#if can('Daemon', 'Write')}
		<CreateButton onclick={handleCreateDaemon} />
	{/if}
{/snippet}

<div class="space-y-6">
	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if isLoading}
		<Loading />
	{:else if daemonsData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={common_noEntityYet({ entity: common_daemons() })}
			subtitle=""
			onClick={can('Daemon', 'Write') ? handleCreateDaemon : undefined}
			cta={can('Daemon', 'Write') ? common_create() : ''}
		/>
	{:else}
		{#if hasSunsetDaemons && sunsetDateDisplay}
			<div class="mb-4">
				<InlineWarning
					title={daemons_sunsetBannerTitle()}
					body={daemons_sunsetBannerBody({ date: sunsetDateDisplay })}
				/>
			</div>
		{/if}
		<DataControls
			title={common_daemons()}
			{toolbarActions}
			items={pageDaemons}
			fields={daemonFields}
			storageKey="scanopy-daemons-table-state"
			defaults={tableDefaults}
			entityType="Daemon"
			canEditTags={can('Tag', 'Write')}
			getItemTags={getDaemonTags}
			getItemId={(item) => item.id}
			serverPagination={daemonsPagination}
			onPageChange={page.onPageChange}
			onQueryChange={page.onQueryChange}
			serverQuery={{ stale: false }}
			onCsvExport={handleCsvExport}
			getActions={daemonActions}
			entityLabel={common_daemons()}
		></DataControls>
	{/if}
</div>

<CreateDaemonModal
	isOpen={showCreateDaemonModal}
	name="create-daemon"
	siteId={createDaemonSiteId}
	onClose={handleCloseCreateDaemon}
	onNavigate={(tab) => {
		window.location.hash = tab;
	}}
/>

<DaemonEditModal
	isOpen={showDaemonEditor}
	name="daemon-editor"
	daemon={editingDaemon}
	onClose={handleCloseDaemonEditor}
/>

{#if upgradingDaemon}
	<DaemonUpgradeModal isOpen={true} onClose={handleCloseUpgrade} daemon={upgradingDaemon} />
{/if}
