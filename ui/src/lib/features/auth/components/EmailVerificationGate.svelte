<script lang="ts">
	import { LogOut, Mail } from '@lucide/svelte';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '#lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EmailChangeForm from './EmailChangeForm.svelte';
	import {
		useCurrentUserQuery,
		useLogoutMutation,
		useResendVerificationMutation
	} from '#lib/features/auth/queries.ts';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import {
		auth_verifyEmailGateBody,
		common_back,
		common_logout,
		common_resend,
		common_saveChanges,
		common_saving,
		common_sending,
		settings_account_updateEmail,
		verifyEmail_sentLinkTo,
		verifyEmail_title
	} from '#lib/paraglide/messages.js';

	// No onClose: the server refuses this user's requests until they verify, so
	// the ways out are verifying (the current-user query refetches on focus while
	// unverified, and the app shell mounts once the flag clears) and logging out.
	let { isOpen }: { isOpen: boolean } = $props();

	const currentUserQuery = useCurrentUserQuery();
	const resendMutation = useResendVerificationMutation();
	const logoutMutation = useLogoutMutation();

	let changingEmail = $state(false);
	let savingEmail = $state(false);
	let emailChangeForm: EmailChangeForm | undefined = $state();
	// The address a link went to after a change in this session. The server
	// keeps the pending address to itself, and Resend by the current address
	// mails it there.
	let pendingEmail = $state<string | null>(null);
	let currentEmail = $derived(currentUserQuery.data?.email ?? null);
	let shownEmail = $derived(pendingEmail ?? currentEmail);
	let canChangeEmail = $derived(currentUserQuery.data?.oidc_provider == null);

	function handleResend() {
		if (currentEmail) resendMutation.mutate({ email: currentEmail });
	}

	let busy = $derived(logoutMutation.isPending || resendMutation.isPending || savingEmail);

	let actions = $derived<ModalAction[]>(
		changingEmail
			? [
					{
						role: 'secondary',
						label: common_back(),
						onClick: () => (changingEmail = false),
						disabled: savingEmail
					},
					{
						role: 'primary',
						label: savingEmail ? common_saving() : common_saveChanges(),
						onClick: () => emailChangeForm?.submit(),
						disabled: savingEmail
					}
				]
			: [
					{
						role: 'secondary',
						label: common_logout(),
						icon: LogOut,
						align: 'start',
						// A full load, as the Account tab does: AppShell stays mounted across a
						// client-side navigation and would keep the old session's queries.
						onClick: () =>
							logoutMutation.mutateAsync().then(
								() => window.location.reload(),
								() => {}
							),
						disabled: busy
					},
					// A linked identity provider owns the address, and the server refuses to change it.
					...(canChangeEmail
						? [
								{
									role: 'secondary' as const,
									label: settings_account_updateEmail(),
									onClick: () => (changingEmail = true),
									disabled: busy
								}
							]
						: []),
					{
						role: 'primary',
						label: resendMutation.isPending ? common_sending() : common_resend(),
						onClick: handleResend,
						disabled: busy || !currentEmail
					}
				]
	);
</script>

<GenericModal
	title={verifyEmail_title()}
	{isOpen}
	size="md"
	preventCloseOnClickOutside={true}
	showCloseButton={false}
	{actions}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={Mail} color="Blue" />
	{/snippet}
	<form
		class="px-6 py-4"
		onsubmit={(e) => {
			e.preventDefault();
			if (changingEmail) emailChangeForm?.submit();
		}}
	>
		{#if changingEmail}
			<EmailChangeForm
				bind:this={emailChangeForm}
				bind:saving={savingEmail}
				onChanged={(newEmail) => {
					pendingEmail = newEmail;
					changingEmail = false;
				}}
			/>
		{:else}
			<div class="space-y-3 text-sm text-secondary">
				{#if shownEmail}
					<p>{verifyEmail_sentLinkTo({ email: shownEmail })}</p>
				{/if}
				<p>{auth_verifyEmailGateBody()}</p>
			</div>
		{/if}
	</form>
</GenericModal>
