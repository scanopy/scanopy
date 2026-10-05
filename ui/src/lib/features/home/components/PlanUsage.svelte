<script lang="ts">
	import type { components } from '$lib/api/schema';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import ProgressTrack from '$lib/shared/components/data/ProgressTrack.svelte';

	type PlanUsage = components['schemas']['PlanUsage'];
	type BillingPlan = components['schemas']['BillingPlan'];

	let {
		planUsage,
		plan,
		isOwner
	}: {
		planUsage: PlanUsage;
		plan: BillingPlan | null;
		isOwner: boolean;
	} = $props();

	let hasLimits = $derived(
		planUsage.host_limit != null || planUsage.site_limit != null || planUsage.seat_limit != null
	);

	interface UsageRow {
		label: string;
		current: number;
		limit: number;
		pct: number;
		hasOverage: boolean;
	}

	let rows = $derived.by(() => {
		const list: UsageRow[] = [];
		if (planUsage.host_limit != null) {
			const pct = planUsage.host_count / planUsage.host_limit;
			list.push({
				label: 'Hosts',
				current: planUsage.host_count,
				limit: planUsage.host_limit,
				pct,
				hasOverage: plan?.host_cents != null
			});
		}
		if (planUsage.site_limit != null) {
			const pct = planUsage.site_count / planUsage.site_limit;
			list.push({
				label: 'Sites',
				current: planUsage.site_count,
				limit: planUsage.site_limit,
				pct,
				hasOverage: plan?.site_cents != null
			});
		}
		if (planUsage.seat_limit != null) {
			const pct = planUsage.seat_count / planUsage.seat_limit;
			list.push({
				label: 'Seats',
				current: planUsage.seat_count,
				limit: planUsage.seat_limit,
				pct,
				hasOverage: plan?.seat_cents != null
			});
		}
		return list;
	});

	let showUpgrade = $derived(
		hasLimits && isOwner && rows.some((r) => r.pct >= 0.8 && !r.hasOverage)
	);

	function barColor(row: UsageRow): string {
		if (row.hasOverage) return 'bg-blue-500';
		if (row.pct >= 0.8) return 'bg-yellow-500';
		return 'bg-blue-500';
	}

	function textColor(row: UsageRow): string {
		if (row.hasOverage) return 'text-secondary';
		if (row.pct >= 0.8) return 'text-yellow-600 dark:text-yellow-400';
		return 'text-secondary';
	}
</script>

{#if hasLimits}
	<section>
		<div class="mb-3 flex items-center justify-between">
			<h3 class="text-primary text-base font-semibold">Plan Usage</h3>
			{#if showUpgrade}
				<UpgradeButton feature="plan_usage" surface="home_plan_usage" gate_type="limit_hit" />
			{/if}
		</div>
		<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
			{#each rows as row (row.label)}
				<div class="card card-static">
					<div class="mb-2 flex items-center justify-between text-sm">
						<span class="text-secondary">{row.label}</span>
						<span class={textColor(row)}>{row.current} / {row.limit}</span>
					</div>
					{#if row.hasOverage && row.pct > 1}
						<ProgressTrack>
							<div class="flex h-full">
								<div
									class="h-full rounded-l-full bg-blue-500"
									style="width: {(1 / row.pct) * 100}%"
								></div>
								<div
									class="h-full rounded-r-full bg-green-500"
									style="width: {((row.pct - 1) / row.pct) * 100}%"
								></div>
							</div>
						</ProgressTrack>
					{:else}
						<ProgressTrack progress={Math.min(row.pct * 100, 100)} color={barColor(row)} />
					{/if}
				</div>
			{/each}
		</div>
	</section>
{/if}
