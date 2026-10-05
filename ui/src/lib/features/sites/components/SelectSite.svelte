<script lang="ts">
	import { useSitesQuery } from '$lib/features/sites/queries';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { SiteDisplay } from '$lib/shared/components/forms/selection/display/SiteDisplay.svelte';
	import { common_site } from '$lib/paraglide/messages';

	/**
	 * Site picker for TanStack Form fields:
	 *    <SelectSite selectedSiteId={field.state.value} onSiteChange={(id) => field.handleChange(id)} />
	 *
	 * Selects the first site when none is selected.
	 */
	interface Props {
		selectedSiteId?: string | null;
		disabled?: boolean;
		disabledReason?: string;
		onSiteChange: (siteId: string) => void;
	}

	let {
		selectedSiteId = null,
		disabled = false,
		disabledReason = '',
		onSiteChange
	}: Props = $props();

	let helpText = $derived(disabled ? disabledReason : '');

	const sitesQuery = useSitesQuery();
	let sitesData = $derived(sitesQuery.data ?? []);

	// The displayed value. Callers hand in a form-store value that Svelte doesn't track, so a
	// programmatic change (the auto-select below) would update the store but not the trigger.
	let currentId = $derived(selectedSiteId);

	function select(siteId: string) {
		currentId = siteId;
		onSiteChange(siteId);
	}

	// Auto-select first site if none selected
	$effect(() => {
		if (!currentId && sitesData.length > 0) {
			select(sitesData[0].id);
		}
	});
</script>

<RichSelect
	label={common_site()}
	selectedValue={currentId}
	options={sitesData}
	displayComponent={SiteDisplay}
	onSelect={select}
	{disabled}
	loading={sitesQuery.isPending}
	{helpText}
/>
