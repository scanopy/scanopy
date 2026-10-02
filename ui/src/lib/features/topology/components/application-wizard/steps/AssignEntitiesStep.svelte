<script lang="ts">
	import { ChevronDown, ChevronRight } from 'lucide-svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import type { HostDisplayContext } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import type { ServiceDisplayContext } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import type { Host } from '$lib/features/hosts/types/base';
	import type { Tag as TagType } from '$lib/features/tags/types/base';
	import { useHostSummariesQuery } from '$lib/features/hosts/queries';
	import { hostDisplayContext } from '$lib/features/hosts/host-picker.svelte';
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
		network_ids: [networkId],
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

	let allIpAddresses = $derived((hostsQuery.data?.items ?? []).flatMap((h) => h.ip_addresses));

	function getHostContext(host: Host): HostDisplayContext {
		return hostDisplayContext(allIpAddresses, allServices, {
			showEntityTagPicker: true,
			entityTags: getEntityTags(host),
			allowTagCreate: false
		});
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
					<div class="mt-2 w-full border-t border-gray-200 pt-2 dark:border-gray-700">
						<button
							type="button"
							class="text-tertiary flex items-center gap-1 text-xs"
							onclick={(e) => {
								e.stopPropagation();
								toggleExpanded(host.id);
							}}
						>
							{#if expandedHostIds.has(host.id)}
								<ChevronDown class="h-3.5 w-3.5" />
							{:else}
								<ChevronRight class="h-3.5 w-3.5" />
							{/if}
							{hostServices.length} services
						</button>
						{#if expandedHostIds.has(host.id)}
							<div class="mt-1 divide-y divide-gray-200 dark:divide-gray-700/50">
								{#each hostServices as service (service.id)}
									<div class="rounded px-2 py-1.5 pl-6 odd:bg-gray-50 dark:odd:bg-gray-800/30">
										<ListSelectItem
											item={service}
											context={getServiceContext(service)}
											displayComponent={ServiceDisplay}
										/>
									</div>
								{/each}
							</div>
						{/if}
					</div>
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
