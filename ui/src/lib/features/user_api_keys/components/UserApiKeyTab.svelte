<script lang="ts">
	import { Edit, Trash2 } from 'lucide-svelte';
	import type { CardAction } from '$lib/shared/components/data/types';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import { siteItems } from '$lib/features/sites/columns';
	import { permissions } from '$lib/shared/stores/metadata';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import { Plus } from 'lucide-svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import UserApiKeyModal from './UserApiKeyModal.svelte';
	import {
		useUserApiKeysQuery,
		useUpdateUserApiKeyMutation,
		useDeleteUserApiKeyMutation,
		useBulkDeleteUserApiKeysMutation,
		type UserApiKey
	} from '../queries';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';

	import {
		common_enabled,
		common_expired,
		common_expires,
		common_lastUsed,
		common_never,
		common_edit,
		common_delete,
		common_apiKeys,
		common_confirmBulkDelete,
		common_confirmDeleteName,
		common_create,
		common_created,
		common_name,
		common_sites,
		common_permissions,
		common_tags,
		common_updated,
		userApiKeys_apiAccessUnavailableSubtitle,
		userApiKeys_apiAccessUnavailableTitle,
		userApiKeys_noApiKeysSubtitle,
		userApiKeys_noApiKeysYet,
		userApiKeys_subtitle,
		userApiKeys_verifyEmailToCreate
	} from '$lib/paraglide/messages';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import { modalState } from '$lib/shared/stores/modal-registry';

	let { isReadOnly = false }: TabProps = $props();

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);
	let isEmailVerified = $derived(currentUser?.email_verified ?? true);

	// Check if plan has api_access feature before querying
	const organizationQuery = useOrganizationQuery();
	let hasApiAccess = $derived.by(() => {
		const org = organizationQuery.data;
		if (!org?.plan) return false;
		return billingPlans.getMetadata(org.plan.type).features.api_access;
	});

	// Queries
	const tagsQuery = useTagsQuery();
	const userApiKeysQuery = useUserApiKeysQuery({ enabled: () => hasApiAccess });
	const sitesQuery = useSitesQuery();

	// Mutations
	const updateMutation = useUpdateUserApiKeyMutation();
	const deleteMutation = useDeleteUserApiKeyMutation();
	const bulkDeleteMutation = useBulkDeleteUserApiKeysMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let userApiKeysData = $derived(userApiKeysQuery.data ?? []);
	let sitesData = $derived(sitesQuery.data ?? []);
	let isLoading = $derived(userApiKeysQuery.isPending);

	let showModal = $state(false);
	let editingApiKey = $state<UserApiKey | null>(null);

	// Deep-link: open user API key editor from URL
	$effect(() => {
		if ($modalState.name === 'user-api-key' && !showModal) {
			if ($modalState.id) {
				const entity = userApiKeysData.find((e) => e.id === $modalState.id);
				if (entity) {
					editingApiKey = entity;
					showModal = true;
				}
			} else {
				editingApiKey = null;
				showModal = true;
			}
		}
	});

	async function handleDelete(apiKey: UserApiKey) {
		if (confirm(common_confirmDeleteName({ name: apiKey.name }))) {
			deleteMutation.mutate(apiKey.id);
		}
	}

	async function handleUpdate(apiKey: UserApiKey) {
		await updateMutation.mutateAsync(apiKey);
		showModal = false;
		editingApiKey = null;
	}

	function handleCreate() {
		showModal = true;
		editingApiKey = null;
	}

	function handleClose() {
		showModal = false;
		editingApiKey = null;
	}

	function handleEdit(apiKey: UserApiKey) {
		showModal = true;
		editingApiKey = apiKey;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_apiKeys() }))) {
			await bulkDeleteMutation.mutateAsync(ids);
		}
	}

	function getUserApiKeyTags(apiKey: UserApiKey): string[] {
		return apiKey.tags ?? [];
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('UserApiKey', {});
	}

	/** Row actions. */
	function userApiKeyActions(apiKey: UserApiKey): CardAction[] {
		return [
			{ label: common_edit(), icon: Edit, onClick: () => handleEdit(apiKey) },
			{
				label: common_delete(),
				icon: Trash2,
				class: 'btn-icon-danger',
				onClick: () => handleDelete(apiKey)
			}
		];
	}

	const tableDefaults: TableDefaults<string> = { sort: { field: 'name', direction: 'asc' } };

	const apiKeyFields: FieldConfig<UserApiKey>[] = [
		{
			key: 'name',
			label: common_name(),
			type: 'string',
			searchable: true,
			sortable: true
		},
		{
			key: 'permissions',
			type: 'string',
			label: common_permissions(),
			searchable: true,
			filterable: true,
			groupable: true,
			sortable: true,
			// Every role the backend defines, named the way the chip renders them.
			filterOptions: permissions.getItems().map((role) => permissions.getName(role.id)),
			getValue: (item) =>
				item.permissions ? permissions.getName(item.permissions) || item.permissions : null,
			display: {
				getItems: (item) => {
					const role = item.permissions;
					if (!role) return [];
					return [
						{
							id: role,
							label: permissions.getName(role) || role,
							color: permissions.getColorHelper(role).color
						}
					];
				}
			}
		},
		{
			key: 'site_ids',
			type: 'array',
			label: common_sites(),
			searchable: true,
			// Keys share sites, so this filters; as an array it neither sorts nor groups.
			filterable: true,
			getValue(item) {
				const ids = item.site_ids ?? [];
				return ids
					.map((id) => sitesData.find((n) => n.id === id)?.name)
					.filter((name): name is string => !!name);
			},
			display: { getItems: (item) => siteItems(item.site_ids, sitesData) }
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
				return (entity.tags ?? [])
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
	<TabHeader title={common_apiKeys()} subtitle={userApiKeys_subtitle()}>
		<svelte:fragment slot="actions">
			{#if !isReadOnly && hasApiAccess}
				{#if !isEmailVerified}
					<span data-tooltip={userApiKeys_verifyEmailToCreate()} use:tooltip>
						<button class="btn-primary flex items-center opacity-50" disabled>
							<Plus class="h-5 w-5" />{common_create()}
						</button>
					</span>
				{:else}
					<button class="btn-primary flex items-center" onclick={handleCreate}>
						<Plus class="h-5 w-5" />{common_create()}
					</button>
				{/if}
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if !hasApiAccess}
		<EmptyState
			title={userApiKeys_apiAccessUnavailableTitle()}
			subtitle={userApiKeys_apiAccessUnavailableSubtitle()}
		>
			<UpgradeButton feature="api_access" surface="api_keys_tab" />
		</EmptyState>
	{:else if isLoading}
		<Loading />
	{:else if userApiKeysData.length === 0}
		<EmptyState
			title={userApiKeys_noApiKeysYet()}
			subtitle={userApiKeys_noApiKeysSubtitle()}
			onClick={isEmailVerified ? handleCreate : undefined}
			cta={isEmailVerified ? common_create() : undefined}
		/>
	{:else}
		<DataControls
			items={userApiKeysData}
			fields={apiKeyFields}
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'UserApiKey'}
			getItemTags={getUserApiKeyTags}
			storageKey="scanopy-user-api-keys-table-state"
			defaults={tableDefaults}
			getItemId={(item) => item.id}
			getActions={userApiKeyActions}
			onCsvExport={handleCsvExport}
		></DataControls>
	{/if}
</div>

<UserApiKeyModal
	name="user-api-key"
	isOpen={showModal}
	onClose={handleClose}
	onUpdate={handleUpdate}
	apiKey={editingApiKey}
/>
