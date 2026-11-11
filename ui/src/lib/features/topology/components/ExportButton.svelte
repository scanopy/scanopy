<script lang="ts">
	import { Download } from '@lucide/svelte';
	import { common_export } from '#lib/paraglide/messages.js';

	let { onclick }: { onclick: () => void } = $props();
</script>

<button class="btn-secondary flex items-center gap-1" {onclick} title={common_export()}>
	<Download class="my-1 h-5 w-5" />
</button>
