<script lang="ts">
	import { actionColumns } from '$lib/features/users/role-comparison';
</script>

<div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-tertiary">
	{#each actionColumns() as action (action.id)}
		{@const Icon = action.icon}
		<span class="flex items-center gap-1.5">
			<Icon class="h-3.5 w-3.5 text-primary" aria-hidden="true" />
			{action.label}
		</span>
	{/each}
</div>
