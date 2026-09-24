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
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';

	// Shared components
	import ApiKeyGenerator from '$lib/shared/components/api-keys/ApiKeyGenerator.svelte';
	import PermissionSelect from '$lib/shared/components/api-keys/PermissionSelect.svelte';
	import NetworkAccessSelect from '$lib/shared/components/api-keys/NetworkAccessSelect.svelte';

	import type { UserApiKey } from '../queries';
	import {
		createEmptyUserApiKeyFormData,
		useCreateUserApiKeyMutation,
		useRotateUserApiKeyMutation
	} from '../queries';
	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import {
		common_apiKeyNameHelp,
		common_close,
		common_delete,
		common_deleting,
		common_editName,
		common_enableApiKey,
		common_expirationDateOptional,
		common_expirationNeverHelp,
		common_keyDetails,
		common_name,
		common_nameRequired,
		common_networkRequired,
		common_permissions,
		common_save,
		common_saving,
		userApiKeys_createApiKey,
		userApiKeys_enableHelp,
		userApiKeys_namePlaceholder,
		userApiKeys_permissionsHelp,
		userApiKeys_shareIntegration
	} from '$lib/paraglide/messages';

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

	// Track permission value for NetworkAccessSelect
	let permissionsValue = $derived(form.state.values.permissions);

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
		if (!formData.network_ids?.length) {
			pushError(common_networkRequired());
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

	// Handle network selection changes
	function handleNetworkChange(networkIds: string[]) {
		form.setFieldValue('network_ids', networkIds);
	}

	let colorHelper = entities.getColorHelper('UserApiKey');
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={apiKey?.id}
	size="xl"
	onClose={handleOnClose}
	onOpen={handleOpen}
	showCloseButton={true}
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
				<InlineSuccess
					title={userApiKeys_shareIntegration()}
					body="Creating an integration that you think others might benefit from? Scanopy will be adding an integration library in an upcoming release. Go to the <a class='underline hover:no-underline' target='_blank' href='https://github.com/scanopy/integrations'>Scanopy integrations GitHub</a> and create a PR to get started."
				></InlineSuccess>

				<!-- Key Details Section -->
				<div class="space-y-4">
					<h3 class="text-primary text-lg font-medium">{common_keyDetails()}</h3>

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

					<form.Field name="network_ids">
						{#snippet children(field)}
							<NetworkAccessSelect
								selectedNetworkIds={field.state.value ?? []}
								onChange={handleNetworkChange}
								permissionLevel={permissionsValue}
								helpText="Leave empty for org-scoped resources only (tags, users)"
								alwaysShowSelection={true}
								required={false}
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
						onclick={handleOnClose}
						class="btn-secondary"
					>
						{common_close()}
					</button>
					{#if isEditing}
						<button type="submit" disabled={loading || deleting} class="btn-primary">
							{loading ? common_saving() : common_save()}
						</button>
					{/if}
				</div>
			</div>
		</div>
	</form>
</GenericModal>
