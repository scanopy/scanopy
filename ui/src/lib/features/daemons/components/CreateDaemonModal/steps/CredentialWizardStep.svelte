<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { validateForm } from '$lib/shared/components/forms/form-context';
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { CredentialTypeDisplay } from '$lib/shared/components/forms/selection/display/CredentialTypeDisplay.svelte';
	import { CredentialDisplay } from '$lib/shared/components/forms/selection/display/CredentialDisplay.svelte';
	import CredentialForm from '$lib/features/credentials/components/CredentialForm.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import { credentialTypes, entities } from '$lib/shared/stores/metadata';
	import type { Credential, CredentialType } from '$lib/features/credentials/types/base';
	import { createDefaultCredential } from '$lib/features/credentials/types/base';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { v4 as uuidv4 } from 'uuid';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import {
		daemons_credentialWizardTitle,
		daemons_credentialWizardDescription,
		daemons_credentialWizardDescriptionLinkText,
		daemons_credentialWizardSelectType,
		daemons_credentialWizardEmpty,
		daemons_credentialWizardNetworkCredentials,
		daemons_credentialWizardCreateNew,
		daemons_credentialWizardAddExisting,
		daemons_credentialWizardSelectExisting,
		daemons_credentialWizardExistingDescription
	} from '$lib/paraglide/messages';

	export interface PendingCredential {
		credential: Credential;
		targetIps: string[];
		fieldValues: Record<string, string>;
		isExisting?: boolean;
	}

	interface Props {
		daemonName?: string;
		networkId?: string;
		pendingCredentials: PendingCredential[];
		onRemoveCredential?: (credential: Credential) => void;
		description?: string;
		descriptionLinkText?: string;
	}

	let {
		daemonName = 'scanopy-daemon',
		networkId = '',
		pendingCredentials = $bindable([]),
		onRemoveCredential,
		description,
		descriptionLinkText
	}: Props = $props();

	// Query network and credential data for network-level credential display
	const networksQuery = useNetworksQuery();
	const credentialsQuery = useCredentialsQuery();

	let networkCredentials = $derived.by(() => {
		if (!networkId || !networksQuery.data || !credentialsQuery.data) return [];
		const network = networksQuery.data.find((n) => n.id === networkId);
		if (!network?.credential_ids?.length) return [];
		return credentialsQuery.data.filter((c) => network.credential_ids!.includes(c.id));
	});

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	// Local items array for ListConfigEditor display
	let items = $derived(pendingCredentials.map((p) => p.credential));

	let typeOptions = $derived(credentialTypes.getItems());

	// Available existing credentials (filter out already-added and network-level)
	let availableExistingCredentials = $derived.by(() => {
		if (!credentialsQuery.data) return [];
		const pendingIds = new Set(pendingCredentials.map((p) => p.credential.id));
		const networkCredIds = new Set(networkCredentials.map((c) => c.id));
		return credentialsQuery.data.filter((c) => !pendingIds.has(c.id) && !networkCredIds.has(c.id));
	});

	// Refs to each CredentialForm for buildCredentialType()
	let credentialFormRefs: (ReturnType<typeof CredentialForm> | undefined)[] = $state([]);

	// Build form default values from pendingCredentials
	function buildFormDefaults() {
		const credentials: Record<string, unknown>[] = pendingCredentials.map((p) => ({
			targetIps: [...p.targetIps],
			fields: { ...p.fieldValues }
		}));
		return { credentials };
	}

	// TanStack form owns all credential field data
	const form = createForm(() => ({
		defaultValues: buildFormDefaults(),
		onSubmit: async () => {
			// Handled externally via validate()
		}
	}));

	function syncFormDefaults() {
		form.reset(buildFormDefaults());
	}

	function initDefaultFieldValues(typeId: string): Record<string, string> {
		const meta = credentialTypes.getMetadata(typeId);
		const fields = meta?.fields ?? [];
		const values: Record<string, string> = {};
		for (const field of fields) {
			if (field.field_type === 'pathorinline') {
				values[field.id] = JSON.stringify({ mode: 'Inline', value: '' });
			} else {
				values[field.id] = field.default_value ?? '';
			}
		}
		return values;
	}

	function handleAddCredential(typeId: string) {
		if (!organization) return;

		const cred = {
			...createDefaultCredential(organization.id),
			id: uuidv4(),
			name: credentialTypes.getName(typeId),
			credential_type: { type: typeId } as Credential['credential_type']
		};

		// Set defaults from fixture metadata
		const meta = credentialTypes.getMetadata(typeId);
		if (meta?.fields) {
			const ct = cred.credential_type as unknown as Record<string, unknown>;
			for (const field of meta.fields) {
				if (field.default_value != null && ct[field.id] === undefined) {
					if (field.field_type === 'secretpathorinline' || field.field_type === 'pathorinline') {
						ct[field.id] = { mode: 'Inline', value: field.default_value };
					} else {
						const num = Number(field.default_value);
						ct[field.id] = !isNaN(num) ? num : field.default_value;
					}
				}
			}
		}

		const fieldValues = initDefaultFieldValues(typeId);
		pendingCredentials = [
			...pendingCredentials,
			{ credential: cred, targetIps: [''], fieldValues }
		];
		syncFormDefaults();
	}

	function handleAddExistingCredential(credentialId: string) {
		const existing = credentialsQuery.data?.find((c) => c.id === credentialId);
		if (!existing) return;
		pendingCredentials = [
			...pendingCredentials,
			{ credential: existing, targetIps: [''], fieldValues: {}, isExisting: true }
		];
		syncFormDefaults();
	}

	function handleRemoveCredential(index: number) {
		const removed = pendingCredentials[index];
		if (removed && !removed.isExisting) {
			onRemoveCredential?.(removed.credential);
		}
		pendingCredentials = pendingCredentials.filter((_, i) => i !== index);
		syncFormDefaults();
	}

	function handleCredentialChange(credential: Credential, index: number) {
		pendingCredentials = pendingCredentials.map((p, i) => (i === index ? { ...p, credential } : p));
	}

	function handleConfigChange(
		index: number,
		data: { targetIps?: string[]; fieldValues?: Record<string, string> }
	) {
		pendingCredentials = pendingCredentials.map((p, i) => {
			if (i !== index) return p;
			const updated = { ...p };
			if (data.targetIps !== undefined) {
				updated.targetIps = data.targetIps;
				// Update credential name based on first targetIp (only for new credentials)
				if (!p.isExisting) {
					const ip = (data.targetIps[0] ?? '').trim();
					const isLocalhost = ip === '127.0.0.1' || ip === '::1' || ip === 'localhost' || ip === '';
					const name = isLocalhost ? daemonName : ip;
					updated.credential = { ...p.credential, name };
				}
			}
			if (data.fieldValues !== undefined) {
				updated.fieldValues = data.fieldValues;
			}
			return updated;
		});
	}

	/** Validate all fields across all credentials. Returns true if valid. */
	export async function validate(): Promise<boolean> {
		const isValid = await validateForm(form);
		return isValid;
	}

	/** Get new credentials ready for bulk creation (with built credential_type from fieldValues). */
	export function getCredentialsForCreate(): { credential: Credential; targetIps: string[] }[] {
		return pendingCredentials
			.map((p, i) => ({ p, i }))
			.filter(({ p }) => !p.isExisting)
			.map(({ p, i }) => {
				const ref = credentialFormRefs[i];
				const credentialType =
					ref?.buildCredentialType() ?? (p.credential.credential_type as CredentialType);
				return {
					credential: {
						...p.credential,
						credential_type: credentialType
					},
					targetIps: p.targetIps
				};
			});
	}

	/** Get existing credentials that were added (already saved on server). */
	export function getExistingCredentials(): { credentialId: string; targetIps: string[] }[] {
		return pendingCredentials
			.filter((p) => p.isExisting)
			.map((p) => ({ credentialId: p.credential.id, targetIps: p.targetIps }));
	}
</script>

{#snippet credentialHelpSnippet()}
	<DocsHint
		text={description ?? daemons_credentialWizardDescription()}
		href="https://scanopy.net/docs/using-scanopy/credentials/"
		linkText={descriptionLinkText ?? daemons_credentialWizardDescriptionLinkText()}
	/>
	{#if networkCredentials.length > 0}
		<p class="text-tertiary mt-1 text-xs">
			{daemons_credentialWizardNetworkCredentials()}
			{#each networkCredentials as cred (cred.id)}
				<EntityTag
					entityRef={{
						entityType: 'Credential',
						entityId: cred.id,
						data: cred
					}}
					label={cred.name}
					color={entities.getColorHelper('Credential').color}
				/>
			{/each}
		</p>
	{/if}
{/snippet}

<div class="flex min-h-0 flex-1 flex-col">
	<ListConfigEditor {items} onChange={handleCredentialChange}>
		<svelte:fragment slot="list" let:items let:onEdit let:highlightedIndex let:onItemSelect>
			<ListManager
				label={daemons_credentialWizardTitle()}
				helpSnippet={credentialHelpSnippet}
				placeholder={daemons_credentialWizardSelectType()}
				emptyMessage={daemons_credentialWizardEmpty()}
				options={typeOptions}
				itemClickAction="edit"
				allowReorder={false}
				allowDuplicates={true}
				optionDisplayComponent={CredentialTypeDisplay}
				itemDisplayComponent={CredentialDisplay}
				primaryOptionsLabel={daemons_credentialWizardCreateNew()}
				secondaryOptions={availableExistingCredentials}
				secondaryOptionDisplayComponent={CredentialDisplay}
				secondaryPlaceholder={daemons_credentialWizardSelectExisting()}
				secondaryOptionsLabel={daemons_credentialWizardAddExisting()}
				onAddSecondary={handleAddExistingCredential}
				{items}
				onAdd={handleAddCredential}
				onRemove={handleRemoveCredential}
				onClick={onItemSelect}
				{onEdit}
				{highlightedIndex}
			/>
		</svelte:fragment>

		<svelte:fragment slot="config" let:selectedItem let:selectedIndex>
			<!-- Render ALL config panels, hide non-selected (like InterfacesForm) -->
			{#each pendingCredentials as pending, index (`${pending.credential.id}-${index}`)}
				<div class:hidden={selectedIndex !== index}>
					{#if pending.isExisting}
						<p class="text-muted mb-4 text-xs">
							{daemons_credentialWizardExistingDescription()}
						</p>
						<CredentialForm
							bind:this={credentialFormRefs[index]}
							{form}
							compact={true}
							hideFields={true}
							fieldPrefix={`credentials[${index}].`}
							fixedCredentialType={pending.credential.credential_type.type}
							fixedName={pending.credential.name}
							onChange={(data) => handleConfigChange(index, data)}
						/>
					{:else}
						<CredentialForm
							bind:this={credentialFormRefs[index]}
							{form}
							compact={true}
							fieldPrefix={`credentials[${index}].`}
							fixedCredentialType={pending.credential.credential_type.type}
							fixedName={pending.credential.name}
							onChange={(data) => handleConfigChange(index, data)}
						/>
					{/if}
				</div>
			{/each}

			{#if !selectedItem}
				<EntityConfigEmpty title={daemons_credentialWizardSelectType()} subtitle="" />
			{/if}
		</svelte:fragment>
	</ListConfigEditor>
</div>
