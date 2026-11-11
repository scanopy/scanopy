<script lang="ts">
	import ProgressTrack from '#lib/shared/components/data/ProgressTrack.svelte';
	import { tooltip } from '#lib/shared/actions/tooltip.ts';
	import {
		common_utilization,
		subnets_utilizationInUse,
		subnets_utilizationNested,
		common_usable,
		subnets_utilizationValue
	} from '#lib/paraglide/messages.js';
	import type { SubnetResponse } from '../types/base';
	import { utilizationRatio } from '../nesting';

	let { subnet }: { subnet: SubnetResponse } = $props();

	const format = new Intl.NumberFormat(undefined, { notation: 'compact' });
	const exact = new Intl.NumberFormat();

	let ratio = $derived(utilizationRatio(subnet));
	let color = $derived(
		ratio >= 0.95 ? 'bg-red-500' : ratio >= 0.8 ? 'bg-amber-500' : 'bg-blue-500'
	);
	let used = $derived(format.format(subnet.used_addresses));
	let usable = $derived(format.format(subnet.usable_addresses));
	// The cell shows compact counts; the tooltip gives them exactly.
	let content = $derived({
		title: common_utilization(),
		sections: [
			{
				rows: [
					{ label: subnets_utilizationInUse(), value: exact.format(subnet.used_addresses) },
					{ label: common_usable(), value: exact.format(subnet.usable_addresses) }
				]
			},
			{ text: subnets_utilizationNested() }
		]
	});
</script>

<div class="flex min-w-0 items-center gap-2" use:tooltip={{ content }}>
	<ProgressTrack progress={ratio * 100} {color} size="sm" class="w-16 shrink-0" />
	<span class="truncate text-xs text-tertiary tabular-nums"
		>{subnets_utilizationValue({ used, usable })}</span
	>
</div>
