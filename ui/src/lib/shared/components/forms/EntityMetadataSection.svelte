<script lang="ts">
	import { Braces } from 'lucide-svelte';
	import CodeContainer from '../data/CodeContainer.svelte';

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let { entities = [null] }: { entities?: (any | null)[] } = $props();

	let expanded = $state(false);
</script>

<!--
	A row of its own between the content and the footer, so the toggle never covers content.
	The row has no background; only the pill does.
-->
{#if entities.length > 0}
	{#if expanded}
		<div class="shrink-0 px-4 pt-2">
			<CodeContainer expandable={false} expanded={true} code={JSON.stringify(entities, null, 2)} />
		</div>
	{/if}
	<div class="shrink-0 px-4 py-2">
		<button
			type="button"
			class="text-tertiary hover:text-secondary flex items-center gap-1 rounded-full border border-[var(--color-border)] bg-[var(--color-bg-elevated)] px-2 py-1 text-xs shadow-sm transition-colors"
			aria-expanded={expanded}
			onclick={() => (expanded = !expanded)}
		>
			<Braces class="h-3 w-3" />
			<span>JSON</span>
		</button>
	</div>
{/if}
