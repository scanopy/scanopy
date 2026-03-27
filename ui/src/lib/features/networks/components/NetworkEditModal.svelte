<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Network } from '../types';
	import { createEmptyNetworkFormData } from '../queries';
	import { pushError } from '$lib/shared/stores/feedback';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
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
		common_details,
		common_editName,
		common_name,
		common_saving,
		common_update,
		common_credentialDemoReadOnly,
		networks_createNetwork,
		networks_credentialHelp,
		networks_credentialHelpLinkText,
		networks_networkNamePlaceholder
	} from '$lib/paraglide/messages';

	let {
		network = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined
	}: {
		network?: Network | null;
		isOpen?: boolean;
		onCreate: (data: Network) => Promise<void> | void;
		onUpdate: (id: string, data: Network) => Promise<void> | void;
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
	let isDemoOrg = $derived(organization?.plan?.type === 'Demo');
	let isNonOwnerInDemo = $derived(isDemoOrg && currentUser?.permissions !== 'Owner');

	// TanStack Query for credentials — filter to Broadcast-scoped types for network assignment
	const credentialsQuery = useCredentialsQuery();
	let allCredentials = $derived(
		(credentialsQuery.data ?? []).filter((c) => {
			const meta = credentialTypes.getMetadata(getCredentialTypeId(c));
			return (meta?.scope_models ?? []).includes('Broadcast');
		})
	);

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(network !== null);
	let title = $derived(
		isEditing ? common_editName({ name: network?.name ?? '' }) : networks_createNetwork()
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
		return network
			? { ...network, seedData: false }
			: { ...createEmptyNetworkFormData(), seedData: true };
	}

	// Create form
	const form = createForm(() => ({
		defaultValues: {
			...createEmptyNetworkFormData(),
			seedData: true
		},
		onSubmit: async ({ value }) => {
			if (!organization) {
				pushError(common_couldNotLoadUser());
				handleClose();
				return;
			}

			const networkData: Network = {
				...(value as Network),
				name: value.name.trim(),
				organization_id: organization.id,
				credential_ids: selectedCredentialIds
			};

			loading = true;
			try {
				if (isEditing && network) {
					await onUpdate(network.id, networkData);
				} else {
					await onCreate(networkData);
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
		if (onDelete && network) {
			deleting = true;
			try {
				await onDelete(network.id);
			} finally {
				deleting = false;
			}
		}
	}

	let colorHelper = entities.getColorHelper('Network');
</script>

{#snippet networkCredentialHelpSnippet()}
	<DocsHint
		text={networks_credentialHelp()}
		href="https://scanopy.net/docs/using-scanopy/credentials/#scope-models"
		linkText={networks_credentialHelpLinkText()}
	/>
{/snippet}

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={network?.id}
	size="xl"
	onClose={handleClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Network')} color={colorHelper.color} />
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
				<!-- Network Details Section -->
				<div class="space-y-4">
					<h3 class="text-primary text-lg font-medium">{common_details()}</h3>

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
								placeholder={networks_networkNamePlaceholder()}
								required
							/>
						{/snippet}
					</form.Field>

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
						label="Credentials"
						helpSnippet={isNonOwnerInDemo ? undefined : networkCredentialHelpSnippet}
						helpText={isNonOwnerInDemo ? common_credentialDemoReadOnly() : undefined}
						placeholder="Select a credential to add"
						emptyMessage="No credentials assigned"
						allowReorder={false}
						options={allCredentials}
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

		{#if isEditing && network}
			<EntityMetadataSection entities={[network]} />
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
