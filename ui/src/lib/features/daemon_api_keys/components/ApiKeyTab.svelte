<script lang="ts">
	import { Edit, Trash2 } from 'lucide-svelte';
	import type { CardAction } from '$lib/shared/components/data/types';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import { siteItems } from '$lib/features/sites/columns';
	import CreateApiKeyModal from './ApiKeyModal.svelte';
	import type { ApiKey } from '../types/base';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useApiKeysQuery,
		useUpdateApiKeyMutation,
		useDeleteApiKeyMutation,
		useBulkDeleteApiKeysMutation
	} from '../queries';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState } from '$lib/shared/stores/modal-registry';
	import {
		common_enabled,
		common_expired,
		common_expires,
		common_lastUsed,
		common_never,
		common_edit,
		common_delete,
		common_confirmBulkDelete,
		common_confirmDeleteName,
		common_created,
		common_name,
		common_site,
		common_noEntityYet,
		common_tags,
		common_updated,
		common_unknownSite,
		daemonApiKeys_title,
		daemonApiKeys_provisionOnlyHint
	} from '$lib/paraglide/messages';

	let { isReadOnly = false }: TabProps = $props();

	// Queries
	const tagsQuery = useTagsQuery();
	const apiKeysQuery = useApiKeysQuery();
	const sitesQuery = useSitesQuery();
	// Daemons query — also used to determine which API keys are in use
	const daemonsQuery = useDaemonsQuery();

	// Mutations
	const updateApiKeyMutation = useUpdateApiKeyMutation();
	const deleteApiKeyMutation = useDeleteApiKeyMutation();
	const bulkDeleteApiKeysMutation = useBulkDeleteApiKeysMutation();

	// Derived data. Only legacy keys are shown here — a key bound 1:1 to a daemon
	// (daemon_id set) is managed from the daemon record, not this tab.
	let tagsData = $derived(tagsQuery.data ?? []);
	let apiKeysData = $derived((apiKeysQuery.data ?? []).filter((k) => k.daemon_id == null));
	let sitesData = $derived(sitesQuery.data ?? []);
	let isLoading = $derived(apiKeysQuery.isPending);
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
		if ($modalState.name === 'daemon-api-key' && !showCreateApiKeyModal) {
			if ($modalState.id) {
				const entity = (apiKeysQuery.data ?? []).find((e) => e.id === $modalState.id);
				if (entity) {
					editingApiKey = entity;
					showCreateApiKeyModal = true;
				}
			} else {
				editingApiKey = null;
				showCreateApiKeyModal = true;
			}
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
		await downloadCsv('DaemonApiKey', {});
	}

	/** Row actions. */
	function apiKeyActions(apiKey: ApiKey): CardAction[] {
		if (isReadOnly) return [];

		return [
			{ label: common_edit(), icon: Edit, onClick: () => handleEditApiKey(apiKey) },
			{
				label: common_delete(),
				icon: Trash2,
				class: 'btn-icon-danger',
				onClick: () => handleDeleteApiKey(apiKey),
				// A key a daemon is using cannot be deleted.
				disabled: apiKeyIdsInUse.has(apiKey.id)
			}
		];
	}

	const tableDefaults: TableDefaults<string> = { sort: { field: 'name', direction: 'asc' } };

	const apiKeyFields: FieldConfig<ApiKey>[] = [
		{
			key: 'name',
			label: common_name(),
			type: 'string',
			searchable: true,
			sortable: true
		},
		{
			key: 'site_id',
			type: 'string',
			label: common_site(),
			searchable: true,
			filterable: true,
			groupable: true,
			sortable: true,
			getValue(item) {
				return sitesData.find((n) => n.id == item.site_id)?.name || common_unknownSite();
			},
			display: { getItems: (item) => siteItems(item.site_id, sitesData) }
		},
		{
			key: 'is_enabled',
			label: common_enabled(),
			type: 'boolean',
			filterable: true,
			groupable: true,
			getValue: (key) => key.is_enabled ?? false
		},
		{
			key: 'last_used',
			label: common_lastUsed(),
			type: 'date',
			sortable: true,
			getValue: (key) => key.last_used ?? null,
			display: { recency: true }
		},
		{
			key: 'expires_at',
			label: common_expires(),
			type: 'date',
			sortable: true,
			getValue: (key) => key.expires_at ?? null,
			display: {
				// Expired reads as a state, not a date — the date has stopped being the useful
				// part once it has passed. Returning undefined shows the date itself.
				getItems: (key) =>
					!key.expires_at
						? [{ id: 'never', label: common_never(), color: 'Gray' }]
						: new Date(key.expires_at) < new Date()
							? [{ id: 'expired', label: common_expired(), color: 'Red' }]
							: undefined
			}
		},
		{
			key: 'tags',
			label: common_tags(),
			type: 'array',
			searchable: true,
			filterable: true,
			getValue: (entity) => {
				// Return tag names for search/filter display
				return entity.tags
					.map((id) => tagsData.find((t) => t.id === id)?.name)
					.filter((name): name is string => !!name);
			}
		},
		{
			key: 'created_at',
			label: common_created(),
			type: 'date',
			sortable: true
		},
		{
			key: 'updated_at',
			label: common_updated(),
			type: 'date',
			sortable: true,
			display: { hiddenByDefault: true }
		}
	];
</script>

<div class="space-y-6">
	<!-- Header. No create action: daemon keys are now minted 1:1 through daemon
	     provisioning, so this tab only lists (and lets you manage) existing keys. -->
	<!--
		Every key on this tab is unbound, so the legacy explanation sits in the tab
		subtitle rather than as a per-row tag that would say the same thing on
		every row.
	-->
	<!-- Loading state -->
	{#if isLoading}
		<Loading />
	{:else if apiKeysData.length === 0}
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
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'DaemonApiKey'}
			getItemTags={getApiKeyTags}
			storageKey="scanopy-api-keys-table-state"
			defaults={tableDefaults}
			getItemId={(item) => item.id}
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
