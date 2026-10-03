<script lang="ts">
	import { formatLongDate } from '$lib/shared/utils/formatting';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import { hasSunsetWarning, getDaemonStatusTag, osLabel } from '$lib/features/daemons/utils';
	import OsTag from '$lib/features/hosts/components/OsTag.svelte';
	import { hostOsOfDaemonOs } from '$lib/features/hosts/host-os';
	import CreateDaemonModal from './CreateDaemonModal/CreateDaemonModal.svelte';
	import { defineFields, type CardAction } from '$lib/shared/components/data/types';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { tagNames } from '$lib/features/tags/columns';
	import { networkItems } from '$lib/features/networks/columns';
	import { entityRef } from '$lib/shared/components/data/types';
	import { entities } from '$lib/shared/stores/metadata';
	import { isUserManagedSubnet, useSubnetsQuery } from '$lib/features/subnets/queries';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import { Plus, Trash2, Edit, ArrowBigUp, RefreshCw } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useDaemonsQuery,
		useDeleteDaemonMutation,
		useBulkDeleteDaemonsMutation,
		useRetryDaemonConnectionMutation
	} from '$lib/features/daemons/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
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
	import {
		common_create,
		common_operatingSystem,
		common_confirmBulkDelete,
		common_confirmDeleteName,
		common_created,
		common_daemons,
		common_delete,
		common_edit,
		common_host,
		common_name,
		common_network,
		common_status,
		common_tags,
		common_noEntityYet,
		common_unknownEntity,
		common_unknownNetwork,
		common_update,
		common_updated,
		daemons_retryConnection,
		common_version,
		daemons_config_mode,
		daemons_interfacesWith,
		daemons_lastSeen,
		daemons_mode_daemonPoll,
		daemons_mode_serverPoll,
		daemons_sunsetBannerTitle,
		daemons_sunsetBannerBody
	} from '$lib/paraglide/messages';

	type DaemonOrderField = components['schemas']['DaemonOrderField'];

	let { isReadOnly = false }: TabProps = $props();

	// Queries
	const tagsQuery = useTagsQuery();
	const daemonsQuery = useDaemonsQuery();
	const networksQuery = useNetworksQuery();
	// Shared subnets cache, to resolve each daemon's interfaced subnet ids.
	const subnetsQuery = useSubnetsQuery();

	// Mutations
	const deleteDaemonMutation = useDeleteDaemonMutation();
	const bulkDeleteDaemonsMutation = useBulkDeleteDaemonsMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let daemonsData = $derived(daemonsQuery.data ?? []);

	// Only the hosts the daemons actually run on. This was an unpaginated
	// org-wide hosts query (~1.9MB on a few hundred hosts) issued to resolve one
	// name per daemon card — and because TanStack dedupes by key, it was shared
	// with every other consumer, so it loaded on pages that never showed a
	// daemon. Scoped to the ids in hand and passed down to the cards.
	let daemonHostIds = $derived([
		...new Set(daemonsData.map((d) => d.host_id).filter((id): id is string => !!id))
	]);
	const daemonHostsQuery = useHostsByIds(() => daemonHostIds);
	let daemonHosts = $derived(daemonHostsQuery.data ?? []);

	// Any daemon with a scheduled/active sunset. Drives a non-dismissable banner
	// so the warning re-arms as long as an affected daemon exists.
	let hasSunsetDaemons = $derived(daemonsData.some(hasSunsetWarning));
	// The shared sunset date across affected daemons (server-published, same value
	// the email uses), formatted for display. All below-floor daemons share the
	// one announced date.
	let sunsetDateDisplay = $derived.by(() => {
		const iso = daemonsData.find(hasSunsetWarning)?.version_status.sunset_date;
		if (!iso) return null;
		return formatLongDate(iso, 'UTC');
	});
	let networksData = $derived(networksQuery.data ?? []);
	let subnetsData = $derived((subnetsQuery.data ?? []).filter(isUserManagedSubnet));
	let isLoading = $derived(daemonsQuery.isPending || networksQuery.isPending);

	let showCreateDaemonModal = $state(false);
	let showDaemonEditor = $state(false);
	let editingDaemon = $state<Daemon | null>(null);

	// Auto-open modal when deep-linked via ?modal=create-daemon
	$effect(() => {
		if ($modalState.name === 'create-daemon' && !showCreateDaemonModal) {
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

	/**
	 * Escalate the upgrade affordance by lifecycle stage: a scheduled or active
	 * sunset is a warning/danger action, a plain newer release is neutral-info.
	 */
	function upgradeButtonClass(daemon: Daemon): string {
		switch (daemon.version_status.status) {
			case 'Unsupported':
				return 'btn-icon-danger';
			case 'Deprecated':
				return 'btn-icon-warning';
			case 'Outdated':
				return 'btn-icon-info';
			default:
				return 'btn-icon';
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

	/** Row actions. Upgrade and retry appear only when the daemon's state calls for them. */
	function daemonActions(daemon: Daemon): CardAction[] {
		if (isReadOnly) return [];

		const hasUpdateAvailable =
			daemon.version_status.status === 'Outdated' ||
			daemon.version_status.status === 'Deprecated' ||
			daemon.version_status.status === 'Unsupported';
		// The scheduled sunset date is server-published — the UI never computes it.
		const sunsetDate = daemon.version_status.sunset_date ?? null;

		const actions: CardAction[] = [
			{
				label: common_delete(),
				icon: Trash2,
				class: 'btn-icon-danger',
				onClick: () => handleDeleteDaemon(daemon)
			}
		];

		if (hasUpdateAvailable && (daemon.is_unreachable !== true || sunsetDate !== null)) {
			actions.push({
				label: common_update(),
				icon: ArrowBigUp,
				class: upgradeButtonClass(daemon),
				onClick: () => handleOpenUpgrade(daemon),
				forceLabel: true
			});
		}

		// An unreachable ServerPoll daemon is the one case the server can be asked
		// to try again on demand.
		if (daemon.is_unreachable === true && daemon.mode === 'server_poll') {
			actions.push({
				label: daemons_retryConnection(),
				icon: RefreshCw,
				class: 'btn-icon-info',
				onClick: () => retryConnectionMutation.mutate(daemon.id),
				disabled: retryConnectionMutation.isPending,
				forceLabel: true
			});
		}

		// Edit sits last (rightmost) per the convention shared by every other entity.
		// It opens the daemon management modal, which is also where the daemon's 1:1
		// key is managed — and where a legacy daemon can be given one.
		actions.push({ label: common_edit(), icon: Edit, onClick: () => handleEditDaemon(daemon) });

		return actions;
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
		showCreateDaemonModal = true;
	}

	function handleCloseCreateDaemon() {
		showCreateDaemonModal = false;
		// Only redirect to home if no daemons AND we're still on the daemons tab
		// (user may have navigated away via "View Topology" button)
		if (daemonsData.length === 0 && window.location.hash === '#daemons') {
			window.location.hash = 'home';
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
			.filter((subnet): subnet is Subnet => subnet !== undefined);
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Daemon', {});
	}

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all DaemonOrderField values are covered
	let daemonFields = $derived(
		defineFields<Daemon, DaemonOrderField>(
			{
				// Identity field: grouping by it would render a header per daemon.
				name: {
					label: common_name(),
					type: 'string',
					searchable: true,
					groupable: false,
					display: { order: 0, primary: true, width: 220 }
				},
				network_id: {
					label: common_network(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getValue: (item) =>
						networksData.find((n) => n.id == item.network_id)?.name || common_unknownNetwork(),
					display: { order: 3, getItems: (item) => networkItems(item.network_id, networksData) }
				},
				last_seen: {
					label: daemons_lastSeen(),
					type: 'date',
					display: { order: 2, recency: true }
				},
				created_at: { label: common_created(), type: 'date', display: { hiddenByDefault: true } },
				updated_at: { label: common_updated(), type: 'date', display: { hiddenByDefault: true } }
			},
			[
				{
					// Host, version and subnet interfaces were card-only. Declaring
					// them here is what makes them exist for both views at once.
					// Display-only: none is a DaemonOrderField, so the server cannot
					// order on them and defineFields rightly refuses them above.
					key: 'host_id',
					label: common_host(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getValue: (daemon) => {
						const host = daemonHosts.find((h) => h.id === daemon.host_id);
						return host ? hostDisplayName(host) : common_unknownEntity({ entity: common_host() });
					},
					display: {
						hiddenByDefault: true,
						getItems: (daemon) => {
							const host = daemonHosts.find((h) => h.id === daemon.host_id);
							if (!host) return [];
							return [
								{
									id: host.id,
									label: hostDisplayName(host),
									color: entities.getColorHelper('Host').color,
									entityRef: entityRef('Host', host.id, host)
								}
							];
						}
					}
				},
				{
					// One mapping for both views. This used to compute its own
					// active/standby/unreachable strings while the card called
					// getDaemonStatusTag, so the same daemon read "Active" in the table
					// and "Healthy" on the card. Both now come from the one helper,
					// which also carries version lifecycle (deprecated, unsupported).
					key: 'status',
					label: common_status(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getValue: (daemon) => getDaemonStatusTag(daemon).label,
					display: {
						order: 1,
						statusTag: true,
						getItems: (daemon) => {
							const tag = getDaemonStatusTag(daemon);
							return [{ id: tag.label, label: tag.label, color: tag.color, icon: tag.icon }];
						}
					}
				},
				{
					key: 'os',
					label: common_operatingSystem(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getValue: (daemon) => (daemon.os ? osLabel(daemon.os) : ''),
					display: { order: 4.5, cell: osCell }
				},
				{
					key: 'mode',
					label: daemons_config_mode(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getValue: (daemon) =>
						daemon.mode === 'server_poll' ? daemons_mode_serverPoll() : daemons_mode_daemonPoll(),
					display: {
						order: 5,
						getItems: (daemon) => [
							{
								id: daemon.mode,
								label:
									daemon.mode === 'server_poll'
										? daemons_mode_serverPoll()
										: daemons_mode_daemonPoll()
							}
						]
					}
				},
				{
					key: 'version',
					label: common_version(),
					type: 'string',
					searchable: true,
					filterable: true,
					getValue: (daemon) => daemon.version ?? '',
					display: { order: 4 }
				},
				{
					key: 'interfaced_subnet_ids',
					label: daemons_interfacesWith(),
					type: 'array',
					searchable: true,
					getValue: (daemon) => interfacedSubnets(daemon).map((s) => s.name),
					display: {
						order: 6,
						getItems: (daemon) =>
							interfacedSubnets(daemon).map((subnet) => ({
								id: subnet.id,
								label: subnet.name,
								color: entities.getColorHelper('Subnet').color,
								entityRef: entityRef('Subnet', subnet.id, subnet)
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
				}
			]
		)
	);
</script>

{#snippet osCell(daemon: Daemon)}
	{@const os = daemon.os ? hostOsOfDaemonOs(daemon.os) : null}
	{#if os}
		<OsTag {os} />
	{:else}
		<span class="text-tertiary text-sm">—</span>
	{/if}
{/snippet}

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_daemons()}>
		<svelte:fragment slot="actions">
			{#if !isReadOnly}
				<button class="btn-primary flex items-center" onclick={handleCreateDaemon}
					><Plus class="h-5 w-5" />{common_create()}</button
				>
			{/if}
		</svelte:fragment>
	</TabHeader>

	<!-- Loading state -->
	{#if isLoading}
		<Loading />
	{:else if daemonsData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={common_noEntityYet({ entity: common_daemons() })}
			subtitle=""
			onClick={handleCreateDaemon}
			cta={common_create()}
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
			items={daemonsData}
			fields={daemonFields}
			storageKey="scanopy-daemons-table-state"
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'Daemon'}
			getItemTags={getDaemonTags}
			getItemId={(item) => item.id}
			getIcon={() => ({
				icon: entities.getIconComponent('Daemon'),
				color: entities.getColorHelper('Daemon').icon
			})}
			onCsvExport={handleCsvExport}
			getActions={daemonActions}
			entityLabel={common_daemons()}
		></DataControls>
	{/if}
</div>

<CreateDaemonModal
	isOpen={showCreateDaemonModal}
	name="create-daemon"
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
