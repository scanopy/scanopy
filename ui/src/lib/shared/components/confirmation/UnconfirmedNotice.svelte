<!--
	The Unconfirmed notice for an inspector, with the action that opens the entity's resolve modal.
	Renders nothing for a confirmed entity. The label and reason are the shared verdict's
	(`confirmation.ts`), so a host and a subnet say it the same way.
-->
<script lang="ts">
	import { CircleDashedCheck } from '@lucide/svelte';
	import InlineInfo from '#lib/shared/components/feedback/InlineInfo.svelte';
	import {
		unconfirmedReason,
		unconfirmedVia,
		type ConfirmationSubject
	} from '#lib/shared/utils/confirmation.ts';
	import { common_unconfirmed } from '#lib/paraglide/messages.js';

	let {
		entity,
		resolveLabel,
		onResolve
	}: {
		entity: ConfirmationSubject;
		resolveLabel: string;
		/** Omitted where the viewer cannot change the entity (a share). */
		onResolve?: () => void;
	} = $props();

	let via = $derived(unconfirmedVia(entity));
</script>

{#if via}
	<div class="space-y-2">
		<InlineInfo title={common_unconfirmed()} body={unconfirmedReason(via)} />
		{#if onResolve}
			<button
				type="button"
				class="btn-secondary flex items-center gap-1 text-xs"
				onclick={onResolve}
			>
				<CircleDashedCheck class="h-3.5 w-3.5" />
				{resolveLabel}
			</button>
		{/if}
	</div>
{/if}
