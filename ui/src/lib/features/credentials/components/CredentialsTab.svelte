<script lang="ts">
	import CreateButton from '$lib/shared/components/data/controls/CreateButton.svelte';
	import {
		useCredentialsQuery,
		useCredentialsListQuery,
		useCreateCredentialMutation,
		useUpdateCredentialMutation,
		useDeleteCredentialMutation,
		useBulkDeleteCredentialsMutation
	} from '../queries';
	import CredentialEditModal from './CredentialEditModal.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import type { Credential } from '../types/base';
	import type { CredentialOrderField } from '../types/base';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import {
		disabledWhen,
		type CardAction,
		type CardFieldItem
	} from '$lib/shared/components/data/types';
	import { credentialColumns } from './CredentialsTab.columns';
	import { collectionLookup } from '$lib/shared/entity-fields';
	import type { Site } from '$lib/features/sites/types';
	import { Trash2, Edit } from '@lucide/svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import { getCredentialTypeId, isLoginType } from '$lib/features/credentials/types/base';
	import {
		modalState,
		openModal,
		resolveModalDeepLink,
		type ModalState
	} from '$lib/shared/stores/modal-registry';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { listPageState } from '$lib/shared/components/data/listPage.svelte';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import type { Host } from '$lib/features/hosts/types/base';
	import {
		common_confirmDeleteName,
		common_create,
		common_delete,
		common_edit,
		credentials_bulkDeleteConfirm,
		credentials_bulkDeleteImpact,
		credentials_deleteImpact,
		credentials_emptyNoDaemonHint,
		credentials_emptySubtitle,
		credentials_subtitle,
		common_credentials,
		common_notApplicable,
		common_noEntityYet,
		credentials_defaultCantDelete,
		credentials_loginInUse,
		credentials_loginInUseBulk,
		credentials_usedBy,
		gettingStarted_stepDaemonLabel
	} from '$lib/paraglide/messages';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import type { components } from '$lib/api/schema';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];

	import { useCan } from '$lib/features/auth/access';

	let { isActive = false }: TabProps = $props();
	const can = useCan();

	let showCredentialEditor = $state(false);
	let editingCredential: Credential | null = $state(null);
	let prefill = $state<ModalState['prefill']>();

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
			// Create mode opened from a prompt carries its integration and hosts in the URL.
			prefill = result === null ? $modalState.prefill : undefined;
			editingCredential = result;
			showCredentialEditor = true;
		}
	});

	// Queries and mutations
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);
	let onboarding = $derived((organization?.onboarding ?? []) as OnboardingOperation[]);

	// With no daemon there are no hosts to assign to yet. The daemon setup's credentials step
	// targets devices by IP address instead, so the empty page points there too.
	function handleInstallDaemon() {
		window.location.hash = 'daemons';
		openModal('create-daemon');
	}

	// Every credential, for the deep link: it can name one on another page.
	const credentialsQuery = useCredentialsQuery();
	// The page the table shows, paged, filtered and ordered on the server.
	const page = listPageState(() => isActive);
	const credentialsPageQuery = useCredentialsListQuery(() => page.params());
	const createCredentialMutation = useCreateCredentialMutation();
	const updateCredentialMutation = useUpdateCredentialMutation();
	const deleteCredentialMutation = useDeleteCredentialMutation();
	const bulkDeleteCredentialsMutation = useBulkDeleteCredentialsMutation();

	const tagsQuery = useTagsQuery();
	let tagsData = $derived(tagsQuery.data ?? []);

	// Sites for delete impact preview
	const sitesQuery = useSitesQuery();
	let sitesData = $derived(sitesQuery.data ?? []);

	// Derived state
	let credentials = $derived(credentialsQuery.data ?? []);
	let pageCredentials = $derived(credentialsPageQuery.data?.items ?? []);
	let credentialsPagination = $derived(credentialsPageQuery.data?.pagination ?? null);

	// Which hosts a credential is assigned to is already on the credential —
	// `host_assignments` is hydrated from the same `host_credentials` junction
	// table that produces the host's `credential_assignments`. So the impact
	// counts need no host data at all, and the hosts column's chips only need those
	// ids resolved to names.
	//
	// This was `useHostsQuery({ limit: 0 })`: every host in the organisation,
	// unpaginated (~1.9MB on a 440-host estate), to label a few chips and put two
	// numbers in a confirm() dialog. Because TanStack dedupes by key it was shared
	// with every other consumer, so it loaded on pages that never showed a
	// credential.
	let assignedHostIds = $derived([
		...new Set(pageCredentials.flatMap((c) => (c.host_assignments ?? []).map((a) => a.host_id)))
	]);
	const assignedHostsQuery = useHostsByIds(() => assignedHostIds);
	let assignedHostsData = $derived(assignedHostsQuery.data ?? []);

	function hostsForCredential(credential: Credential): Host[] {
		const ids = new Set((credential.host_assignments ?? []).map((a) => a.host_id));
		return assignedHostsData.filter((h) => ids.has(h.id));
	}

	function sitesForCredential(credential: Credential): Site[] {
		return sitesData.filter((n) => (n.credential_ids ?? []).includes(credential.id));
	}

	/** The scopes a credential's type can target at all. */
	function targetsFor(credential: Credential): string[] {
		return credentialTypes.getMetadata(getCredentialTypeId(credential))?.targets ?? [];
	}

	/** Reaches a host only as some daemon's host, never hosts in general. */
	function daemonHostOnly(credential: Credential): boolean {
		const targets = targetsFor(credential);
		return targets.includes('DaemonHost') && !targets.includes('Hosts');
	}

	/**
	 * "Not applicable" is not the same as "none assigned".
	 *
	 * A Docker socket credential cannot target sites at all, which is a
	 * different statement from a SNMP credential that targets sites and
	 * happens to have none, so an out-of-scope assignment column says so rather
	 * than sitting empty.
	 */
	function assignmentItems(applicable: boolean, items: CardFieldItem[]): CardFieldItem[] {
		if (applicable) return items;
		return [{ id: 'not-applicable', label: common_notApplicable(), color: 'Gray' }];
	}

	/** A login is assigned nowhere; what it has is the credentials that run over it. */
	function loginUsage(credential: Credential): CardFieldItem[] | null {
		if (!isLoginType(getCredentialTypeId(credential))) return null;
		return [
			{
				id: 'used-by',
				label: credentials_usedBy({ count: credential.used_by.length }),
				color: 'Gray'
			}
		];
	}
	let isLoading = $derived(credentialsPageQuery.isPending && !credentialsPageQuery.data);
	let pageLoad = $derived(loadFailure(organizationQuery, credentialsPageQuery));

	let canManage = $derived(can('Credential', 'Write'));

	function handleCreateCredential() {
		prefill = undefined;
		editingCredential = null;
		showCredentialEditor = true;
	}

	/** Row actions. */
	function credentialActions(credential: Credential): CardAction[] {
		if (!canManage) return [];

		return [
			{
				label: common_edit(),
				icon: Edit,
				isOpen: true,
				onClick: () => handleEditCredential(credential)
			},
			{
				label: common_delete(),
				icon: Trash2,
				tone: 'danger',
				isDelete: true,
				onBulk: handleBulkDelete,
				onClick: () => handleDeleteCredential(credential),
				unavailable:
					disabledWhen(credential.is_default, credentials_defaultCantDelete()) ??
					disabledWhen(
						credential.used_by.length > 0,
						credentials_loginInUse({ count: credential.used_by.length }),
						credentials_loginInUseBulk()
					)
			}
		];
	}

	function handleEditCredential(credential: Credential) {
		editingCredential = credential;
		showCredentialEditor = true;
	}

	async function handleDeleteCredential(credential: Credential) {
		const affectedSites = sitesData.filter((n) => (n.credential_ids ?? []).includes(credential.id));
		const affectedHostCount = (credential.host_assignments ?? []).length;
		let message: string = common_confirmDeleteName({ name: credential.name });
		if (affectedSites.length > 0 || affectedHostCount > 0) {
			message +=
				'\n\n' +
				credentials_deleteImpact({
					siteCount: affectedSites.length,
					hostCount: affectedHostCount
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
		prefill = undefined;
	}

	async function handleCredentialUpdate(_id: string, data: Credential) {
		await updateCredentialMutation.mutateAsync(data);
		showCredentialEditor = false;
		editingCredential = null;
	}

	function handleCloseCredentialEditor() {
		showCredentialEditor = false;
		editingCredential = null;
		prefill = undefined;
	}

	async function handleBulkDelete(ids: string[]) {
		const affectedSites = sitesData.filter((n) =>
			(n.credential_ids ?? []).some((id) => ids.includes(id))
		);
		// Distinct hosts across the selected credentials — a host assigned two of
		// them must not be counted twice.
		const affectedHostCount = new Set(
			credentials
				.filter((c) => ids.includes(c.id))
				.flatMap((c) => (c.host_assignments ?? []).map((a) => a.host_id))
		).size;
		let message: string = credentials_bulkDeleteConfirm({ count: ids.length });
		if (affectedSites.length > 0 || affectedHostCount > 0) {
			message +=
				'\n\n' +
				credentials_bulkDeleteImpact({
					siteCount: affectedSites.length,
					hostCount: affectedHostCount
				});
		}
		if (confirm(message)) {
			await bulkDeleteCredentialsMutation.mutateAsync(ids);
		}
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Credential', page.listQuery);
	}

	function getCredentialTags(credential: Credential): string[] {
		return credential.tags;
	}

	const tableDefaults: TableDefaults<CredentialOrderField> = {
		group: 'integration',
		sort: { field: 'name', direction: 'asc' }
	};

	// The registry's credential fields are all the credential's own, so the lookup holds nothing.
	const lookup = collectionLookup({});

	const credentialFields = credentialColumns({
		lookup: () => lookup,
		tags: () => tagsData,
		sites: () => sitesData,
		sitesForCredential,
		hostsForCredential,
		targetsFor,
		daemonHostOnly,
		assignmentItems,
		loginUsage
	});
</script>

<!-- The page's own actions, last in the table toolbar beside the filter and columns. -->
{#snippet toolbarActions()}
	{#if canManage}
		<CreateButton onclick={handleCreateCredential} />
	{/if}
{/snippet}

<div class="space-y-6">
	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if isLoading}
		<Loading />
	{:else if page.isEmpty(credentialsPageQuery)}
		<EmptyState
			title={common_noEntityYet({ entity: common_credentials() })}
			subtitle={credentials_emptySubtitle()}
			onClick={canManage ? handleCreateCredential : undefined}
			cta={canManage ? common_create() : ''}
		/>
		{#if canManage && !hasDaemon(onboarding)}
			<p class="-mt-6 text-center text-sm text-tertiary">
				{credentials_emptyNoDaemonHint()}
				<button type="button" class="text-link hover:underline" onclick={handleInstallDaemon}>
					{gettingStarted_stepDaemonLabel()}
				</button>
			</p>
		{/if}
	{:else}
		<DataControls
			title={common_credentials()}
			subtitle={credentials_subtitle()}
			{toolbarActions}
			items={pageCredentials}
			fields={credentialFields}
			storageKey="scanopy-credentials-table-state"
			defaults={tableDefaults}
			entityType="Credential"
			canEditTags={can('Tag', 'Write')}
			getItemTags={getCredentialTags}
			getItemId={(item) => item.id}
			serverPagination={credentialsPagination}
			onPageChange={page.onPageChange}
			onQueryChange={page.onQueryChange}
			serverQuery={{ stale: false }}
			onCsvExport={handleCsvExport}
			getActions={credentialActions}
			entityLabel={common_credentials()}
		></DataControls>
	{/if}
</div>

<CredentialEditModal
	name="credential-editor"
	isOpen={showCredentialEditor}
	credential={editingCredential}
	initialIntegration={prefill?.integration ?? null}
	initialHostIds={prefill?.hosts?.split(',') ?? []}
	initialSiteIds={prefill?.sites?.split(',') ?? []}
	onCreate={handleCredentialCreate}
	onUpdate={handleCredentialUpdate}
	onClose={handleCloseCredentialEditor}
	onDelete={editingCredential && !editingCredential.is_default
		? () => {
				handleDeleteCredential(editingCredential!);
				handleCloseCredentialEditor();
			}
		: null}
/>
