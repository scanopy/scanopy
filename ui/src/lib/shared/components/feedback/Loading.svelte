<script lang="ts">
	import { Radar } from '@lucide/svelte';
	import { common_loading } from '#lib/paraglide/messages.js';

	export let message: string | undefined = undefined;

	$: displayMessage = message ?? common_loading();
</script>

<div class="py-8 text-center">
	<Radar class="mx-auto h-8 w-8 animate-[spin_1.5s_linear_infinite] text-blue-500" />
	<p class="mt-2 text-secondary">{displayMessage}</p>
</div>
