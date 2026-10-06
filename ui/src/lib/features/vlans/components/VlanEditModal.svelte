<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { max } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import type { Vlan } from '../types/base';
	import {
		common_cancel,
		common_description,
		common_editName,
		common_saving,
		common_update,
		vlans_descriptionPlaceholder,
		vlans_vlanTitle
	} from '$lib/paraglide/messages';

	let {
		vlan = null,
		isOpen = false,
		onUpdate,
		onClose,
		name = undefined
	}: {
		vlan?: Vlan | null;
		isOpen?: boolean;
		onUpdate: (data: Vlan) => Promise<void> | void;
		onClose: () => void;
		name?: string;
	} = $props();

	let loading = $state(false);

	let title = $derived(
		common_editName({
			name: vlan ? vlans_vlanTitle({ number: vlan.vlan_number, name: vlan.name }) : ''
		})
	);

	const form = createForm(() => ({
		defaultValues: { description: '' },
		onSubmit: async ({ value }) => {
			if (!vlan) return;
			loading = true;
			try {
				await onUpdate({ ...vlan, description: value.description.trim() || null });
			} finally {
				loading = false;
			}
		}
	}));

	function handleOpen() {
		form.reset({ description: vlan?.description ?? '' });
	}

	const colorHelper = entities.getColorHelper('Vlan');
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	{form}
	entityId={vlan?.id}
	size="xl"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Vlan')} color={colorHelper.color} />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			submitForm(form);
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="min-h-0 flex-1 overflow-auto p-6">
			<form.Field
				name="description"
				validators={{
					onBlur: ({ value }) => max(500)(value)
				}}
			>
				{#snippet children(field)}
					<TextArea
						label={common_description()}
						id="description"
						{field}
						placeholder={vlans_descriptionPlaceholder()}
					/>
				{/snippet}
			</form.Field>
		</div>

		{#if vlan}
			<EntityMetadataSection entities={[vlan]} />
		{/if}

		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				<button type="button" disabled={loading} onclick={onClose} class="btn-secondary">
					{common_cancel()}
				</button>
				<button type="submit" disabled={loading} class="btn-primary">
					{loading ? common_saving() : common_update()}
				</button>
			</div>
		</div>
	</form>
</GenericModal>
