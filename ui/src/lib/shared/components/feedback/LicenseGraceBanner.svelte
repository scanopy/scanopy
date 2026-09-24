<script lang="ts">
	import { ShieldAlert } from 'lucide-svelte';
	import { license_renewByBanner } from '$lib/paraglide/messages';
	import { formatLongDate } from '$lib/shared/utils/formatting';
	import AppBanner from './AppBanner.svelte';

	let { validThrough, hardExpiry }: { validThrough: string; hardExpiry: string } = $props();
</script>

<AppBanner
	variant="warning"
	icon={ShieldAlert}
	body={license_renewByBanner({
		validThrough: formatLongDate(validThrough),
		lockout: formatLongDate(hardExpiry, 'UTC')
	})}
/>
