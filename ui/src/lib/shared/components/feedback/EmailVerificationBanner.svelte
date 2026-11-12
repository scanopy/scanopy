<script lang="ts">
	import { AlertTriangle } from '@lucide/svelte';
	import { useResendVerificationMutation } from '#lib/features/auth/queries.ts';
	import AppBanner from './AppBanner.svelte';
	import {
		auth_verifyEmailBannerBody,
		common_resend,
		common_sending
	} from '#lib/paraglide/messages.js';

	let { email }: { email: string } = $props();

	const resendMutation = useResendVerificationMutation();

	// The mutation shows the success toast; the API client toasts errors.
	function handleResend() {
		resendMutation.mutate({ email });
	}
</script>

<AppBanner variant="warning" icon={AlertTriangle} body={auth_verifyEmailBannerBody()}>
	{#snippet actions()}
		<button
			onclick={handleResend}
			disabled={resendMutation.isPending}
			class="ml-2 rounded-sm px-2 py-0.5 text-xs font-medium underline hover:no-underline disabled:opacity-50"
		>
			{resendMutation.isPending ? common_sending() : common_resend()}
		</button>
	{/snippet}
</AppBanner>
