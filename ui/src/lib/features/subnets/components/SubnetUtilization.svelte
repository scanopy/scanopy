<script lang="ts">
	import ProgressTrack from '$lib/shared/components/data/ProgressTrack.svelte';
	import { subnets_utilizationDetail, subnets_utilizationValue } from '$lib/paraglide/messages';
	import type { SubnetResponse } from '../types/base';
	import { utilizationRatio } from '../nesting';

	let { subnet }: { subnet: SubnetResponse } = $props();

	const format = new Intl.NumberFormat(undefined, { notation: 'compact' });

	let ratio = $derived(utilizationRatio(subnet));
	let color = $derived(
		ratio >= 0.95 ? 'bg-red-500' : ratio >= 0.8 ? 'bg-amber-500' : 'bg-blue-500'
	);
	let used = $derived(format.format(subnet.used_addresses));
	let usable = $derived(format.format(subnet.usable_addresses));
</script>

<div class="flex min-w-0 items-center gap-2" title={subnets_utilizationDetail({ used, usable })}>
	<ProgressTrack progress={ratio * 100} {color} size="sm" class="w-16 shrink-0" />
	<span class="text-tertiary truncate text-xs tabular-nums"
		>{subnets_utilizationValue({ used, usable })}</span
	>
</div>
