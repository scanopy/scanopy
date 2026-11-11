<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { AlertCircle, Home } from '@lucide/svelte';
	import { resolve } from '$app/paths';

	const message = $derived(page.url.searchParams.get('message') || 'An unexpected error occurred.');

	async function goHome() {
		await goto(resolve('/'));
	}
</script>

<div class="flex min-h-screen items-center justify-center bg-(--color-bg-elevated) p-8">
	<div class="w-full max-w-md">
		<div class="rounded-lg border border-red-600/30 bg-red-900/20 p-8 shadow-lg">
			<div class="mb-6 flex items-center gap-3">
				<AlertCircle class="h-8 w-8 text-red-400" />
				<h1 class="text-2xl font-bold text-white">Error</h1>
			</div>

			<p class="mb-6 text-gray-300">
				{message}
			</p>

			<button
				onclick={goHome}
				class="flex w-full items-center justify-center gap-2 rounded-sm bg-blue-600 px-4 py-3 font-medium text-white transition-colors hover:bg-blue-700"
			>
				<Home class="h-4 w-4" />
				Return to Application
			</button>
		</div>
	</div>
</div>
