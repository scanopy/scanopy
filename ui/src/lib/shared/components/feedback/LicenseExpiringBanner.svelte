<script lang="ts">
	import { Clock } from 'lucide-svelte';
	import { license_renewByBanner, license_validThroughBanner } from '$lib/paraglide/messages';
	import { formatLongDate } from '$lib/shared/utils/formatting';
	import AppBanner from './AppBanner.svelte';

	let { validThrough, hardExpiry }: { validThrough: string; hardExpiry: string } = $props();

	// A cloud-minted key reaches this window after its paid period has ended, so
	// the date is usually past. A key minted without a paid-through date reaches
	// it before its expiry.
	let body = $derived(
		Date.parse(validThrough) > Date.now()
			? license_validThroughBanner({ validThrough: formatLongDate(validThrough) })
			: license_renewByBanner({
					validThrough: formatLongDate(validThrough),
					lockout: formatLongDate(hardExpiry, 'UTC')
				})
	);
</script>

<AppBanner variant="info" icon={Clock} {body} />
