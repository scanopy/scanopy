<script lang="ts">
	import { Server } from '@lucide/svelte';
	import { license_instanceLimitBanner } from '#lib/paraglide/messages.js';
	import AppBanner from './AppBanner.svelte';

	let { active, limit }: { active: number; limit: number } = $props();
</script>

<AppBanner variant="warning" icon={Server} body={license_instanceLimitBanner({ active, limit })} />
