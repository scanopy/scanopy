<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max, min } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { Topology } from '../types/base';
	import {
		createEmptyTopologyFormData,
		useCreateTopologyMutation,
		useTopologiesQuery,
		useUpdateMetadataMutation,
		selectedTopologyId,
		topologyOptions,
		sanitizeOptionsForApi
	} from '../queries';
	import { entities } from '$lib/shared/stores/metadata';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import RadioGroup from '$lib/shared/components/forms/input/RadioGroup.svelte';
	import { TopologyDisplay } from '$lib/shared/components/forms/selection/display/TopologyDisplay.svelte';
	import {
		common_cancel,
		common_editName,
		common_name,
		common_parent,
		common_save,
		common_saving,
		topology_branchFromExisting,
		topology_createTopology,
		topology_creationMode,
		topology_namePlaceholder,
		topology_selectParent,
		topology_startFresh
	} from '$lib/paraglide/messages';

	// TanStack Query hooks
	const networksQuery = useNetworksQuery();
	const topologiesQuery = useTopologiesQuery();
	const createTopologyMutation = useCreateTopologyMutation();
	const updateMetadataMutation = useUpdateMetadataMutation();

	let networksData = $derived(networksQuery.data ?? []);
	let topologiesData = $derived(topologiesQuery.data ?? []);
	let defaultNetworkId = $derived(networksData[0]?.id ?? '');

	let {
		isOpen = $bindable(false),
		onSubmit,
		onClose,
		topo = null,
		name = undefined
	}: {
		isOpen: boolean;
		onSubmit: () => Promise<void> | void;
		onClose: () => void;
		topo: Topology | null;
		name?: string;
	} = $props();

	let isEditing = $derived(topo != null);
	let title = $derived(
		isEditing ? common_editName({ name: topo?.name ?? '' }) : topology_createTopology()
	);

	let loading = $state(false);

	function getDefaultValues(): Topology {
		if (topo) {
			return { ...topo };
		}
		// For new topologies, pre-select the currently viewed topology as parent
		const currentTopology = $selectedTopologyId
			? topologiesData.find((t) => t.id === $selectedTopologyId)
			: null;
		const networkId = currentTopology?.network_id ?? defaultNetworkId;
		const defaults = createEmptyTopologyFormData(networkId);
		// Default to current topology, or first available on this network
		if (currentTopology) {
			defaults.parent_id = currentTopology.id;
		} else {
			const firstAvailable = topologiesData.find((t) => t.network_id === networkId);
			if (firstAvailable) {
				defaults.parent_id = firstAvailable.id;
			}
		}
		return defaults;
	}

	// Create form with additional creation_mode field for UI
	const form = createForm(() => ({
		defaultValues: { ...createEmptyTopologyFormData(''), creation_mode: 'branch' },
		onSubmit: async ({ value }) => {
			// eslint-disable-next-line @typescript-eslint/no-unused-vars
			const { creation_mode, ...topologyFields } = value as Topology & { creation_mode: string };
			const topologyData: Topology = {
				...topologyFields,
				name: topologyFields.name.trim(),
				options: sanitizeOptionsForApi($topologyOptions)
			};

			loading = true;
			try {
				if (isEditing) {
					// Use lightweight metadata update (fixes HTTP 413 for large topologies)
					await updateMetadataMutation.mutateAsync({
						topologyId: topologyData.id,
						networkId: topologyData.network_id,
						name: topologyData.name,
						parentId: topologyData.parent_id ?? null
					});
				} else {
					const created = await createTopologyMutation.mutateAsync(topologyData);
					// Select the newly created topology
					selectedTopologyId.set(created.id);
				}
				await onSubmit();
			} finally {
				loading = false;
			}
		}
	}));

	// Local state for network_id to enable Svelte 5 reactivity
	// (form.state.values is NOT tracked by $derived)
	let selectedNetworkId = $state<string>('');
	let selectedParentId = $state<string | null>(null);

	// Sync form values to local state on store changes
	$effect(() => {
		return form.store.subscribe(() => {
			selectedNetworkId = form.state.values.network_id;
			selectedParentId = form.state.values.parent_id ?? null;
		});
	});

	// Clear parent_id when network changes and current parent isn't on the new network
	$effect(() => {
		// Read reactive deps unconditionally for Svelte 5 tracking
		const topos = availableTopologies;
		const networkId = selectedNetworkId;

		const currentParentId = form.state.values.parent_id;
		if (currentParentId && networkId) {
			const parentOnNetwork = topos.find((t) => t.id === currentParentId);
			if (!parentOnNetwork) {
				const newParentId = topos[0]?.id ?? null;
				form.setFieldValue('parent_id', newParentId);
				selectedParentId = newParentId;
				if (topos.length === 0) {
					creationMode = 'fresh';
					previousCreationMode = 'fresh';
				}
			}
		}
	});

	// Local state for creation mode to enable Svelte 5 reactivity
	let creationMode = $state<'branch' | 'fresh'>('branch');
	let previousCreationMode = $state<'branch' | 'fresh'>('branch');

	// Sync creation mode from form store and handle changes
	$effect(() => {
		return form.store.subscribe(() => {
			const newMode = (form.state.values as { creation_mode?: string }).creation_mode as
				| 'branch'
				| 'fresh';
			if (newMode !== previousCreationMode) {
				previousCreationMode = newMode;
				creationMode = newMode;
				// Update parent_id based on mode change
				if (newMode === 'fresh') {
					form.setFieldValue('parent_id', null);
				} else if (availableTopologies.length > 0 && !form.state.values.parent_id) {
					form.setFieldValue('parent_id', availableTopologies[0].id);
				}
			}
		});
	});

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		const hasParent = defaults.parent_id !== null;
		const mode = hasParent ? 'branch' : 'fresh';
		form.reset({
			...defaults,
			creation_mode: mode
		});
		selectedNetworkId = defaults.network_id;
		selectedParentId = defaults.parent_id ?? null;
		creationMode = mode;
		previousCreationMode = mode;
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	// Creation mode options
	const creationModeOptions = [
		{ value: 'branch', label: topology_branchFromExisting() },
		{ value: 'fresh', label: topology_startFresh() }
	];

	// Available topologies for parent selection (exclude current and filter by network)
	let availableTopologies = $derived(
		topologiesData.filter(
			(t) => t.id !== form.state.values.id && t.network_id === selectedNetworkId
		)
	);

	let colorHelper = entities.getColorHelper('Topology');
	let Icon = entities.getIconComponent('Topology');
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={topo?.id}
	size="md"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon {Icon} color={colorHelper.color} />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-4">
				<form.Field name="network_id">
					{#snippet children(field)}
						<SelectNetwork
							selectedNetworkId={field.state.value}
							onNetworkChange={(id) => field.handleChange(id)}
							disabled={isEditing}
						/>
					{/snippet}
				</form.Field>

				{#if !isEditing && availableTopologies.length > 0}
					<form.Field name="creation_mode">
						{#snippet children(field)}
							<RadioGroup
								label={topology_creationMode()}
								id="creation_mode"
								{field}
								options={creationModeOptions}
								disabled={isEditing}
							/>
						{/snippet}
					</form.Field>
				{/if}

				{#if creationMode === 'branch' && availableTopologies.length > 0}
					<form.Field name="parent_id">
						{#snippet children(field)}
							<div>
								<RichSelect
									label={isEditing ? common_parent() : topology_selectParent()}
									displayComponent={TopologyDisplay}
									required={false}
									disabled={isEditing}
									selectedValue={selectedParentId}
									onSelect={(id) => field.handleChange(id)}
									options={availableTopologies}
								/>
							</div>
						{/snippet}
					</form.Field>
				{/if}

				<form.Field
					name="name"
					validators={{
						onBlur: ({ value }) => required(value) || max(100)(value) || min(3)(value)
					}}
				>
					{#snippet children(field)}
						<TextInput
							label={common_name()}
							id="name"
							{field}
							placeholder={topology_namePlaceholder()}
							required
						/>
					{/snippet}
				</form.Field>
			</div>
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				<button type="button" disabled={loading} onclick={onClose} class="btn-secondary">
					{common_cancel()}
				</button>
				<button type="submit" disabled={loading} class="btn-primary">
					{loading ? common_saving() : common_save()}
				</button>
			</div>
		</div>
	</form>
</GenericModal>
