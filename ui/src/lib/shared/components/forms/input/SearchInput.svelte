<!--
	The search box shared by the Cmd+K palette and the topology's Cmd+F search. It is the same
	input as every other text field (`input-field`), with the search icon inside it the way the
	table search shows it, plus optional trailing controls beside it.

	Bound to a TanStack field. The caller keeps its own `$state` copy of the query for reactive
	reads, since `field.state.value` is not tracked by `$derived`; `onInput` is where it updates it.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { Search } from 'lucide-svelte';

	let {
		field,
		value,
		id,
		placeholder,
		inputEl = $bindable(),
		onInput,
		onkeydown,
		trailing
	}: {
		field: AnyFieldApi;
		/** The caller's reactive copy of the query, which the input renders. */
		value: string;
		id: string;
		/** Names what the search covers, so it needs no separate help text. */
		placeholder: string;
		inputEl?: HTMLInputElement;
		onInput?: (value: string) => void;
		onkeydown?: (event: KeyboardEvent) => void;
		trailing?: Snippet;
	} = $props();

	function handleInput(event: Event) {
		const next = (event.currentTarget as HTMLInputElement).value;
		field.handleChange(next);
		onInput?.(next);
	}
</script>

<div class="flex items-center gap-2 px-4 py-3">
	<div class="relative min-w-0 flex-1">
		<Search
			class="text-tertiary pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2"
		/>
		<input
			bind:this={inputEl}
			{id}
			{value}
			oninput={handleInput}
			{onkeydown}
			type="text"
			autocomplete="off"
			{placeholder}
			aria-label={placeholder}
			title={placeholder}
			class="input-field w-full pl-10 text-sm"
		/>
	</div>
	{#if trailing}
		{@render trailing()}
	{/if}
</div>
