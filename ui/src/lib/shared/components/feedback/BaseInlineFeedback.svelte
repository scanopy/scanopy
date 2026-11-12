<script lang="ts">
	import type { IconComponent } from '#lib/shared/utils/types.ts';
	import { X } from '@lucide/svelte';
	import { onMount, type Snippet } from 'svelte';
	import { common_dismiss } from '#lib/paraglide/messages.js';

	let {
		title,
		body = null,
		items = null,
		dismissableKey = null,
		Icon,
		borderColor,
		bgColor,
		textColor,
		children
	}: {
		title: string;
		body?: string | null;
		/**
		 * Several independent statements, rendered as a bulleted list. Prefer this over joining
		 * them into `body`: a run that hit three unrelated problems reads as three problems.
		 *
		 * Unlike `body` these are escaped, not `{@html}`-ed — items can carry text this app did
		 * not author (a scan warning quotes the error a remote endpoint returned), so they must
		 * never be interpreted as markup.
		 */
		items?: string[] | null;
		dismissableKey?: string | null;
		Icon: IconComponent;
		borderColor: string;
		bgColor: string;
		textColor: string;
		/** An action that belongs to the message, rendered under the text (e.g. a Resend link). */
		children?: Snippet;
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
				{#if items && items.length > 0}
					<ul
						class={`${title || body ? 'mt-1' : ''} list-disc space-y-1 pl-4 text-sm ${textColor}`}
					>
						{#each items as item, i (i)}
							<li>{item}</li>
						{/each}
					</ul>
				{/if}
				{#if children}
					<div class="mt-1 text-sm {textColor}">{@render children()}</div>
				{/if}
			</div>
			{#if dismissableKey}
				<button
					onclick={dismiss}
					class="shrink-0 rounded-sm p-0.5 transition-colors hover:bg-white/10"
					aria-label={common_dismiss()}
				>
					<X class="h-4 w-4 {textColor}" />
				</button>
			{/if}
		</div>
	</div>
{/if}
