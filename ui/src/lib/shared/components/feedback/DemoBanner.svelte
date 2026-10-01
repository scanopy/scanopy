<script lang="ts">
	import { Rocket } from 'lucide-svelte';
	import AppBanner from './AppBanner.svelte';
	import { BOOK_CALL_URL } from '$lib/features/support/support';
	import { auth_createAccount, demo_bannerBody, demo_talkToSales } from '$lib/paraglide/messages';
</script>

<AppBanner variant="info" icon={Rocket} body={demo_bannerBody()}>
	{#snippet actions()}
		<a
			href="https://app.scanopy.net/onboarding?utm_source=demo&utm_medium=in_app&utm_campaign=demo_signup&utm_content=banner_cta"
			target="_blank"
			rel="noopener noreferrer"
			class="ml-2 rounded px-2 py-0.5 text-xs font-medium underline hover:no-underline"
		>
			{auth_createAccount()}
		</a>
		<!-- eslint-disable svelte/no-navigation-without-resolve -->
		<a
			href={`${BOOK_CALL_URL}?overlayCalendar=true`}
			target="_blank"
			rel="noopener noreferrer"
			class="ml-2 rounded px-2 py-0.5 text-xs font-medium underline hover:no-underline"
		>
			{demo_talkToSales()}
		</a>
		<!-- eslint-enable svelte/no-navigation-without-resolve -->
	{/snippet}
</AppBanner>
