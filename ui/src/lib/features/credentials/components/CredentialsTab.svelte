<script lang="ts">
	import {
		useCredentialsQuery,
		useCreateCredentialMutation,
		useUpdateCredentialMutation,
		useDeleteCredentialMutation,
		useBulkDeleteCredentialsMutation
	} from '../queries';
	import CredentialCard from './CredentialCard.svelte';
	import CredentialEditModal from './CredentialEditModal.svelte';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import type { Credential } from '../types/base';
	import type { CredentialOrderField } from '../types/base';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields } from '$lib/shared/components/data/types';
	import { Plus } from 'lucide-svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { permissions, credentialTypes } from '$lib/shared/stores/metadata';
	import { getCredentialTypeId } from '$lib/features/credentials/types/base';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import {
		common_confirmDeleteName,
		common_create,
		common_created,
		common_name,
		common_updated,
		credentials_bulkDeleteConfirm,
		credentials_bulkDeleteImpact,
		credentials_deleteImpact,
		credentials_subtitle,
		common_credentials,
		common_scope
	} from '$lib/paraglide/messages';

	let { isReadOnly = false }: TabProps = $props();

	let showCredentialEditor = $state(false);
	let editingCredential: Credential | null = $state(null);

	// Deep-link: open credential editor from URL
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'credential-editor',
			credentials,
			showCredentialEditor,
			editingCredential?.id
		);
		if (result !== undefined) {
			editingCredential = result;
			showCredentialEditor = true;
		}
	});

	// Queries and mutations
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	const credentialsQuery = useCredentialsQuery();
	const createCredentialMutation = useCreateCredentialMutation();
	const updateCredentialMutation = useUpdateCredentialMutation();
	const deleteCredentialMutation = useDeleteCredentialMutation();
	const bulkDeleteCredentialsMutation = useBulkDeleteCredentialsMutation();

	// Networks and hosts for delete impact preview
	const networksQuery = useNetworksQuery();
	const hostsQuery = useHostsQuery({ limit: 0 });
	let networksData = $derived(networksQuery.data ?? []);
	let hostsData = $derived(hostsQuery.data?.items ?? []);

	// Derived state
	let credentials = $derived(credentialsQuery.data ?? []);
	let isLoading = $derived(credentialsQuery.isLoading);

	// Demo mode check
	let isDemoOrg = $derived(organization?.plan?.type === 'Demo');
	let isNonOwnerInDemo = $derived(isDemoOrg && currentUser?.permissions !== 'Owner');

	let canManage = $derived(
		!isReadOnly &&
			!isNonOwnerInDemo &&
			currentUser &&
			permissions.getMetadata(currentUser.permissions).manage_org_entities
	);

	let allowBulkDelete = $derived(
		!isReadOnly && !isNonOwnerInDemo && currentUser
			? permissions.getMetadata(currentUser.permissions).manage_org_entities
			: false
	);

	function handleCreateCredential() {
		editingCredential = null;
		showCredentialEditor = true;
	}

	function handleEditCredential(credential: Credential) {
		editingCredential = credential;
		showCredentialEditor = true;
	}

	async function handleDeleteCredential(credential: Credential) {
		const affectedNetworks = networksData.filter((n) =>
			(n.credential_ids ?? []).includes(credential.id)
		);
		const affectedHosts = hostsData.filter((h) =>
			(h.credential_assignments ?? []).some((a) => a.credential_id === credential.id)
		);
		let message: string = common_confirmDeleteName({ name: credential.name });
		if (affectedNetworks.length > 0 || affectedHosts.length > 0) {
			message +=
				'\n\n' +
				credentials_deleteImpact({
					networkCount: affectedNetworks.length,
					hostCount: affectedHosts.length
				});
		}
		if (confirm(message)) {
			await deleteCredentialMutation.mutateAsync(credential.id);
		}
	}

	async function handleCredentialCreate(data: Credential) {
		await createCredentialMutation.mutateAsync(data);
		showCredentialEditor = false;
		editingCredential = null;
	}

	async function handleCredentialUpdate(_id: string, data: Credential) {
		await updateCredentialMutation.mutateAsync(data);
		showCredentialEditor = false;
		editingCredential = null;
	}

	function handleCloseCredentialEditor() {
		showCredentialEditor = false;
		editingCredential = null;
	}

	async function handleBulkDelete(ids: string[]) {
		const affectedNetworks = networksData.filter((n) =>
			(n.credential_ids ?? []).some((id) => ids.includes(id))
		);
		const affectedHosts = hostsData.filter((h) =>
			(h.credential_assignments ?? []).some((a) => ids.includes(a.credential_id))
		);
		let message: string = credentials_bulkDeleteConfirm({ count: ids.length });
		if (affectedNetworks.length > 0 || affectedHosts.length > 0) {
			message +=
				'\n\n' +
				credentials_bulkDeleteImpact({
					networkCount: affectedNetworks.length,
					hostCount: affectedHosts.length
				});
		}
		if (confirm(message)) {
			await bulkDeleteCredentialsMutation.mutateAsync(ids);
		}
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Credential', {});
	}

	function getCredentialTags(credential: Credential): string[] {
		return credential.tags;
	}

	// Define field configuration for the DataTableControls
	const credentialFields = defineFields<Credential, CredentialOrderField>(
		{
			name: { label: common_name(), type: 'string', searchable: true },
			created_at: { label: common_created(), type: 'date' },
			updated_at: { label: common_updated(), type: 'date' }
		},
		[
			{
				key: 'credential_type',
				label: 'Type',
				type: 'string',
				filterable: true,
				filterMode: 'include',
				filterOptions: credentialTypes.getItems().map((t) => t.name ?? t.id),
				getValue: (item: Credential) => credentialTypes.getName(getCredentialTypeId(item))
			},
			{
				key: 'scope_model',
				label: common_scope(),
				type: 'array',
				filterable: true,
				groupable: true,
				filterMode: 'include',
				filterOptions: ['Broadcast', 'PerHost'],
				getValue: (item: Credential) => {
					const typeId = getCredentialTypeId(item);
					const meta = credentialTypes.getMetadata(typeId);
					return meta?.scope_models ?? [];
				}
			}
		]
	);
</script>

<div class="space-y-6">
	<TabHeader title={common_credentials()} subtitle={credentials_subtitle()}>
		<svelte:fragment slot="actions">
			{#if canManage}
				<button class="btn-primary flex items-center" onclick={handleCreateCredential}>
					<Plus class="h-5 w-5" />{common_create()}
				</button>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if isLoading}
		<Loading />
	{:else if credentials.length === 0}
		<EmptyState
			title="No credentials yet"
			subtitle="Create credentials to authenticate with network devices and services."
			onClick={handleCreateCredential}
			cta={common_create()}
		/>
	{:else}
		<DataControls
			items={credentials}
			fields={credentialFields}
			{allowBulkDelete}
			storageKey="scanopy-credentials-table-state"
			onBulkDelete={handleBulkDelete}
			entityType={allowBulkDelete ? 'Credential' : undefined}
			getItemTags={getCredentialTags}
			getItemId={(item) => item.id}
			onCsvExport={handleCsvExport}
		>
			{#snippet children(
				item: Credential,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<CredentialCard
					credential={item}
					assignedNetworks={networksData.filter((n) => (n.credential_ids ?? []).includes(item.id))}
					assignedHosts={hostsData.filter((h) =>
						(h.credential_assignments ?? []).some((a) => a.credential_id === item.id)
					)}
					selected={isSelected}
					{onSelectionChange}
					{viewMode}
					onEdit={handleEditCredential}
					onDelete={handleDeleteCredential}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<CredentialEditModal
	name="credential-editor"
	isOpen={showCredentialEditor}
	credential={editingCredential}
	onCreate={handleCredentialCreate}
	onUpdate={handleCredentialUpdate}
	onClose={handleCloseCredentialEditor}
	onDelete={editingCredential
		? () => {
				handleDeleteCredential(editingCredential!);
				handleCloseCredentialEditor();
			}
		: null}
/>
