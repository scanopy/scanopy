<script lang="ts">
	import { CloudOff } from 'lucide-svelte';
	import { license_pendingBanner } from '$lib/paraglide/messages';
	import AppBanner from './AppBanner.svelte';
</script>

<AppBanner variant="warning" icon={CloudOff} body={license_pendingBanner()} />
