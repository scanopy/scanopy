<script lang="ts">
	/**
	 * The vertical guide lines before a row or section header in a tree grouping: one line per
	 * level in `depth`, each centred in its 1rem column. A section header's chevron fills the column
	 * one level in from its own lines, so the line its rows draw there starts directly under the
	 * chevron. Each line runs the full height of its cell, so it reads as one line down the section.
	 *
	 * The containing cell must be `position: relative`; `offset` is where the first column starts
	 * inside it, and must match between headers and rows for their lines to meet. Decorative: the
	 * section headers carry the nesting, so screen readers skip the lines.
	 */
	let { depth, offset = '0px' }: { depth: number; offset?: string } = $props();
</script>

{#each { length: depth }, index (index)}
	<span
		class="pointer-events-none absolute inset-y-0 w-px"
		style="left: calc({offset} + {index}rem + 0.5rem); background-color: var(--color-border)"
		aria-hidden="true"
	></span>
{/each}
