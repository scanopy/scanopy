<!--
	The search box every search in the app uses: the Cmd+K palette, the topology's Cmd+F find and
	each list page's filter. It is the same input as every other text field (`input-field`), with
	the search icon inside it, the shortcut that focuses it as a key chip on the right while it is
	empty, and a clear button in that place once it has text.

	Bound to a TanStack field. The caller keeps its own `$state` copy of the query for reactive
	reads, since `field.state.value` is not tracked by `$derived`; `onInput` is where it updates it.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { Search, X } from 'lucide-svelte';
	import KbdKey from '$lib/shared/components/feedback/KbdKey.svelte';
	import { common_clearSearch } from '$lib/paraglide/messages';

	let {
		field,
		value,
		id,
		placeholder,
		shortcut = undefined,
		inset = true,
		inputEl = $bindable(),
		onInput,
		onClear = undefined,
		onkeydown,
		trailing
	}: {
		field: AnyFieldApi;
		/** The caller's reactive copy of the query, which the input renders. */
		value: string;
		id: string;
		/** Names what the search covers, so it needs no separate help text. */
		placeholder: string;
		/** The key that focuses this search, shown as a chip while it is empty. */
		shortcut?: string;
		/** Pad the box for a modal or overlay; off where the surrounding toolbar spaces it. */
		inset?: boolean;
		inputEl?: HTMLInputElement;
		onInput?: (value: string) => void;
		/** Shows a clear button while there is text. */
		onClear?: () => void;
		onkeydown?: (event: KeyboardEvent) => void;
		trailing?: Snippet;
	} = $props();

	function handleInput(event: Event) {
		const next = (event.currentTarget as HTMLInputElement).value;
		field.handleChange(next);
		onInput?.(next);
	}

	let showClear = $derived(!!onClear && value.length > 0);
	let showShortcut = $derived(!!shortcut && !showClear);
</script>

<div class="flex items-center gap-2 {inset ? 'px-4 py-3' : ''}">
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
			aria-keyshortcuts={shortcut}
			title={placeholder}
			class="input-field w-full pl-10 text-sm {showClear || showShortcut ? 'pr-12' : ''}"
		/>
		{#if showClear}
			<button
				type="button"
				onclick={() => {
					onClear?.();
					inputEl?.focus();
				}}
				class="text-tertiary hover:text-secondary absolute right-3 top-1/2 -translate-y-1/2 transition-colors"
				aria-label={common_clearSearch()}
			>
				<X class="h-4 w-4" />
			</button>
		{:else if showShortcut}
			<KbdKey
				key={shortcut!}
				size="sm"
				class="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2"
			/>
		{/if}
	</div>
	{#if trailing}
		{@render trailing()}
	{/if}
</div>
