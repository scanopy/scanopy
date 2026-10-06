<script lang="ts">
	import type { TreeGuide } from './controls/grouping';

	/**
	 * The vertical guide lines before a row or section header in a tree grouping, one 1rem column
	 * per entry in `guides`. Each `bar` runs the full height of its cell, so a section's line reads
	 * as one line down every row inside it.
	 *
	 * The containing cell must be `position: relative`; `offset` is where the first column starts
	 * inside it. Decorative: the section headers carry the nesting, so screen readers skip the
	 * lines.
	 */
	let { guides, offset = '0px' }: { guides: TreeGuide[]; offset?: string } = $props();
</script>

{#each guides as guide, index (index)}
	{#if guide === 'bar'}
		<span
			class="pointer-events-none absolute inset-y-0 w-px"
			style="left: calc({offset} + {index}rem + 0.5rem); background-color: var(--color-border)"
			aria-hidden="true"
		></span>
	{/if}
{/each}
