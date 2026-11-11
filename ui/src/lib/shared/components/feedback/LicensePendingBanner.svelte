<script lang="ts">
	import { CloudOff } from '@lucide/svelte';
	import { license_pendingBanner } from '#lib/paraglide/messages.js';
	import {
		licenseProblemBanner,
		type LicenseProblem
	} from '#lib/features/settings/license-problem.ts';
	import AppBanner from './AppBanner.svelte';

	let { problem = null }: { problem?: LicenseProblem | null } = $props();
</script>

<AppBanner
	variant="danger"
	icon={CloudOff}
	body={problem ? licenseProblemBanner(problem) : license_pendingBanner()}
/>
