<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { Discovery } from '../../types/base';
	import { discoveryFields } from '../../queries';
	import DiscoveryEditModal from '../DiscoveryModal/DiscoveryEditModal.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import DiscoveryRunCard from '../cards/DiscoveryScheduledCard.svelte';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import { Plus } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useDiscoveriesQuery,
		useCreateDiscoveryMutation,
		useUpdateDiscoveryMutation,
		useDeleteDiscoveryMutation,
		useBulkDeleteDiscoveriesMutation,
		useInitiateDiscoveryMutation
	} from '../../queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import type { components } from '$lib/api/schema';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import {
		common_confirmDeleteName,
		common_create,
		common_created,
		common_tags,
		discovery_confirmDeleteScheduled,
		discovery_legacyDaemonsWarning,
		discovery_noScheduledSessions,
		discovery_runType,
		discovery_scheduledTitle
	} from '$lib/paraglide/messages';

	type OnboardingOperation = components['schemas']['OnboardingOperation'];

	let { isReadOnly = false }: TabProps = $props();

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Queries
	const tagsQuery = useTagsQuery();
	const discoveriesQuery = useDiscoveriesQuery();
	const daemonsQuery = useDaemonsQuery();
	const networksQuery = useNetworksQuery();
	// Use limit: 0 to get all hosts for modal dropdown
	const hostsQuery = useHostsQuery({ limit: 0 });

	// Mutations
	const createDiscoveryMutation = useCreateDiscoveryMutation();
	const updateDiscoveryMutation = useUpdateDiscoveryMutation();
	const deleteDiscoveryMutation = useDeleteDiscoveryMutation();
	const bulkDeleteDiscoveriesMutation = useBulkDeleteDiscoveriesMutation();
	const initiateDiscoveryMutation = useInitiateDiscoveryMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let discoveriesData = $derived(discoveriesQuery.data ?? []);
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let hostsData = $derived(hostsQuery.data?.items ?? []);
	let isLoading = $derived(
		discoveriesQuery.isPending || daemonsQuery.isPending || hostsQuery.isPending
	);
	let hasLegacyDaemons = $derived(
		daemonsData.some((d) => d.version_status?.supports_unified_discovery === false)
	);

	let showDiscoveryModal = $state(false);
	let editingDiscovery: Discovery | null = $state(null);

	// Deep-link: open discovery editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'discovery-editor',
			discoveriesData,
			showDiscoveryModal,
			editingDiscovery?.id
		);
		if (result !== undefined) {
			editingDiscovery = result;
			showDiscoveryModal = true;
		}
	});

	function handleCreateDiscovery() {
		editingDiscovery = null;
		showDiscoveryModal = true;
	}

	function handleEditDiscovery(discovery: Discovery) {
		editingDiscovery = discovery;
		showDiscoveryModal = true;
	}

	function handleDeleteDiscovery(discovery: Discovery) {
		if (confirm(common_confirmDeleteName({ name: discovery.name }))) {
			deleteDiscoveryMutation.mutate(discovery.id);
		}
	}

	function handleDiscoveryRun(discovery: Discovery) {
		initiateDiscoveryMutation.mutate(discovery.id);
	}

	function handleToggleEnabled(discovery: Discovery) {
		if (discovery.run_type.type !== 'Scheduled') return;
		updateDiscoveryMutation.mutate({
			...discovery,
			run_type: {
				...discovery.run_type,
				enabled: !discovery.run_type.enabled
			}
		});
	}

	async function handleDiscoveryCreate(data: Discovery) {
		await createDiscoveryMutation.mutateAsync(data);
		showDiscoveryModal = false;
		editingDiscovery = null;
	}

	async function handleDiscoveryUpdate(id: string, data: Discovery) {
		await updateDiscoveryMutation.mutateAsync(data);
		showDiscoveryModal = false;
		editingDiscovery = null;
	}

	function handleCloseEditor() {
		showDiscoveryModal = false;
		editingDiscovery = null;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(discovery_confirmDeleteScheduled({ count: ids.length }))) {
			await bulkDeleteDiscoveriesMutation.mutateAsync(ids);
		}
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Discovery', {});
	}

	let fields: FieldConfig<Discovery>[] = $derived([
		...discoveryFields(daemonsData, networksData),
		{
			key: 'run_type',
			label: discovery_runType(),
			type: 'string',
			searchable: true,
			filterable: true,
			groupable: true,
			getValue: (item) => item.run_type.type
		},
		{
			key: 'tags',
			label: common_tags(),
			type: 'array',
			searchable: true,
			filterable: true,
			getValue: (entity) => {
				// Return tag names for search/filter display
				return entity.tags
					.map((id) => tagsData.find((t) => t.id === id)?.name)
					.filter((name): name is string => !!name);
			}
		},
		{
			key: 'created_at',
			label: common_created(),
			type: 'date',
			sortable: true
		}
	]);
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={discovery_scheduledTitle()}>
		<svelte:fragment slot="actions">
			{#if hasDaemon(onboarding) && !isReadOnly}
				<button class="btn-primary flex items-center" onclick={handleCreateDiscovery}
					><Plus class="h-5 w-5" />{common_create()}</button
				>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if hasLegacyDaemons}
		<InlineWarning
			title=""
			body={discovery_legacyDaemonsWarning()}
			dismissableKey="unified-discovery-migration"
		/>
	{/if}

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title="Install a daemon to start running discoveries on your network." />
	{:else if isLoading}
		<Loading />
	{:else if discoveriesData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={discovery_noScheduledSessions()}
			subtitle=""
			onClick={isReadOnly ? undefined : handleCreateDiscovery}
			cta={isReadOnly ? undefined : common_create()}
		/>
	{:else}
		<DataControls
			items={discoveriesData.filter(
				(d) => d.run_type.type == 'AdHoc' || d.run_type.type == 'Scheduled'
			)}
			{fields}
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			storageKey="scanopy-discovery-scheduled-table-state"
			getItemId={(item) => item.id}
			entityType={isReadOnly ? undefined : 'Discovery'}
			getItemTags={(item) => item.tags}
			onCsvExport={handleCsvExport}
		>
			{#snippet children(
				item: Discovery,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<DiscoveryRunCard
					discovery={item}
					selected={isSelected}
					{onSelectionChange}
					onDelete={isReadOnly ? undefined : handleDeleteDiscovery}
					onEdit={isReadOnly ? undefined : handleEditDiscovery}
					onRun={isReadOnly ? undefined : handleDiscoveryRun}
					onToggleEnabled={isReadOnly ? undefined : handleToggleEnabled}
					{viewMode}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<DiscoveryEditModal
	name="discovery-editor"
	isOpen={showDiscoveryModal}
	daemons={daemonsData}
	hosts={hostsData}
	discovery={editingDiscovery}
	onCreate={handleDiscoveryCreate}
	onUpdate={handleDiscoveryUpdate}
	onClose={handleCloseEditor}
	onDelete={editingDiscovery
		? () => {
				handleDeleteDiscovery(editingDiscovery!);
				handleCloseEditor();
			}
		: null}
/>
