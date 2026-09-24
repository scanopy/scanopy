<script lang="ts">
	import { CloudOff } from 'lucide-svelte';
	import { license_pendingBanner } from '$lib/paraglide/messages';
	import AppBanner from './AppBanner.svelte';
</script>

<AppBanner variant="danger" icon={CloudOff} body={license_pendingBanner()} />
