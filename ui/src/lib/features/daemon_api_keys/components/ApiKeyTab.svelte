<script lang="ts">
	import { collectionLookup } from '$lib/shared/entity-fields';
	import { Edit, Trash2 } from '@lucide/svelte';
	import { disabledWhen, type CardAction } from '$lib/shared/components/data/types';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import { apiKeyColumns } from './ApiKeyTab.columns';
	import CreateApiKeyModal from './ApiKeyModal.svelte';
	import type { ApiKey } from '../types/base';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useApiKeysQuery,
		useUnboundApiKeysListQuery,
		useUpdateApiKeyMutation,
		useDeleteApiKeyMutation,
		useBulkDeleteApiKeysMutation
	} from '../queries';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { listPageState } from '$lib/shared/components/data/listPage.svelte';
	import { useFieldValuesQuery } from '$lib/shared/api/field-values';
	import type { components } from '$lib/api/schema';

	type DaemonApiKeyOrderField = components['schemas']['DaemonApiKeyOrderField'];
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import {
		common_edit,
		common_delete,
		common_confirmBulkDelete,
		common_confirmDeleteName,
		common_noEntityYet,
		daemonApiKeys_title,
		daemonApiKeys_provisionOnlyHint,
		daemonApiKeys_inUse,
		daemonApiKeys_inUseBulk
	} from '$lib/paraglide/messages';

	import { refusedUnless, useCan } from '$lib/features/auth/access';

	let { isActive = false }: TabProps = $props();
	const can = useCan();

	// Queries
	const tagsQuery = useTagsQuery();
	// Every key, bound or not: a deep link can name a key bound to a daemon.
	const apiKeysQuery = useApiKeysQuery({ enabled: () => can('DaemonApiKey', 'Read') });
	// The page the table shows: keys no daemon is bound to, paged on the server. A key bound 1:1 to
	// a daemon is managed from the daemon record, not this tab.
	const page = listPageState(() => isActive);
	const keysPageQuery = useUnboundApiKeysListQuery(() => page.params());
	const siteValuesQuery = useFieldValuesQuery(
		'/api/v1/daemon-api-keys/field-values/{field}',
		'site_id',
		() => ({
			unbound: true
		}),
		() => isActive
	);
	const sitesQuery = useSitesQuery();
	// Daemons query — also used to determine which API keys are in use
	const daemonsQuery = useDaemonsQuery();

	// Mutations
	const updateApiKeyMutation = useUpdateApiKeyMutation();
	const deleteApiKeyMutation = useDeleteApiKeyMutation();
	const bulkDeleteApiKeysMutation = useBulkDeleteApiKeysMutation();

	let tagsData = $derived(tagsQuery.data ?? []);
	let apiKeysData = $derived(keysPageQuery.data?.items ?? []);
	let apiKeysPagination = $derived(keysPageQuery.data?.pagination ?? null);
	let sitesData = $derived(sitesQuery.data ?? []);
	let isLoading = $derived(keysPageQuery.isPending && !keysPageQuery.data);
	let pageLoad = $derived(loadFailure(keysPageQuery));
	let apiKeyIdsInUse = $derived(
		new Set(
			(daemonsQuery.data ?? []).map((d) => d.api_key_id).filter((id): id is string => id != null)
		)
	);

	let showCreateApiKeyModal = $state(false);
	let editingApiKey = $state<ApiKey | null>(null);

	// Deep-link: open daemon API key editor from URL. Resolve the id against the FULL,
	// unfiltered key list — a deep link can name a key bound 1:1 to a daemon
	// (daemon_id set), which is deliberately excluded from `apiKeysData`
	// (the legacy-only tab list). Resolving against the filtered list would never find it.
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'daemon-api-key',
			apiKeysQuery.data ?? [],
			showCreateApiKeyModal,
			editingApiKey?.id
		);
		if (result !== undefined) {
			editingApiKey = result;
			showCreateApiKeyModal = true;
		}
	});

	async function handleDeleteApiKey(apiKey: ApiKey) {
		if (confirm(common_confirmDeleteName({ name: apiKey.name }))) {
			deleteApiKeyMutation.mutate(apiKey.id);
		}
	}

	async function handleUpdateApiKey(apiKey: ApiKey) {
		await updateApiKeyMutation.mutateAsync(apiKey);
		showCreateApiKeyModal = false;
		editingApiKey = null;
	}

	function handleCloseCreateApiKey() {
		showCreateApiKeyModal = false;
		editingApiKey = null;
	}

	function handleEditApiKey(apiKey: ApiKey) {
		showCreateApiKeyModal = true;
		editingApiKey = apiKey;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: daemonApiKeys_title() }))) {
			await bulkDeleteApiKeysMutation.mutateAsync(ids);
		}
	}

	function getApiKeyTags(apiKey: ApiKey): string[] {
		return apiKey.tags;
	}

	// CSV export handler
	async function handleCsvExport() {
		// A one-off copy for the request, never read reactively.
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const params = new URLSearchParams(page.listQuery);
		params.set('unbound', 'true');
		await downloadCsv('DaemonApiKey', params);
	}

	/** Row actions. */
	function apiKeyActions(apiKey: ApiKey): CardAction[] {
		if (!can('DaemonApiKey', 'Write')) return [];

		return [
			{
				label: common_edit(),
				icon: Edit,
				isOpen: true,
				onClick: () => handleEditApiKey(apiKey),
				unavailable: refusedUnless(can('DaemonApiKey', 'Write', apiKey.site_id))
			},
			{
				label: common_delete(),
				icon: Trash2,
				tone: 'danger',
				isDelete: true,
				onBulk: handleBulkDelete,
				onClick: () => handleDeleteApiKey(apiKey),
				unavailable:
					refusedUnless(can('DaemonApiKey', 'Delete', apiKey.site_id)) ??
					disabledWhen(
						apiKeyIdsInUse.has(apiKey.id),
						daemonApiKeys_inUse(),
						daemonApiKeys_inUseBulk()
					)
			}
		];
	}

	const tableDefaults: TableDefaults<DaemonApiKeyOrderField> = {
		sort: { field: 'name', direction: 'asc' }
	};

	// Related entities the registry's key fields read: the site.
	let lookup = $derived(collectionLookup({ Site: sitesData }));

	let apiKeyFields = $derived(
		apiKeyColumns({
			lookup: () => lookup,
			sites: () => sitesData,
			tags: () => tagsData,
			siteValues: () => siteValuesQuery.data
		})
	);
</script>

<div class="space-y-6">
	<!-- Header. No create action: daemon keys are now minted 1:1 through daemon
	     provisioning, so this tab only lists (and lets you manage) existing keys. -->
	<!--
		Every key on this tab is unbound, so the legacy explanation sits in the tab
		subtitle rather than as a per-row tag that would say the same thing on
		every row.
	-->
	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if isLoading}
		<Loading />
	{:else if page.isEmpty(keysPageQuery)}
		<!-- Empty state -->
		<EmptyState
			title={common_noEntityYet({ entity: daemonApiKeys_title() })}
			subtitle={daemonApiKeys_provisionOnlyHint()}
		/>
	{:else}
		<DataControls
			title={daemonApiKeys_title()}
			items={apiKeysData}
			fields={apiKeyFields}
			entityType="DaemonApiKey"
			canEditTags={can('Tag', 'Write')}
			getItemTags={getApiKeyTags}
			storageKey="scanopy-api-keys-table-state"
			defaults={tableDefaults}
			getItemId={(item) => item.id}
			serverPagination={apiKeysPagination}
			onPageChange={page.onPageChange}
			onQueryChange={page.onQueryChange}
			serverQuery={{ stale: false }}
			getActions={apiKeyActions}
			onCsvExport={handleCsvExport}
		></DataControls>
	{/if}
</div>

<CreateApiKeyModal
	name="daemon-api-key"
	isOpen={showCreateApiKeyModal}
	onClose={handleCloseCreateApiKey}
	onUpdate={handleUpdateApiKey}
	apiKey={editingApiKey}
/>
