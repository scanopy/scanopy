<script lang="ts">
	import {
		useCredentialsQuery,
		useCreateCredentialMutation,
		useUpdateCredentialMutation,
		useDeleteCredentialMutation,
		useBulkDeleteCredentialsMutation
	} from '../queries';
	import CredentialEditModal from './CredentialEditModal.svelte';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import type { Credential } from '../types/base';
	import type { CredentialOrderField } from '../types/base';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import {
		defineFields,
		entityRef,
		type CardAction,
		type CardFieldItem
	} from '$lib/shared/components/data/types';
	import type { Network } from '$lib/features/networks/types';
	import { Plus, Trash2, Edit } from 'lucide-svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import {
		permissions,
		credentialTypes,
		billingPlans,
		entities
	} from '$lib/shared/stores/metadata';
	import {
		getCredentialTypeId,
		getStabilityTagProps,
		getTargetTagProps,
		getUpstreamSupportTagProps
	} from '$lib/features/credentials/types/base';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import type { Host } from '$lib/features/hosts/types/base';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import osFamilies from '$lib/data/os-families.json';
	import {
		common_confirmDeleteName,
		common_create,
		common_created,
		common_delete,
		common_description,
		common_edit,
		common_name,
		common_type,
		common_updated,
		credentials_bulkDeleteConfirm,
		credentials_bulkDeleteImpact,
		credentials_daemonOs,
		credentials_deleteImpact,
		credentials_emptySubtitle,
		credentials_subtitle,
		common_credentials,
		common_hosts,
		common_networks,
		common_notApplicable,
		common_noEntityYet,
		common_targets
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

	// Networks for delete impact preview
	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);

	// Derived state
	let credentials = $derived(credentialsQuery.data ?? []);

	// Which hosts a credential is assigned to is already on the credential —
	// `host_assignments` is hydrated from the same `host_credentials` junction
	// table that produces the host's `credential_assignments`. So the impact
	// counts need no host data at all, and the card's host chips only need those
	// ids resolved to names.
	//
	// This was `useHostsQuery({ limit: 0 })`: every host in the organisation,
	// unpaginated (~1.9MB on a 440-host estate), to label a few chips and put two
	// numbers in a confirm() dialog. Because TanStack dedupes by key it was shared
	// with every other consumer, so it loaded on pages that never showed a
	// credential.
	let assignedHostIds = $derived([
		...new Set(credentials.flatMap((c) => (c.host_assignments ?? []).map((a) => a.host_id)))
	]);
	const assignedHostsQuery = useHostsByIds(() => assignedHostIds);
	let assignedHostsData = $derived(assignedHostsQuery.data ?? []);

	function hostsForCredential(credential: Credential): Host[] {
		const ids = new Set((credential.host_assignments ?? []).map((a) => a.host_id));
		return assignedHostsData.filter((h) => ids.has(h.id));
	}

	function networksForCredential(credential: Credential): Network[] {
		return networksData.filter((n) => (n.credential_ids ?? []).includes(credential.id));
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
	 * A Docker socket credential cannot target networks at all, which is a
	 * different statement from a SNMP credential that targets networks and
	 * happens to have none. The card drew that distinction and the table should
	 * too, so an out-of-scope assignment column says so rather than sitting empty.
	 */
	function assignmentItems(applicable: boolean, items: CardFieldItem[]): CardFieldItem[] {
		if (applicable) return items;
		return [{ id: 'not-applicable', label: common_notApplicable(), color: 'Gray' }];
	}
	let isLoading = $derived(credentialsQuery.isLoading);

	// Demo mode check
	let isDemoOrg = $derived(
		billingPlans.getMetadata(organization?.plan?.type ?? null).is_demo === true
	);
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

	/** Row actions for table mode, matching what the card offers. */
	function credentialActions(credential: Credential): CardAction[] {
		if (!canManage) return [];

		return [
			{ label: common_edit(), icon: Edit, onClick: () => handleEditCredential(credential) },
			{
				label: common_delete(),
				icon: Trash2,
				class: 'btn-icon-danger',
				onClick: () => handleDeleteCredential(credential)
			}
		];
	}

	function handleEditCredential(credential: Credential) {
		editingCredential = credential;
		showCredentialEditor = true;
	}

	async function handleDeleteCredential(credential: Credential) {
		const affectedNetworks = networksData.filter((n) =>
			(n.credential_ids ?? []).includes(credential.id)
		);
		const affectedHostCount = (credential.host_assignments ?? []).length;
		let message: string = common_confirmDeleteName({ name: credential.name });
		if (affectedNetworks.length > 0 || affectedHostCount > 0) {
			message +=
				'\n\n' +
				credentials_deleteImpact({
					networkCount: affectedNetworks.length,
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
		// Distinct hosts across the selected credentials — a host assigned two of
		// them must not be counted twice.
		const affectedHostCount = new Set(
			credentials
				.filter((c) => ids.includes(c.id))
				.flatMap((c) => (c.host_assignments ?? []).map((a) => a.host_id))
		).size;
		let message: string = credentials_bulkDeleteConfirm({ count: ids.length });
		if (affectedNetworks.length > 0 || affectedHostCount > 0) {
			message +=
				'\n\n' +
				credentials_bulkDeleteImpact({
					networkCount: affectedNetworks.length,
					hostCount: affectedHostCount
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
			// Identity field: grouping by it would render a header per credential.
			name: {
				label: common_name(),
				type: 'string',
				searchable: true,
				groupable: false,
				display: { primary: true, width: 220 }
			},
			created_at: { label: common_created(), type: 'date', display: { hiddenByDefault: true } },
			updated_at: { label: common_updated(), type: 'date', display: { hiddenByDefault: true } }
		},
		[
			{
				// Set only on credentials that read files or sockets on the daemon; empty otherwise.
				key: 'daemon_os',
				label: credentials_daemonOs(),
				type: 'string',
				filterable: true,
				groupable: true,
				getValue: (item: Credential) =>
					item.daemon_os
						? (osFamilies.find((family) => family.id === item.daemon_os)?.name ?? item.daemon_os)
						: null,
				display: { hiddenByDefault: true }
			},
			{
				key: 'credential_type',
				label: common_type(),
				type: 'string',
				searchable: true,
				filterable: true,
				// Type is the axis credentials are actually organized by.
				groupable: true,
				sortable: true,
				filterMode: 'include',
				filterOptions: credentialTypes.getItems().map((t) => t.name ?? t.id),
				getValue: (item: Credential) => credentialTypes.getName(getCredentialTypeId(item)),
				display: {
					// Beta and unofficial-API ride with the type, which is what they qualify, and so
					// land ahead of the scope column — the same order `CredentialTypeDisplay` uses in
					// the wizard and the type dropdown. Without them a beta credential looked no
					// different here from a stable one.
					getItems: (item: Credential) => {
						const typeId = getCredentialTypeId(item);
						const meta = credentialTypes.getMetadata(typeId);
						return [
							{
								id: typeId,
								label: credentialTypes.getName(typeId),
								color: credentialTypes.getColorHelper(typeId).color,
								icon: credentialTypes.getIconComponent(typeId)
							},
							...[
								getStabilityTagProps(meta?.stability),
								getUpstreamSupportTagProps(meta?.upstream_support)
							]
								.filter((tag) => tag !== null)
								.map((tag) => ({ id: `${typeId}-${tag.label}`, ...tag }))
						];
					}
				}
			},
			{ key: 'description', label: common_description(), type: 'string', searchable: true },
			{
				// Assignments were card-only, so the credentials table could not show
				// what a credential actually applies to.
				key: 'assigned_networks',
				label: common_networks(),
				type: 'array',
				searchable: true,
				getValue: (item: Credential) => networksForCredential(item).map((n) => n.name),
				display: {
					getItems: (item: Credential) =>
						assignmentItems(
							targetsFor(item).includes('Network'),
							networksForCredential(item).map((network) => ({
								id: network.id,
								label: network.name,
								color: entities.getColorHelper('Network').color,
								entityRef: entityRef('Network', network.id, network)
							}))
						)
				}
			},
			{
				key: 'assigned_hosts',
				label: common_hosts(),
				type: 'array',
				searchable: true,
				getValue: (item: Credential) => hostsForCredential(item).map(hostDisplayName),
				display: {
					getItems: (item: Credential) =>
						assignmentItems(
							// DaemonHost-only credentials (Docker/Podman sockets) are
							// assigned to their daemon's host through the same junction.
							targetsFor(item).some((t) => t === 'Hosts' || t === 'DaemonHost'),
							hostsForCredential(item).map((host) => ({
								id: host.id,
								label: hostDisplayName(host),
								// A daemon-host credential reaches that host *through* its
								// daemon, so it reads as a daemon relationship rather than a
								// host one. Only credentials that target hosts generally get
								// the host colour.
								color: entities.getColorHelper(daemonHostOnly(item) ? 'Daemon' : 'Host').color,
								entityRef: entityRef('Host', host.id, host)
							}))
						)
				}
			},
			{
				key: 'target',
				label: common_targets(),
				type: 'array',
				searchable: true,
				filterable: true,
				groupable: true,
				filterMode: 'include',
				filterOptions: ['Network', 'Hosts', 'DaemonHost'],
				getValue: (item: Credential) => {
					const typeId = getCredentialTypeId(item);
					const meta = credentialTypes.getMetadata(typeId);
					return meta?.targets ?? [];
				},
				display: {
					// Off by default: the target set is a property of the credential *type*, so it repeats
					// down the column for every credential of the same type and earns its width
					// only when someone is actually sorting or filtering by it. Still filterable
					// and groupable, and still shown on the cards.
					hiddenByDefault: true,
					// Same chip props the card uses, so a target reads identically in
					// both views rather than falling back to undifferentiated grey.
					getItems: (item: Credential) => {
						const meta = credentialTypes.getMetadata(getCredentialTypeId(item));
						return (meta?.targets ?? []).map((target: string) => ({
							id: target,
							...getTargetTagProps(target)
						}));
					}
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
			title={common_noEntityYet({ entity: common_credentials() })}
			subtitle={credentials_emptySubtitle()}
			onClick={canManage ? handleCreateCredential : undefined}
			cta={canManage ? common_create() : ''}
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
			getIcon={(credential) => ({
				icon: credentialTypes.getIconComponent(getCredentialTypeId(credential)),
				color: credentialTypes.getColorHelper(getCredentialTypeId(credential)).icon
			})}
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
