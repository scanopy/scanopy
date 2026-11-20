<script lang="ts">
	import RichSelect from '#lib/shared/components/forms/selection/RichSelect.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '#lib/shared/components/forms/selection/display/SimpleOptionDisplay.ts';
	import { activeView, topologyOptions, updateTopologyOptions } from '../../../queries';
	import { edgeColoringsFor, effectiveEdgeColoring, type EdgeColoring } from '../../../edge-speed';
	import { metaName, metaDescription } from '#lib/i18n/metadata.ts';
	import {
		topology_edgeColoringHelp,
		topology_edgeColoringLabel
	} from '#lib/paraglide/messages.js';

	let colorings = $derived(edgeColoringsFor($activeView));

	let options: SimpleOption[] = $derived(
		colorings.map((coloring) => ({
			value: coloring.id,
			label: metaName('edge_colorings', coloring.id, coloring.name),
			description: metaDescription('edge_colorings', coloring.id, coloring.description)
		}))
	);

	let selected = $derived(effectiveEdgeColoring($topologyOptions.local, $activeView));

	// A viewer setting: it changes how links are drawn, not what the topology holds, so it stays
	// in the local options and needs no edit access.
	function handleSelect(value: string) {
		if (value === selected) return;
		const view = $activeView;
		updateTopologyOptions((opts) => ({
			...opts,
			local: {
				...opts.local,
				edge_coloring: { ...opts.local.edge_coloring, [view]: value as EdgeColoring }
			}
		}));
	}
</script>

<!-- Only a view with a choice to make shows the picker. -->
{#if colorings.length > 1}
	<RichSelect
		label={topology_edgeColoringLabel()}
		selectedValue={selected}
		{options}
		onSelect={handleSelect}
		displayComponent={SimpleOptionDisplay}
		helpText={topology_edgeColoringHelp()}
	/>
{/if}
