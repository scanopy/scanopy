<script lang="ts">
	import { submitForm } from '#lib/shared/components/forms/form-context.ts';
	import { createApiKeyForm } from '../form';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '#lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { pushError } from '#lib/shared/stores/feedback.ts';
	import { entities } from '#lib/shared/stores/metadata.ts';
	import type { ApiKey } from '../types/base';
	import {
		createEmptyApiKeyFormData,
		useCreateApiKeyMutation,
		useRotateApiKeyMutation
	} from '../queries';
	import EntityMetadataSection from '#lib/shared/components/forms/EntityMetadataSection.svelte';
	import { useSitesQuery } from '#lib/features/sites/queries.ts';
	import ApiKeyFormFields from './ApiKeyFormFields.svelte';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import {
		common_close,
		common_delete,
		common_deleting,
		common_editName,
		common_save,
		common_saving,
		daemonApiKeys_createApiKey,
		daemonApiKeys_noSiteAvailable
	} from '#lib/paraglide/messages.js';

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
	const sitesQuery = useSitesQuery();
	const createApiKeyMutation = useCreateApiKeyMutation();
	const rotateApiKeyMutation = useRotateApiKeyMutation();

	let sitesData = $derived(sitesQuery.data ?? []);
	let defaultSiteId = $derived(sitesData[0]?.id ?? '');

	let loading = $state(false);
	let deleting = $state(false);
	let generatedKey = $state<string | null>(null);

	let isEditing = $derived(apiKey !== null);
	let title = $derived(
		isEditing ? common_editName({ name: apiKey?.name || 'API Key' }) : daemonApiKeys_createApiKey()
	);

	function getDefaultValues(): ApiKey {
		return apiKey ? { ...apiKey } : createEmptyApiKeyFormData(defaultSiteId);
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

		// If site_id is empty but we have a default, set it
		if (!defaults.site_id && defaultSiteId) {
			form.setFieldValue('site_id', defaultSiteId);
		}
	}

	function handleOnClose() {
		generatedKey = null;
		onClose();
	}

	async function handleGenerateKey() {
		const formData = form.state.values as ApiKey;

		// Ensure site_id is set
		if (!formData.site_id) {
			if (defaultSiteId) {
				formData.site_id = defaultSiteId;
			} else {
				pushError(daemonApiKeys_noSiteAvailable());
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
	</form>
</GenericModal>
