<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { ModalTab } from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { components } from '$lib/api/schema';
	import type { Credential } from '../types/base';
	import { createDefaultCredential, getCredentialTypeId } from '../types/base';
	import { credentialTypes, entities } from '$lib/shared/stores/metadata';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { pushError, pushWarning } from '$lib/shared/stores/feedback';
	import { pruneAssignmentsForTargets } from '../utils/credentialTargets';
	import CredentialForm from './CredentialForm.svelte';
	import CredentialAssignmentsSection from './CredentialAssignmentsSection.svelte';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import { Info, Link } from 'lucide-svelte';
	import {
		common_assignments,
		common_couldNotLoadOrganization,
		common_create,
		common_delete,
		common_deleting,
		common_details,
		common_editName,
		common_saving,
		common_update,
		credentials_assignmentsClearedOnTypeChange,
		credentials_createCredential,
		credentials_description,
		credentials_docsGuide,
		credentials_docsGuideLinkText
	} from '$lib/paraglide/messages';

	type CredentialHostAssignment = components['schemas']['CredentialHostAssignment'];

	let {
		credential = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined
	}: {
		credential?: Credential | null;
		isOpen?: boolean;
		onCreate: (data: Credential) => Promise<void> | void;
		onUpdate: (id: string, data: Credential) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
	} = $props();

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	// Hosts that run a daemon — the only hosts a daemon-host-only type (the local socket)
	// can be assigned to. `undefined` until the query resolves so a not-yet-loaded list is
	// never mistaken for "there are none", which would strip legitimate assignments on save.
	const daemonsQuery = useDaemonsQuery();
	let daemonHostIds = $derived(daemonsQuery.data?.map((d) => d.host_id));

	let isEditing = $derived(credential !== null);
	let title = $derived(
		isEditing ? common_editName({ name: credential?.name ?? '' }) : credentials_createCredential()
	);

	let colorHelper = $derived(entities.getColorHelper('Credential'));

	let credentialFormRef: ReturnType<typeof CredentialForm> | undefined = $state();
	let loading = $state(false);
	let deleting = $state(false);
	let saveLabel = $derived(isEditing ? common_update() : common_create());

	// Tabs
	let activeTab = $state('details');
	let tabs: ModalTab[] = $derived([
		{ id: 'details', label: common_details(), icon: Info },
		{ id: 'assignments', label: common_assignments(), icon: Link }
	]);

	// Assignment state (source of truth; synced into the submit payload).
	// The selected type drives which assignment surface(s) show; kept in sync via
	// CredentialForm's onTypeChange and reset on open.
	let selectedTypeId = $state('SnmpV2c');
	let assignedNetworkIds = $state<string[]>([]);
	let hostAssignments = $state<CredentialHostAssignment[]>([]);

	function getDefaultValues(): Credential {
		if (credential) return { ...credential };
		if (organization) return createDefaultCredential(organization.id);
		return createDefaultCredential('');
	}

	// Form owns the name field; CredentialForm handles the rest
	const form = createForm(() => ({
		defaultValues: getDefaultValues(),
		onSubmit: async ({ value }) => {
			if (!organization) {
				pushError(common_couldNotLoadOrganization());
				return;
			}

			const credentialType = credentialFormRef?.buildCredentialType();
			if (!credentialType) return;

			// Final gate on what the type actually permits. `handleTypeChange` already prunes
			// so the user sees a warning when they switch, but this is the choke point every
			// assignment edit passes through — it holds regardless of effect ordering, and of
			// any future surface that mutates the assignment state.
			const permitted = pruneAssignmentsForTargets(
				credentialTypes.getMetadata(credentialType.type)?.targets,
				{ assignedNetworkIds, hostAssignments },
				daemonHostIds
			);

			// `value` also carries CredentialForm's scratch state — `fields` (raw per-field
			// strings) and `targetIps` — because those inputs register under this form. Neither
			// is part of `Credential`, and `fields` would put uncoerced strings on the wire
			// beside the built `credential_type`, so drop both here.
			const formValues: Record<string, unknown> = { ...(value as Record<string, unknown>) };
			delete formValues.fields;
			delete formValues.targetIps;

			const credentialData: Credential = {
				...(formValues as unknown as Credential),
				organization_id: organization.id,
				credential_type: credentialType,
				daemon_os: credentialFormRef?.getDaemonOs() ?? null,
				description: (formValues.description as string | null | undefined)?.trim() || null,
				assigned_network_ids: permitted.assignedNetworkIds,
				host_assignments: permitted.hostAssignments
			};

			if (isEditing && credential) {
				await onUpdate(credential.id, credentialData);
			} else {
				await onCreate(credentialData);
			}
		}
	}));

	/**
	 * Adopt a newly selected credential type, dropping any assignment the new type's
	 * `targets` don't permit.
	 *
	 * The assignment surfaces are chosen by `targets`, so switching type hides a surface
	 * without clearing it — and the hidden value is still submitted. Guarded on an actual
	 * change because CredentialForm reports its type on mount too, which must not prune a
	 * credential that was just loaded for editing.
	 */
	function handleTypeChange(typeId: string) {
		const previous = selectedTypeId;
		selectedTypeId = typeId;
		// The type selector is disabled while editing, so a real switch only ever happens on a
		// new credential; re-notifications during open/reset report the id unchanged.
		if (isEditing || typeId === previous) return;

		const pruned = pruneAssignmentsForTargets(
			credentialTypes.getMetadata(typeId)?.targets,
			{ assignedNetworkIds, hostAssignments },
			daemonHostIds
		);
		if (!pruned.changed) return;
		assignedNetworkIds = pruned.assignedNetworkIds;
		hostAssignments = pruned.hostAssignments;
		pushWarning(credentials_assignmentsClearedOnTypeChange());
	}

	function handleOpen() {
		activeTab = 'details';
		assignedNetworkIds = credential?.assigned_network_ids ?? [];
		hostAssignments = credential?.host_assignments ?? [];
		selectedTypeId = credential ? getCredentialTypeId(credential) : 'SnmpV2c';
		form.reset(getDefaultValues());
		credentialFormRef?.reset();
	}

	async function handleDelete() {
		if (onDelete && credential) {
			deleting = true;
			try {
				await onDelete(credential.id);
			} finally {
				deleting = false;
			}
		}
	}

	async function handleSave() {
		loading = true;
		try {
			await submitForm(form, (path) => credentialFormRef?.fieldLabel(path) ?? path);
		} finally {
			loading = false;
		}
	}
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={credential?.id}
	size="xl"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={true}
	{tabs}
	{activeTab}
	onTabChange={(id) => (activeTab = id)}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Credential')} color={colorHelper.color} />
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col overflow-auto p-6">
		<div class="space-y-4" class:hidden={activeTab !== 'details'}>
			<p class="text-secondary text-sm">
				{credentials_description()}
			</p>
			<DocsHint
				text={credentials_docsGuide()}
				href="https://scanopy.net/docs/using-scanopy/credentials/"
				linkText={credentials_docsGuideLinkText()}
			/>

			<CredentialForm
				bind:this={credentialFormRef}
				{form}
				{credential}
				onTypeChange={handleTypeChange}
			/>
		</div>

		{#if activeTab === 'assignments'}
			<CredentialAssignmentsSection
				credentialTypeId={selectedTypeId}
				credentialId={credential?.id}
				bind:assignedNetworkIds
				bind:hostAssignments
			/>
		{/if}
	</div>

	{#if isEditing && credential}
		<EntityMetadataSection entities={[credential]} />
	{/if}

	{#snippet footer()}
		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if isEditing && onDelete && credential}
						<button
							type="button"
							disabled={deleting || loading}
							onclick={handleDelete}
							class="btn-danger"
						>
							{deleting ? common_deleting() : common_delete()}
						</button>
					{/if}
				</div>
				<button
					type="button"
					disabled={loading || deleting}
					class="btn-primary"
					onclick={handleSave}
				>
					{loading ? common_saving() : saveLabel}
				</button>
			</div>
		</div>
	{/snippet}
</GenericModal>
