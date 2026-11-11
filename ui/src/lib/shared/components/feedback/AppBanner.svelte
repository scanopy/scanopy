<script lang="ts">
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { X } from '@lucide/svelte';
	import { common_dismiss } from '#lib/paraglide/messages.js';
	import type { IconComponent } from '#lib/shared/utils/types.ts';

	let {
		variant = 'warning',
		icon: Icon,
		body,
		actions,
		dismissableKey = null,
		onDismiss = null
	}: {
		variant: 'info' | 'warning' | 'danger';
		icon: IconComponent;
		/** Banner body. Rendered as HTML to support inline anchors (e.g. mailto links). */
		body: string;
		actions?: Snippet;
		dismissableKey?: string | null;
		onDismiss?: (() => void) | null;
	} = $props();

	const variantClasses = {
		info: 'border-blue-300 bg-blue-100 text-blue-700 dark:border-blue-600/30 dark:bg-blue-900/20 dark:text-blue-300',
		warning:
			'border-yellow-300 bg-yellow-100 text-warning dark:border-yellow-600/30 dark:bg-yellow-900/20',
		danger:
			'border-red-300 bg-red-100 text-red-700 dark:border-red-600/30 dark:bg-red-900/20 dark:text-red-300'
	};

	let dismissed = $state(false);

	onMount(() => {
		if (dismissableKey) {
			dismissed = localStorage.getItem(`appbanner_dismissed:${dismissableKey}`) === 'true';
		}
	});

	function dismiss() {
		if (!dismissableKey) return;
		localStorage.setItem(`appbanner_dismissed:${dismissableKey}`, 'true');
		dismissed = true;
		onDismiss?.();
	}
</script>

{#if !dismissed}
	<div class="border-b px-4 py-2 {variantClasses[variant]}">
		<div class="mx-auto flex items-center justify-center gap-2 text-sm">
			<Icon class="h-4 w-4 shrink-0" />
			<!-- eslint-disable-next-line svelte/no-at-html-tags -- body sources are paraglide-authored i18n strings, never user input -->
			<span>{@html body}</span>
			{#if actions}
				{@render actions()}
			{/if}
			{#if dismissableKey}
				<button
					onclick={dismiss}
					class="ml-2 shrink-0 rounded-sm p-0.5 transition-colors hover:bg-black/10 dark:hover:bg-white/10"
					aria-label={common_dismiss()}
				>
					<X class="h-4 w-4" />
				</button>
			{/if}
		</div>
	</div>
{/if}
