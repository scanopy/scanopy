<script lang="ts">
	/** What `role` can do, split into what it holds per site and across the organization. */
	import { ChevronDown, ChevronRight } from '@lucide/svelte';
	import { resources } from '#lib/shared/stores/metadata.ts';
	import { roleActions, roleResources } from '#lib/features/users/role-access.ts';
	import {
		common_organizationWide,
		permissions_perSite,
		permissions_whatThisRoleCanDo
	} from '#lib/paraglide/messages.js';
	import RoleActionIcons from './RoleActionIcons.svelte';

	let { role }: { role: string } = $props();

	let expanded = $state(false);
	/** Fewest actions first. Ties keep their product area order. */
	let held = $derived(
		roleResources(role).toSorted(
			(a, b) => roleActions(a, role).length - roleActions(b, role).length
		)
	);
	let sections = $derived([
		{
			label: permissions_perSite(),
			rows: held.filter((resource) => resources.getMetadata(resource).per_site)
		},
		{
			label: common_organizationWide(),
			rows: held.filter((resource) => !resources.getMetadata(resource).per_site)
		}
	]);
</script>

<div>
	<button
		type="button"
		class="flex items-center gap-1 text-xs font-medium text-secondary hover:text-primary"
		onclick={() => (expanded = !expanded)}
		aria-expanded={expanded}
	>
		{#if expanded}
			<ChevronDown class="h-3.5 w-3.5" />
		{:else}
			<ChevronRight class="h-3.5 w-3.5" />
		{/if}
		{permissions_whatThisRoleCanDo()}
	</button>

	{#if expanded}
		<div class="mt-2 space-y-3">
			{#each sections as section (section.label)}
				<div class="space-y-1">
					<h4 class="text-xs font-semibold text-primary">{section.label}</h4>
					<div>
						{#each section.rows as resource (resource)}
							<div
								class="flex items-center justify-between gap-3 rounded-sm px-1.5 py-1 text-xs text-secondary odd:bg-gray-100 dark:odd:bg-gray-700/25"
							>
								<span class="truncate">{resources.getName(resource)}</span>
								<RoleActionIcons held={roleActions(resource, role)} />
							</div>
						{/each}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
