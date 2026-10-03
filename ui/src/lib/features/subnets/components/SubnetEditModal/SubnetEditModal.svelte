<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max, cidrNotation } from '$lib/shared/components/forms/validators';
	import { createEmptySubnetFormData, isContainerSubnet } from '../../queries';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import { entities, subnetTypes } from '$lib/shared/stores/metadata';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Subnet } from '../../types/base';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '$lib/shared/components/forms/selection/display/SimpleOptionDisplay';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import {
		common_cancel,
		common_cidr,
		common_create,
		common_delete,
		common_deleting,
		common_description,
		common_details,
		common_editName,
		common_name,
		common_saving,
		common_update,
		subnets_cidrHelp,
		subnets_cidrPlaceholder,
		subnets_createSubnet,
		subnets_descriptionPlaceholder,
		subnets_namePlaceholder,
		subnets_subnetType
	} from '$lib/paraglide/messages';

	// TanStack Query hooks
	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);
	let defaultNetworkId = $derived(networksData[0]?.id ?? '');

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

	function getDefaultValues(): Subnet {
		return subnet ? { ...subnet } : createEmptySubnetFormData(defaultNetworkId);
	}

	// Create form with initial empty values - we'll reset it when the modal opens
	const form = createForm(() => ({
		defaultValues: createEmptySubnetFormData(''),
		onSubmit: async ({ value }) => {
			const subnetData: Subnet = {
				...value,
				name: value.name.trim(),
				description: value.description?.trim() || '',
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
		return isContainerSubnet(form.state.values) || isEditing;
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

	// Prepare subnet type options
	let subnetTypeOptions: SimpleOption[] = $derived(
		subnetTypes.getItems().map((st) => ({
			value: st.id,
			label: st.name ?? st.id,
			icon: subnetTypes.getIconComponent(st.id),
			iconColor: subnetTypes.getColorHelper(st.id).icon
		}))
	);
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={subnet?.id}
	size="xl"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={true}
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
					<h3 class="text-primary text-lg font-medium">{common_details()}</h3>

					<!-- Name Field -->
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

					<!-- CIDR Field -->
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
								helpText={subnets_cidrHelp()}
								required
							/>
						{/snippet}
					</form.Field>

					<!-- Network Selection -->
					<form.Field name="network_id">
						{#snippet children(field)}
							<SelectNetwork
								selectedNetworkId={field.state.value}
								onNetworkChange={(id) => field.handleChange(id)}
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
							<TagPicker
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
