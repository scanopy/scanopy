<script lang="ts">
	import { toastStore, dismissToast, type Toast } from '$lib/shared/stores/feedback';
	import { fly } from 'svelte/transition';
	import { AlertCircle, AlertTriangle, Info, CheckCircle, X } from 'lucide-svelte';
	import { common_dismiss } from '$lib/paraglide/messages';

	$: toasts = $toastStore;

	function getToastIcon(type: Toast['type']) {
		switch (type) {
			case 'error':
				return AlertCircle;
			case 'warning':
				return AlertTriangle;
			case 'info':
				return Info;
			case 'success':
				return CheckCircle;
			default:
				return Info;
		}
	}

	function getToastColors(type: Toast['type']) {
		switch (type) {
			case 'error':
				return {
					bg: 'bg-red-900/90',
					border: 'border-red-500/50',
					icon: 'text-red-400',
					text: 'text-red-100',
					button: 'text-red-300 hover:text-red-200'
				};
			case 'warning':
				return {
					bg: 'bg-yellow-900/90',
					border: 'border-yellow-500/50',
					icon: 'text-yellow-400',
					text: 'text-yellow-100',
					button: 'text-yellow-300 hover:text-yellow-200'
				};
			case 'info':
				return {
					bg: 'bg-blue-900/90',
					border: 'border-blue-500/50',
					icon: 'text-blue-400',
					text: 'text-blue-100',
					button: 'text-blue-300 hover:text-blue-200'
				};
			case 'success':
				return {
					bg: 'bg-green-900/90',
					border: 'border-green-500/50',
					icon: 'text-green-400',
					text: 'text-green-100',
					button: 'text-green-300 hover:text-green-200'
				};
			default:
				return {
					bg: 'bg-gray-900/90',
					border: 'border-gray-500/50',
					icon: 'text-gray-400',
					text: 'text-gray-100',
					button: 'text-gray-300 hover:text-gray-200'
				};
		}
	}

	function handleActionClick(action: () => void, toastId: string) {
		action();
		dismissToast(toastId);
	}
</script>

<!-- Toast Container -->
<!-- Above the modal band. Modals are also z-50 and the billing modal is rendered
     last on the page, so an equal z-index left toasts painted behind it by DOM
     order. Fixed here rather than by reordering, because four routes mount this. -->
<div class="pointer-events-none fixed bottom-4 right-4 z-[100] flex flex-col gap-2">
	{#each toasts as toast (toast.id)}
		{@const colors = getToastColors(toast.type)}
		{@const IconComponent = getToastIcon(toast.type)}

		<div
			class="pointer-events-auto min-w-80 max-w-md rounded-lg border {colors.bg} {colors.border} shadow-lg backdrop-blur-sm"
			transition:fly={{ x: 300, duration: 300 }}
		>
			<div class="p-4">
				<div class="flex items-start gap-3">
					<!-- Icon -->
					<div class="mt-0.5 flex-shrink-0">
						<svelte:component this={IconComponent} class="h-5 w-5 {colors.icon}" />
					</div>

					<!-- Content -->
					<div class="min-w-0 flex-1">
						<p class="text-sm {colors.text} break-words">
							{toast.message}
						</p>

						<!-- Actions -->
						{#if toast.actions && toast.actions.length > 0}
							<div class="mt-3 flex gap-2">
								{#each toast.actions as action (action.label)}
									<button
										type="button"
										on:click={() => handleActionClick(action.action, toast.id)}
										class="rounded px-2 py-1 text-xs {action.style === 'primary'
											? 'bg-white/20 text-white hover:bg-white/30'
											: `${colors.button} hover:underline`} transition-colors"
									>
										{action.label}
									</button>
								{/each}
							</div>
						{/if}
					</div>

					<!-- Dismiss Button -->
					{#if toast.dismissible !== false}
						<button
							type="button"
							on:click={() => dismissToast(toast.id)}
							class="flex-shrink-0 rounded p-1 {colors.button} transition-colors hover:bg-white/10"
							title={common_dismiss()}
						>
							<X class="h-4 w-4" />
						</button>
					{/if}
				</div>
			</div>

			<!-- Progress bar for timed toasts -->
			{#if toast.timeout && toast.timeout > 0}
				<div class="h-1 overflow-hidden rounded-b-lg bg-white/20">
					<div
						class="animate-shrink h-full rounded-b-lg bg-white/40"
						style="animation-duration: {toast.timeout}ms"
					></div>
				</div>
			{/if}
		</div>
	{/each}
</div>

<style>
	@keyframes shrink {
		from {
			width: 100%;
		}
		to {
			width: 0%;
		}
	}

	.animate-shrink {
		animation: shrink linear;
	}
</style>
