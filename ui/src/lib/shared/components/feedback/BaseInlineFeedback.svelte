<script lang="ts">
	import type { IconComponent } from '$lib/shared/utils/types';
	import { X } from 'lucide-svelte';
	import { onMount } from 'svelte';

	let {
		title,
		body = null,
		dismissableKey = null,
		Icon,
		borderColor,
		bgColor,
		textColor
	}: {
		title: string;
		body?: string | null;
		dismissableKey?: string | null;
		Icon: IconComponent;
		borderColor: string;
		bgColor: string;
		textColor: string;
	} = $props();

	let dismissed = $state(false);

	onMount(() => {
		if (dismissableKey) {
			dismissed = localStorage.getItem(dismissableKey) === 'true';
		}
	});

	function dismiss() {
		if (dismissableKey) {
			localStorage.setItem(dismissableKey, 'true');
			dismissed = true;
		}
	}
</script>

{#if !dismissed}
	<div class="rounded-lg border p-2.5 {borderColor} {bgColor}">
		<div class="flex items-start gap-2">
			<Icon class="mt-0.5 h-4 w-4 shrink-0 {textColor}" />
			<div class="flex-1">
				{#if title}
					<!-- eslint-disable-next-line svelte/no-at-html-tags -- trusted: all callers pass i18n or hardcoded strings -->
					<p class="text-sm font-medium {textColor}">{@html title}</p>
				{/if}
				{#if body}
					<!-- eslint-disable-next-line svelte/no-at-html-tags -- trusted: all callers pass i18n or hardcoded strings -->
					<p class={`${title ? 'mt-1' : ''} text-sm ${textColor}`}>{@html body}</p>
				{/if}
			</div>
			{#if dismissableKey}
				<button
					onclick={dismiss}
					class="shrink-0 rounded p-0.5 transition-colors hover:bg-white/10"
					aria-label="Dismiss"
				>
					<X class="h-4 w-4 {textColor}" />
				</button>
			{/if}
		</div>
	</div>
{/if}
