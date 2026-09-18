<script lang="ts" module>
	export function setLastLoginMethod(method: string) {
		localStorage.setItem('scanopy_last_login_method', method);
	}
</script>

<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, email } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { LoginRequest } from '../types/base';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import AuthMethodSelector from './AuthMethodSelector.svelte';
	import {
		auth_demoModeBody,
		auth_demoModeTitle,
		auth_dontHaveAccount,
		auth_enterYourEmail,
		auth_enterYourPassword,
		auth_forgotYourPassword,
		auth_passwordLoginDisabledNoProviders,
		auth_registerHere,
		auth_resetPassword,
		auth_scanopyLogo,
		auth_signInToScanopy,
		auth_signInWithEmail,
		auth_signingIn,
		auth_youreInvitedBody,
		auth_youreInvitedTitle,
		common_back,
		common_email,
		common_password
	} from '$lib/paraglide/messages';

	interface Props {
		orgName?: string | null;
		invitedBy?: string | null;
		demoMode?: boolean;
		isOpen?: boolean;
		onLogin: (data: LoginRequest) => Promise<void> | void;
		onClose: () => void;
		onSwitchToRegister?: (() => void) | null;
		onSwitchToForgot?: (() => void) | null;
	}

	let {
		orgName = null,
		invitedBy = null,
		demoMode = false,
		isOpen = false,
		onLogin,
		onClose,
		onSwitchToRegister = null,
		onSwitchToForgot = null
	}: Props = $props();

	let signingIn = $state(false);
	let oidcLoadingSlug = $state<string | null>(null);
	let subStep = $state<'method' | 'credentials'>('method');
	let lastLoginMethod = $state<string | null>(null);
	let hasAutoAdvanced = $state(false);

	const configQuery = useConfigQuery();
	let configData = $derived(configQuery.data);

	let disableRegistration = $derived(configData?.disable_registration ?? false);
	let disablePasswordLogin = $derived(configData?.disable_password_login ?? false);
	let oidcProviders = $derived(configData?.oidc_providers ?? []);
	let hasOidcProviders = $derived(oidcProviders.length > 0);
	let enablePasswordReset = $derived(configData?.has_email_service ?? false);

	// Auto-advance past method step when no OIDC providers or in demo mode
	$effect(() => {
		if (configData && (!hasOidcProviders || demoMode) && subStep === 'method' && !hasAutoAdvanced) {
			hasAutoAdvanced = true;
			subStep = 'credentials';
		}
	});

	// Create form
	const form = createForm(() => ({
		defaultValues: {
			email: '',
			password: ''
		},
		onSubmit: async ({ value }) => {
			signingIn = true;
			try {
				await onLogin({
					email: value.email.trim(),
					password: value.password
				});
			} finally {
				signingIn = false;
			}
		}
	}));

	// Reset form when modal opens
	function handleOpen() {
		form.reset({ email: '', password: '' });
		subStep = 'method';
		hasAutoAdvanced = false;
		lastLoginMethod = localStorage.getItem('scanopy_last_login_method');
	}

	function handleOidcLogin(providerSlug: string) {
		oidcLoadingSlug = providerSlug;
		const returnUrl = encodeURIComponent(window.location.origin);
		window.location.href = `/api/auth/oidc/${providerSlug}/authorize?flow=login&return_url=${returnUrl}`;
	}

	async function handleSubmit() {
		await submitForm(form);
	}
</script>

<GenericModal
	{isOpen}
	title={auth_signInToScanopy()}
	size="lg"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={false}
	showBackdrop={false}
	preventCloseOnClickOutside={true}
	centerTitle={true}
>
	{#snippet headerIcon()}
		<img src="/logos/scanopy-logo.png" alt={auth_scanopyLogo()} class="h-8 w-8" />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			if (subStep === 'credentials') {
				handleSubmit();
			}
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class={disablePasswordLogin && hasOidcProviders ? 'p-0' : 'flex-1 overflow-auto p-6'}>
			{#if disablePasswordLogin && !hasOidcProviders}
				<div class="p-6">
					<InlineDanger title={auth_passwordLoginDisabledNoProviders()} />
				</div>
			{:else if subStep === 'method'}
				<!-- Method selector step -->
				<AuthMethodSelector
					providers={oidcProviders}
					{lastLoginMethod}
					{disablePasswordLogin}
					{oidcLoadingSlug}
					onOidcSelect={handleOidcLogin}
					onEmailSelect={() => (subStep = 'credentials')}
				/>
			{:else}
				<!-- Credentials step -->
				{#if demoMode}
					<div class="mb-6">
						<InlineInfo title={auth_demoModeTitle()} body={auth_demoModeBody()} />
						<div class="card mt-3 !rounded-md !p-3 font-mono text-sm">
							<div class="text-secondary">
								<span class="text-tertiary">{common_email()}:</span>
								<span class="text-primary ml-2">demo@scanopy.net</span>
							</div>
							<div class="text-secondary mt-1">
								<span class="text-tertiary">{common_password()}:</span>
								<span class="text-primary ml-2">password123</span>
							</div>
						</div>
					</div>
				{:else if orgName && invitedBy}
					<div class="mb-6">
						<InlineInfo
							title={auth_youreInvitedTitle()}
							body={auth_youreInvitedBody({ orgName, invitedBy })}
						/>
					</div>
				{/if}

				<div class="space-y-6">
					<div class="space-y-4">
						<form.Field
							name="email"
							validators={{
								onBlur: ({ value }) => required(value) || email(value)
							}}
						>
							{#snippet children(field)}
								<TextInput
									label={common_email()}
									id="email"
									{field}
									placeholder={auth_enterYourEmail()}
									required
									autofocus
								/>
							{/snippet}
						</form.Field>

						<form.Field
							name="password"
							validators={{
								onBlur: ({ value }) => required(value)
							}}
						>
							{#snippet children(field)}
								<TextInput
									label={common_password()}
									id="password"
									type="password"
									{field}
									placeholder={auth_enterYourPassword()}
									required
								/>
							{/snippet}
						</form.Field>
					</div>
				</div>
			{/if}
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex w-full flex-col gap-4">
				{#if subStep === 'credentials' && !disablePasswordLogin}
					{#if hasOidcProviders && !demoMode}
						<div class="flex gap-3">
							<button type="button" onclick={() => (subStep = 'method')} class="btn-secondary">
								{common_back()}
							</button>
							<button type="submit" disabled={signingIn} class="btn-primary flex-1">
								{signingIn ? auth_signingIn() : auth_signInWithEmail()}
							</button>
						</div>
					{:else}
						<button type="submit" disabled={signingIn} class="btn-primary w-full">
							{signingIn ? auth_signingIn() : auth_signInWithEmail()}
						</button>
					{/if}

					{#if enablePasswordReset && !demoMode}
						<div class="text-center">
							<p class="text-tertiary text-sm">
								{auth_forgotYourPassword()}
								<button
									type="button"
									onclick={onSwitchToForgot}
									class="text-link font-medium hover:underline"
								>
									{auth_resetPassword()}
								</button>
							</p>
						</div>
					{/if}
				{/if}

				<!-- Register Link -->
				{#if onSwitchToRegister && !disableRegistration && !demoMode}
					<div class="text-center">
						<p class="text-tertiary text-sm">
							{auth_dontHaveAccount()}
							<button
								type="button"
								onclick={onSwitchToRegister}
								class="text-link font-medium hover:underline"
							>
								{auth_registerHere()}
							</button>
						</p>
					</div>
				{/if}
			</div>
		</div>
	</form>
</GenericModal>
