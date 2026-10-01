<script lang="ts">
	import GenericModal from './GenericModal.svelte';
	import SupportOptions from '$lib/features/support/SupportOptions.svelte';
	import { daemons_troubleshootingTitle } from '$lib/paraglide/messages';

	let {
		isOpen = false,
		onClose
	}: {
		isOpen?: boolean;
		onClose: () => void;
	} = $props();
</script>

<GenericModal title={daemons_troubleshootingTitle()} {isOpen} {onClose} size="xl">
	<div class="p-6">
		<SupportOptions isTroubleshooting={true} />
	</div>
</GenericModal>
