<script lang="ts">
	/**
	 * Every resource against the four roles. Opens over the modal holding `PermissionSelect`, so it
	 * has no `name`: the modal underneath stays mounted with its form.
	 */
	import { X } from '@lucide/svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ComparisonGrid, {
		type ComparisonColumn
	} from '$lib/shared/components/data/ComparisonGrid.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import type { Resource } from '$lib/features/auth/access';
	import { roleActions, roleColumns, roleSections } from '$lib/features/users/role-comparison';
	import RoleActionIcons from './RoleActionIcons.svelte';
	import RoleActionLegend from './RoleActionLegend.svelte';
	import { resources } from '$lib/shared/stores/metadata';
	import {
		common_access,
		common_organizationWide,
		permissions_compareRoles,
		permissions_perSite
	} from '$lib/paraglide/messages';

	let { isOpen = false, onClose }: { isOpen?: boolean; onClose: () => void } = $props();
</script>

{#snippet rowLabel(resource: Resource)}
	<div class="flex flex-col">
		<span
			class="explainer self-start text-xs font-medium lg:text-sm"
			data-tooltip={resources.getDescription(resource)}
			use:tooltip>{resources.getName(resource)}</span
		>
		<span class="text-[10px] text-tertiary lg:text-xs">
			{resources.getMetadata(resource).per_site ? permissions_perSite() : common_organizationWide()}
		</span>
	</div>
{/snippet}

{#snippet cell(resource: Resource, column: ComparisonColumn)}
	{@const held = roleActions(resource, column.id)}
	{#if held.length > 0}
		<RoleActionIcons {held} />
	{:else}
		<X class="mx-auto h-4 w-4 text-muted lg:h-5 lg:w-5" />
	{/if}
{/snippet}

<GenericModal {isOpen} {onClose} title={permissions_compareRoles()} size="xl">
	<div class="flex min-h-0 flex-1 flex-col">
		<div class="shrink-0 border-b px-4 py-2" style="border-color: var(--color-border)">
			<RoleActionLegend />
		</div>
		<ComparisonGrid
			framed={false}
			class="min-h-0 flex-1"
			labelHeading={common_access()}
			columns={roleColumns()}
			sections={roleSections()}
			getRowKey={(resource) => resource}
			{rowLabel}
			{cell}
		/>
	</div>
</GenericModal>
