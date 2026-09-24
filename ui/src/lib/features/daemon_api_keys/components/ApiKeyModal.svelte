<script lang="ts">
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { createApiKeyForm } from '../form';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { pushError } from '$lib/shared/stores/feedback';
	import { entities } from '$lib/shared/stores/metadata';
	import type { ApiKey } from '../types/base';
	import {
		createEmptyApiKeyFormData,
		useCreateApiKeyMutation,
		useRotateApiKeyMutation
	} from '../queries';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import ApiKeyFormFields from './ApiKeyFormFields.svelte';
	import {
		common_close,
		common_delete,
		common_deleting,
		common_editName,
		common_save,
		common_saving,
		daemonApiKeys_createApiKey,
		daemonApiKeys_noNetworkAvailable
	} from '$lib/paraglide/messages';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		onUpdate: (data: ApiKey) => Promise<void> | void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		apiKey?: ApiKey | null;
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

	// TanStack Query hooks
	const networksQuery = useNetworksQuery();
	const createApiKeyMutation = useCreateApiKeyMutation();
	const rotateApiKeyMutation = useRotateApiKeyMutation();

	let networksData = $derived(networksQuery.data ?? []);
	let defaultNetworkId = $derived(networksData[0]?.id ?? '');

	let loading = $state(false);
	let deleting = $state(false);
	let generatedKey = $state<string | null>(null);

	let isEditing = $derived(apiKey !== null);
	let title = $derived(
		isEditing ? common_editName({ name: apiKey?.name || 'API Key' }) : daemonApiKeys_createApiKey()
	);

	function getDefaultValues(): ApiKey {
		return apiKey ? { ...apiKey } : createEmptyApiKeyFormData(defaultNetworkId);
	}

	const form = createApiKeyForm(async (value) => {
		loading = true;
		try {
			if (isEditing) {
				await onUpdate(value);
			}
		} finally {
			loading = false;
		}
	});

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);
		generatedKey = null;

		// If network_id is empty but we have a default, set it
		if (!defaults.network_id && defaultNetworkId) {
			form.setFieldValue('network_id', defaultNetworkId);
		}
	}

	function handleOnClose() {
		generatedKey = null;
		onClose();
	}

	async function handleGenerateKey() {
		const formData = form.state.values as ApiKey;

		// Ensure network_id is set
		if (!formData.network_id) {
			if (defaultNetworkId) {
				formData.network_id = defaultNetworkId;
			} else {
				pushError(daemonApiKeys_noNetworkAvailable());
				return;
			}
		}

		loading = true;
		try {
			const result = await createApiKeyMutation.mutateAsync(formData);
			generatedKey = result.keyString;
		} catch {
			// The API client reports the failure.
		} finally {
			loading = false;
		}
	}

	async function handleRotateKey() {
		const formData = form.state.values as ApiKey;
		loading = true;
		try {
			const newKey = await rotateApiKeyMutation.mutateAsync(formData.id);
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

	let colorHelper = entities.getColorHelper('DaemonApiKey');
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
		<ModalHeaderIcon Icon={entities.getIconComponent('DaemonApiKey')} color={colorHelper.color} />
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
			<ApiKeyFormFields
				{form}
				{isEditing}
				{generatedKey}
				{loading}
				onGenerate={handleGenerateKey}
				onRotate={handleRotateKey}
			/>
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
