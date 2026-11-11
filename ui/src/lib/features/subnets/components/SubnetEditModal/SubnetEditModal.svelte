<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '#lib/shared/components/forms/form-context.ts';
	import { required, max, cidrNotation } from '#lib/shared/components/forms/validators.ts';
	import { createEmptySubnetFormData, isContainerSubnet } from '../../queries';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import { entities, subnetTypes } from '#lib/shared/stores/metadata.ts';
	import ModalHeaderIcon from '#lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '#lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Subnet } from '../../types/base';
	import SelectSite from '#lib/features/sites/components/SelectSite.svelte';
	import TagPickerInline from '#lib/features/tags/components/TagPickerInline.svelte';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '#lib/shared/components/forms/input/TextArea.svelte';
	import RichSelect from '#lib/shared/components/forms/selection/RichSelect.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '#lib/shared/components/forms/selection/display/SimpleOptionDisplay.ts';
	import { useSitesQuery } from '#lib/features/sites/queries.ts';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import {
		common_tags,
		common_cancel,
		common_cidr,
		common_create,
		common_delete,
		common_deleting,
		common_description,
		common_editName,
		common_name,
		common_saving,
		common_update,
		subnets_cidrPlaceholder,
		subnets_createSubnet,
		subnets_descriptionPlaceholder,
		subnets_namePlaceholder,
		subnets_subnetType
	} from '#lib/paraglide/messages.js';

	// TanStack Query hooks
	const sitesQuery = useSitesQuery();
	let sitesData = $derived(sitesQuery.data ?? []);
	let defaultSiteId = $derived(sitesData[0]?.id ?? '');

	interface Props {
		subnet?: Subnet | null;
		isOpen?: boolean;
		onCreate: (data: Subnet) => Promise<void> | void;
		onUpdate: (id: string, data: Subnet) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
	}

	let {
		subnet = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined
	}: Props = $props();

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(subnet !== null);
	let title = $derived(
		isEditing ? common_editName({ name: subnet?.name ?? '' }) : subnets_createSubnet()
	);
	let saveLabel = $derived(isEditing ? common_update() : common_create());

	// The fields the form edits, merged onto the subnet on submit. Not the whole `Subnet`: the
	// generated type is an intersection of the range, name and type carriers, and TanStack's
	// field-path types exceed TypeScript's instantiation depth walking it. The form edits values,
	// never their sources; the server stamps one a person changed `Manual`.
	interface SubnetFormValues {
		name: string;
		cidr: string;
		description: string;
		site_id: string;
		subnet_type: Subnet['subnet_type'];
		tags: string[];
	}

	function editing(): Subnet {
		return subnet ?? createEmptySubnetFormData(defaultSiteId);
	}

	function formValues(source: Subnet): SubnetFormValues {
		return {
			name: source.name,
			cidr: source.cidr,
			description: source.description ?? '',
			site_id: source.site_id,
			subnet_type: source.subnet_type,
			tags: source.tags
		};
	}

	function getDefaultValues(): SubnetFormValues {
		return formValues(editing());
	}

	// Create form with initial empty values - we'll reset it when the modal opens
	const form = createForm(() => ({
		defaultValues: formValues(createEmptySubnetFormData('')),
		onSubmit: async ({ value }) => {
			const subnetData: Subnet = {
				...editing(),
				...value,
				name: value.name.trim(),
				description: value.description.trim(),
				cidr: value.cidr.trim()
			};

			loading = true;
			try {
				if (isEditing && subnet) {
					await onUpdate(subnet.id, subnetData);
				} else {
					await onCreate(subnetData);
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

	// CIDR disabled state - use a function to avoid reactive dependency on form.state
	function getIsCidrDisabled(): boolean {
		return isContainerSubnet(editing()) || isEditing;
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && subnet) {
			deleting = true;
			try {
				await onDelete(subnet.id);
			} finally {
				deleting = false;
			}
		}
	}

	let colorHelper = entities.getColorHelper('Subnet');

	// Prepare subnet type options. Every site already has its Internet and Remote ranges, so those
	// are offered only to a subnet that already holds one.
	let subnetTypeOptions: SimpleOption[] = $derived(
		subnetTypes
			.getItems()
			.filter(
				(st) => !subnetTypes.getMetadata(st.id).is_seeded_per_site || st.id === subnet?.subnet_type
			)
			.map((st) => ({
				value: st.id,
				label: st.name ?? st.id,
				icon: subnetTypes.getIconComponent(st.id),
				iconColor: subnetTypes.getColorHelper(st.id).icon
			}))
	);

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
		{ role: 'cancel', label: common_cancel(), onClick: onClose, disabled: loading || deleting },
		{
			role: 'primary',
			label: loading ? common_saving() : saveLabel,
			onClick: handleSubmit,
			disabled: loading || deleting
		}
	]);
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={subnet?.id}
	{form}
	size="xl"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={true}
	{actions}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Subnet')} color={colorHelper.color} />
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
				<!-- Subnet Details Section -->
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
									placeholder={subnets_namePlaceholder()}
									required
								/>
							{/snippet}
						</form.Field>
						<form.Field
							name="cidr"
							validators={{
								onBlur: ({ value }) => required(value) || cidrNotation(value)
							}}
						>
							{#snippet children(field)}
								<TextInput
									label={common_cidr()}
									id="cidr"
									{field}
									placeholder={subnets_cidrPlaceholder()}
									disabled={getIsCidrDisabled()}
									required
								/>
							{/snippet}
						</form.Field>
					</div>

					<div class="grid grid-cols-1 items-start gap-4 sm:grid-cols-2">
						<!-- Site Selection -->
						<form.Field name="site_id">
							{#snippet children(field)}
								<SelectSite
									selectedSiteId={field.state.value}
									onSiteChange={(id) => field.handleChange(id)}
								/>
							{/snippet}
						</form.Field>

						<!-- Subnet Type -->
						<form.Field name="subnet_type">
							{#snippet children(field)}
								<RichSelect
									label={subnets_subnetType()}
									selectedValue={field.state.value}
									options={subnetTypeOptions}
									displayComponent={SimpleOptionDisplay}
									showSearch={true}
									onSelect={(value) => field.handleChange(value as Subnet['subnet_type'])}
								/>
							{/snippet}
						</form.Field>
					</div>

					<!-- Description Field -->
					<form.Field
						name="description"
						validators={{
							onBlur: ({ value }) => max(500)(value || '')
						}}
					>
						{#snippet children(field)}
							<TextArea
								label={common_description()}
								id="description"
								{field}
								placeholder={subnets_descriptionPlaceholder()}
								rows={3}
							/>
						{/snippet}
					</form.Field>

					<!-- Tags -->
					<form.Field name="tags">
						{#snippet children(field)}
							<TagPickerInline
								entityType="Subnet"
								label={common_tags()}
								selectedTagIds={field.state.value || []}
								onChange={(tags) => field.handleChange(tags)}
							/>
						{/snippet}
					</form.Field>
				</div>
			</div>
		</div>

		{#if isEditing && subnet}
			<EntityMetadataSection entities={[subnet]} />
		{/if}
	</form>
</GenericModal>
