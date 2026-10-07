<script lang="ts">
	/**
	 * Shared component for selecting site access
	 * Used by user API keys, user invites, and user management
	 *
	 * Filters available sites based on the current user's site access
	 * (users can only grant access to sites they have access to)
	 */
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { SiteDisplay } from '$lib/shared/components/forms/selection/display/SiteDisplay.svelte';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { permissions } from '$lib/shared/stores/metadata';
	import type { Site } from '$lib/features/sites/types';
	import type { UserOrgPermissions } from '$lib/features/users/types';

	interface Props {
		/** Currently selected site IDs */
		selectedSiteIds: string[];
		/** Callback when selection changes */
		onChange: (siteIds: string[]) => void;
		/** The permission level being granted (affects whether site selection is shown) */
		permissionLevel?: UserOrgPermissions;
		/** Label for the list manager */
		label?: string;
		/** Help text to display */
		helpText?: string;
		/** Whether selection is required */
		required?: boolean;
		/** Always show site selection regardless of permission level (for API keys) */
		alwaysShowSelection?: boolean;
	}

	let {
		selectedSiteIds,
		onChange,
		permissionLevel = 'Viewer',
		label = 'Sites',
		helpText = 'Select sites this entity will have access to',
		required = false,
		alwaysShowSelection = false
	}: Props = $props();

	// Get current user and sites
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const sitesQuery = useSitesQuery();
	let sitesData = $derived(sitesQuery.data ?? []);

	// Permissions that have access to all sites (don't need explicit selection)
	let sitesNotNeeded = $derived(
		permissions
			.getItems()
			.filter((p) => p.metadata.manage_org_entities)
			.map((p) => p.id)
	);

	// Check if site selection is needed for this permission level
	// Always show if alwaysShowSelection is true (e.g., for API keys)
	let needsSiteSelection = $derived(
		alwaysShowSelection || !sitesNotNeeded.includes(permissionLevel)
	);

	// Convert selected IDs to Site objects
	let selectedSites = $derived(
		selectedSiteIds
			.map((id) => sitesData.find((n) => n.id === id))
			.filter((n): n is Site => n !== undefined)
	);

	// Available sites (filtered by current user's access, excluding already selected)
	let siteOptions = $derived(
		sitesData
			.filter((n) => {
				// If current user is Owner/Admin, show all sites
				if (currentUser && sitesNotNeeded.includes(currentUser.permissions)) {
					return true;
				}
				// Otherwise, only show sites the current user has access to
				return currentUser ? currentUser.site_ids.includes(n.id) : false;
			})
			.filter((n) => !selectedSiteIds.includes(n.id))
	);

	function handleAddSite(id: string) {
		onChange([...selectedSiteIds, id]);
	}

	function handleRemoveSite(index: number) {
		const newIds = [...selectedSiteIds];
		newIds.splice(index, 1);
		onChange(newIds);
	}
</script>

{#if needsSiteSelection}
	<ListManager
		{label}
		{helpText}
		{required}
		allowReorder={false}
		allowAddFromOptions={true}
		allowCreateNew={false}
		allowItemEdit={() => false}
		disableCreateNewButton={false}
		onAdd={handleAddSite}
		onRemove={handleRemoveSite}
		options={siteOptions}
		optionDisplayComponent={SiteDisplay}
		items={selectedSites}
		itemDisplayComponent={SiteDisplay}
	/>
{:else}
	<div class="card card-static">
		<p class="text-secondary text-sm">
			Users with {permissionLevel} permissions have access to all sites.
		</p>
	</div>
{/if}
