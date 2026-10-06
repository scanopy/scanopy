<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { billingPlans, entities } from '$lib/shared/stores/metadata';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Site } from '../types';
	import { createEmptySiteFormData } from '../queries';
	import { pushError } from '$lib/shared/stores/feedback';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import DurationInput from '$lib/shared/components/forms/input/DurationInput.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import type { Credential } from '$lib/features/credentials/types/base';
	import { getCredentialTypeId } from '$lib/features/credentials/types/base';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { CredentialDisplay } from '$lib/shared/components/forms/selection/display/CredentialDisplay.svelte';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import {
		common_cancel,
		common_couldNotLoadUser,
		common_create,
		common_delete,
		common_deleting,
		common_editName,
		common_name,
		common_saving,
		common_update,
		common_credentialDemoReadOnly,
		common_credentials,
		credentials_selectToAddPlaceholder,
		sites_createSite,
		sites_credentialHelp,
		sites_credentialHelpLinkText,
		sites_siteNamePlaceholder,
		sites_staleAfterHours,
		sites_staleAfterHoursHelp,
		sites_noCredentialsAssigned
	} from '$lib/paraglide/messages';

	let {
		site = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined
	}: {
		site?: Site | null;
		isOpen?: boolean;
		onCreate: (data: Site) => Promise<void> | void;
		onUpdate: (id: string, data: Site) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
	} = $props();

	// TanStack Query for organization and current user
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	// Demo mode check
	let isDemoOrg = $derived(
		billingPlans.getMetadata(organization?.plan?.type ?? null).is_demo === true
	);
	let isNonOwnerInDemo = $derived(isDemoOrg && currentUser?.permissions !== 'Owner');

	// TanStack Query for credentials. Only Site-targetable types are offered for
	// assignment, but the *selected* list resolves against every credential (as the host
	// modal does): a credential whose type can't broadcast must still be visible here so it
	// can be removed. Resolving against the filtered list instead would render it as nothing
	// while still re-submitting its id on every unrelated site edit.
	const credentialsQuery = useCredentialsQuery();
	let allCredentials = $derived(credentialsQuery.data ?? []);
	let assignableCredentials = $derived(
		allCredentials.filter((c) => {
			const meta = credentialTypes.getMetadata(getCredentialTypeId(c));
			return (meta?.targets ?? []).includes('Site');
		})
	);

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(site !== null);
	let title = $derived(
		isEditing ? common_editName({ name: site?.name ?? '' }) : sites_createSite()
	);
	let saveLabel = $derived(isEditing ? common_update() : common_create());

	// Local state for selected credentials
	let selectedCredentialIds = $state<string[]>([]);

	// Resolve selected credential IDs to full credential objects
	let selectedCredentials = $derived(
		selectedCredentialIds
			.map((id) => allCredentials.find((c) => c.id === id))
			.filter((c): c is Credential => c != null)
	);

	function getDefaultValues() {
		return site ? { ...site, seedData: false } : { ...createEmptySiteFormData(), seedData: true };
	}

	// Create form
	const form = createForm(() => ({
		defaultValues: {
			...createEmptySiteFormData(),
			seedData: true
		},
		onSubmit: async ({ value }) => {
			if (!organization) {
				pushError(common_couldNotLoadUser());
				handleClose();
				return;
			}

			const siteData: Site = {
				...(value as Site),
				name: value.name.trim(),
				organization_id: organization.id,
				credential_ids: selectedCredentialIds
			};

			loading = true;
			try {
				if (isEditing && site) {
					await onUpdate(site.id, siteData);
				} else {
					await onCreate(siteData);
				}
			} finally {
				loading = false;
			}
		}
	}));

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		selectedCredentialIds = defaults.credential_ids ?? [];

		form.reset({
			...defaults
		});
	}

	function handleClose() {
		onClose();
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && site) {
			deleting = true;
			try {
				await onDelete(site.id);
			} finally {
				deleting = false;
			}
		}
	}

	let colorHelper = entities.getColorHelper('Site');
</script>

{#snippet siteCredentialHelpSnippet()}
	<DocsHint
		text={sites_credentialHelp()}
		href="https://scanopy.net/docs/using-scanopy/credentials/#where-a-credential-applies"
		linkText={sites_credentialHelpLinkText()}
	/>
{/snippet}

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={site?.id}
	{form}
	size="xl"
	onClose={handleClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Site')} color={colorHelper.color} />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="min-h-0 flex-1 overflow-auto p-6">
			<div class="space-y-8">
				<!-- Site Details Section -->
				<div class="space-y-4">
					<div class="grid grid-cols-1 items-start gap-4 sm:grid-cols-2">
						<form.Field
							name="name"
							validators={{
								onBlur: ({ value }) => required(value) || max(100)(value)
							}}
						>
							{#snippet children(field)}
								<TextInput
									label={common_name()}
									id="name"
									{field}
									placeholder={sites_siteNamePlaceholder()}
									required
								/>
							{/snippet}
						</form.Field>

						<form.Field name="stale_after_hours">
							{#snippet children(field)}
								<DurationInput
									label={sites_staleAfterHours()}
									id="stale_after_hours"
									{field}
									initialHours={site?.stale_after_hours ?? null}
									placeholderHours={site?.effective_stale_after_hours ?? null}
									helpText={sites_staleAfterHoursHelp()}
								/>
							{/snippet}
						</form.Field>
					</div>

					<form.Field name="tags">
						{#snippet children(field)}
							<TagPicker
								selectedTagIds={field.state.value || []}
								onChange={(tags) => field.handleChange(tags)}
							/>
						{/snippet}
					</form.Field>

					<!-- Credentials Selection -->
					<ListManager
						label={common_credentials()}
						helpSnippet={isNonOwnerInDemo ? undefined : siteCredentialHelpSnippet}
						helpText={isNonOwnerInDemo ? common_credentialDemoReadOnly() : undefined}
						placeholder={credentials_selectToAddPlaceholder()}
						emptyMessage={sites_noCredentialsAssigned()}
						allowReorder={false}
						options={assignableCredentials}
						items={selectedCredentials}
						optionDisplayComponent={CredentialDisplay}
						itemDisplayComponent={CredentialDisplay}
						onAdd={(id) => {
							if (!selectedCredentialIds.includes(id)) {
								selectedCredentialIds = [...selectedCredentialIds, id];
							}
						}}
						onRemove={(index) => {
							selectedCredentialIds = selectedCredentialIds.filter((_, i) => i !== index);
						}}
					/>
				</div>
			</div>
		</div>

		{#if isEditing && site}
			<EntityMetadataSection entities={[site]} />
		{/if}

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if isEditing && onDelete}
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
				<div class="flex items-center gap-3">
					<button
						type="button"
						disabled={loading || deleting}
						onclick={handleClose}
						class="btn-secondary"
					>
						{common_cancel()}
					</button>
					<button type="submit" disabled={loading || deleting} class="btn-primary">
						{loading ? common_saving() : saveLabel}
					</button>
				</div>
			</div>
		</div>
	</form>
</GenericModal>
