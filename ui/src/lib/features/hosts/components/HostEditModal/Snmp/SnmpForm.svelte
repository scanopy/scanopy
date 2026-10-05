<script lang="ts">
	import type { HostFormData, IPAddress } from '$lib/features/hosts/types/base';
	import type { Site } from '$lib/features/sites/types';
	import type { Credential } from '$lib/features/credentials/types/base';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import ConfigHeader from '$lib/shared/components/forms/config/ConfigHeader.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import { billingPlans, entities } from '$lib/shared/stores/metadata';
	import { CredentialDisplay } from '$lib/shared/components/forms/selection/display/CredentialDisplay.svelte';
	import {
		IPAddressDisplay,
		type IPAddressDisplayContext
	} from '$lib/shared/components/forms/selection/display/IPAddressDisplay.svelte';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import { getCredentialTypeId } from '$lib/features/credentials/types/base';
	import {
		claimedIntegrations,
		daemonHostBlockReason
	} from '$lib/features/credentials/utils/daemonHostBlocking';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import {
		common_credentialDemoReadOnly,
		credentials_addInterfaces,
		credentials_ipTargetAllDefault,
		credentials_ipTargetLabel,
		credentials_ipTargetPlaceholder,
		credentials_noCredentialSelected,
		credentials_selectCredentialSubtitle,
		credentials_selectToAddPlaceholder,
		hosts_credentialOverrideHelp,
		hosts_credentialOverrideHelpLinkText,
		hosts_credentialTargetSubtitle,
		hosts_snmp_credentialOverride,
		hosts_snmp_noOverrides,
		daemons_credentialWizardSiteCredentials
	} from '$lib/paraglide/messages';

	interface Props {
		formData: HostFormData;
		site?: Site | null;
	}

	let { formData = $bindable(), site = null }: Props = $props();

	// TanStack Query for organization and current user (for demo mode check)
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	// Demo mode check: only Owner can modify credential settings in demo orgs
	let isDemoOrg = $derived(
		billingPlans.getMetadata(organization?.plan?.type ?? null).is_demo === true
	);
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

	// This host is a daemon's own host if a daemon reports it as its host_id. Daemon
	// hosts can also be assigned daemon-host-only credentials (e.g. a Docker/Podman socket).
	const daemonsQuery = useDaemonsQuery();
	let isDaemonHost = $derived((daemonsQuery.data ?? []).some((d) => d.host_id === formData.id));

	// Filter to credentials assignable to this host: 'Hosts'-targetable always, plus
	// daemon-host-only ('DaemonHost') credentials when this host belongs to a daemon.
	let perHostCredentials = $derived(
		allCredentials.filter((c) => {
			const targets = credentialTypes.getMetadata(getCredentialTypeId(c))?.targets ?? [];
			return targets.includes('Hosts') || (isDaemonHost && targets.includes('DaemonHost'));
		})
	);

	let availableCredentials = $derived(
		perHostCredentials.filter(
			(c) => !(formData.credential_assignments ?? []).some((a) => a.credential_id === c.id)
		)
	);

	// Socket↔proxy exclusion: a daemon host holds only one transport per single-endpoint
	// integration. Disable (with a reason) any candidate whose integration is already claimed
	// by a credential currently assigned to this host. Shares the predicate with the discovery
	// modal and credential modal via daemonHostBlocking.
	let claimedHostIntegrations = $derived(claimedIntegrations(selectedCredentials));

	// Resolve site default credentials to full objects for EntityTag display
	let siteDefaultCredentials = $derived(
		(site?.credential_ids ?? [])
			.map((id: string) => allCredentials.find((c) => c.id === id))
			.filter((c): c is Credential => c != null)
	);

	let credentialColorHelper = $derived(entities.getColorHelper('Credential'));
	let credentialIcon = $derived(entities.getIconComponent('Credential'));

	function getAssignmentForIndex(index: number) {
		return (formData.credential_assignments ?? [])[index] ?? null;
	}

	// Resolve ip_address_ids for a credential assignment into IPAddress objects
	function getScopedInterfaces(index: number): IPAddress[] {
		const assignment = getAssignmentForIndex(index);
		if (!assignment || assignment.ip_address_ids === null) return [];
		return assignment.ip_address_ids
			.map((id) => formData.ip_addresses.find((i) => i.id === id))
			.filter((i): i is IPAddress => i != null);
	}

	function getInterfaceContext(): IPAddressDisplayContext {
		return { subnets };
	}

	function addInterfaceToScope(credentialIndex: number, interfaceId: string) {
		const assignments = [...(formData.credential_assignments ?? [])];
		if (!assignments[credentialIndex]) return;
		const current = assignments[credentialIndex].ip_address_ids;
		if (current === null) {
			// First add: switch from "all" to explicit list with just this interface
			assignments[credentialIndex] = {
				...assignments[credentialIndex],
				ip_address_ids: [interfaceId]
			};
		} else if (!current.includes(interfaceId)) {
			assignments[credentialIndex] = {
				...assignments[credentialIndex],
				ip_address_ids: [...current, interfaceId]
			};
		}
		formData.credential_assignments = assignments;
	}

	function removeInterfaceFromScope(credentialIndex: number, interfaceIndex: number) {
		const assignments = [...(formData.credential_assignments ?? [])];
		if (!assignments[credentialIndex]) return;
		const current = assignments[credentialIndex].ip_address_ids;
		if (current === null) return;
		const updated = current.filter((_, i) => i !== interfaceIndex);
		assignments[credentialIndex] = {
			...assignments[credentialIndex],
			// Revert to null (all interfaces) when list empties
			ip_address_ids: updated.length === 0 ? null : updated
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
	{#if siteDefaultCredentials.length > 0}
		<p class="text-tertiary mt-1 flex flex-wrap items-center gap-1 text-xs">
			<span>{daemons_credentialWizardSiteCredentials()}</span>
			{#each siteDefaultCredentials as cred (cred.id)}
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
					label={hosts_snmp_credentialOverride()}
					helpSnippet={isNonOwnerInDemo ? undefined : hostCredentialHelpSnippet}
					helpText={isNonOwnerInDemo ? common_credentialDemoReadOnly() : undefined}
					placeholder={credentials_selectToAddPlaceholder()}
					emptyMessage={hosts_snmp_noOverrides()}
					allowReorder={false}
					options={availableCredentials}
					getOptionContext={(c) => ({
						disabledReason: daemonHostBlockReason(getCredentialTypeId(c), claimedHostIntegrations)
					})}
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
								{ credential_id: id, ip_address_ids: null }
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
			{#if selectedItem && formData.ip_addresses.length > 0}
				<div class="space-y-4">
					<ConfigHeader title={selectedItem.name} subtitle={hosts_credentialTargetSubtitle()} />
					<ListManager
						label={credentials_ipTargetLabel()}
						emptyMessage={credentials_ipTargetAllDefault()}
						placeholder={credentials_ipTargetPlaceholder()}
						allowReorder={false}
						options={formData.ip_addresses}
						items={getScopedInterfaces(selectedIndex)}
						optionDisplayComponent={IPAddressDisplay}
						itemDisplayComponent={IPAddressDisplay}
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
