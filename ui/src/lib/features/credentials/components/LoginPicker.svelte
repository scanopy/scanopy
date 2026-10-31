<script lang="ts">
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { CredentialDisplay } from '$lib/shared/components/forms/selection/display/CredentialDisplay.svelte';
	import { Plus } from '@lucide/svelte';
	import type { Credential } from '../types/base';
	import { isLoginType } from '../types/base';
	import { useCreateCredentialMutation, useCredentialsQuery } from '../queries';
	import CredentialEditModal from './CredentialEditModal.svelte';
	import {
		credentials_createSshLogin,
		credentials_sshLogin,
		credentials_sshLoginHelp,
		credentials_sshLoginPlaceholder
	} from '$lib/paraglide/messages';

	let {
		selectedId,
		onSelect,
		error = null
	}: {
		/** The chosen login's id, or null. Owned by the form, which keeps it in `$state`. */
		selectedId: string | null;
		onSelect: (id: string) => void;
		error?: string | null;
	} = $props();

	const credentialsQuery = useCredentialsQuery();
	const createMutation = useCreateCredentialMutation();
	let logins = $derived(
		(credentialsQuery.data ?? []).filter((c) => isLoginType(c.credential_type.type))
	);

	// A login made here, in a modal stacked on the credential's own. Unnamed, so the URL registry
	// leaves the credential's modal open beneath it.
	let creating = $state(false);

	async function handleCreate(login: Credential) {
		const created = await createMutation.mutateAsync(login);
		onSelect(created.id);
		creating = false;
	}
</script>

<div class="space-y-2">
	<RichSelect
		label={credentials_sshLogin()}
		helpText={credentials_sshLoginHelp()}
		placeholder={credentials_sshLoginPlaceholder()}
		required={true}
		selectedValue={selectedId}
		options={logins}
		displayComponent={CredentialDisplay}
		{error}
		{onSelect}
	/>
	<button type="button" class="btn-secondary" onclick={() => (creating = true)}>
		<Plus class="h-4 w-4" />
		{credentials_createSshLogin()}
	</button>
</div>

<CredentialEditModal
	isOpen={creating}
	loginOnly={true}
	onCreate={handleCreate}
	onUpdate={() => {}}
	onClose={() => (creating = false)}
/>
