<script lang="ts">
	import { X, ArrowRight } from 'lucide-svelte';
	import { onMount } from 'svelte';

	let {
		id,
		title,
		description,
		actionLabel,
		onAction,
		onDismiss,
		dismissable = true
	}: {
		id: string;
		title: string;
		description: string;
		actionLabel: string;
		onAction: () => void;
		onDismiss?: () => void;
		dismissable?: boolean;
	} = $props();

	let dismissed = $state(false);
	const dismissKey = `nudge-dismissed:${id}`;

	onMount(() => {
		dismissed = localStorage.getItem(dismissKey) === 'true';
	});

	function dismiss() {
		localStorage.setItem(dismissKey, 'true');
		dismissed = true;
		onDismiss?.();
	}
</script>

{#if !dismissed}
	<div class="card card-static">
		<div class="flex items-start justify-between gap-3">
			<div class="flex-1">
				<h4 class="text-primary mb-1 text-sm font-semibold">{title}</h4>
				<p class="text-tertiary text-sm">{description}</p>
				<button
					onclick={onAction}
					class="mt-2 inline-flex items-center gap-1 text-sm font-medium text-blue-400 transition-colors hover:text-blue-300"
				>
					{actionLabel}
					<ArrowRight class="h-3.5 w-3.5" />
				</button>
			</div>
			{#if dismissable}
				<button
					onclick={dismiss}
					class="text-tertiary shrink-0 rounded p-0.5 transition-colors hover:bg-white/10"
					aria-label="Dismiss"
				>
					<X class="h-4 w-4" />
				</button>
			{/if}
		</div>
	</div>
{/if}
