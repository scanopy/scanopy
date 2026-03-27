<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Credential } from '../types/base';
	import { createDefaultCredential } from '../types/base';
	import { entities } from '$lib/shared/stores/metadata';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { pushError } from '$lib/shared/stores/feedback';
	import CredentialForm from './CredentialForm.svelte';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import {
		common_couldNotLoadOrganization,
		common_create,
		common_delete,
		common_deleting,
		common_editName,
		common_saving,
		common_update,
		credentials_createCredential,
		credentials_description,
		credentials_docsGuide,
		credentials_docsGuideLinkText
	} from '$lib/paraglide/messages';

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

	let isEditing = $derived(credential !== null);
	let title = $derived(
		isEditing ? common_editName({ name: credential?.name ?? '' }) : credentials_createCredential()
	);

	let colorHelper = $derived(entities.getColorHelper('Credential'));

	let credentialFormRef: ReturnType<typeof CredentialForm> | undefined = $state();
	let loading = $state(false);
	let deleting = $state(false);
	let saveLabel = $derived(isEditing ? common_update() : common_create());

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

			const credentialData: Credential = {
				...(value as Credential),
				organization_id: organization.id,
				credential_type: credentialType
			};

			if (isEditing && credential) {
				await onUpdate(credential.id, credentialData);
			} else {
				await onCreate(credentialData);
			}
		}
	}));

	function handleOpen() {
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
			await submitForm(form);
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
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Credential')} color={colorHelper.color} />
	{/snippet}

	<div class="min-h-0 flex-1 overflow-auto p-6">
		<div class="space-y-4">
			<p class="text-secondary text-sm">
				{credentials_description()}
			</p>
			<DocsHint
				text={credentials_docsGuide()}
				href="https://scanopy.net/docs/using-scanopy/credentials/"
				linkText={credentials_docsGuideLinkText()}
			/>

			<CredentialForm bind:this={credentialFormRef} {form} {credential} />
		</div>
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
