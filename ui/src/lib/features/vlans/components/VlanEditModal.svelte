<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '#lib/shared/components/forms/form-context.ts';
	import { max, required, vlanNumber } from '#lib/shared/components/forms/validators.ts';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '#lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '#lib/shared/components/forms/EntityMetadataSection.svelte';
	import TextArea from '#lib/shared/components/forms/input/TextArea.svelte';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import SelectSite from '#lib/features/sites/components/SelectSite.svelte';
	import TagPickerInline from '#lib/features/tags/components/TagPickerInline.svelte';
	import { useSitesQuery } from '#lib/features/sites/queries.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import { entities } from '#lib/shared/stores/metadata.ts';
	import { createEmptyVlanFormData } from '../queries';
	import type { Vlan } from '../types/base';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import {
		common_tags,
		common_cancel,
		common_create,
		common_description,
		common_editName,
		common_name,
		common_saving,
		common_update,
		vlans_createVlan,
		vlans_descriptionPlaceholder,
		vlans_namePlaceholder,
		vlans_vlanNumber,
		vlans_vlanTitle
	} from '#lib/paraglide/messages.js';

	let {
		vlan = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		name = undefined
	}: {
		vlan?: Vlan | null;
		isOpen?: boolean;
		onCreate: (data: Vlan) => Promise<void> | void;
		onUpdate: (data: Vlan) => Promise<void> | void;
		onClose: () => void;
		name?: string;
	} = $props();

	const sitesQuery = useSitesQuery();
	const organizationQuery = useOrganizationQuery();

	let loading = $state(false);
	let isEditing = $derived(vlan !== null);

	let title = $derived(
		vlan
			? common_editName({ name: vlans_vlanTitle({ number: vlan.vlan_number, name: vlan.name }) })
			: vlans_createVlan()
	);

	interface VlanFormValues {
		vlan_number: string;
		name: string;
		site_id: string;
		description: string;
		tags: string[];
	}

	function formValues(source: Vlan | null): VlanFormValues {
		return {
			vlan_number: source ? String(source.vlan_number) : '',
			name: source?.name ?? '',
			site_id: source?.site_id ?? sitesQuery.data?.[0]?.id ?? '',
			description: source?.description ?? '',
			tags: source?.tags ?? []
		};
	}

	const form = createForm(() => ({
		defaultValues: formValues(null),
		onSubmit: async ({ value }) => {
			loading = true;
			try {
				if (vlan) {
					await onUpdate({
						...vlan,
						description: value.description.trim() || null,
						tags: value.tags
					});
				} else {
					await onCreate({
						...createEmptyVlanFormData(value.site_id, organizationQuery.data?.id ?? ''),
						vlan_number: Number(value.vlan_number),
						name: value.name.trim(),
						description: value.description.trim() || null,
						tags: value.tags
					});
				}
			} finally {
				loading = false;
			}
		}
	}));

	function handleOpen() {
		form.reset(formValues(vlan));
	}

	const colorHelper = entities.getColorHelper('Vlan');

	let actions = $derived<ModalAction[]>([
		{ role: 'cancel', label: common_cancel(), onClick: onClose, disabled: loading },
		{
			role: 'primary',
			label: loading ? common_saving() : isEditing ? common_update() : common_create(),
			onClick: () => submitForm(form),
			disabled: loading
		}
	]);
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
	{actions}
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
			<div class="space-y-4">
				<!-- On an existing VLAN the number, name and site are read-only: the number and site are
				     its identity (the server keeps both on update), and discovery rewrites the name. -->
				<div class="grid grid-cols-1 items-start gap-4 sm:grid-cols-2">
					<form.Field
						name="vlan_number"
						validators={{ onBlur: ({ value }) => (isEditing ? undefined : vlanNumber(value)) }}
					>
						{#snippet children(field)}
							<TextInput
								label={vlans_vlanNumber()}
								id="vlan_number"
								type="number"
								{field}
								disabled={isEditing}
								required={!isEditing}
							/>
						{/snippet}
					</form.Field>

					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }) => (isEditing ? undefined : required(value) || max(100)(value))
						}}
					>
						{#snippet children(field)}
							<TextInput
								label={common_name()}
								id="name"
								{field}
								placeholder={vlans_namePlaceholder()}
								disabled={isEditing}
								required={!isEditing}
							/>
						{/snippet}
					</form.Field>
				</div>

				<form.Field name="site_id">
					{#snippet children(field)}
						<SelectSite
							selectedSiteId={field.state.value}
							disabled={isEditing}
							onSiteChange={(id) => field.handleChange(id)}
						/>
					{/snippet}
				</form.Field>

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

				<form.Field name="tags">
					{#snippet children(field)}
						<TagPickerInline
							entityType="Vlan"
							label={common_tags()}
							selectedTagIds={field.state.value || []}
							onChange={(tags) => field.handleChange(tags)}
						/>
					{/snippet}
				</form.Field>
			</div>
		</div>

		{#if vlan}
			<EntityMetadataSection entities={[vlan]} />
		{/if}
	</form>
</GenericModal>
