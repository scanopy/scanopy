<script lang="ts">
	import type { components } from '$lib/api/schema';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';

	type PlanUsage = components['schemas']['PlanUsage'];

	let {
		planUsage,
		planType,
		isOwner
	}: {
		planUsage: PlanUsage;
		planType: string | null;
		isOwner: boolean;
	} = $props();

	let hasLimits = $derived(
		planUsage.host_limit != null || planUsage.network_limit != null || planUsage.seat_limit != null
	);

	let showUpgrade = $derived(hasLimits && isOwner && planType === 'Free');

	interface UsageRow {
		label: string;
		current: number;
		limit: number;
		pct: number;
	}

	let rows = $derived.by(() => {
		const list: UsageRow[] = [];
		if (planUsage.host_limit != null) {
			const pct = planUsage.host_count / planUsage.host_limit;
			list.push({
				label: 'Hosts',
				current: planUsage.host_count,
				limit: planUsage.host_limit,
				pct
			});
		}
		if (planUsage.network_limit != null) {
			const pct = planUsage.network_count / planUsage.network_limit;
			list.push({
				label: 'Networks',
				current: planUsage.network_count,
				limit: planUsage.network_limit,
				pct
			});
		}
		if (planUsage.seat_limit != null) {
			const pct = planUsage.seat_count / planUsage.seat_limit;
			list.push({
				label: 'Seats',
				current: planUsage.seat_count,
				limit: planUsage.seat_limit,
				pct
			});
		}
		return list;
	});

	function barColor(pct: number): string {
		if (pct >= 0.8) return 'bg-yellow-500';
		return 'bg-blue-500';
	}

	function textColor(pct: number): string {
		if (pct >= 0.8) return 'text-yellow-400';
		return 'text-secondary';
	}
</script>

{#if hasLimits}
	<section>
		<div class="mb-3 flex items-center justify-between">
			<h3 class="text-primary text-base font-semibold">Plan Usage</h3>
			{#if showUpgrade}
				<UpgradeButton feature="home_plan_usage" />
			{/if}
		</div>
		<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
			{#each rows as row (row.label)}
				<div class="card card-static">
					<div class="mb-2 flex items-center justify-between text-sm">
						<span class="text-secondary">{row.label}</span>
						<span class={textColor(row.pct)}>{row.current} / {row.limit}</span>
					</div>
					<div class="h-2 overflow-hidden rounded-full bg-gray-700">
						<div
							class="h-full rounded-full transition-all {barColor(row.pct)}"
							style="width: {Math.min(row.pct * 100, 100)}%"
						></div>
					</div>
				</div>
			{/each}
		</div>
	</section>
{/if}
