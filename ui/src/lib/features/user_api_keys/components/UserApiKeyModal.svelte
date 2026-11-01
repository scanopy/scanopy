<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { pushError } from '$lib/shared/stores/feedback';
	import { entities } from '$lib/shared/stores/metadata';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import DateInput from '$lib/shared/components/forms/input/DateInput.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';

	// Shared components
	import ApiKeyGenerator from '$lib/shared/components/api-keys/ApiKeyGenerator.svelte';
	import PermissionSelect from '$lib/shared/components/api-keys/PermissionSelect.svelte';
	import RoleAccessSummary from '$lib/shared/components/api-keys/RoleAccessSummary.svelte';
	import SiteAccessSelect from '$lib/shared/components/api-keys/SiteAccessSelect.svelte';

	import type { UserApiKey } from '../queries';
	import type { ModalAction } from '$lib/shared/components/layout/modalActions';
	import {
		createEmptyUserApiKeyFormData,
		useCreateUserApiKeyMutation,
		useRotateUserApiKeyMutation
	} from '../queries';
	import {
		common_tags,
		common_apiKeyNameHelp,
		common_close,
		common_delete,
		common_deleting,
		common_editName,
		common_enableApiKey,
		common_expirationDateOptional,
		common_expirationNeverHelp,
		common_name,
		common_nameRequired,
		common_siteRequired,
		common_permissions,
		common_save,
		common_saving,
		userApiKeys_createApiKey,
		userApiKeys_enableHelp,
		userApiKeys_namePlaceholder,
		userApiKeys_permissionsHelp,
		userApiKeys_verifyEmailToCreate
	} from '$lib/paraglide/messages';
	import EmailVerificationNotice from '$lib/features/auth/components/EmailVerificationNotice.svelte';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		onUpdate: (data: UserApiKey) => Promise<void> | void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		apiKey?: UserApiKey | null;
		name?: string;
	}

	let {
		isOpen = false,
		onClose,
		onUpdate,
		onDelete = null,
		apiKey = null,
		name = undefined
	}: Props = $props();

	// Mutations
	const createMutation = useCreateUserApiKeyMutation();
	const rotateMutation = useRotateUserApiKeyMutation();

	let loading = $state(false);
	let deleting = $state(false);
	let generatedKey = $state<string | null>(null);

	let isEditing = $derived(apiKey !== null);
	let title = $derived(
		isEditing ? common_editName({ name: apiKey?.name ?? '' }) : userApiKeys_createApiKey()
	);

	// Get minimum date (now) in local time format for datetime-local input
	function getLocalDateTimeMin(): string {
		const now = new Date();
		const year = now.getFullYear();
		const month = String(now.getMonth() + 1).padStart(2, '0');
		const day = String(now.getDate()).padStart(2, '0');
		const hours = String(now.getHours()).padStart(2, '0');
		const minutes = String(now.getMinutes()).padStart(2, '0');
		return `${year}-${month}-${day}T${hours}:${minutes}`;
	}
	const today = getLocalDateTimeMin();

	function getDefaultValues(): UserApiKey {
		return apiKey ? { ...apiKey } : createEmptyUserApiKeyFormData();
	}

	// Create form
	const form = createForm(() => ({
		defaultValues: createEmptyUserApiKeyFormData(),
		onSubmit: async ({ value }) => {
			loading = true;
			try {
				if (isEditing) {
					await onUpdate(value as UserApiKey);
				}
			} finally {
				loading = false;
			}
		}
	}));

	// Mirrored from the form store for SiteAccessSelect: form.state.values is not tracked by $derived.
	let permissionsValue = $state(createEmptyUserApiKeyFormData().permissions);
	$effect(() => {
		const { unsubscribe } = form.store.subscribe(() => {
			permissionsValue = form.state.values.permissions;
		});
		return unsubscribe;
	});

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);
		generatedKey = null;
	}

	function handleOnClose() {
		generatedKey = null;
		onClose();
	}

	async function handleGenerateKey() {
		const formData = form.state.values as UserApiKey;

		// Validate required fields before creating
		if (!formData.name?.trim()) {
			pushError(common_nameRequired());
			return;
		}
		if (!formData.site_ids?.length) {
			pushError(common_siteRequired());
			return;
		}

		loading = true;
		try {
			const result = await createMutation.mutateAsync(formData);
			generatedKey = result.keyString;
		} catch {
			// The API client reports the failure.
		} finally {
			loading = false;
		}
	}

	async function handleRotateKey() {
		const formData = form.state.values as UserApiKey;
		loading = true;
		try {
			const newKey = await rotateMutation.mutateAsync(formData.id);
			generatedKey = newKey;
		} catch {
			// The API client reports the failure.
		} finally {
			loading = false;
		}
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && apiKey) {
			deleting = true;
			try {
				await onDelete(apiKey.id);
			} finally {
				deleting = false;
			}
		}
	}

	// Handle site selection changes
	function handleSiteChange(siteIds: string[]) {
		form.setFieldValue('site_ids', siteIds);
	}

	let colorHelper = entities.getColorHelper('UserApiKey');

	let actions = $derived<ModalAction[]>([
		...(isEditing && onDelete
			? [
					{
						role: 'danger' as const,
						label: deleting ? common_deleting() : common_delete(),
						onClick: handleDelete,
						disabled: deleting || loading
					}
				]
			: []),
		{
			role: 'cancel',
			label: common_close(),
			onClick: handleOnClose,
			disabled: loading || deleting
		},
		...(isEditing
			? [
					{
						role: 'primary' as const,
						label: loading ? common_saving() : common_save(),
						onClick: handleSubmit,
						disabled: loading || deleting
					}
				]
			: [])
	]);
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={apiKey?.id}
	{form}
	size="xl"
	onClose={handleOnClose}
	onOpen={handleOpen}
	showCloseButton={true}
	{actions}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('UserApiKey')} color={colorHelper.color} />
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
			<div class="space-y-6">
				<div class="space-y-4">
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
								placeholder={userApiKeys_namePlaceholder()}
								helpText={common_apiKeyNameHelp()}
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field name="permissions">
						{#snippet children(field)}
							<PermissionSelect
								{field}
								label={common_permissions()}
								helpText={userApiKeys_permissionsHelp()}
								context="api_key"
							/>
						{/snippet}
					</form.Field>

					<form.Field name="site_ids">
						{#snippet children(field)}
							<SiteAccessSelect
								selectedSiteIds={field.state.value ?? []}
								onChange={handleSiteChange}
								permissionLevel={permissionsValue}
								helpText=""
								alwaysShowSelection={true}
								required={false}
							/>
						{/snippet}
					</form.Field>

					<RoleAccessSummary role={permissionsValue} />

					<form.Field name="tags">
						{#snippet children(field)}
							<TagPickerInline
								entityType="UserApiKey"
								label={common_tags()}
								selectedTagIds={field.state.value || []}
								onChange={(tags) => field.handleChange(tags)}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="expires_at">
						{#snippet children(field)}
							<DateInput
								label={common_expirationDateOptional()}
								id="expires_at"
								{field}
								helpText={common_expirationNeverHelp()}
								min={today}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="is_enabled">
						{#snippet children(field)}
							<Checkbox
								{field}
								label={common_enableApiKey()}
								helpText={userApiKeys_enableHelp()}
								id="enableApiKey"
							/>
						{/snippet}
					</form.Field>
				</div>

				<EmailVerificationNotice body={userApiKeys_verifyEmailToCreate()} />

				<!-- Key generation section -->
				<ApiKeyGenerator
					{generatedKey}
					{isEditing}
					{loading}
					onGenerate={handleGenerateKey}
					onRotate={handleRotateKey}
				/>
			</div>
		</div>

		{#if isEditing && apiKey}
			<EntityMetadataSection entities={[apiKey]} />
		{/if}
	</form>
</GenericModal>
