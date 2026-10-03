<script lang="ts">
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { NetworkDisplay } from '$lib/shared/components/forms/selection/display/NetworkDisplay.svelte';
	import { common_network, networks_selectNetwork } from '$lib/paraglide/messages';

	/**
	 * Network picker for TanStack Form fields:
	 *    <SelectNetwork selectedNetworkId={field.state.value} onNetworkChange={(id) => field.handleChange(id)} />
	 *
	 * Selects the first network when none is selected.
	 */
	interface Props {
		selectedNetworkId?: string | null;
		disabled?: boolean;
		disabledReason?: string;
		onNetworkChange: (networkId: string) => void;
	}

	let {
		selectedNetworkId = null,
		disabled = false,
		disabledReason = '',
		onNetworkChange
	}: Props = $props();

	let helpText = $derived(disabled && disabledReason ? disabledReason : networks_selectNetwork());

	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);

	// The displayed value. Callers hand in a form-store value that Svelte doesn't track, so a
	// programmatic change (the auto-select below) would update the store but not the trigger.
	let currentId = $derived(selectedNetworkId);

	function select(networkId: string) {
		currentId = networkId;
		onNetworkChange(networkId);
	}

	// Auto-select first network if none selected
	$effect(() => {
		if (!currentId && networksData.length > 0) {
			select(networksData[0].id);
		}
	});
</script>

<RichSelect
	label={common_network()}
	selectedValue={currentId}
	options={networksData}
	displayComponent={NetworkDisplay}
	onSelect={select}
	{disabled}
	loading={networksQuery.isPending}
	{helpText}
/>
