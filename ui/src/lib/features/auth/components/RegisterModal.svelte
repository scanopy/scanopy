<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import {
		required,
		email as emailValidator,
		password as passwordValidator,
		confirmPasswordMatch
	} from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import Password from '$lib/shared/components/forms/input/Password.svelte';
	import { Loader2 } from 'lucide-svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { useCheckEmailMutation } from '../queries';
	import type { RegisterRequest } from '../types/base';
	import {
		auth_continueWithEmail,
		auth_createAccount,
		auth_createAccountWith,
		auth_createYourAccount,
		auth_creatingAccount,
		auth_emailAlreadyInUse,
		auth_enterYourEmail,
		auth_passwordLoginDisabledNoProviders,
		auth_scanopyLogo,
		auth_signInInstead,
		auth_signUpForUpdates,
		auth_termsAndPrivacy,
		auth_youreInvitedBody,
		auth_youreInvitedTitle,
		common_back,
		common_change,
		common_continue,
		common_email,
		common_or
	} from '$lib/paraglide/messages';

	let {
		orgName = null,
		invitedBy = null,
		isOpen = false,
		onRegister,
		onClose,
		onSwitchToLogin
	}: {
		orgName?: string | null;
		invitedBy?: string | null;
		isOpen?: boolean;
		onRegister: (data: RegisterRequest, subscribed: boolean) => Promise<void> | void;
		onClose: () => void;
		onSwitchToLogin?: () => void;
	} = $props();

	let registering = $state(false);
	let oidcLoadingSlug = $state<string | null>(null);
	let subStep = $state<'method' | 'email' | 'password'>('method');
	let emailValue = $state('');
	let emailError = $state<'email_in_use' | 'generic' | null>(null);
	let checkingEmail = $state(false);
	let honeypotValue = $state('');
	let hasAutoAdvanced = $state(false);

	const configQuery = useConfigQuery();
	let configData = $derived(configQuery.data);

	let disablePasswordLogin = $derived(configData?.disable_password_login ?? false);
	let oidcProviders = $derived(configData?.oidc_providers ?? []);
	let hasOidcProviders = $derived(oidcProviders.length > 0);
	let enableEmailOptIn = $derived(configData?.has_email_opt_in ?? false);
	let enableTermsCheckbox = $derived(configData?.billing_enabled ?? false);

	$effect(() => {
		if (
			configData &&
			!hasOidcProviders &&
			!disablePasswordLogin &&
			!enableTermsCheckbox &&
			subStep === 'method' &&
			!hasAutoAdvanced
		) {
			hasAutoAdvanced = true;
			subStep = 'email';
		}
	});

	const checkEmailMutation = useCheckEmailMutation();

	// Create form
	const form = createForm(() => ({
		defaultValues: {
			email: '',
			password: '',
			confirmPassword: '',
			subscribed: false,
			terms_accepted: false
		},
		onSubmit: async ({ value }) => {
			registering = true;
			try {
				await onRegister(
					{
						email: value.email.trim(),
						password: value.password,
						terms_accepted: enableTermsCheckbox && value.terms_accepted,
						website: honeypotValue || undefined
					},
					value.subscribed
				);
			} finally {
				registering = false;
			}
		}
	}));

	// Reset form and sub-step when modal opens
	function handleOpen() {
		form.reset({
			email: '',
			password: '',
			confirmPassword: '',
			subscribed: false,
			terms_accepted: false
		});
		subStep = 'method';
		emailValue = '';
		emailError = null;
	}

	async function handleContinue() {
		// Validate email field
		const currentEmail = form.state.values.email.trim();
		const emailValidationError = required(currentEmail) || emailValidator(currentEmail);
		if (emailValidationError) {
			// Trigger field validation to show error
			form.getFieldMeta('email');
			await form.validateField('email', 'blur');
			return;
		}

		// Check email availability
		checkingEmail = true;
		emailError = null;
		try {
			await checkEmailMutation.mutateAsync({ email: currentEmail });
			emailValue = currentEmail;
			subStep = 'password';
		} catch (err: unknown) {
			const error = err as Error & { code?: string };
			if (error.code === 'user_email_in_use') {
				emailError = 'email_in_use';
			} else {
				emailError = 'generic';
			}
		} finally {
			checkingEmail = false;
		}
	}

	function handleChangeEmail() {
		subStep = 'method';
		emailError = null;
	}

	function handleOidcRegister(providerSlug: string) {
		oidcLoadingSlug = providerSlug;
		const returnUrl = encodeURIComponent(window.location.origin);
		window.location.href = `/api/auth/oidc/${providerSlug}/authorize?flow=register&return_url=${returnUrl}&terms_accepted=${enableTermsCheckbox && form.state.values.terms_accepted}&marketing_opt_in=${form.state.values.subscribed}`;
	}

	async function handleSubmit() {
		await submitForm(form);
	}
</script>

<GenericModal
	{isOpen}
	title={auth_createYourAccount()}
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
			if (subStep === 'method') {
				// No-op — method step has no form submit; buttons handle navigation
			} else if (subStep === 'email') {
				handleContinue();
			} else {
				handleSubmit();
			}
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div style="position: absolute; left: -9999px; top: -9999px;" aria-hidden="true">
			<input
				type="text"
				name="website"
				tabindex="-1"
				autocomplete="off"
				bind:value={honeypotValue}
			/>
		</div>

		<div class="flex-1 overflow-auto p-4 sm:p-6">
			{#if orgName && invitedBy}
				<div class="mb-6">
					<InlineInfo
						title={auth_youreInvitedTitle()}
						body={auth_youreInvitedBody({ orgName, invitedBy })}
					/>
				</div>
			{/if}

			{#if subStep === 'method'}
				<!-- Sub-step: Choose method (OIDC-first) -->
				<div class="space-y-4">
					{#if disablePasswordLogin && !hasOidcProviders}
						<InlineDanger title={auth_passwordLoginDisabledNoProviders()} />
					{:else}
						{#if enableTermsCheckbox || enableEmailOptIn}
							<div class="flex flex-col gap-2">
								{#if enableTermsCheckbox}
									<form.Field name="terms_accepted">
										{#snippet children(field)}
											<Checkbox label={auth_termsAndPrivacy()} helpText="" {field} id="terms" />
										{/snippet}
									</form.Field>
								{/if}
								{#if enableEmailOptIn}
									<form.Field name="subscribed">
										{#snippet children(field)}
											<Checkbox
												{field}
												label={auth_signUpForUpdates()}
												id="subscribe"
												helpText=""
											/>
										{/snippet}
									</form.Field>
								{/if}
							</div>
						{/if}

						<form.Subscribe selector={(state) => state.values.terms_accepted}>
							{#snippet children(termsAccepted)}
								{#if hasOidcProviders}
									<div class="space-y-2">
										{#each oidcProviders as provider (provider.slug)}
											<button
												onclick={() => handleOidcRegister(provider.slug)}
												disabled={(enableTermsCheckbox && !termsAccepted) ||
													oidcLoadingSlug !== null}
												type="button"
												class="btn-primary flex w-full items-center justify-center gap-3"
											>
												{#if oidcLoadingSlug === provider.slug}
													<Loader2 class="h-5 w-5 animate-spin" />
												{:else if provider.logo}
													<img src={provider.logo} alt={provider.name} class="h-5 w-5" />
												{/if}
												{auth_createAccountWith({ provider: provider.name })}
											</button>
										{/each}
									</div>

									{#if !disablePasswordLogin}
										<div class="relative">
											<div class="absolute inset-0 flex items-center">
												<div
													class="w-full border-t"
													style="border-color: var(--color-border)"
												></div>
											</div>
											<div class="relative flex justify-center text-sm">
												<span class="text-tertiary bg-[var(--color-bg-elevated)] px-2"
													>{common_or()}</span
												>
											</div>
										</div>
									{/if}
								{/if}

								{#if !disablePasswordLogin}
									<button
										type="button"
										onclick={() => (subStep = 'email')}
										disabled={enableTermsCheckbox && !termsAccepted}
										class="btn-primary w-full"
									>
										{auth_continueWithEmail()}
									</button>
								{/if}
							{/snippet}
						</form.Subscribe>
					{/if}
				</div>
			{:else if subStep === 'email'}
				<!-- Sub-step: Email -->
				<div class="space-y-6">
					<form.Field
						name="email"
						validators={{
							onBlur: ({ value }) => required(value) || emailValidator(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label={common_email()}
								id="email"
								{field}
								placeholder={auth_enterYourEmail()}
								required
							/>
						{/snippet}
					</form.Field>

					{#if emailError === 'email_in_use'}
						<InlineDanger title={auth_emailAlreadyInUse()} />
						{#if onSwitchToLogin}
							<button
								type="button"
								onclick={onSwitchToLogin}
								class="text-link text-sm hover:underline"
							>
								{auth_signInInstead()}
							</button>
						{/if}
					{:else if emailError === 'generic'}
						<InlineDanger title="Something went wrong. Please try again." />
					{/if}
				</div>
			{:else}
				<!-- Sub-step: Password -->
				<div class="space-y-6">
					<div class="card card-static flex items-center justify-between !rounded-lg !px-4 !py-3">
						<span class="text-secondary text-sm">{emailValue}</span>
						<button
							type="button"
							onclick={handleChangeEmail}
							class="text-link text-sm hover:underline"
						>
							{common_change()}
						</button>
					</div>

					<form.Field
						name="password"
						validators={{
							onBlur: ({ value }) => required(value) || passwordValidator(value)
						}}
					>
						{#snippet children(passwordField)}
							<form.Field
								name="confirmPassword"
								validators={{
									onBlur: ({ value, fieldApi }) =>
										required(value) ||
										confirmPasswordMatch(() => fieldApi.form.getFieldValue('password'))(value)
								}}
							>
								{#snippet children(confirmPasswordField)}
									<Password {passwordField} {confirmPasswordField} required={true} />
								{/snippet}
							</form.Field>
						{/snippet}
					</form.Field>
				</div>
			{/if}
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex w-full flex-col gap-4">
				{#if subStep === 'email'}
					<div class="flex gap-3">
						<button
							type="button"
							onclick={() => {
								subStep = 'method';
								emailError = null;
							}}
							class="btn-secondary"
						>
							{common_back()}
						</button>
						<button type="submit" disabled={checkingEmail} class="btn-primary flex-1">
							{checkingEmail ? 'Checking...' : common_continue()}
						</button>
					</div>
				{:else if subStep === 'password'}
					<button type="submit" disabled={registering} class="btn-primary w-full">
						{registering ? auth_creatingAccount() : auth_createAccount()}
					</button>
				{/if}
			</div>
		</div>
	</form>
</GenericModal>
