<script lang="ts">
	import { X } from '@lucide/svelte';
	import { onMount } from 'svelte';
	import { common_dismiss } from '#lib/paraglide/messages.js';

	export let title: string | null = null;
	export let variant: 'default' | 'compact' = 'default';
	export let dismissableKey: string | null = null;
	export let onDismiss: (() => void) | null = null;

	let dismissed = false;

	onMount(() => {
		if (dismissableKey) {
			dismissed = localStorage.getItem(`infocard_dismissed:${dismissableKey}`) === 'true';
		}
	});

	function dismiss() {
		if (!dismissableKey) return;
		localStorage.setItem(`infocard_dismissed:${dismissableKey}`, 'true');
		dismissed = true;
		onDismiss?.();
	}
</script>

{#if !dismissed}
	<div class="card-static card">
		<div class="flex items-start gap-2">
			<div class="min-w-0 flex-1">
				{#if title}
					<h3 class="mb-3 text-sm font-semibold text-primary">{title}</h3>
				{/if}
				<div class={variant === 'compact' ? 'space-y-2' : 'space-y-3'}>
					<slot />
				</div>
			</div>
			{#if dismissableKey}
				<button
					on:click={dismiss}
					class="shrink-0 rounded-sm p-0.5 text-secondary transition-colors hover:bg-white/10 hover:text-primary"
					aria-label={common_dismiss()}
				>
					<X class="h-4 w-4" />
				</button>
			{/if}
		</div>
	</div>
{/if}
