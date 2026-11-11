<script lang="ts">
	import { onMount } from 'svelte';
	import type { Snippet } from 'svelte';
	import { QueryClientProvider } from '@tanstack/svelte-query';
	import { queryClient } from '#lib/api/query-client.ts';
	import AppShell from '#lib/shared/components/layout/AppShell.svelte';
	import '../app.css';
	import { VERSION } from '#lib/version.ts';
	import { browser } from '$app/env';
	import { captureFirstTouch } from '#lib/shared/utils/first-touch.ts';
	import { trackPointerFocus } from '#lib/shared/utils/pointer-focus.ts';

	// Snapshot landing URL/UTMs before any auth redirect can rewrite the URL
	if (browser) {
		captureFirstTouch();
	}

	let { children }: { children: Snippet } = $props();

	onMount(() => {
		console.log('🌻 Scanopy v' + VERSION);
		return trackPointerFocus();
	});
</script>

<QueryClientProvider client={queryClient}><AppShell {children} /></QueryClientProvider>
