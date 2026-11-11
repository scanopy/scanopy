<script lang="ts">
	import { CloudOff } from '@lucide/svelte';
	import AppBanner from './AppBanner.svelte';
	import { serverUnreachable } from '#lib/api/server-reachability.ts';
	import { serverUnreachable_body } from '#lib/paraglide/messages.js';
</script>

<!-- Not dismissable: it clears by itself once the server answers again. -->
{#if $serverUnreachable}
	<AppBanner variant="warning" icon={CloudOff} body={serverUnreachable_body()} />
{/if}
