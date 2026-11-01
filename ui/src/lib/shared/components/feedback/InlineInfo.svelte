<script lang="ts">
	import { Info } from '@lucide/svelte';
	import type { Snippet } from 'svelte';
	import BaseInlineFeedback from './BaseInlineFeedback.svelte';

	let {
		title = '',
		body = null,
		dismissableKey = null,
		children
	}: {
		title?: string;
		body?: string | null;
		dismissableKey?: string | null;
		children?: Snippet;
	} = $props();
</script>

<BaseInlineFeedback
	{title}
	{body}
	{dismissableKey}
	{children}
	Icon={Info}
	borderColor="border-blue-200 dark:border-blue-600/30"
	bgColor="bg-blue-50 dark:bg-blue-900/20"
	textColor="text-info"
/>
