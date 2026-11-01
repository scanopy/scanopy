<script lang="ts">
	/**
	 * One icon per action, in a fixed order, so rows line up into columns. An action `held` doesn't
	 * include keeps its slot, dimmed.
	 */
	import type { components } from '$lib/api/schema';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { actionColumns } from '$lib/features/users/role-access';

	let { held }: { held: components['schemas']['DisplayAction'][] } = $props();
</script>

<div class="flex shrink-0 items-center gap-1.5">
	{#each actionColumns() as action (action.id)}
		{@const Icon = action.icon}
		{#if held.includes(action.id)}
			<span class="text-primary" data-tooltip={action.label} aria-label={action.label} use:tooltip>
				<Icon class="h-3.5 w-3.5" />
			</span>
		{:else}
			<Icon class="h-3.5 w-3.5 text-muted opacity-30" aria-hidden="true" />
		{/if}
	{/each}
</div>
