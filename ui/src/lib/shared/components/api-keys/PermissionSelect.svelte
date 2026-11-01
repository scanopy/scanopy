<script lang="ts">
	/**
	 * Shared component for selecting user permissions
	 * Used by user API keys, user invites, and user management
	 *
	 * Filters available permission options based on the current user's permissions
	 * and the context (API keys vs user management have different rules)
	 */
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '$lib/shared/components/forms/selection/display/SimpleOptionDisplay';
	import { permissions, resources } from '$lib/shared/stores/metadata';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { roleActions, roleResources } from '$lib/features/users/role-comparison';
	import { permissions_compareRoles } from '$lib/paraglide/messages';
	import RoleActionIcons from './RoleActionIcons.svelte';
	import RoleActionLegend from './RoleActionLegend.svelte';
	import RoleComparisonModal from './RoleComparisonModal.svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';

	interface Props {
		/** TanStack Form field for the permission value */
		field: AnyFieldApi;
		/** Label for the select input */
		label?: string;
		/** Help text to display below the input */
		helpText?: string;
		/** Whether the input is disabled */
		disabled?: boolean;
		/** Context determines which permission rules apply:
		 * - 'api_key': Users can grant their own level or below
		 * - 'user': Only Owners can grant Admin; Admins can only grant Member/Viewer
		 */
		context?: 'api_key' | 'user';
		/** Optional filter to further restrict available permissions */
		permissionFilter?: (permissionId: string) => boolean;
	}

	let {
		field,
		label = 'Permissions Level',
		helpText = 'Choose the access level',
		disabled = false,
		context = 'user',
		permissionFilter
	}: Props = $props();

	let showComparison = $state(false);

	// Get current user to determine which permissions they can grant
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	// Build permission options based on context and what current user can grant
	let permissionOptions: SimpleOption[] = $derived(
		permissions
			.getItems()
			.filter((p) => {
				if (!currentUser) return false;

				const metadata = permissions.getMetadata(currentUser.permissions);
				if (!metadata) return false;

				const grantablePermissions =
					context === 'api_key'
						? metadata.grantable_api_key_permissions
						: metadata.grantable_user_permissions;

				if (!grantablePermissions?.includes(p.id)) return false;

				// Apply any additional filter
				if (permissionFilter && !permissionFilter(p.id)) return false;

				return true;
			})
			.map((p) => ({
				value: p.id,
				label: p.name ?? '',
				description: p.description ?? ''
			}))
	);
</script>

<RichSelect
	{label}
	selectedValue={field.state.value}
	options={permissionOptions}
	displayComponent={SimpleOptionDisplay}
	onSelect={(value) => field.handleChange(value)}
	{disabled}
	error={field.state.meta.isTouched ? (field.state.meta.errors?.[0] ?? null) : null}
	{helpText}
/>

<div class="mt-2 space-y-2">
	<RoleActionLegend />
	<div class="grid gap-x-6 gap-y-1 sm:grid-cols-2">
		{#each roleResources(field.state.value) as resource (resource)}
			<div class="flex items-center justify-between gap-3 text-xs text-secondary">
				<span class="truncate">{resources.getName(resource)}</span>
				<RoleActionIcons held={roleActions(resource, field.state.value)} />
			</div>
		{/each}
	</div>
	<button
		type="button"
		class="text-xs text-link hover:underline"
		onclick={() => (showComparison = true)}
	>
		{permissions_compareRoles()}
	</button>
</div>

<RoleComparisonModal isOpen={showComparison} onClose={() => (showComparison = false)} />
