<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import AuthBackground from '$lib/shared/components/layout/AuthBackground.svelte';
	import { goto, preloadCode } from '$app/navigation';
	import {
		useVerifyEmailMutation,
		useResendVerificationMutation
	} from '$lib/features/auth/queries';
	import Toast from '$lib/shared/components/feedback/Toast.svelte';
	import { navigate } from '$lib/shared/utils/navigation';
	import { fetchOrganization } from '$lib/features/organizations/queries';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { Mail } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import {
		auth_backToLogin,
		auth_checkYourEmail,
		common_sending,
		verifyEmail_didntReceive,
		verifyEmail_failed,
		verifyEmail_noToken,
		verifyEmail_noTokenBody,
		verifyEmail_resend,
		verifyEmail_sentLinkTo,
		verifyEmail_title,
		verifyEmail_tryRegisteringAgain,
		verifyEmail_verified,
		verifyEmail_verifiedBody,
		verifyEmail_verifying,
		verifyEmail_verifyingBody
	} from '$lib/paraglide/messages';

	const verifyMutation = useVerifyEmailMutation();
	const resendMutation = useResendVerificationMutation();

	let isResending = $derived(resendMutation.isPending);

	type Status = 'verifying' | 'success' | 'error' | 'no-token' | 'pending';
	let status = $state<Status>('verifying');
	let errorMessage = $state('');
	let email = $state('');

	onMount(async () => {
		const token = $page.url.searchParams.get('token');
		const emailParam = $page.url.searchParams.get('email');

		if (emailParam) {
			email = emailParam;
		}

		if (!token) {
			// No token - show pending state for resend
			status = emailParam ? 'pending' : 'no-token';
			return;
		}

		// The app follows straight after verification, so its code is ready by then.
		void preloadCode(resolve('/'));

		try {
			await verifyMutation.mutateAsync({ token });
			status = 'success';
			// The app decides what to open from the org, so it arrives with this in the cache.
			await fetchOrganization();
			void navigate();
		} catch (e) {
			status = 'error';
			errorMessage = e instanceof Error ? e.message : verifyEmail_failed();
		}
	});

	async function handleResend() {
		if (!email) return;
		try {
			await resendMutation.mutateAsync({ email });
		} catch {
			// Error handled by mutation
		}
	}

	function handleBackToLogin() {
		goto(resolve('/login'));
	}
</script>

<div class="relative flex min-h-screen flex-col items-center bg-(--color-bg-elevated) p-4">
	<AuthBackground />

	<!-- Spacer to push modal down -->
	<div class="flex flex-1 items-center justify-center">
		<!-- Modal Content -->
		<div class="relative z-10">
			<GenericModal
				isOpen={true}
				onClose={() => {}}
				showCloseButton={false}
				preventCloseOnClickOutside={true}
				title={verifyEmail_title()}
			>
				{#snippet headerIcon()}
					<ModalHeaderIcon Icon={Mail} color="Blue" />
				{/snippet}

				{#if status === 'verifying'}
					<div class="p-6 text-center">
						<h2 class="mb-2 text-xl font-semibold text-primary">{verifyEmail_verifying()}</h2>
						<p class="text-tertiary">{verifyEmail_verifyingBody()}</p>
					</div>
				{:else if status === 'success'}
					<div class="p-6 text-center">
						<h2 class="mb-2 text-xl font-semibold text-primary">{verifyEmail_verified()}</h2>
						<p class="text-tertiary">{verifyEmail_verifiedBody()}</p>
					</div>
				{:else if status === 'error'}
					<div class="p-6 text-center">
						<h2 class="mb-2 text-xl font-semibold text-primary">{verifyEmail_failed()}</h2>
						<p class="mb-4 text-tertiary">{errorMessage}</p>
						{#if email}
							<button
								type="button"
								class="btn-primary w-full"
								onclick={handleResend}
								disabled={isResending}
							>
								{isResending ? common_sending() : verifyEmail_resend()}
							</button>
						{:else}
							<p class="text-sm text-muted">
								{verifyEmail_tryRegisteringAgain()}
							</p>
						{/if}
					</div>
				{:else if status === 'pending'}
					<div class="p-6 text-center">
						<h2 class="mb-2 text-xl font-semibold text-primary">{auth_checkYourEmail()}</h2>
						<p class="mb-4 text-tertiary">
							{verifyEmail_sentLinkTo({ email })}
						</p>
						<div class="space-y-3">
							<p class="text-sm text-muted">{verifyEmail_didntReceive()}</p>
							<button
								type="button"
								class="btn-secondary w-full"
								onclick={handleResend}
								disabled={isResending}
							>
								{isResending ? common_sending() : verifyEmail_resend()}
							</button>
						</div>
					</div>
				{:else}
					<div class="p-6 text-center">
						<h2 class="mb-2 text-xl font-semibold text-primary">{verifyEmail_noToken()}</h2>
						<p class="mb-4 text-tertiary">
							{verifyEmail_noTokenBody()}
						</p>
						<button type="button" class="btn-secondary w-full" onclick={handleBackToLogin}>
							{auth_backToLogin()}
						</button>
					</div>
				{/if}
			</GenericModal>
		</div>
	</div>

	<Toast />
</div>
