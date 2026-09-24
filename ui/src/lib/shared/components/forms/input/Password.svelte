<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { AlertCircle } from 'lucide-svelte';
	import {
		common_password,
		common_passwordConfirm,
		common_passwordContainsLowercase,
		common_passwordContainsNumber,
		common_passwordContainsUppercase,
		common_passwordCreatePlaceholder,
		common_passwordMinChars,
		common_passwordReenterPlaceholder,
		common_passwordRequirements
	} from '$lib/paraglide/messages';

	interface Props {
		passwordField: AnyFieldApi;
		confirmPasswordField?: AnyFieldApi;
		label?: string;
		confirmLabel?: string;
		required?: boolean;
		autofocus?: boolean;
	}

	let {
		passwordField,
		confirmPasswordField,
		label,
		confirmLabel,
		required = true,
		autofocus = false
	}: Props = $props();

	function focusOnMount(node: HTMLInputElement) {
		if (autofocus) node.focus();
	}

	let passwordLabel = $derived(label ?? common_password());
	let confirmPasswordLabel = $derived(confirmLabel ?? common_passwordConfirm());

	// Password requirements derived from field value
	let value = $derived(passwordField.state.value as string);
	let hasUppercase = $derived(/[A-Z]/.test(value));
	let hasLowercase = $derived(/[a-z]/.test(value));
	let hasNumber = $derived(/[0-9]/.test(value));
	let passwordLongEnough = $derived(value.length >= 10);

	// Password field errors
	let passwordErrors = $derived(passwordField.state.meta.errors);
	let showPasswordErrors = $derived(
		passwordField.state.meta.isTouched && passwordErrors.length > 0
	);

	// Confirm field errors
	let confirmErrors = $derived(confirmPasswordField?.state.meta.errors ?? []);
	let showConfirmErrors = $derived(
		confirmPasswordField?.state.meta.isTouched && confirmErrors.length > 0
	);
</script>

<div class="space-y-4">
	<div class="space-y-2">
		<label for="password" class="text-secondary block text-sm font-medium">
			{passwordLabel}
			{#if required}<span class="text-red-400">*</span>{/if}
		</label>
		<input
			use:focusOnMount
			id="password"
			type="password"
			value={passwordField.state.value}
			onblur={() => passwordField.handleBlur()}
			oninput={(e) => passwordField.handleChange(e.currentTarget.value)}
			placeholder={common_passwordCreatePlaceholder()}
			class="input-field"
			class:input-field-error={showPasswordErrors}
		/>

		<!-- Password Requirements -->
		{#if value}
			<div class="space-y-1 rounded-md p-3" style="background: var(--color-bg-surface)">
				<p class="text-secondary text-xs font-medium">{common_passwordRequirements()}</p>
				<p class="text-xs {passwordLongEnough ? 'text-success' : 'text-tertiary'}">
					{passwordLongEnough ? '✓' : '○'}
					{common_passwordMinChars()}
				</p>
				<p class="text-xs {hasUppercase ? 'text-success' : 'text-tertiary'}">
					{hasUppercase ? '✓' : '○'}
					{common_passwordContainsUppercase()}
				</p>
				<p class="text-xs {hasLowercase ? 'text-success' : 'text-tertiary'}">
					{hasLowercase ? '✓' : '○'}
					{common_passwordContainsLowercase()}
				</p>
				<p class="text-xs {hasNumber ? 'text-success' : 'text-tertiary'}">
					{hasNumber ? '✓' : '○'}
					{common_passwordContainsNumber()}
				</p>
			</div>
		{/if}

		{#if showPasswordErrors}
			<div class="text-danger flex items-center gap-2">
				<AlertCircle size={16} />
				<p class="text-xs">{passwordErrors[0]}</p>
			</div>
		{/if}
	</div>

	{#if confirmPasswordField}
		<div class="space-y-2">
			<label for="confirmPassword" class="text-secondary block text-sm font-medium">
				{confirmPasswordLabel}
				{#if required}<span class="text-red-400">*</span>{/if}
			</label>
			<input
				id="confirmPassword"
				type="password"
				value={confirmPasswordField.state.value}
				onblur={() => confirmPasswordField.handleBlur()}
				oninput={(e) => confirmPasswordField.handleChange(e.currentTarget.value)}
				placeholder={common_passwordReenterPlaceholder()}
				class="input-field"
				class:input-field-error={showConfirmErrors}
			/>
			{#if showConfirmErrors}
				<div class="text-danger flex items-center gap-2">
					<AlertCircle size={16} />
					<p class="text-xs">{confirmErrors[0]}</p>
				</div>
			{/if}
		</div>
	{/if}
</div>
