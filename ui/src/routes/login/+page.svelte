<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { themeStore } from '$lib/shared/stores/theme.svelte';
	import { goto } from '$app/navigation';
	import {
		useLoginMutation,
		useForgotPasswordMutation,
		useResetPasswordMutation
	} from '$lib/features/auth/queries';
	import LoginModal, { setLastLoginMethod } from '$lib/features/auth/components/LoginModal.svelte';
	import ForgotPasswordModal from '$lib/features/auth/components/ForgotPasswordModal.svelte';
	import ResetPasswordModal from '$lib/features/auth/components/ResetPasswordModal.svelte';
	import type { LoginRequest } from '$lib/features/auth/types/base';
	import Toast from '$lib/shared/components/feedback/Toast.svelte';
	import { navigate } from '$lib/shared/utils/navigation';
	import { fetchOrganization } from '$lib/features/organizations/queries';
	import { resolve } from '$app/paths';

	// TanStack Query mutations
	const loginMutation = useLoginMutation();
	const forgotPasswordMutation = useForgotPasswordMutation();
	const resetPasswordMutation = useResetPasswordMutation();

	type ModalType = 'login' | 'forgot' | 'reset';
	let activeModal = $state<ModalType>('login');
	let resetToken = $state<string>('');
	let demoMode = $state(false);

	onMount(() => {
		// Check if we have a reset token in the URL
		const token = $page.url.searchParams.get('token');
		if (token) {
			activeModal = 'reset';
			resetToken = token;
		}

		// Check if demo mode based on hostname
		demoMode = $page.url.hostname === 'demo.scanopy.net';

		// Note: Auth check is handled by +layout.svelte - no need to check here
	});

	async function handleLogin(data: LoginRequest) {
		try {
			await loginMutation.mutateAsync(data);
			setLastLoginMethod('email');
			// Fetch organization data before navigating
			await fetchOrganization();
			// Navigate to correct destination
			await navigate();
		} catch {
			// Error handled by mutation
		}
	}

	async function handleRequestReset(email: string) {
		try {
			await forgotPasswordMutation.mutateAsync({ email });
		} catch {
			// Error handled by mutation
		}
	}

	async function handleResetPassword(token: string, password: string) {
		try {
			await resetPasswordMutation.mutateAsync({ password, token });
			// Fetch organization data before navigating
			await fetchOrganization();
			// Navigate to correct destination
			await navigate();
		} catch {
			// Error handled by mutation
		}
	}

	function switchToForgot() {
		activeModal = 'forgot';
	}

	function switchToLogin() {
		activeModal = 'login';
	}

	function switchToSignUp() {
		// Preserve the query string so signup params (e.g. `hosting`, UTM) survive
		// eslint-disable-next-line svelte/no-navigation-without-resolve
		goto(`${resolve('/onboarding')}${$page.url.search}`);
	}

	// Dummy onClose since we don't want to close these modals
	function handleClose() {}
</script>

<div class="relative flex min-h-screen flex-col items-center bg-[var(--color-bg-elevated)] p-4">
	<!-- Background image with overlay -->
	<div class="absolute inset-0 z-0">
		<div
			class="h-full w-full bg-cover bg-center bg-no-repeat blur-[2px]"
			style="background-image: url('/images/background-{themeStore.resolvedTheme}.webp')"
		></div>
		<div
			class="absolute inset-0 {themeStore.resolvedTheme === 'dark' ? 'bg-black/30' : 'bg-white/15'}"
		></div>
	</div>

	<!-- GitHub Stars - positioned absolutely at bottom -->
	<!-- <div class="absolute bottom-10 left-10 z-[100] hidden md:block">
		<GithubStars />
	</div> -->

	<!-- Spacer to push modal down -->
	<div class="flex flex-1 items-center justify-center">
		<!-- Modal Content -->
		<div class="relative z-10">
			{#if activeModal === 'login'}
				<LoginModal
					isOpen={true}
					{demoMode}
					onLogin={handleLogin}
					onClose={handleClose}
					onSwitchToRegister={switchToSignUp}
					onSwitchToForgot={switchToForgot}
				/>
			{:else if activeModal === 'forgot'}
				<ForgotPasswordModal
					isOpen={true}
					onRequestReset={handleRequestReset}
					onClose={handleClose}
					onBackToLogin={switchToLogin}
				/>
			{:else if activeModal === 'reset'}
				<ResetPasswordModal
					isOpen={true}
					token={resetToken}
					onResetPassword={handleResetPassword}
					onClose={handleClose}
					onBackToLogin={switchToLogin}
				/>
			{/if}
		</div>
	</div>

	<Toast />
</div>
