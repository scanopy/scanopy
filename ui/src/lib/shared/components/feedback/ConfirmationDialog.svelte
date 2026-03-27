<script lang="ts">
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import {
		common_confirmAction,
		common_areYouSure,
		common_confirm,
		common_cancel
	} from '$lib/paraglide/messages';
	import InlineWarning from './InlineWarning.svelte';
	import InlineDanger from './InlineDanger.svelte';
	import InlineInfo from './InlineInfo.svelte';

	export let isOpen: boolean = false;
	export let title: string | undefined = undefined;
	export let message: string | undefined = undefined;
	export let details: string[] = [];
	export let confirmLabel: string | undefined = undefined;
	export let cancelLabel: string | undefined = undefined;
	export let variant: 'danger' | 'warning' | 'info' = 'warning';
	export let onConfirm: () => void;
	export let onCancel: () => void;
	/** Called when modal is dismissed via X or backdrop click. Required - should close the modal without side effects. */
	export let onClose: () => void;

	$: resolvedTitle = title ?? common_confirmAction();
	$: resolvedMessage = message ?? common_areYouSure();
	$: resolvedConfirmLabel = confirmLabel ?? common_confirm();
	$: resolvedCancelLabel = cancelLabel ?? common_cancel();

	$: detailsBody = details.length > 0 ? details.join(', ') : null;

	const confirmButtonClasses = {
		danger: 'btn-danger',
		warning: 'btn-primary',
		info: 'btn-primary'
	};
</script>

<GenericModal {isOpen} title={resolvedTitle} {onClose} size="sm">
	<div class="space-y-4 p-6">
		{#if variant === 'danger'}
			<InlineDanger title={resolvedMessage} body={detailsBody} />
		{:else if variant === 'info'}
			<InlineInfo title={resolvedMessage} body={detailsBody} />
		{:else}
			<InlineWarning title={resolvedMessage} body={detailsBody} />
		{/if}
	</div>

	{#snippet footer()}
		<div class="modal-footer">
			<div class="flex justify-end gap-3">
				<button type="button" class="btn-secondary" on:click={onCancel}>
					{resolvedCancelLabel}
				</button>
				<button type="button" class={confirmButtonClasses[variant]} on:click={onConfirm}>
					{resolvedConfirmLabel}
				</button>
			</div>
		</div>
	{/snippet}
</GenericModal>
