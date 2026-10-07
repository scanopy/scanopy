<script lang="ts">
	import { Braces } from 'lucide-svelte';
	import CodeContainer from '../data/CodeContainer.svelte';

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let { entities = [null] }: { entities?: (any | null)[] } = $props();

	let expanded = $state(false);
</script>

<!--
	A transparent row of its own just above the footer, so the toggle never sits on top of the
	content when it is scrolled to the bottom.
-->
{#if entities.length > 0}
	{#if expanded}
		<div class="shrink-0 bg-transparent px-4 pt-2">
			<CodeContainer expandable={false} expanded={true} code={JSON.stringify(entities, null, 2)} />
		</div>
	{/if}
	<div class="flex shrink-0 items-end bg-transparent px-3">
		<button
			type="button"
			class="text-tertiary hover:text-secondary flex items-center gap-1 p-1 text-xs transition-colors"
			aria-expanded={expanded}
			onclick={() => (expanded = !expanded)}
		>
			<Braces class="h-3 w-3" />
			<span>JSON</span>
		</button>
	</div>
{/if}
