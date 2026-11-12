<script lang="ts">
	import { ShieldAlert } from '@lucide/svelte';
	import {
		license_expiredBanner,
		license_instanceRefusedBanner,
		license_invalidBanner
	} from '#lib/paraglide/messages.js';
	import {
		licenseProblemBanner,
		type LicenseProblem
	} from '#lib/features/settings/license-problem.ts';
	import AppBanner from './AppBanner.svelte';

	let {
		status,
		refusedFor = null,
		problem = null
	}: {
		status: 'expired' | 'invalid';
		// Set when the cloud refused this server for running on more servers
		// than the plan covers.
		refusedFor?: { active: number; limit: number } | null;
		// Why the key is unusable, when the server knows.
		problem?: LicenseProblem | null;
	} = $props();
</script>

<AppBanner
	variant="danger"
	icon={ShieldAlert}
	body={status === 'expired'
		? license_expiredBanner()
		: refusedFor
			? license_instanceRefusedBanner(refusedFor)
			: problem
				? licenseProblemBanner(problem)
				: license_invalidBanner()}
/>
