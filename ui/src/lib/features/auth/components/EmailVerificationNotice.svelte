<script lang="ts">
	// Help text for an action the server refuses until the user verifies their email. Renders
	// nothing once verification isn't pending.
	import InlineWarning from '#lib/shared/components/feedback/InlineWarning.svelte';
	import {
		useEmailVerificationPending,
		useResendVerificationMutation
	} from '#lib/features/auth/queries.ts';
	import { common_resend, common_sending, verifyEmail_title } from '#lib/paraglide/messages.js';

	let { body }: { body: string } = $props();

	const verification = useEmailVerificationPending();
	const resendMutation = useResendVerificationMutation();

	function handleResend() {
		if (verification.email) resendMutation.mutate({ email: verification.email });
	}
</script>

{#if verification.pending}
	<InlineWarning title={verifyEmail_title()} {body}>
		<button
			type="button"
			class="font-medium underline hover:no-underline disabled:opacity-50"
			onclick={handleResend}
			disabled={resendMutation.isPending}
		>
			{resendMutation.isPending ? common_sending() : common_resend()}
		</button>
	</InlineWarning>
{/if}
