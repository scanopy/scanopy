<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { ModalTab } from '$lib/shared/components/layout/GenericModal.svelte';
	import type { ModalAction } from '$lib/shared/components/layout/modalActions';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { components } from '$lib/api/schema';
	import type { Credential } from '../types/base';
	import {
		createDefaultCredential,
		getCredentialTypeId,
		isDaemonHostOnly,
		isLoginType
	} from '../types/base';
	import { credentialIntegrations, credentialTypes, entities } from '$lib/shared/stores/metadata';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { pushError, pushWarning } from '$lib/shared/stores/feedback';
	import { pruneAssignmentsForTargets } from '../utils/credentialTargets';
	import { prefillCredentialName, typesForIntegration } from '../utils/prefill';
	import { useHostSummariesQuery } from '$lib/features/hosts/queries';
	import CredentialForm from './CredentialForm.svelte';
	import CredentialAssignmentsSection from './CredentialAssignmentsSection.svelte';
	import { submitForm, validateForm } from '$lib/shared/components/forms/form-context';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import { Info, KeyRound, Link } from '@lucide/svelte';
	import {
		common_assignments,
		common_back,
		common_credential,
		common_couldNotLoadOrganization,
		common_create,
		common_delete,
		common_deleting,
		common_details,
		common_editName,
		common_next,
		common_saving,
		common_update,
		credentials_assignmentsClearedOnTypeChange,
		credentials_createCredential,
		credentials_description,
		credentials_docsGuide,
		credentials_docsGuideLinkText,
		credentials_loginInUse,
		credentials_typeNeedsDaemon
	} from '$lib/paraglide/messages';

	type CredentialHostAssignment = components['schemas']['CredentialHostAssignment'];

	let {
		credential = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined,
		initialIntegration = null,
		initialHostIds = [],
		initialSiteIds = [],
		loginOnly = false
	}: {
		credential?: Credential | null;
		isOpen?: boolean;
		onCreate: (data: Credential) => Promise<void> | void;
		onUpdate: (id: string, data: Credential) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
		/** Create mode only: open for this integration, its type picker limited to its transports. */
		initialIntegration?: string | null;
		/** Create mode only: hosts the new credential starts assigned to. */
		initialHostIds?: string[];
		/** Create mode only: sites the new credential starts assigned to. */
		initialSiteIds?: string[];
		/** Create mode only: the type picker offers only SSH logins, for a login made from another
		 *  credential's login picker. */
		loginOnly?: boolean;
	} = $props();

	// Declared before the queries below, whose options TanStack reads as each query is created.
	let isEditing = $derived(credential !== null);

	// The transports a pre-filled create flow offers: those that can apply where it comes assigned,
	// falling back to all of the integration's when none can.
	let typeChoices = $derived.by(() => {
		if (isEditing) return undefined;
		const types = credentialTypes.getItems() ?? [];
		if (loginOnly) return types.filter((t) => isLoginType(t.id)).map((t) => t.id);
		if (!initialIntegration) return undefined;
		const target = initialSiteIds.length ? 'Site' : initialHostIds.length ? 'Hosts' : null;
		const fitting = typesForIntegration(types, initialIntegration, target);
		return fitting.length ? fitting : typesForIntegration(types, initialIntegration, null);
	});

	// A pre-filled credential starts named after its host, which the host ids alone cannot give.
	const prefillHostsQuery = useHostSummariesQuery(() => ({
		ids: initialHostIds,
		enabled: isOpen && !isEditing && !!initialIntegration && initialHostIds.length > 0
	}));
	let prefillName = $derived.by(() => {
		if (isEditing || !initialIntegration) return null;
		if (initialHostIds.length && !prefillHostsQuery.data) return null;
		const integrationName =
			credentialIntegrations.getName(initialIntegration) ?? initialIntegration;
		return prefillCredentialName(prefillHostsQuery.data?.items ?? [], integrationName);
	});

	// Seed the name once it is known, which can be after the modal opened. Never over a name the
	// user has started typing.
	$effect(() => {
		if (isOpen && prefillName && !form.state.values.name) {
			form.setFieldValue('name', prefillName);
		}
	});

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	// Hosts that run a daemon — the only hosts a daemon-host-only type (the local socket)
	// can be assigned to. `undefined` until the query resolves so a not-yet-loaded list is
	// never mistaken for "there are none", which would strip legitimate assignments on save.
	const daemonsQuery = useDaemonsQuery();
	let daemonHostIds = $derived(daemonsQuery.data?.map((d) => d.host_id));

	// With no daemon, a daemon-host-only type has nowhere it could ever apply, so it can't be
	// picked. The daemon setup creates those credentials alongside the daemon.
	function typeDisabledReason(typeId: string): string | null {
		if (daemonHostIds === undefined || daemonHostIds.length > 0) return null;
		return isDaemonHostOnly(credentialTypes.getMetadata(typeId)?.targets)
			? credentials_typeNeedsDaemon()
			: null;
	}

	let title = $derived(
		isEditing ? common_editName({ name: credential?.name ?? '' }) : credentials_createCredential()
	);

	let colorHelper = $derived(entities.getColorHelper('Credential'));

	let credentialFormRef: ReturnType<typeof CredentialForm> | undefined = $state();
	let loading = $state(false);
	let deleting = $state(false);
	// Tabs, which are also the create flow's steps, in order: who the credential is, what it
	// holds, where it applies. Creating walks them with Next and a stepper; editing jumps freely.
	let selectedTypeId = $state('SnmpCommunity');
	const allSteps = ['details', 'credential', 'assignments'] as const;
	type Step = (typeof allSteps)[number];
	// A login is never assigned, so it has no Assignments step.
	let steps = $derived<readonly Step[]>(
		isLoginType(selectedTypeId) ? allSteps.filter((s) => s !== 'assignments') : allSteps
	);
	let activeTab = $state<Step>('details');
	let furthestReached = $state(0);
	let stepIndex = $derived(steps.indexOf(activeTab));
	let isLastStep = $derived(stepIndex === steps.length - 1);
	let tabs: ModalTab[] = $derived(
		[
			{ id: 'details', label: common_details(), icon: Info },
			{ id: 'credential', label: common_credential(), icon: KeyRound },
			{ id: 'assignments', label: common_assignments(), icon: Link }
		]
			.filter((tab) => steps.includes(tab.id as Step))
			.map((tab, i) => ({ ...tab, disabled: !isEditing && i > furthestReached }))
	);
	let saveLabel = $derived(
		isEditing ? common_update() : isLastStep ? common_create() : common_next()
	);

	async function handlePrimary() {
		if (isEditing || isLastStep) {
			await handleSave();
			return;
		}
		// Only the current step's fields are mounted, so this checks just them, as in HostEditor.
		const valid = await validateForm(
			form,
			undefined,
			(path) => credentialFormRef?.fieldLabel(path) ?? path
		);
		if (!valid) return;
		furthestReached = Math.max(furthestReached, stepIndex + 1);
		activeTab = steps[stepIndex + 1];
	}

	function handleBack() {
		if (stepIndex > 0) activeTab = steps[stepIndex - 1];
	}

	// Assignment state (source of truth; synced into the submit payload).
	// The selected type drives which assignment surface(s) show; kept in sync via
	// CredentialForm's onTypeChange and reset on open.
	let assignedSiteIds = $state<string[]>([]);
	let hostAssignments = $state<CredentialHostAssignment[]>([]);

	// The form leaves out `source`: the server sets it, and its nested union is too deep for the
	// form's field-path types.
	type CredentialFormValues = Omit<Credential, 'source'>;
	function formValuesOf(c: Credential): CredentialFormValues {
		// eslint-disable-next-line @typescript-eslint/no-unused-vars
		const { source, ...values } = c;
		return values;
	}

	function getDefaultValues(): CredentialFormValues {
		if (credential) return formValuesOf(credential);
		if (organization) return formValuesOf(createDefaultCredential(organization.id));
		return formValuesOf(createDefaultCredential(''));
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
				{ assignedSiteIds, hostAssignments },
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
				source: credential?.source ?? { type: 'Manual' },
				organization_id: organization.id,
				credential_type: credentialType,
				daemon_os: credentialFormRef?.getDaemonOs() ?? null,
				login_credential_id: credentialFormRef?.getLoginCredentialId() ?? null,
				description: (formValues.description as string | null | undefined)?.trim() || null,
				assigned_site_ids: permitted.assignedSiteIds,
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
			{ assignedSiteIds, hostAssignments },
			daemonHostIds
		);
		if (!pruned.changed) return;
		assignedSiteIds = pruned.assignedSiteIds;
		hostAssignments = pruned.hostAssignments;
		pushWarning(credentials_assignmentsClearedOnTypeChange());
	}

	function handleOpen() {
		activeTab = 'details';
		furthestReached = 0;
		assignedSiteIds = credential?.assigned_site_ids ?? initialSiteIds;
		hostAssignments =
			credential?.host_assignments ??
			initialHostIds.map((host_id) => ({ host_id, ip_address_ids: null }));
		selectedTypeId = credential
			? getCredentialTypeId(credential)
			: (typeChoices?.[0] ?? 'SnmpCommunity');
		form.reset(getDefaultValues());
		credentialFormRef?.reset();
		// After both resets, which each clear the shared form's name.
		if (prefillName) form.setFieldValue('name', prefillName);
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
		// The login field unmounts off its step, so a save from Assignments checks it here.
		if (!credentialFormRef?.validateLogin()) {
			activeTab = 'credential';
			return;
		}
		loading = true;
		try {
			await submitForm(form, (path) => credentialFormRef?.fieldLabel(path) ?? path);
		} finally {
			loading = false;
		}
	}

	let actions = $derived<ModalAction[]>([
		...(isEditing && onDelete && credential
			? [
					{
						role: 'danger' as const,
						label: deleting ? common_deleting() : common_delete(),
						onClick: handleDelete,
						// A login still in use can't go; the server refuses it too.
						disabled: deleting || loading || credential.used_by.length > 0,
						tooltip:
							credential.used_by.length > 0
								? credentials_loginInUse({ count: credential.used_by.length })
								: undefined
					}
				]
			: []),
		...(!isEditing && stepIndex > 0
			? [{ role: 'secondary' as const, label: common_back(), onClick: handleBack }]
			: []),
		{
			role: 'primary',
			label: loading ? common_saving() : saveLabel,
			onClick: handlePrimary,
			disabled: loading || deleting
		}
	]);
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={credential?.id}
	{form}
	size="xl"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={true}
	{tabs}
	{activeTab}
	tabStyle={isEditing ? 'tabs' : 'stepper'}
	onTabChange={(id) => (activeTab = id as Step)}
	{actions}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Credential')} color={colorHelper.color} />
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col overflow-auto p-6">
		<!-- One form for both steps, kept mounted while Assignments shows, so field values and
		     modes survive moving between tabs. -->
		<div class="space-y-4" class:hidden={activeTab === 'assignments'}>
			{#if activeTab === 'details'}
				<p class="text-sm text-secondary">
					{credentials_description()}
				</p>
				<DocsHint
					text={credentials_docsGuide()}
					href="https://scanopy.net/docs/using-scanopy/credentials/"
					linkText={credentials_docsGuideLinkText()}
				/>
			{/if}

			<CredentialForm
				bind:this={credentialFormRef}
				{form}
				{credential}
				{typeChoices}
				{typeDisabledReason}
				onTypeChange={handleTypeChange}
				section={activeTab === 'credential' ? 'fields' : 'identity'}
				onSubmitRequest={handlePrimary}
			/>
		</div>

		{#if activeTab === 'assignments'}
			<CredentialAssignmentsSection
				credentialTypeId={selectedTypeId}
				credentialId={credential?.id}
				bind:assignedSiteIds
				bind:hostAssignments
			/>
		{/if}
	</div>

	{#if isEditing && credential}
		<EntityMetadataSection entities={[credential]} />
	{/if}
</GenericModal>
