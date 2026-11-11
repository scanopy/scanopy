<!--
	A host's actions in the topology inspector: edit it, and, while it is unconfirmed, the shared
	Unconfirmed notice with a way to resolve it. Both hand off to the Hosts tab, where the editor and
	the resolve modal live, with a way back to the map.
-->
<script lang="ts">
	import { Edit } from '@lucide/svelte';
	import UnconfirmedNotice from '#lib/shared/components/confirmation/UnconfirmedNotice.svelte';
	import { navigateToEntity, openModal } from '#lib/shared/stores/modal-registry.ts';
	import type { Host } from '../types/base';
	import { isUnconfirmed } from '#lib/shared/utils/confirmation.ts';
	import {
		common_edit,
		hosts_resolveFirst,
		hosts_resolveUnconfirmed
	} from '#lib/paraglide/messages.js';

	let {
		host,
		editable = true,
		class: className = ''
	}: {
		host: Pick<Host, 'id' | 'source'> & Record<string, unknown>;
		editable?: boolean;
		class?: string;
	} = $props();

	// Resolve settles an unconfirmed host before anything else is done with it.
	let unresolved = $derived(isUnconfirmed(host));

	function resolve() {
		window.location.hash = 'hosts';
		openModal('unconfirmed-host', { id: host.id, returnUrl: window.location.href });
	}
</script>

<div class="space-y-2 {className}">
	<UnconfirmedNotice
		entity={host}
		resolveLabel={hosts_resolveUnconfirmed()}
		onResolve={editable ? resolve : undefined}
	/>
	{#if editable}
		<button
			type="button"
			class="btn-secondary flex items-center gap-1 text-xs"
			disabled={unresolved}
			title={unresolved ? hosts_resolveFirst() : undefined}
			onclick={() => navigateToEntity('Host', host.id, host)}
		>
			<Edit class="h-3.5 w-3.5" />
			{common_edit()}
		</button>
	{/if}
</div>
