<script lang="ts">
	import { useCurrentUserQuery, useLogoutMutation } from '$lib/features/auth/queries';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { queryKeys } from '$lib/api/query-client';
	import { apiClient } from '$lib/api/client';
	import type { User } from '$lib/features/users/types';
	import { pushError, pushSuccess } from '$lib/shared/stores/feedback';
	import { Link, Key, LogOut, Shield } from 'lucide-svelte';
	import CookieSettingsPanel from '$lib/shared/components/feedback/CookieSettingsPanel.svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import {
		required,
		email as emailValidator,
		password as passwordValidator,
		confirmPasswordMatch
	} from '$lib/shared/components/forms/validators';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import Password from '$lib/shared/components/forms/input/Password.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import {
		common_back,
		common_close,
		common_currentPassword,
		common_email,
		common_emailAndPassword,
		common_link,
		common_logout,
		common_organization,
		common_permissions,
		common_redirecting,
		common_saveChanges,
		common_saving,
		common_unlink,
		common_update,
		common_userId,
		settings_account_authMethods,
		settings_account_credentialsUpdated,
		settings_account_currentPasswordPlaceholder,
		settings_account_emailChangeIntro,
		settings_account_enterEmail,
		settings_account_linkedOn,
		settings_account_loadingUser,
		settings_account_noChanges,
		settings_account_notLinked,
		settings_account_oidcLinkHelp,
		settings_account_oidcUnlinked,
		settings_account_setPassword,
		settings_account_signOut,
		settings_account_unlinkFirst,
		settings_account_updateCredentials,
		settings_account_updateEmailPassword,
		settings_account_userInfo,
		settings_account_verificationSentTo,
		settings_account_cookiePreferencesDesc,
		cookies_preferences,
		common_manage
	} from '$lib/paraglide/messages';

	let {
		subView = $bindable<'main' | 'credentials' | 'email-change' | 'cookies'>('main'),
		onClose,
		dismissible = true
	}: {
		subView?: 'main' | 'credentials' | 'email-change' | 'cookies';
		onClose: () => void;
		/** When false (hard-gated modal), the main-view Close button is hidden so
		 * the gate can't be escaped from this tab. Sub-view Back/Save stay. */
		dismissible?: boolean;
	} = $props();

	// TanStack Query for current user and organization
	const currentUserQuery = useCurrentUserQuery();
	const logoutMutation = useLogoutMutation();
	const queryClient = useQueryClient();
	const organizationQuery = useOrganizationQuery();

	let user = $derived(currentUserQuery.data);
	let organization = $derived(organizationQuery.data);

	const configQuery = useConfigQuery();
	let configData = $derived(configQuery.data);

	let oidcProviders = $derived(configData?.oidc_providers ?? []);
	let hasOidcProviders = $derived(oidcProviders.length > 0);

	let linkingProviderSlug: string | null = $state(null);
	let savingCredentials = $state(false);
	let emailChangeLoading = $state(false);

	// Create form for password update
	const form = createForm(() => ({
		defaultValues: { currentPassword: '', password: '', confirmPassword: '' },
		onSubmit: async ({ value }) => {
			savingCredentials = true;
			try {
				if (!value.password) {
					pushError(settings_account_noChanges());
					return;
				}

				const body: { current_password?: string; new_password: string } = {
					new_password: value.password
				};

				if (value.currentPassword) {
					body.current_password = value.currentPassword;
				}

				const { data } = await apiClient.POST('/api/auth/update', {
					body
				});

				if (data?.success && data.data) {
					queryClient.setQueryData<User>(queryKeys.auth.currentUser(), data.data);
					pushSuccess(settings_account_credentialsUpdated());
					subView = 'main';
				}
			} finally {
				savingCredentials = false;
			}
		}
	}));

	// Create form for email change
	const emailChangeForm = createForm(() => ({
		defaultValues: { currentPassword: '', newEmail: '' },
		onSubmit: async ({ value }) => {
			if (!value.newEmail) return;
			emailChangeLoading = true;
			try {
				const { data } = await apiClient.POST('/api/auth/request-email-change', {
					body: { current_password: value.currentPassword, new_email: value.newEmail }
				});
				if (data?.success) {
					pushSuccess(settings_account_verificationSentTo({ email: value.newEmail }));
					emailChangeForm.reset({ currentPassword: '', newEmail: '' });
					subView = 'main';
				}
			} finally {
				emailChangeLoading = false;
			}
		}
	}));

	// Reset form when switching to credentials view
	export function resetForm() {
		linkingProviderSlug = null;
		form.reset({ currentPassword: '', password: '', confirmPassword: '' });
		emailChangeForm.reset({ currentPassword: '', newEmail: '' });
	}

	// Find which provider (if any) is linked to this user
	let linkedProvider = $derived(
		user?.oidc_provider ? oidcProviders.find((p) => p.slug === user.oidc_provider) : null
	);

	function linkOidcAccount(providerSlug: string) {
		linkingProviderSlug = providerSlug;
		const returnUrl = encodeURIComponent(window.location.origin);
		window.location.href = `/api/auth/oidc/${providerSlug}/authorize?flow=link&return_url=${returnUrl}`;
	}

	async function unlinkOidcAccount(providerSlug: string) {
		const { data } = await apiClient.POST('/api/auth/oidc/{slug}/unlink', {
			params: { path: { slug: providerSlug } }
		});

		if (data?.success && data.data) {
			queryClient.setQueryData<User>(queryKeys.auth.currentUser(), data.data);
			pushSuccess(settings_account_oidcUnlinked());
		}
	}

	async function handleSubmit() {
		if (subView === 'email-change') {
			await submitForm(emailChangeForm);
		} else {
			await submitForm(form);
		}
	}

	function handleCancel() {
		if (subView === 'credentials' || subView === 'email-change' || subView === 'cookies') {
			subView = 'main';
			form.reset({ currentPassword: '', password: '', confirmPassword: '' });
			emailChangeForm.reset({ currentPassword: '', newEmail: '' });
		} else {
			onClose();
		}
	}

	async function handleLogout() {
		try {
			await logoutMutation.mutateAsync();
			window.location.reload();
			onClose();
		} catch {
			// Error handled by mutation
		}
	}

	let hasLinkedOidc = $derived(!!user?.oidc_provider);
	let showSave = $derived(subView === 'credentials' || subView === 'email-change');
	let cancelLabel = $derived(subView === 'main' ? common_close() : common_back());
</script>

<form
	onsubmit={(e) => {
		e.preventDefault();
		e.stopPropagation();
		if (subView === 'email-change' || subView === 'credentials') handleSubmit();
	}}
	class="flex min-h-0 flex-1 flex-col"
>
	<div class="flex-1 overflow-auto p-6">
		{#if subView === 'main'}
			{#if user}
				<div class="space-y-6">
					<!-- User Info -->
					<InfoCard title={settings_account_userInfo()}>
						<InfoRow label={common_organization()}>{organization?.name}</InfoRow>
						<InfoRow label={common_email()}>
							<div class="flex items-center gap-2">
								<span>{user.email}</span>
								{#if !hasLinkedOidc}
									<button
										type="button"
										onclick={() => {
											emailChangeForm.reset({ currentPassword: '', newEmail: '' });
											subView = 'email-change';
										}}
										class="text-xs text-blue-500 hover:text-blue-700"
									>
										Change
									</button>
								{/if}
							</div>
						</InfoRow>
						<InfoRow label={common_permissions()} mono={true}>{user.permissions}</InfoRow>
						<InfoRow label={common_userId()} mono={true}>{user.id}</InfoRow>
					</InfoCard>

					<!-- Authentication Methods -->
					<div>
						<h3 class="text-primary mb-3 text-sm font-semibold">
							{settings_account_authMethods()}
						</h3>
						<div class="space-y-3">
							<!-- Email & Password -->
							<InfoCard variant="compact">
								<div class="flex items-center justify-between">
									<div class="flex items-center gap-4">
										<Key class="text-secondary h-5 w-5 flex-shrink-0" />
										<div>
											<p class="text-primary text-sm font-medium">
												{common_emailAndPassword()}
											</p>
											<p class="text-secondary text-xs">
												{settings_account_updateEmailPassword()}
											</p>
										</div>
									</div>
									<button
										type="button"
										onclick={() => {
											subView = 'credentials';
											form.reset({ currentPassword: '', password: '', confirmPassword: '' });
										}}
										class="btn-primary"
									>
										{common_update()}
									</button>
								</div>
							</InfoCard>

							<!-- OIDC Providers -->
							{#if hasOidcProviders}
								<div class="space-y-3">
									<p class="text-secondary text-xs">
										{settings_account_oidcLinkHelp()}
									</p>

									{#each oidcProviders as provider (provider.slug)}
										{@const isLinked = hasLinkedOidc && user.oidc_provider === provider.slug}
										{@const isDisabled = hasLinkedOidc && !isLinked}
										<InfoCard variant="compact">
											<div class="flex items-center justify-between">
												<div class="mr-2 flex items-center gap-4">
													{#if provider.logo}
														<img src={provider.logo} alt={provider.name} class="h-5 w-5" />
													{:else}
														<Link class="text-secondary h-5 w-5 flex-shrink-0" />
													{/if}
													<div>
														<p class="text-primary text-sm font-medium">{provider.name}</p>
														{#if isLinked}
															<p class="text-secondary text-xs">
																{settings_account_linkedOn({
																	date: new Date(user.oidc_linked_at || '').toLocaleDateString()
																})}
															</p>
														{:else if isDisabled}
															<p class="text-secondary text-xs">
																{settings_account_unlinkFirst({
																	provider: linkedProvider?.name || ''
																})}
															</p>
														{:else}
															<p class="text-secondary text-xs">{settings_account_notLinked()}</p>
														{/if}
													</div>
												</div>
												{#if isLinked}
													<button
														type="button"
														onclick={() => unlinkOidcAccount(provider.slug)}
														class="btn-danger"
													>
														{common_unlink()}
													</button>
												{:else if !hasLinkedOidc}
													<button
														type="button"
														onclick={() => linkOidcAccount(provider.slug)}
														disabled={(linkingProviderSlug &&
															linkingProviderSlug != provider.slug) ||
															isDisabled}
														class={isDisabled ? 'btn-disabled' : 'btn-primary'}
													>
														{linkingProviderSlug == provider.slug
															? common_redirecting()
															: common_link()}
													</button>
												{:else}
													<button type="button" disabled={isDisabled} class="btn-primary">
														{common_link()}
													</button>
												{/if}
											</div>
										</InfoCard>
									{/each}
								</div>
							{/if}
						</div>
					</div>

					<!-- Cookie Preferences -->
					{#if configData?.needs_cookie_consent}
						<InfoCard variant="compact">
							<div class="flex items-center justify-between">
								<div class="flex items-center gap-4">
									<Shield class="text-secondary h-5 w-5 flex-shrink-0" />
									<div>
										<p class="text-primary text-sm font-medium">
											{cookies_preferences()}
										</p>
										<p class="text-secondary text-xs">
											{settings_account_cookiePreferencesDesc()}
										</p>
									</div>
								</div>
								<button type="button" onclick={() => (subView = 'cookies')} class="btn-primary">
									{common_manage()}
								</button>
							</div>
						</InfoCard>
					{/if}

					<!-- Logout -->
					<InfoCard variant="compact">
						<div class="flex items-center justify-between">
							<div class="flex items-center gap-4">
								<LogOut class="text-secondary h-5 w-5" />
								<span class="text-primary text-sm">{settings_account_signOut()}</span>
							</div>
							<button type="button" onclick={handleLogout} class="btn-secondary">
								{common_logout()}
							</button>
						</div>
					</InfoCard>
				</div>
			{:else}
				<div class="text-secondary py-8 text-center">{settings_account_loadingUser()}</div>
			{/if}
		{:else if subView === 'credentials'}
			<div class="space-y-2">
				<p class="text-secondary mb-2 text-sm">
					{user?.has_password
						? settings_account_updateCredentials()
						: settings_account_setPassword()}
				</p>
				<div class="space-y-6">
					{#if user?.has_password}
						<form.Field
							name="currentPassword"
							validators={{
								onBlur: ({ value }) => required(value)
							}}
						>
							{#snippet children(field)}
								<TextInput
									label={common_currentPassword()}
									id="currentPassword"
									type="password"
									{field}
									placeholder={settings_account_currentPasswordPlaceholder()}
								/>
							{/snippet}
						</form.Field>
					{/if}

					<form.Field
						name="password"
						validators={{
							onBlur: ({ value }) => passwordValidator(value)
						}}
					>
						{#snippet children(passwordField)}
							<form.Field
								name="confirmPassword"
								validators={{
									onBlur: ({ value, fieldApi }) =>
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
			</div>
		{:else if subView === 'email-change'}
			<div class="space-y-2">
				<p class="text-secondary mb-2 text-sm">
					{settings_account_emailChangeIntro()}
				</p>
				<div class="space-y-6">
					<emailChangeForm.Field
						name="currentPassword"
						validators={{
							onBlur: ({ value }) => required(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label={common_currentPassword()}
								id="emailChangeCurrentPassword"
								type="password"
								{field}
								placeholder={settings_account_currentPasswordPlaceholder()}
							/>
						{/snippet}
					</emailChangeForm.Field>

					<emailChangeForm.Field
						name="newEmail"
						validators={{
							onBlur: ({ value }) => required(value) || emailValidator(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label={common_email()}
								id="newEmail"
								{field}
								placeholder={settings_account_enterEmail()}
								required
							/>
						{/snippet}
					</emailChangeForm.Field>
				</div>
			</div>
		{:else if subView === 'cookies'}
			<CookieSettingsPanel onSave={() => (subView = 'main')} />
		{/if}
	</div>

	<!-- Footer — hidden on the main view when the modal is hard-gated
	     (dismissible=false), since the only button there is Close, which would
	     let the user escape the gate. Sub-views keep their Back/Save footer. -->
	{#if dismissible || subView !== 'main'}
		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				<button type="button" onclick={handleCancel} class="btn-secondary">
					{cancelLabel}
				</button>
				{#if showSave}
					{@const isSaving = subView === 'email-change' ? emailChangeLoading : savingCredentials}
					<button type="submit" disabled={isSaving} class="btn-primary">
						{isSaving ? common_saving() : common_saveChanges()}
					</button>
				{/if}
			</div>
		</div>
	{/if}
</form>
