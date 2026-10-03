<script lang="ts" module>
	export type { PendingCredential } from './CredentialWizardStep.svelte';
</script>

<script lang="ts">
	import { tick } from 'svelte';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import {
		useBulkCreateCredentialsMutation,
		useDeleteCredentialMutation
	} from '$lib/features/credentials/queries';
	import { type Credential } from '$lib/features/credentials/types/base';
	import type { DaemonOS } from '$lib/features/daemons/utils';
	import CredentialWizardStep, {
		type PendingCredential as PendingCredentialType
	} from './CredentialWizardStep.svelte';

	interface Props {
		networkId?: string;
		description?: string;
		/** New credentials being configured (bindable so parents can seed from
		 *  existing assignments and read back, e.g. to derive an install flag). */
		pendingCredentials: PendingCredentialType[];
		/** Server-side ids of credentials created/attached this session (bindable). */
		credentialIds?: string[];
		/**
		 * How auto-local capabilities (e.g. the Docker socket) behave:
		 * - `interactive` (daemon setup): the parent seeds them as ordinary wizard entries.
		 * - `fixed` (editing an installed daemon): they reflect the daemon's actual
		 *   capabilities and claim the daemon host for conflict prevention.
		 */
		localAutoMode?: 'interactive' | 'fixed';
		/** In `fixed` mode, the auto-local type ids the target daemon actually has. */
		fixedCapabilityTypeIds?: string[];
		/** Version of the single daemon this step targets. A credential type option is
		 *  disabled when this version is older than the type's `minimum_daemon_version`.
		 *  Absent/null ⇒ no version gate (e.g. create-daemon flow). */
		daemonVersion?: string | null;
		/** Name of that daemon, used in the version-requirement tooltip. */
		daemonName?: string | null;
		/** The daemon's OS when known (create-daemon flow, or a daemon created with one). New
		 *  credentials take its OS family, and existing ones set up for another are blocked. */
		daemonOs?: DaemonOS | null;
	}

	let {
		networkId = '',
		description,
		pendingCredentials = $bindable([]),
		credentialIds = $bindable([]),
		localAutoMode = 'interactive',
		fixedCapabilityTypeIds = [],
		daemonVersion = null,
		daemonName = null,
		daemonOs = null
	}: Props = $props();

	const bulkCreateCredentialsMutation = useBulkCreateCredentialsMutation();
	const deleteCredentialMutation = useDeleteCredentialMutation();

	let credentialWizardRef: ReturnType<typeof CredentialWizardStep> | undefined = $state();

	// In `fixed` mode the daemon's existing capabilities claim their integration's
	// daemon host, so a single-endpoint credential can't also target it.
	let claimedDaemonHostIntegrations = $derived(
		localAutoMode === 'fixed'
			? fixedCapabilityTypeIds
					.map((id) => credentialTypes.getMetadata(id)?.associated_service)
					.filter((s): s is string => !!s)
			: []
	);

	// Seed the wizard with one new entry per type id — including daemon-host-only sockets,
	// which are configured and assigned to the daemon host like any other credential.
	async function addTypes(typeIds: string[]) {
		await tick();
		credentialWizardRef?.addTypes(typeIds);
	}

	function handleRemoveCredential(credential: Credential) {
		// Delete credentials created this session when removed; leave others.
		if (credentialIds.includes(credential.id)) {
			deleteCredentialMutation.mutate(credential.id);
			credentialIds = credentialIds.filter((id) => id !== credential.id);
		}
	}

	/**
	 * Persist the wizard's credentials: validate + bulk-create new ones (idempotent —
	 * already-created ones are skipped), and return the accumulated credential ids. Returns
	 * `null` if validation fails (caller should not advance).
	 *
	 * Per-credential target IPs are NOT written to `credential.target_ips` anymore (that field
	 * is retired, #637). They are delivered per-daemon via the init command's integration-target
	 * tokens, built by the caller from each pending credential's `targetIps`.
	 */
	async function collectCredentialIds(): Promise<string[] | null> {
		if (!credentialWizardRef) return [...credentialIds];

		const existingCreds = credentialWizardRef.getExistingCredentials();
		const existingIds = existingCreds.map((c) => c.credentialId);

		// Targeting is validated for every row, saved or not. An already-persisted credential
		// whose target IPs were cleared still gets re-serialized into the daemon's integration
		// targets, and a type that can't broadcast has no scope left to fall back on — so
		// skipping this for a batch with nothing new to create would let it silently never run.
		if (!credentialWizardRef.validateTargets()) return null;

		const unsaved = pendingCredentials.filter(
			(p) => !p.isExisting && !credentialIds.includes(p.credential.id)
		);
		if (unsaved.length > 0) {
			const isValid = await credentialWizardRef.validate();
			if (!isValid) return null;
			try {
				const prepared = credentialWizardRef
					.getCredentialsForCreate()
					.filter((p) => !credentialIds.includes(p.credential.id));
				const toCreate = prepared.map((p) => ({ ...p.credential }));
				const created = await bulkCreateCredentialsMutation.mutateAsync(toCreate);
				credentialIds = [
					...new Set([...credentialIds, ...created.map((c) => c.id), ...existingIds])
				];
			} catch {
				return null;
			}
		} else if (existingIds.length > 0) {
			credentialIds = [...new Set([...credentialIds, ...existingIds])];
		}
		return [...credentialIds];
	}

	// Exposed (read `credentialsStep.busy`) so a parent can disable its submit button
	// while a create/update is in flight.
	let busy = $derived(bulkCreateCredentialsMutation.isPending);

	export { busy, addTypes, collectCredentialIds };
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<CredentialWizardStep
		bind:this={credentialWizardRef}
		{networkId}
		{description}
		bind:pendingCredentials
		{claimedDaemonHostIntegrations}
		{daemonVersion}
		{daemonName}
		onRemoveCredential={handleRemoveCredential}
		{daemonOs}
	/>
</div>
