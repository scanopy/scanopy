<script lang="ts">
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import { createForm } from '@tanstack/svelte-form';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import {
		common_confirmAction,
		common_areYouSure,
		common_confirm,
		common_cancel
	} from '#lib/paraglide/messages.js';
	import InlineWarning from './InlineWarning.svelte';
	import InlineDanger from './InlineDanger.svelte';
	import InlineInfo from './InlineInfo.svelte';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';

	interface Props {
		isOpen?: boolean;
		title?: string;
		message?: string;
		details?: string[];
		confirmLabel?: string;
		cancelLabel?: string;
		variant?: 'danger' | 'warning' | 'info';
		/** Text user must type to enable confirm button. If unset, confirm is always enabled. */
		confirmText?: string;
		/** Label for the type-to-confirm input. */
		confirmPlaceholder?: string;
		onConfirm: () => void;
		onCancel: () => void;
		/** Called when modal is dismissed via X or backdrop click. Required - should close the modal without side effects. */
		onClose: () => void;
	}

	let {
		isOpen = false,
		title,
		message,
		details = [],
		confirmLabel,
		cancelLabel,
		variant = 'warning',
		confirmText,
		confirmPlaceholder,
		onConfirm,
		onCancel,
		onClose
	}: Props = $props();

	const form = createForm(() => ({
		defaultValues: { typed: '' }
	}));

	// Clear the typed value whenever the dialog closes
	$effect(() => {
		if (!isOpen) form.reset();
	});

	let resolvedTitle = $derived(title ?? common_confirmAction());
	let resolvedMessage = $derived(message ?? common_areYouSure());
	let resolvedConfirmLabel = $derived(confirmLabel ?? common_confirm());
	let resolvedCancelLabel = $derived(cancelLabel ?? common_cancel());
	let detailsBody = $derived(details.length > 0 ? details.join(', ') : null);

	// The typed text, mirrored from the form: $derived can't track TanStack form state.
	let typed = $state('');
	$effect(() => {
		const { unsubscribe } = form.store.subscribe(() => {
			typed = form.state.values.typed;
		});
		return unsubscribe;
	});

	let actions = $derived<ModalAction[]>([
		{ role: 'cancel', label: resolvedCancelLabel, onClick: onCancel },
		{
			role: 'primary',
			label: resolvedConfirmLabel,
			tone: variant === 'danger' ? 'danger' : undefined,
			onClick: onConfirm,
			disabled: confirmText != null && typed !== confirmText
		}
	]);
</script>

<GenericModal {isOpen} title={resolvedTitle} {onClose} size="sm" {actions}>
	<div class="space-y-4 p-6">
		{#if variant === 'danger'}
			<InlineDanger title={resolvedMessage} body={detailsBody} />
		{:else if variant === 'info'}
			<InlineInfo title={resolvedMessage} body={detailsBody} />
		{:else}
			<InlineWarning title={resolvedMessage} body={detailsBody} />
		{/if}

		{#if confirmText != null}
			<form.Field name="typed">
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label={confirmPlaceholder ?? ''}
						id="confirm-text-input"
						placeholder={confirmText}
						{field}
					/>
				{/snippet}
			</form.Field>
		{/if}
	</div>
</GenericModal>
