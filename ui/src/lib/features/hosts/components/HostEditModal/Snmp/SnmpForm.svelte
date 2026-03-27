<script lang="ts">
	import type { HostFormData, Interface } from '$lib/features/hosts/types/base';
	import type { Network } from '$lib/features/networks/types';
	import type { Credential } from '$lib/features/credentials/types/base';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import ConfigHeader from '$lib/shared/components/forms/config/ConfigHeader.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import { entities } from '$lib/shared/stores/metadata';
	import { CredentialDisplay } from '$lib/shared/components/forms/selection/display/CredentialDisplay.svelte';
	import {
		InterfaceDisplay,
		type InterfaceDisplayContext
	} from '$lib/shared/components/forms/selection/display/InterfaceDisplay.svelte';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import { getCredentialTypeId } from '$lib/features/credentials/types/base';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import {
		common_credentialDemoReadOnly,
		credentials_addInterfaces,
		credentials_noCredentialSelected,
		credentials_selectCredentialSubtitle,
		hosts_credentialOverrideHelp,
		hosts_credentialOverrideHelpLinkText,
		hosts_credentialScopeSubtitle,
		daemons_credentialWizardNetworkCredentials
	} from '$lib/paraglide/messages';

	interface Props {
		formData: HostFormData;
		network?: Network | null;
	}

	let { formData = $bindable(), network = null }: Props = $props();

	// TanStack Query for organization and current user (for demo mode check)
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	// Demo mode check: only Owner can modify credential settings in demo orgs
	let isDemoOrg = $derived(organization?.plan?.type === 'Demo');
	let isNonOwnerInDemo = $derived(isDemoOrg && currentUser?.permissions !== 'Owner');

	// TanStack Query for credentials and subnets
	const credentialsQuery = useCredentialsQuery();
	let allCredentials = $derived(credentialsQuery.data ?? []);

	const subnetsQuery = useSubnetsQuery();
	let subnets = $derived(subnetsQuery.data ?? []);

	// Resolve credential assignments to full credential objects for list display
	let selectedCredentials = $derived(
		(formData.credential_assignments ?? [])
			.map((a) => allCredentials.find((c) => c.id === a.credential_id))
			.filter((c): c is Credential => c != null)
	);

	// Filter to PerHost-scoped credentials, then exclude already-assigned ones
	let perHostCredentials = $derived(
		allCredentials.filter((c) => {
			const meta = credentialTypes.getMetadata(getCredentialTypeId(c));
			return (meta?.scope_models ?? []).includes('PerHost');
		})
	);

	let availableCredentials = $derived(
		perHostCredentials.filter(
			(c) => !(formData.credential_assignments ?? []).some((a) => a.credential_id === c.id)
		)
	);

	// Resolve network default credentials to full objects for EntityTag display
	let networkDefaultCredentials = $derived(
		(network?.credential_ids ?? [])
			.map((id: string) => allCredentials.find((c) => c.id === id))
			.filter((c): c is Credential => c != null)
	);

	let credentialColorHelper = $derived(entities.getColorHelper('Credential'));
	let credentialIcon = $derived(entities.getIconComponent('Credential'));

	function getAssignmentForIndex(index: number) {
		return (formData.credential_assignments ?? [])[index] ?? null;
	}

	// Resolve interface_ids for a credential assignment into Interface objects
	function getScopedInterfaces(index: number): Interface[] {
		const assignment = getAssignmentForIndex(index);
		if (!assignment || assignment.interface_ids === null) return [];
		return assignment.interface_ids
			.map((id) => formData.interfaces.find((i) => i.id === id))
			.filter((i): i is Interface => i != null);
	}

	function getInterfaceContext(): InterfaceDisplayContext {
		return { subnets };
	}

	function addInterfaceToScope(credentialIndex: number, interfaceId: string) {
		const assignments = [...(formData.credential_assignments ?? [])];
		if (!assignments[credentialIndex]) return;
		const current = assignments[credentialIndex].interface_ids;
		if (current === null) {
			// First add: switch from "all" to explicit list with just this interface
			assignments[credentialIndex] = {
				...assignments[credentialIndex],
				interface_ids: [interfaceId]
			};
		} else if (!current.includes(interfaceId)) {
			assignments[credentialIndex] = {
				...assignments[credentialIndex],
				interface_ids: [...current, interfaceId]
			};
		}
		formData.credential_assignments = assignments;
	}

	function removeInterfaceFromScope(credentialIndex: number, interfaceIndex: number) {
		const assignments = [...(formData.credential_assignments ?? [])];
		if (!assignments[credentialIndex]) return;
		const current = assignments[credentialIndex].interface_ids;
		if (current === null) return;
		const updated = current.filter((_, i) => i !== interfaceIndex);
		assignments[credentialIndex] = {
			...assignments[credentialIndex],
			// Revert to null (all interfaces) when list empties
			interface_ids: updated.length === 0 ? null : updated
		};
		formData.credential_assignments = assignments;
	}
</script>

{#snippet hostCredentialHelpSnippet()}
	<DocsHint
		text={hosts_credentialOverrideHelp()}
		href="https://scanopy.net/docs/using-scanopy/credentials/#credential-resolution"
		linkText={hosts_credentialOverrideHelpLinkText()}
	/>
	{#if networkDefaultCredentials.length > 0}
		<p class="text-tertiary mt-1 flex flex-wrap items-center gap-1 text-xs">
			<span>{daemons_credentialWizardNetworkCredentials()}</span>
			{#each networkDefaultCredentials as cred (cred.id)}
				<EntityTag
					entityRef={entityRef('Credential', cred.id, cred)}
					label={cred.name}
					icon={credentialIcon}
					color={credentialColorHelper.color}
				/>
			{/each}
		</p>
	{/if}
{/snippet}

<div class="flex min-h-0 flex-1 flex-col">
	<ListConfigEditor items={selectedCredentials}>
		<svelte:fragment slot="list" let:items let:onEdit let:highlightedIndex>
			<div class="space-y-4">
				<ListManager
					label="Credential Override"
					helpSnippet={isNonOwnerInDemo ? undefined : hostCredentialHelpSnippet}
					helpText={isNonOwnerInDemo ? common_credentialDemoReadOnly() : undefined}
					placeholder="Select a credential to add"
					emptyMessage="No credential overrides — using network defaults"
					allowReorder={false}
					options={availableCredentials}
					{items}
					itemClickAction="edit"
					optionDisplayComponent={CredentialDisplay}
					itemDisplayComponent={CredentialDisplay}
					{onEdit}
					{highlightedIndex}
					onAdd={(id) => {
						const current = formData.credential_assignments ?? [];
						if (!current.some((a) => a.credential_id === id)) {
							formData.credential_assignments = [
								...current,
								{ credential_id: id, interface_ids: null }
							];
						}
					}}
					onRemove={(index) => {
						const current = formData.credential_assignments ?? [];
						formData.credential_assignments = current.filter((_, i) => i !== index);
					}}
				/>
			</div>
		</svelte:fragment>

		<svelte:fragment slot="config" let:selectedItem let:selectedIndex>
			{#if selectedItem && formData.interfaces.length > 0}
				<div class="space-y-4">
					<ConfigHeader title={selectedItem.name} subtitle={hosts_credentialScopeSubtitle()} />
					<ListManager
						label="Interface Scope"
						emptyMessage="All interfaces (default)"
						placeholder="Select an interface to restrict scope"
						allowReorder={false}
						options={formData.interfaces}
						items={getScopedInterfaces(selectedIndex)}
						optionDisplayComponent={InterfaceDisplay}
						itemDisplayComponent={InterfaceDisplay}
						getOptionContext={() => getInterfaceContext()}
						getItemContext={() => getInterfaceContext()}
						onAdd={(id) => addInterfaceToScope(selectedIndex, id)}
						onRemove={(index) => removeInterfaceFromScope(selectedIndex, index)}
					/>
				</div>
			{:else if selectedItem}
				<EntityConfigEmpty title={selectedItem.name} subtitle={credentials_addInterfaces()} />
			{:else}
				<EntityConfigEmpty
					title={credentials_noCredentialSelected()}
					subtitle={credentials_selectCredentialSubtitle()}
				/>
			{/if}
		</svelte:fragment>
	</ListConfigEditor>
</div>
