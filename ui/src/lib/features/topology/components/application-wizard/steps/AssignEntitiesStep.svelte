<script lang="ts">
	import { SvelteSet } from 'svelte/reactivity';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import ExpandableChildList from '$lib/shared/components/forms/selection/ExpandableChildList.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import type { HostDisplayContext } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import type { ServiceDisplayContext } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import type { Host } from '$lib/features/hosts/types/base';
	import type { Tag as TagType } from '$lib/features/tags/types/base';
	import { useHostSummariesQuery } from '$lib/features/hosts/queries';
	import { useServicesQuery } from '$lib/features/services/queries';
	import { useBulkAddTagMutation } from '$lib/features/tags/queries';
	import TagBadge from '$lib/shared/components/data/Tag.svelte';
	import { concepts } from '$lib/shared/stores/metadata';
	import {
		appWizard_assignDescription,
		appWizard_bulkAssign,
		appWizard_selectedCount
	} from '$lib/paraglide/messages';

	let {
		appTags,
		networkId
	}: {
		appTags: TagType[];
		networkId: string;
	} = $props();

	// Bulk-tagging surface: it needs every host on the network, but only their
	// id/name/hostname/tags. Per-host service lists come from `servicesQuery`
	// below, so the nested children this used to download were never read.
	const hostsQuery = useHostSummariesQuery(() => ({ network_id: networkId }));
	const servicesQuery = useServicesQuery(() => ({
		limit: 0,
		network_id: networkId,
		exclude_categories: ['OpenPorts']
	}));
	const bulkAddTagMutation = useBulkAddTagMutation();

	let allServices = $derived(servicesQuery.data?.items ?? []);
	let allHosts = $derived(
		(hostsQuery.data?.items ?? []).toSorted(
			(a, b) =>
				allServices.filter((s) => s.host_id === b.id).length -
				allServices.filter((s) => s.host_id === a.id).length
		)
	);

	// Track which app tag IDs exist for filtering
	let appTagIds = $derived(new Set(appTags.map((t) => t.id)));

	function hasAppTag(entity: { tags: string[] }): boolean {
		return entity.tags.some((tagId) => appTagIds.has(tagId));
	}

	// Selection state
	let selectedHosts: Host[] = $state([]);

	// Expanded hosts (services visible)
	const expandedHostIds = new SvelteSet<string>();

	function toggleExpanded(hostId: string) {
		if (expandedHostIds.has(hostId)) {
			expandedHostIds.delete(hostId);
		} else {
			expandedHostIds.add(hostId);
		}
	}

	function getEntityTags(entity: { tags: string[] }): TagType[] {
		if (hasAppTag(entity)) {
			// Already has an app tag — only show that tag (for removal), no add options
			return appTags.filter((t) => entity.tags.includes(t.id));
		}
		return appTags;
	}

	function getHostContext(host: Host): HostDisplayContext {
		const hostServices = allServices.filter((s) => s.host_id === host.id);
		return {
			showEntityTagPicker: true,
			entityTags: getEntityTags(host),
			allowTagCreate: false,
			services: hostServices
		};
	}

	function getServiceContext(service: { tags: string[] }): ServiceDisplayContext {
		return {
			showEntityTagPicker: true,
			entityTags: getEntityTags(service),
			allowTagCreate: false
		};
	}

	// Bulk assign
	async function handleBulkAssign(tagId: string) {
		const hostIds = selectedHosts.map((h) => h.id);
		if (hostIds.length === 0) return;

		await bulkAddTagMutation.mutateAsync({
			entity_ids: hostIds,
			entity_type: 'Host',
			tag_id: tagId
		});
		selectedHosts = [];
	}
</script>

<div class="flex h-full flex-col">
	<!-- Host list with ListManager -->
	<div class="flex min-h-0 flex-1 flex-col px-2">
		<ListManager
			label=""
			helpText={appWizard_assignDescription()}
			stickyHeader={true}
			items={allHosts}
			itemDisplayComponent={HostDisplay}
			getItemContext={(host) => getHostContext(host)}
			optionDisplayComponent={HostDisplay}
			allowAddFromOptions={false}
			allowReorder={false}
			allowSelection={true}
			itemClickAction="select"
			bind:selectedItems={selectedHosts}
			allowItemEdit={() => false}
			allowItemRemove={() => false}
		>
			{#snippet itemExpandedSnippet({ item })}
				{@const host = item}
				{@const hostServices = allServices.filter((s) => s.host_id === host.id)}
				{#if hostServices.length > 0}
					<ExpandableChildList
						items={hostServices}
						displayComponent={ServiceDisplay}
						getContext={getServiceContext}
						toggleLabel={`${hostServices.length} services`}
						expanded={expandedHostIds.has(host.id)}
						onToggleExpanded={() => toggleExpanded(host.id)}
					/>
				{/if}
			{/snippet}
		</ListManager>
	</div>

	<!-- Bulk assign bar -->
	{#if selectedHosts.length > 0}
		<div class="card card-static flex flex-wrap items-center gap-2 border-t px-4 py-3 shadow-lg">
			<span class="text-secondary whitespace-nowrap text-sm font-medium">
				{appWizard_selectedCount({
					summary: `${selectedHosts.length} ${selectedHosts.length === 1 ? 'host' : 'hosts'}`
				})}
			</span>
			<span class="text-tertiary whitespace-nowrap text-sm">{appWizard_bulkAssign()}</span>
			{#each appTags as tag (tag.id)}
				<button
					type="button"
					class="cursor-pointer"
					onclick={() => handleBulkAssign(tag.id)}
					disabled={bulkAddTagMutation.isPending}
				>
					<TagBadge
						label={tag.name}
						color={tag.color}
						icon={concepts.getIconComponent('Application')}
						isShiny={true}
						pill={true}
					/>
				</button>
			{/each}
		</div>
	{/if}
</div>
