<script lang="ts">
	import { Braces } from 'lucide-svelte';
	import CodeContainer from '../data/CodeContainer.svelte';

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let { entities = [null] }: { entities?: (any | null)[] } = $props();

	let expanded = $state(false);
</script>

<!--
	Collapsed, the toggle floats over the bottom-left corner of the content above it from a
	zero-height anchor, so it covers only its own pill rather than taking a full-width row.
	Expanded, the JSON takes a row of its own: the modal body clips overflow, so a popover would
	be cut off in a short modal. Its bottom padding leaves room for the pill.
-->
{#if entities.length > 0}
	{#if expanded}
		<div class="shrink-0 px-4 pb-11 pt-2">
			<CodeContainer expandable={false} expanded={true} code={JSON.stringify(entities, null, 2)} />
		</div>
	{/if}
	<div class="relative h-0 shrink-0">
		<button
			type="button"
			class="text-tertiary hover:text-secondary absolute bottom-2 left-4 z-20 flex items-center gap-1 rounded-full border border-[var(--color-border)] bg-[var(--color-bg-elevated)] px-2 py-1 text-xs shadow-sm transition-colors"
			aria-expanded={expanded}
			onclick={() => (expanded = !expanded)}
		>
			<Braces class="h-3 w-3" />
			<span>JSON</span>
		</button>
	</div>
{/if}
