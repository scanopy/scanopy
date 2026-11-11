<script lang="ts">
	// The email-change fields and request, shared by the Account tab and the
	// email verification step. The caller owns the surrounding <form> and its
	// buttons, and calls submit() and reset().
	import { useQueryClient } from '@tanstack/svelte-query';
	import { createForm } from '@tanstack/svelte-form';
	import { queryKeys } from '#lib/api/query-client.ts';
	import { apiClient } from '#lib/api/client.ts';
	import { pushSuccess } from '#lib/shared/stores/feedback.ts';
	import { submitForm } from '#lib/shared/components/forms/form-context.ts';
	import { required, email as emailValidator } from '#lib/shared/components/forms/validators.ts';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import { useConfigQuery } from '#lib/shared/stores/config-query.ts';
	import {
		common_currentPassword,
		common_email,
		settings_account_currentPasswordPlaceholder,
		settings_account_emailChangeIntro,
		settings_account_emailChangeIntroImmediate,
		settings_account_emailChanged,
		settings_account_enterEmail,
		settings_account_verificationSentTo
	} from '#lib/paraglide/messages.js';

	let {
		// eslint-disable-next-line no-useless-assignment -- bound by the parent, which reads it
		saving = $bindable(false),
		onChanged
	}: {
		saving?: boolean;
		/** Called once the request succeeds, with the address a link went to or that applied. */
		onChanged: (newEmail: string) => void;
	} = $props();

	const queryClient = useQueryClient();
	const configQuery = useConfigQuery();
	let hasEmailService = $derived(configQuery.data?.has_email_service ?? false);

	const form = createForm(() => ({
		defaultValues: { currentPassword: '', newEmail: '' },
		onSubmit: async ({ value }) => {
			const newEmail = value.newEmail.trim();
			if (!newEmail) return;
			saving = true;
			try {
				const { data } = await apiClient.POST('/api/auth/request-email-change', {
					body: { current_password: value.currentPassword, new_email: newEmail }
				});
				if (data?.success) {
					// A server with no email transport applies the change at once instead of
					// mailing a link to the new address.
					if (hasEmailService) {
						pushSuccess(settings_account_verificationSentTo({ email: newEmail }));
					} else {
						pushSuccess(settings_account_emailChanged({ email: newEmail }));
						await queryClient.invalidateQueries({ queryKey: queryKeys.auth.currentUser() });
					}
					reset();
					onChanged(newEmail);
				}
			} finally {
				saving = false;
			}
		}
	}));

	export function reset() {
		form.reset({ currentPassword: '', newEmail: '' });
	}

	export async function submit() {
		await submitForm(form);
	}
</script>

<div class="space-y-2">
	<p class="mb-2 text-sm text-secondary">
		{hasEmailService
			? settings_account_emailChangeIntro()
			: settings_account_emailChangeIntroImmediate()}
	</p>
	<div class="space-y-6">
		<form.Field
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
		</form.Field>

		<form.Field
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
		</form.Field>
	</div>
</div>
