<script lang="ts">
	import { createForm, type AnyFieldApi } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import SnmpCredentialFields from './SnmpCredentialFields.svelte';
	import type { SnmpCredential } from '../types/base';
	import { createDefaultSnmpCredential } from '../types/base';
	import { entities } from '$lib/shared/stores/metadata';
	import { Key } from 'lucide-svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { pushError } from '$lib/shared/stores/feedback';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import {
		common_cancel,
		common_couldNotLoadOrganization,
		common_create,
		common_delete,
		common_deleting,
		common_details,
		common_editName,
		common_name,
		common_saving,
		common_update,
		m,
		snmp_createCredential,
		snmp_namePlaceholder
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
		credential?: SnmpCredential | null;
		isOpen?: boolean;
		onCreate: (data: SnmpCredential) => Promise<void> | void;
		onUpdate: (id: string, data: SnmpCredential) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
	} = $props();

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(credential !== null);
	let title = $derived(
		isEditing ? common_editName({ name: credential?.name ?? '' }) : snmp_createCredential()
	);
	let saveLabel = $derived(isEditing ? common_update() : common_create());

	function getDefaultValues(): SnmpCredential {
		if (credential) return { ...credential };
		if (organization) return createDefaultSnmpCredential(organization.id);
		return createDefaultSnmpCredential('');
	}

	let colorHelper = $derived(entities.getColorHelper('SnmpCredential'));

	// Create form
	const form = createForm(() => ({
		defaultValues: createDefaultSnmpCredential(''),
		onSubmit: async ({ value }) => {
			if (!organization) {
				pushError(common_couldNotLoadOrganization());
				onClose();
				return;
			}

			const credentialData: SnmpCredential = {
				...(value as SnmpCredential),
				name: value.name.trim(),
				organization_id: organization.id
			};

			loading = true;
			try {
				if (isEditing && credential) {
					await onUpdate(credential.id, credentialData);
				} else {
					await onCreate(credentialData);
				}
			} finally {
				loading = false;
			}
		}
	}));

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);
	}

	async function handleSubmit() {
		await submitForm(form);
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
		<ModalHeaderIcon Icon={entities.getIconComponent("SnmpCredential")} color={colorHelper.color} />
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
				<!-- Credential Details Section -->
				<div class="space-y-4">
					<p class="text-secondary">{m.snmp_modalCreateHelpText()}</p>
					<h3 class="text-primary flex items-center gap-2 text-lg font-medium">
						{common_details()}
					</h3>

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
								placeholder={snmp_namePlaceholder()}
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field name="version">
						{#snippet children(versionField: AnyFieldApi)}
							<form.Field name="community">
								{#snippet children(communityField: AnyFieldApi)}
									<SnmpCredentialFields {versionField} {communityField} />
								{/snippet}
							</form.Field>
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
				</div>
			</div>
		</div>

		{#if isEditing && credential}
			<EntityMetadataSection entities={[credential]} />
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
						onclick={onClose}
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
