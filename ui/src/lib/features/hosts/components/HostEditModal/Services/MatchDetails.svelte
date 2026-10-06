<script lang="ts">
	import { matchReasonLabel, type MatchDetails } from '$lib/shared/types';
	import { matchConfidences } from '$lib/shared/stores/metadata';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import { hosts_services_matchDetails } from '$lib/paraglide/messages';

	let { details }: { details: MatchDetails } = $props();
</script>

<InfoCard title={hosts_services_matchDetails()}>
	<div class="flex items-center justify-between gap-2">
		<span class="text-secondary text-sm">{matchReasonLabel(details.reason)}</span>
		<Tag {...matchConfidences.getTag(details.confidence)} />
	</div>
</InfoCard>
