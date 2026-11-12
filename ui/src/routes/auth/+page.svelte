<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { SvelteURLSearchParams } from 'svelte/reactivity';
	import { resolve } from '$app/paths';

	onMount(() => {
		// Get URL params
		const invitedBy = page.url.searchParams.get('invited_by');

		const token = page.url.searchParams.get('token');
		const orgName = page.url.searchParams.get('org_name');

		// Build redirect URL with params
		if (invitedBy) {
			// Invite flow: redirect to onboarding with params
			const params = new SvelteURLSearchParams();
			if (orgName) params.set('org_name', orgName);
			params.set('invited_by', invitedBy);
			goto(`${resolve('onboarding')}?${params.toString()}`, { replace: true });
		} else if (token) {
			// Password reset flow: redirect to login with token
			goto(`${resolve('login')}?token=${token}`, { replace: true });
		} else {
			// Default: redirect to login
			goto(resolve('login'), { replace: true });
		}
	});
</script>

<!-- Redirect page - shows briefly while redirecting -->
<div class="flex min-h-screen items-center justify-center bg-(--color-bg-elevated)">
	<div class="text-secondary">Redirecting...</div>
</div>
