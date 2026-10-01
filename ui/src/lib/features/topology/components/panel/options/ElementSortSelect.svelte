<script lang="ts">
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '$lib/shared/components/forms/selection/display/SimpleOptionDisplay';
	import { activeView, topologyOptions, updateTopologyOptions } from '../../../queries';
	import { effectiveElementSort, elementSortsFor, type ElementSort } from '../../../element-sort';
	import { metaName, metaDescription } from '$lib/i18n/metadata';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { common_sortByLabel, topology_elementSortHelp } from '$lib/paraglide/messages';

	let {
		disabled = false,
		disabledReason = ''
	}: {
		disabled?: boolean;
		disabledReason?: string;
	} = $props();

	let options: SimpleOption[] = $derived(
		elementSortsFor($activeView).map((sort) => ({
			value: sort.id,
			label: metaName('element_sorts', sort.id, sort.name),
			description: metaDescription('element_sorts', sort.id, sort.description)
		}))
	);

	let selected = $derived(effectiveElementSort($topologyOptions.request, $activeView));

	function handleSelect(value: string) {
		if (disabled || value === selected) return;
		const view = $activeView;
		updateTopologyOptions((opts) => ({
			...opts,
			request: {
				...opts.request,
				element_sort: { ...opts.request.element_sort, [view]: value as ElementSort }
			}
		}));
	}
</script>

<div use:tooltip data-tooltip={disabled && disabledReason ? disabledReason : null}>
	<RichSelect
		label={common_sortByLabel()}
		selectedValue={selected}
		{options}
		{disabled}
		onSelect={handleSelect}
		displayComponent={SimpleOptionDisplay}
		helpText={topology_elementSortHelp()}
	/>
</div>
