<script lang="ts">
	import { FlaskConical } from '@lucide/svelte';
	import { license_nonProductionBanner } from '#lib/paraglide/messages.js';
	import AppBanner from './AppBanner.svelte';
</script>

<AppBanner variant="info" icon={FlaskConical} body={license_nonProductionBanner()} />
