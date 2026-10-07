<script lang="ts">
	import { Braces } from 'lucide-svelte';
	import CodeContainer from '../data/CodeContainer.svelte';

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let { entities = [null] }: { entities?: (any | null)[] } = $props();

	let expanded = $state(false);
</script>

<!--
	The toggle floats over the bottom-left corner of the element before it, from a zero-height
	anchor, with a background behind its text only. The `.entity-metadata-toggle` rule in app.css
	pads that element's bottom so its content scrolls clear of the toggle.
-->
{#if entities.length > 0}
	{#if expanded}
		<div class="shrink-0 px-4 pt-2">
			<CodeContainer expandable={false} expanded={true} code={JSON.stringify(entities, null, 2)} />
		</div>
	{/if}
	<div class="entity-metadata-toggle relative h-0 shrink-0">
		<button
			type="button"
			class="text-tertiary hover:text-secondary absolute bottom-1.5 left-3 z-10 flex items-center gap-1 rounded bg-[var(--color-bg-elevated)] px-1.5 py-0.5 text-xs transition-colors"
			aria-expanded={expanded}
			onclick={() => (expanded = !expanded)}
		>
			<Braces class="h-3 w-3" />
			<span>JSON</span>
		</button>
	</div>
{/if}
