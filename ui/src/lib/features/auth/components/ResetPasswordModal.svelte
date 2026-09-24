<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import {
		required,
		password as passwordValidator,
		confirmPasswordMatch
	} from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import { Key } from 'lucide-svelte';
	import Password from '$lib/shared/components/forms/input/Password.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import {
		auth_backToLogin,
		auth_goToLogin,
		auth_passwordResetComplete,
		auth_passwordUpdatedBody,
		auth_passwordUpdatedTitle,
		auth_resetPassword,
		auth_setNewPassword,
		common_resetting
	} from '$lib/paraglide/messages';

	interface Props {
		isOpen?: boolean;
		token: string;
		onResetPassword: (token: string, password: string) => Promise<void> | void;
		onClose: () => void;
		onBackToLogin: () => void;
	}

	let { isOpen = false, token, onResetPassword, onClose, onBackToLogin }: Props = $props();

	let resetting = $state(false);
	let resetComplete = $state(false);

	// Create form
	const form = createForm(() => ({
		defaultValues: { password: '', confirmPassword: '' },
		onSubmit: async ({ value }) => {
			resetting = true;
			try {
				await onResetPassword(token, value.password);
				resetComplete = true;
			} finally {
				resetting = false;
			}
		}
	}));

	// Reset form when modal opens
	function handleOpen() {
		form.reset({ password: '', confirmPassword: '' });
		resetComplete = false;
	}

	async function handleSubmit() {
		await submitForm(form);
	}
</script>

<GenericModal
	{isOpen}
	title={resetComplete ? auth_passwordResetComplete() : auth_setNewPassword()}
	size="md"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={false}
	showBackdrop={false}
	preventCloseOnClickOutside={true}
	centerTitle={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={Key} color="Blue" />
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
			{#if resetComplete}
				<InlineInfo title={auth_passwordUpdatedTitle()} body={auth_passwordUpdatedBody()} />
			{:else}
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
								<Password {passwordField} {confirmPasswordField} required={true} autofocus />
							{/snippet}
						</form.Field>
					{/snippet}
				</form.Field>
			{/if}
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex w-full flex-col gap-4">
				{#if resetComplete}
					<button type="button" onclick={onBackToLogin} class="btn-primary w-full">
						{auth_goToLogin()}
					</button>
				{:else}
					<button type="submit" disabled={resetting} class="btn-primary w-full">
						{resetting ? common_resetting() : auth_resetPassword()}
					</button>

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
			</div>
		</div>
	</form>
</GenericModal>
