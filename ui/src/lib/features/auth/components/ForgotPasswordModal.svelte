<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '#lib/shared/components/forms/form-context.ts';
	import { required, email } from '#lib/shared/components/forms/validators.ts';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import InlineInfo from '#lib/shared/components/feedback/InlineInfo.svelte';
	import { Mail } from '@lucide/svelte';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import ModalHeaderIcon from '#lib/shared/components/layout/ModalHeaderIcon.svelte';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import {
		auth_backToLogin,
		auth_checkYourEmail,
		auth_enterYourEmail,
		auth_resetLinkSentBody,
		auth_resetLinkSentTitle,
		auth_resetPassword,
		auth_resetPasswordInstructions,
		auth_sendResetLink,
		common_email,
		common_sending
	} from '#lib/paraglide/messages.js';

	interface Props {
		isOpen?: boolean;
		onRequestReset: (email: string) => Promise<void> | void;
		onClose: () => void;
		onBackToLogin: () => void;
	}

	let { isOpen = false, onRequestReset, onClose, onBackToLogin }: Props = $props();

	let requesting = $state(false);
	let emailSent = $state(false);

	// Create form
	const form = createForm(() => ({
		defaultValues: { email: '' },
		onSubmit: async ({ value }) => {
			requesting = true;
			try {
				await onRequestReset(value.email.trim());
				emailSent = true;
			} finally {
				requesting = false;
			}
		}
	}));

	// Reset form when modal opens
	function handleOpen() {
		form.reset({ email: '' });
		emailSent = false;
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	let actions = $derived<ModalAction[]>(
		emailSent
			? [{ role: 'primary', label: auth_backToLogin(), onClick: onBackToLogin }]
			: [
					{
						role: 'primary',
						label: requesting ? common_sending() : auth_sendResetLink(),
						onClick: handleSubmit,
						disabled: requesting
					}
				]
	);
</script>

<GenericModal
	{isOpen}
	title={emailSent ? auth_checkYourEmail() : auth_resetPassword()}
	size="md"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={false}
	showBackdrop={false}
	preventCloseOnClickOutside={true}
	centerTitle={true}
	{actions}
	footerLayout="stack"
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={Mail} color="Blue" />
	{/snippet}

	{#snippet footerNote()}
		{#if !emailSent}
			<div class="text-center">
				<button
					type="button"
					onclick={onBackToLogin}
					class="text-sm font-medium text-blue-400 hover:text-blue-300"
				>
					{auth_backToLogin()}
				</button>
			</div>
		{/if}
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="flex-1 overflow-auto p-6">
			{#if emailSent}
				<InlineInfo title={auth_resetLinkSentTitle()} body={auth_resetLinkSentBody()} />
			{:else}
				<div class="space-y-6">
					<p class="text-sm text-gray-400">
						{auth_resetPasswordInstructions()}
					</p>

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
				</div>
			{/if}
		</div>
	</form>
</GenericModal>
