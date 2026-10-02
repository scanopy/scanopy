<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { validateForm } from '$lib/shared/components/forms/form-context';
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { CredentialTypeDisplay } from '$lib/shared/components/forms/selection/display/CredentialTypeDisplay.svelte';
	import { CredentialDisplay } from '$lib/shared/components/forms/selection/display/CredentialDisplay.svelte';
	import CredentialForm from '$lib/features/credentials/components/CredentialForm.svelte';
	import { slugifyNetworkName } from '$lib/features/daemons/utils';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import { credentialTypes, entities } from '$lib/shared/stores/metadata';
	import type { TypedTypeMetadata, CredentialTypeMetadata } from '$lib/shared/stores/metadata';
	import type { Credential, CredentialType } from '$lib/features/credentials/types/base';
	import { osFamilyOf, type DaemonOS } from '$lib/features/daemons/utils';
	import type { Host } from '$lib/features/hosts/types/base';
	import {
		createDefaultCredential,
		isDaemonHostOnly as isDaemonHostOnlyTargets
	} from '$lib/features/credentials/types/base';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import {
		daemonOsRefusal,
		daemonTooOldForCredential
	} from '$lib/features/credentials/utils/versionGate';
	import { defaultFieldValue } from '$lib/features/credentials/utils/fieldValues';
	import {
		DAEMON_HOST_IP,
		hasExplicitTarget
	} from '$lib/features/credentials/utils/credentialTargets';
	import { v4 as uuidv4 } from 'uuid';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import { pushError } from '$lib/shared/stores/feedback';
	import {
		common_name,
		common_ipAddress,
		daemons_credentialWizardTargetRequired,
		daemons_credentialWizardTitle,
		daemons_credentialWizardDescription,
		daemons_credentialWizardDescriptionLinkText,
		daemons_credentialWizardSelectType,
		daemons_credentialWizardEmpty,
		daemons_credentialWizardNetworkCredentials,
		daemons_credentialWizardCreateNew,
		daemons_credentialWizardAddExisting,
		daemons_credentialWizardSelectExisting,
		daemons_credentialWizardExistingDescription,
		daemons_credentialWizardDaemonHostUnavailable,
		credentials_requiresDaemonVersion
	} from '$lib/paraglide/messages';

	export interface PendingCredential {
		credential: Credential;
		targetIps: string[];
		fieldValues: Record<string, string>;
		isExisting?: boolean;
		/** Hosts on this network the credential is already assigned to through the
		 *  host/credential junction. Listed on the card, not editable here. Resolved by
		 *  the caller — this component has no hosts query. */
		lockedHosts?: Host[];
		// How the new credential is assigned: 'broadcast' (network default) or
		// 'per_host' (target IPs). Defaults based on the type's scope_models.
		scope?: 'broadcast' | 'per_host';
	}

	interface Props {
		networkId?: string;
		pendingCredentials: PendingCredential[];
		onRemoveCredential?: (credential: Credential) => void;
		description?: string;
		descriptionLinkText?: string;
		/** Integrations whose daemon-host endpoint is already occupied by a fixed
		 *  daemon capability (e.g. an installed daemon's local Docker socket). These
		 *  count as claiming the daemon host, so a single-endpoint credential of the
		 *  same integration can't also target it. */
		claimedDaemonHostIntegrations?: string[];
		/** Version + name of the daemon this credential set targets. A credential type
		 *  is disabled in the add-credential dropdown when the daemon is older than the
		 *  type's `minimum_daemon_version`. Absent/null ⇒ no version gate. */
		daemonVersion?: string | null;
		daemonName?: string | null;
		/** The daemon's OS when known (create-daemon flow, or a daemon created with one). New
		 *  credentials take its OS family as their `daemon_os` and hide the Daemon OS picker;
		 *  existing credentials set up for another family are blocked. */
		daemonOs?: DaemonOS | null;
	}

	let {
		networkId = '',
		pendingCredentials = $bindable([]),
		onRemoveCredential,
		description,
		descriptionLinkText,
		claimedDaemonHostIntegrations = [],
		daemonVersion = null,
		daemonName = null,
		daemonOs = null
	}: Props = $props();

	let fixedDaemonOs = $derived(daemonOs ? osFamilyOf(daemonOs) : null);

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
	/** A row listed only because the credential is assigned elsewhere — the user has not
	 *  pointed it at anything on this discovery. It contributes no target. */
	function isLockedOnly(p: PendingCredential): boolean {
		return !!p.lockedHosts?.length && !hasExplicitTarget(p.scope, p.targetIps);
	}

	// A credential assigned through the junction is never removable from this list: the
	// assignment lives on the host/credential, so the trash would either do nothing or
	// imply it had unassigned it. Its targets *here* are removed row by row instead.
	let assignedElsewhereCredIds = $derived(
		pendingCredentials.filter((p) => p.lockedHosts?.length).map((p) => p.credential.id)
	);

	function isDaemonHostOnly(typeId: string): boolean {
		return isDaemonHostOnlyTargets(credentialTypes.getMetadata(typeId)?.targets);
	}

	function isLoopback(ip: string): boolean {
		const t = ip?.trim() ?? '';
		return t === '127.0.0.1' || t === '::1' || t === 'localhost';
	}

	/**
	 * Whether a pending credential claims its integration's daemon host: a daemon-host-only
	 * type (the local socket) always does; a configurable cred does when it targets a loopback.
	 * The daemon host is a single endpoint per `single_endpoint_per_host` integration, so only
	 * one credential may hold it.
	 */
	function claimsDaemonHost(p: PendingCredential): boolean {
		if (p.isExisting) return false;
		const meta = credentialTypes.getMetadata(p.credential.credential_type.type);
		if (!meta?.single_endpoint_per_host) return false;
		return isDaemonHostOnlyTargets(meta.targets) || p.targetIps.some(isLoopback);
	}

	function integrationOf(typeId: string): string | undefined {
		return credentialTypes.getMetadata(typeId)?.associated_service;
	}

	/** Whether an integration's daemon host is already claimed — by a fixed daemon
	 *  capability (`claimedDaemonHostIntegrations`) or by a pending credential other
	 *  than the one at `exceptIndex`. */
	function integrationClaimsDaemonHost(integration: string, exceptIndex?: number): boolean {
		if (claimedDaemonHostIntegrations.includes(integration)) return true;
		return pendingCredentials.some(
			(other, j) =>
				j !== exceptIndex &&
				integrationOf(other.credential.credential_type.type) === integration &&
				claimsDaemonHost(other)
		);
	}

	/** True when the daemon host of this credential's single-endpoint integration is
	 *  already claimed elsewhere — used to disable the "Add daemon host" action. */
	function daemonHostUnavailableFor(index: number): boolean {
		const p = pendingCredentials[index];
		if (!p || p.isExisting) return false;
		const meta = credentialTypes.getMetadata(p.credential.credential_type.type);
		if (!meta?.single_endpoint_per_host || !meta.associated_service) return false;
		return integrationClaimsDaemonHost(meta.associated_service, index);
	}

	// Type dropdown eligibility: offer user-selectable types plus auto-local
	// capabilities (e.g. the Docker socket). A local capability can only be added
	// once (so an already-pending one is filtered out). When its integration's daemon
	// host is already claimed by another pending credential it stays in the list but
	// is shown disabled with a reason (see dropdownDisabledReason). Configurable types
	// (e.g. multiple Docker Proxies on different hosts) are never blanket-blocked.
	let typeOptions = $derived(
		credentialTypes.getItems().filter((t) => {
			// A daemon-host-only type (the local socket) can only be added once (one daemon
			// host). Locked-only rows are excluded: they span every host on the network, and
			// one assigned to some *other* host leaves this daemon's host free. When one
			// genuinely occupies it, that reaches the dropdown through
			// `claimedDaemonHostIntegrations` instead, which disables the option with a
			// reason rather than hiding it.
			if (
				isDaemonHostOnlyTargets(t.metadata?.targets) &&
				pendingCredentials.some(
					(p) => !isLockedOnly(p) && p.credential.credential_type.type === t.id
				)
			) {
				return false;
			}
			return true;
		})
	);

	// Reason an option is unselectable in the add dropdown (null = selectable):
	// (1) the target daemon is too old for the type, or (2) an auto-local capability
	// (Docker socket) whose integration's daemon host is already claimed by another
	// pending credential (e.g. a daemon-host proxy). Version takes precedence — a
	// too-old daemon can't run the type at all.
	function dropdownDisabledReason(type: TypedTypeMetadata<CredentialTypeMetadata>): string | null {
		if (daemonTooOldForCredential(type.metadata?.minimum_daemon_version, daemonVersion)) {
			return credentials_requiresDaemonVersion({
				version: type.metadata?.minimum_daemon_version ?? '',
				name: daemonName ?? ''
			});
		}
		if (!isDaemonHostOnlyTargets(type.metadata?.targets) || !type.metadata.associated_service)
			return null;
		return integrationClaimsDaemonHost(type.metadata.associated_service)
			? daemons_credentialWizardDaemonHostUnavailable({
					integration: type.metadata.associated_service
				})
			: null;
	}

	// Available existing credentials (filter out already-added and network-level)
	let availableExistingCredentials = $derived.by(() => {
		if (!credentialsQuery.data) return [];
		// Every credential gets exactly one card, junction-assigned ones included — adding
		// a target to an existing card is how you target it here, so re-offering it would
		// only produce a duplicate.
		const pendingIds = new Set(pendingCredentials.map((p) => p.credential.id));
		const networkCredIds = new Set(networkCredentials.map((c) => c.id));
		return credentialsQuery.data.filter((c) => !pendingIds.has(c.id) && !networkCredIds.has(c.id));
	});

	// Refs to each CredentialForm for buildCredentialType()
	let credentialFormRefs: (ReturnType<typeof CredentialForm> | undefined)[] = $state([]);

	// Build form default values from pendingCredentials
	function buildFormDefaults() {
		const credentials: Record<string, unknown>[] = pendingCredentials.map((p) => ({
			name: p.credential.name,
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
			values[field.id] = defaultFieldValue(field);
		}
		return values;
	}

	// Auto-generate a stable name: the type kebab-cased plus the next free number
	// (e.g. docker-proxy-1, docker-proxy-2). Avoids collisions on remove/re-add.
	function nextCredentialName(typeId: string): string {
		const prefix = `${slugifyNetworkName(credentialTypes.getName(typeId) ?? typeId)}-`;
		let max = 0;
		for (const p of pendingCredentials) {
			if (!p.credential.name.startsWith(prefix)) continue;
			const n = parseInt(p.credential.name.slice(prefix.length), 10);
			if (Number.isInteger(n)) max = Math.max(max, n);
		}
		return `${prefix}${max + 1}`;
	}

	function handleAddCredential(typeId: string) {
		if (!organization) return;

		const cred = {
			...createDefaultCredential(organization.id),
			id: uuidv4(),
			name: nextCredentialName(typeId),
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
		// Network-capable types (e.g. SNMP) default to broadcast scope, matching
		// CredentialForm's initial target.
		const supportsBroadcast = (credentialTypes.getMetadata(typeId)?.targets ?? []).includes(
			'Network'
		);
		pendingCredentials = [
			...pendingCredentials,
			{
				credential: cred,
				// A daemon-host-only type (the local socket) has exactly one possible target
				// and no picker, so carry it on the row: the save path reads targets from
				// here, and a row that looks targeted in the UI but empty in its data is
				// dropped silently.
				targetIps: isDaemonHostOnly(typeId) ? [DAEMON_HOST_IP] : [],
				fieldValues,
				scope: supportsBroadcast ? 'broadcast' : 'per_host'
			}
		];
		syncFormDefaults();
	}

	/**
	 * Seed the wizard with one new credential per given type id (used to prefill
	 * from the credential-type selection step). Types already present as a new
	 * (non-existing) pending credential are skipped to avoid duplicates.
	 */
	export function addTypes(typeIds: string[]) {
		for (const typeId of typeIds) {
			const alreadyPending = pendingCredentials.some(
				(p) => !p.isExisting && p.credential.credential_type.type === typeId
			);
			if (!alreadyPending) handleAddCredential(typeId);
		}
		// Reconcile daemon-host-only entries (the local socket) with the grid selection: drop
		// any that were deselected. Configurable creds added in the wizard are kept.
		pendingCredentials = pendingCredentials.filter(
			(p) =>
				!isDaemonHostOnly(p.credential.credential_type.type) ||
				typeIds.includes(p.credential.credential_type.type)
		);
	}

	function handleAddExistingCredential(credentialId: string) {
		const existing = credentialsQuery.data?.find((c) => c.id === credentialId);
		if (!existing) return;
		// The picker disables these; this holds for any other path in.
		if (daemonOsRefusal(existing, daemonOs, daemonName)) return;
		pendingCredentials = [
			...pendingCredentials,
			{
				credential: existing,
				// As in handleAddCredential: a daemon-host-only type's single target is
				// implicit, so record it rather than leaving the row looking untargeted.
				targetIps: isDaemonHostOnly(existing.credential_type.type) ? [DAEMON_HOST_IP] : [''],
				fieldValues: {},
				isExisting: true,
				// Mirror handleAddCredential: reset() picks the same default internally, but
				// never emits it, so the row must carry it or a broadcast type would be
				// added with no recorded selection and written out as no target at all.
				scope: (credentialTypes.getMetadata(existing.credential_type.type)?.targets ?? []).includes(
					'Network'
				)
					? 'broadcast'
					: 'per_host'
			}
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
		data: {
			targetIps?: string[];
			fieldValues?: Record<string, string>;
			scope?: 'broadcast' | 'per_host';
			name?: string;
		}
	) {
		pendingCredentials = pendingCredentials.map((p, i) => {
			if (i !== index) return p;
			const updated = { ...p };
			if (data.scope !== undefined) {
				updated.scope = data.scope;
			}
			if (data.targetIps !== undefined) {
				updated.targetIps = data.targetIps;
			}
			if (data.fieldValues !== undefined) {
				updated.fieldValues = data.fieldValues;
			}
			if (data.name !== undefined) {
				updated.credential = { ...p.credential, name: data.name };
			}
			return updated;
		});
	}

	// Map a shared-form field path (e.g. `credentials[1].fields.community`) to a
	// "<credential name>: <field label>" string for the validation toast. Falls back
	// to the index label when the credential name is blank, and to the raw subfield
	// when no friendly label is known.
	function credentialName(idx: number): string {
		return pendingCredentials[idx]?.credential.name?.trim() || `credentials[${idx}]`;
	}

	function credentialFieldLabel(fieldPath: string): string {
		const match = fieldPath.match(/^credentials\[(\d+)\]\.(.+)$/);
		if (!match) return fieldPath.replace(/_/g, ' ');
		const idx = Number(match[1]);
		const sub = match[2];
		const pending = pendingCredentials[idx];
		const credName = credentialName(idx);

		let fieldLabel: string;
		if (sub === 'name') {
			fieldLabel = common_name();
		} else if (sub.startsWith('targetIps')) {
			fieldLabel = common_ipAddress();
		} else if (sub.startsWith('fields.')) {
			const fieldId = sub.slice('fields.'.length);
			const fields = credentialTypes.getMetadata(pending?.credential.credential_type.type)?.fields;
			fieldLabel = fields?.find((f) => f.id === fieldId)?.label ?? fieldId.replace(/_/g, ' ');
		} else {
			fieldLabel = sub.replace(/_/g, ' ');
		}
		return `${credName}: ${fieldLabel}`;
	}

	/**
	 * Validate every credential's target selection. Returns true if valid.
	 *
	 * "Target Specific Hosts" requires at least one host. Auto-local items have no form ref
	 * (undefined) and are skipped. (Daemon-host conflicts are prevented proactively at input —
	 * the "Add daemon host" button is disabled.) Surfaces a toast naming the credentials that
	 * need a target.
	 *
	 * Split out from `validate` because this must run even when there is nothing new to create:
	 * an already-persisted credential whose targets were emptied would otherwise be serialized
	 * with no scope its type permits, and silently never run.
	 */
	export function validateTargets(): boolean {
		// Iterate the credentials, not the ref array: `handleRemoveCredential` never truncates
		// `credentialFormRefs`, so a stale entry past the end would report a phantom failure
		// against a row that no longer exists.
		const missingTargets = pendingCredentials
			.map((p, i) => (credentialFormRefs[i]?.validateTarget() === false ? credentialName(i) : null))
			.filter((n): n is string => n !== null);
		if (missingTargets.length > 0) {
			pushError(daemons_credentialWizardTargetRequired({ credentials: missingTargets.join(', ') }));
		}
		return missingTargets.length === 0;
	}

	/** Validate all fields and targets across all credentials. Returns true if valid. */
	export async function validate(): Promise<boolean> {
		// validateForm surfaces field errors as a toast itself. Only report missing targets
		// once the fields are clean, so one advance doesn't stack two toasts.
		const fieldsValid = await validateForm(form, undefined, credentialFieldLabel);
		return fieldsValid && validateTargets();
	}

	/** Get new credentials ready for bulk creation (with built credential_type from fieldValues).
	 *  Includes local-auto socket types — they are ordinary credentials now (referenced by a
	 *  `<uuid>@127.0.0.1` target), created with their default (auto-detect) config. */
	export function getCredentialsForCreate(): { credential: Credential; targetIps: string[] }[] {
		return pendingCredentials
			.map((p, i) => ({ p, i }))
			.filter(({ p }) => !p.isExisting)
			.map(({ p, i }) => {
				const ref = credentialFormRefs[i];
				const credentialType =
					ref?.buildCredentialType() ?? (p.credential.credential_type as CredentialType);
				const isBroadcast = p.scope === 'broadcast';
				return {
					credential: {
						...p.credential,
						credential_type: credentialType,
						// Set only when the credential reads something on the daemon; the form applies
						// `fixedDaemonOs` itself. Without a form the server clears an unused one.
						daemon_os: ref ? ref.getDaemonOs() : (fixedDaemonOs ?? p.credential.daemon_os),
						// Broadcast: assign as a network default. Per-host: leave to target_ips.
						assigned_network_ids: isBroadcast && networkId ? [networkId] : []
					},
					targetIps: isBroadcast ? [] : p.targetIps
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
				showSearch={true}
				getOptionContext={(option) => ({ disabledReason: dropdownDisabledReason(option) })}
				itemClickAction="edit"
				allowReorder={false}
				allowDuplicates={true}
				optionDisplayComponent={CredentialTypeDisplay}
				itemDisplayComponent={CredentialDisplay}
				primaryOptionsLabel={daemons_credentialWizardCreateNew()}
				secondaryOptions={availableExistingCredentials}
				getSecondaryOptionContext={(c) => ({
					disabledReason: daemonOsRefusal(c, daemonOs, daemonName)
				})}
				secondaryOptionDisplayComponent={CredentialDisplay}
				secondaryPlaceholder={daemons_credentialWizardSelectExisting()}
				secondaryOptionsLabel={daemons_credentialWizardAddExisting()}
				onAddSecondary={handleAddExistingCredential}
				{items}
				onAdd={handleAddCredential}
				onRemove={handleRemoveCredential}
				allowItemRemove={(c) => !assignedElsewhereCredIds.includes(c.id)}
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
						<!-- Existing credential added by reference (incl. daemon-host-only sockets):
						     read-only here (fields shown disabled). Checked before isDaemonHostOnly so an
						     existing socket cred renders as a reference card, not an editable new form.
						     A credential already assigned to a host through the junction lands here too,
						     with those assignments rendered as locked target rows — one card per
						     credential, whether this discovery targets it, it is assigned elsewhere,
						     or both. -->
						<div class="mb-4">
							<InlineInfo body={daemons_credentialWizardExistingDescription()} />
						</div>
						<CredentialForm
							bind:this={credentialFormRefs[index]}
							{form}
							credential={pending.credential}
							compact={true}
							disabled={true}
							fieldPrefix={`credentials[${index}].`}
							fixedCredentialType={pending.credential.credential_type.type}
							fixedName={pending.credential.name}
							daemonHostUnavailable={daemonHostUnavailableFor(index)}
							lockedHosts={pending.lockedHosts ?? []}
							targetIps={pending.targetIps}
							scope={pending.scope}
							onChange={(data) => handleConfigChange(index, data)}
						/>
					{:else if isDaemonHostOnly(pending.credential.credential_type.type)}
						<!-- Daemon-host-only credential (e.g. the local Docker/Podman socket): its
						     target is implicitly the daemon host (127.0.0.1), so no target picker —
						     but it's a real credential with optional config (e.g. socket_path). -->
						<CredentialForm
							bind:this={credentialFormRefs[index]}
							{form}
							compact={true}
							hideTargets={true}
							fieldPrefix={`credentials[${index}].`}
							fixedCredentialType={pending.credential.credential_type.type}
							{fixedDaemonOs}
							onChange={(data) => handleConfigChange(index, data)}
						/>
					{:else}
						<CredentialForm
							bind:this={credentialFormRefs[index]}
							{form}
							compact={true}
							fieldPrefix={`credentials[${index}].`}
							fixedCredentialType={pending.credential.credential_type.type}
							daemonHostUnavailable={daemonHostUnavailableFor(index)}
							targetIps={pending.targetIps}
							scope={pending.scope}
							{fixedDaemonOs}
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
