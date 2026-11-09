<script lang="ts">
	import { untrack, tick } from 'svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { queryKeys } from '$lib/api/query-client';
	import { validateForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import type { ModalTab } from '$lib/shared/components/layout/GenericModal.svelte';
	import type { ModalAction } from '$lib/shared/components/layout/modalActions';
	import { pushError, pushSuccess, pushWarning } from '$lib/shared/stores/feedback';
	import { copyText } from '$lib/shared/utils/clipboard';
	import { common_failedToCopy } from '$lib/paraglide/messages';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { entities } from '$lib/shared/stores/metadata';
	import {
		Settings,
		Terminal,
		Loader2,
		ArrowLeft,
		Mail,
		Copy,
		Check,
		KeyRound
	} from '@lucide/svelte';
	import confetti from 'canvas-confetti';
	import type { DaemonMode } from '../../types/base';
	import { fillInstallArtifactsKey, installStepCommand } from '../../types/base';
	import {
		useProvisionDaemonMutation,
		useDaemonQuery,
		useDaemonInstallCommandQuery,
		useEmailInstallCommandMutation,
		useProvisionEligibilityQuery,
		type InstallCommandParams
	} from '../../queries';
	import { useConfigQuery, isCloud } from '$lib/shared/stores/config-query';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useTestReachabilityMutation } from '../../queries';
	import { getVisibleFieldIds } from '../../config';
	import {
		buildDefaultValues,
		buildInstallConfig,
		constructDaemonUrl,
		detectOS,
		provisionGate,
		slugifySiteName,
		type DaemonOS
	} from '../../utils';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { daemonSetupState, type DaemonConnectionStatus } from '../../stores/daemon-setup';
	import ConfigureStep from './steps/ConfigureStep.svelte';
	import InstallStep from './steps/InstallStep.svelte';
	import AdvancedStep from './steps/AdvancedStep.svelte';
	import UnconnectedDaemonsNotice from './UnconnectedDaemonsNotice.svelte';
	import CredentialsStep, {
		type PendingCredential
	} from '$lib/features/credentials/components/CredentialsStep.svelte';
	import {
		isDaemonHostOnly as isDaemonHostOnlyTargets,
		type Credential,
		type IntegrationTarget
	} from '$lib/features/credentials/types/base';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import {
		integrationTargetFor,
		isLockedOnly,
		junctionHostIds,
		junctionHostsOnSite
	} from '$lib/features/credentials/utils/credentialTargets';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { useDiscoveriesQuery, useUpdateDiscoveryMutation } from '$lib/features/discovery/queries';
	import {
		common_close,
		common_configure,
		common_install,
		common_integrations,
		common_next,
		daemons_createDaemon,
		daemons_credentialWizardReturn,
		daemons_credentialWizardReturnToInstall,
		daemons_credentialWizardTargetRequired,
		daemons_provisioningDaemon,
		daemons_emailInstallCommand,
		daemons_installCommandEmailed,
		daemons_installIveRunCommand,
		daemons_installIveStartedDocker,
		daemons_installCopyCommand,
		daemons_installCopiedToastLinux,
		daemons_installCopiedToastDocker,
		daemons_installCopiedToastMac,
		daemons_installCopiedToastWindows,
		daemons_installBackToInstall,
		daemons_installReturnToCommands,
		daemons_testingConnectionTo,
		daemons_verifyEmailForMoreDaemons,
		daemons_planRequiredForMoreDaemons,
		billing_lapsedBannerCta
	} from '$lib/paraglide/messages';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import EmailVerificationNotice from '$lib/features/auth/components/EmailVerificationNotice.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import { triggerUpgrade } from '$lib/features/billing/trigger-upgrade';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		onNavigate?: (tab: string) => void;
		name?: string;
		/** The site the wizard opens on, as when Home asks for a daemon on a site with none. */
		siteId?: string | null;
	}

	let { isOpen = false, onClose, onNavigate, name = undefined, siteId = null }: Props = $props();

	// Queries & mutations
	const configQuery = useConfigQuery();
	const organizationQuery = useOrganizationQuery();
	const provisionDaemonMutation = useProvisionDaemonMutation();

	// Derived data
	let isCloudDeployment = $derived(configQuery.data ? isCloud(configQuery.data) : false);
	let org = $derived(organizationQuery.data);
	let isFirstDaemon = $derived(!org?.daemon_connected);
	// Snapshot: tracks whether wizard was opened as first-daemon flow.
	// Prevents reactivity from flipping showWaitingUI when FirstDaemonRegistered appears.
	let startedAsFirstDaemon = $state(false);

	// Email install command
	let hasEmail = $derived(configQuery.data?.has_email_service ?? false);
	const emailInstallMutation = useEmailInstallCommandMutation();

	// Sites
	const sitesQuery = useSitesQuery();
	let sitesData = $derived(sitesQuery.data ?? []);

	// Site selection
	let selectedSiteId = $state('');
	let nameManuallyEdited = $state(false);

	// API key state
	let keyState = $state<string | null>(null);

	// Credentials is its own stepper step (activeTab === 'credentials'); the shared
	// CredentialsStep owns the credential list and persistence.
	let credentialsStep: ReturnType<typeof CredentialsStep> | undefined = $state();
	let pendingCredentials = $state<PendingCredential[]>([]);
	let credentialIds = $state<string[]>([]);
	// The socket defaults are seeded once per open, so a row the user removed stays removed
	// when they go back to Setup and forward again.
	let credentialsSeeded = $state(false);

	// The org's default credentials whose type targets only the daemon host (the Docker and
	// Podman sockets) are listed by default as existing rows, each targeting the daemon host (a
	// `<uuid>@127.0.0.1` token). Setup creates none of them.
	const credentialsQuery = useCredentialsQuery();
	async function defaultDaemonHostCredentials(): Promise<Credential[]> {
		const credentials = credentialsQuery.data ?? (await credentialsQuery.refetch()).data ?? [];
		return credentials.filter(
			(c) =>
				c.is_default &&
				isDaemonHostOnlyTargets(credentialTypes.getMetadata(c.credential_type.type)?.targets)
		);
	}
	// Hosts on the selected site each existing credential is already assigned to through the
	// host/credential junction, listed on its card as in the discovery modal.
	const assignedHostsQuery = useHostsByIds(() => junctionHostIds(credentialsQuery.data ?? []));
	let junctionHostsByCredential = $derived(
		junctionHostsOnSite(credentialsQuery.data ?? [], assignedHostsQuery.data ?? [], selectedSiteId)
	);
	// Rows are added after the hosts resolve, and the site can change, so this re-applies on
	// both. It writes only when a row's hosts differ, so it settles after one pass.
	$effect(() => {
		const byCredential = junctionHostsByCredential;
		const hostKey = (hosts?: { id: string }[]) => (hosts ?? []).map((h) => h.id).join(',');
		let changed = false;
		const next = pendingCredentials.map((p) => {
			if (!p.isExisting) return p;
			const assigned = byCredential.get(p.credential.id);
			if (hostKey(p.lockedHosts) === hostKey(assigned)) return p;
			changed = true;
			return { ...p, lockedHosts: assigned };
		});
		if (changed) pendingCredentials = next;
	});

	// Real new credentials being created. Drives the CTA count. Existing rows, the default
	// sockets among them, are not created and not counted; this matches the creation filter (see
	// getCredentialsForCreate / collectCredentialIds).
	let unsavedCredentialCount = $derived(
		pendingCredentials.filter((p) => !p.isExisting && !credentialIds.includes(p.credential.id))
			.length
	);

	// OS selection
	let selectedOS: DaemonOS = $state(detectOS());
	// Chosen on the Configure step: the integrations created next take its OS as their daemon OS,
	// and the Install step shows that OS's commands without offering a different one.
	function handleOsSelect(os: DaemonOS) {
		selectedOS = os;
		trackEvent('daemon_install_os_selected', { os });
	}
	let linuxMethod = $state<'binary' | 'docker'>('binary');
	let windowsMethod = $state<'exe' | 'msi'>('exe');
	let isDockerInstall = $derived(selectedOS === 'linux' && linuxMethod === 'docker');
	let installCtaLabel = $derived(
		isDockerInstall ? daemons_installIveStartedDocker() : daemons_installIveRunCommand()
	);
	let hasCopied = $state(false);

	// Reset hasCopied when the install command changes (Advanced settings, credentials, etc.)
	let prevInstallCommand = $state('');
	$effect(() => {
		const cmd = currentInstallCommand ?? '';
		if (hasCopied && prevInstallCommand && cmd !== prevInstallCommand) {
			hasCopied = false;
		}
		prevInstallCommand = cmd;
	});

	// ServerPoll reachability state
	let serverPollReachable = $state<boolean | null>(null);
	let isTestingReachability = $state(false);
	let serverPollReachabilityResult = $state<{ reachable: boolean; error?: string } | null>(null);
	const testReachabilityMutation = useTestReachabilityMutation();

	// Connection waiting state
	let provisionedDaemonId = $state('');
	let isProvisioning = $state(false);

	// The server refuses another daemon on Free, and while no owner of the org has verified their email. It
	// says so up front, so the wizard stops on its first step instead of after the user has
	// configured everything.
	const provisionEligibilityQuery = useProvisionEligibilityQuery({
		enabled: () => isOpen && !provisionedDaemonId
	});
	let provisionGateState = $derived(provisionGate(provisionEligibilityQuery.data));
	let provisionRefused = $derived(provisionGateState !== null);

	// Daemon identity (name + site) is frozen once Configure is left, so credentials — which
	// are created against the selected site — can't be orphaned by a later site change.
	let configureCommitted = $state(false);

	const discoveriesQuery = useDiscoveriesQuery();
	const updateDiscoveryMutation = useUpdateDiscoveryMutation();

	// Advanced settings committed for the install-command builder. Set at provision and on
	// leaving the Advanced panel — the builder is a pure read, so changing these re-fetches the
	// command without re-minting the key.
	let committedInstallParams = $state<InstallCommandParams | null>(null);

	// Install commands come from the pure builder (with an <API_KEY> placeholder); the minted key
	// is substituted in for display. Regenerating them never rotates the key.
	const installCommandQuery = useDaemonInstallCommandQuery(
		() => provisionedDaemonId || null,
		() => committedInstallParams ?? { purpose: 'install' },
		{ enabled: () => !!provisionedDaemonId && !!committedInstallParams }
	);
	let installArtifacts = $derived.by(() =>
		installCommandQuery.data && keyState
			? fillInstallArtifactsKey(installCommandQuery.data, keyState)
			: null
	);
	// The server's command for the chosen method, or null until it and the key both exist.
	let currentInstallCommand = $derived(
		installStepCommand(installCommandQuery.data, keyState, selectedOS, linuxMethod)
	);
	let connectionStatus = $state<DaemonConnectionStatus>('idle');
	let troubleTimeoutId = $state<ReturnType<typeof setTimeout> | null>(null);

	// Daemon-specific queries for connection detection
	const queryClient = useQueryClient();
	const provisionedDaemonQuery = useDaemonQuery(() => provisionedDaemonId || null, {
		enabled: () =>
			(connectionStatus === 'waiting' || connectionStatus === 'trouble') && !!provisionedDaemonId
	});
	function getDefaultDaemonName(siteId: string): string {
		const site = sitesData.find((n) => n.id === siteId);
		if (site) {
			const slug = slugifySiteName(site.name);
			if (slug) return `scanopy-daemon-${slug}`;
		}
		return 'scanopy-daemon';
	}

	// Auto-select first site when SelectSite is hidden (first daemon)
	$effect(() => {
		if (isFirstDaemon && !selectedSiteId && sitesData.length > 0) {
			selectedSiteId = sitesData[0].id;
		}
	});

	// Per-step funnel analytics: fire once each time a step becomes the active step
	let lastViewedStep = $state<string | null>(null);
	$effect(() => {
		if (!isOpen) {
			lastViewedStep = null;
			return;
		}
		if (activeTab !== lastViewedStep) {
			lastViewedStep = activeTab;
			trackEvent('daemon_wizard_step_viewed', { step: activeTab });
		}
	});

	$effect(() => {
		if (selectedSiteId && !nameManuallyEdited) {
			const defaultName = getDefaultDaemonName(selectedSiteId);
			untrack(() => form.setFieldValue('name', defaultName));
		}
	});

	// TanStack Form
	const form = createForm(() => ({
		defaultValues: buildDefaultValues(),
		onSubmit: async () => {
			// No-op; submission is handled by step navigation
		}
	}));

	// Reactive form values (form.state.values is NOT tracked by $derived)
	let formValues = $state<Record<string, string | number | boolean>>(buildDefaultValues());

	$effect(() => {
		const { unsubscribe } = form.store.subscribe(() => {
			formValues = { ...form.state.values } as Record<string, string | number | boolean>;
		});
		return unsubscribe;
	});

	// Derived commands.
	//
	// Integration targeting for this daemon, seeded onto its discovery row at provision
	// (`seed_credential_refs`) so the credential is probed on the first scan and assigned to a
	// host only once that probe succeeds. The scope is derived from the type's `targets`
	// metadata, not from the IP count alone — "no IPs" means site-wide for a broadcast-capable
	// type but "nothing chosen" for one that excludes Site, which yields no target at all.
	// The wizard validates the selection first, so a dropped target means an unusable one.
	let seedCredentialRefs = $derived(
		pendingCredentials.flatMap((p): IntegrationTarget[] => {
			// Only persisted credentials can be referenced: existing ones, and new ones once created.
			// A row listed only for its assigned hosts already runs on them; nothing to seed.
			if (!credentialIds.includes(p.credential.id) || isLockedOnly(p)) return [];
			const target = integrationTargetFor(
				p.credential.id,
				credentialTypes.getMetadata(p.credential.credential_type.type)?.targets,
				p.targetIps
			);
			return target ? [target] : [];
		})
	);
	// Persisted credentials whose current selection maps to no scope their type permits. A
	// `$derived` can't raise an error, so the write paths check this and refuse rather than
	// silently pushing a ref list with the credential missing.
	let untargetableCredentialNames = $derived(
		pendingCredentials
			.filter(
				(p) =>
					credentialIds.includes(p.credential.id) &&
					!isLockedOnly(p) &&
					integrationTargetFor(
						p.credential.id,
						credentialTypes.getMetadata(p.credential.credential_type.type)?.targets,
						p.targetIps
					) === null
			)
			.map((p) => p.credential.name)
	);
	// Check for form validation errors (only visible fields)
	let visibleFields = $derived(getVisibleFieldIds(formValues));
	let hasErrors = $derived.by(() => {
		const fieldMeta = form.state.fieldMeta;
		for (const fieldKey of Object.keys(fieldMeta)) {
			if (!visibleFields.has(fieldKey)) continue;
			const meta = fieldMeta[fieldKey];
			if (meta?.errors && meta.errors.length > 0) {
				return true;
			}
		}
		return false;
	});

	// --- Tab / step state ---
	// Steps: Configure -> Credentials (optional) -> Install
	let activeTab = $state('configure');
	let furthestReached = $state(0);
	let showAdvanced = $state(false);

	let tabs: ModalTab[] = $derived([
		{ id: 'configure', label: common_configure(), icon: Settings },
		{
			id: 'credentials',
			label: common_integrations(),
			icon: KeyRound,
			disabled: furthestReached < 1
		},
		{
			id: 'install',
			label: common_install(),
			icon: Terminal,
			disabled: furthestReached < 1
		}
	]);

	function handleTabChange(tabId: string) {
		showAdvanced = false;
		activeTab = tabId;
		// The Install step needs a provisioned daemon regardless of how it's reached — including
		// jumping straight here from the tab strip, skipping Credentials entirely. Kicked off
		// rather than awaited: GenericModal has already switched its tab strip, so blocking here
		// would leave the strip and the content out of step.
		if (tabId === 'install') void ensureProvisioned();
	}

	// --- Provisioning ---
	/**
	 * Provision the daemon record and its 1:1 key, seeding any credentials created in the
	 * Credentials step onto its discovery row. Runs on the way into the Install step — later
	 * than the daemon's identity is settled, because the credentials don't exist until then.
	 *
	 * Idempotent: once provisioned, further credential edits are pushed to the discovery row
	 * rather than re-provisioning, which would rotate the key out from under a command the user
	 * may already have copied.
	 */
	async function ensureProvisioned() {
		if (provisionedDaemonId) {
			await syncSeededCredentialRefs();
			return;
		}
		if (isProvisioning) return;

		const daemonName = (form.state.values['name'] as string) ?? 'daemon';
		const mode = (form.state.values['mode'] as DaemonMode) ?? 'daemon_poll';
		const daemonUrlBase = (form.state.values['daemonUrl'] as string) ?? '';
		const daemonPort = (() => {
			const port = form.state.values['daemonPort'];
			return typeof port === 'number' ? port : 60073;
		})();

		// Both modes provision a record bound 1:1 to a fresh key. ServerPoll also
		// captures the reachable URL the server dials; DaemonPoll dials out, so it
		// has none. Install commands are fetched separately from the builder.
		const isServerPoll = mode === 'server_poll';
		isProvisioning = true;
		try {
			const result = await provisionDaemonMutation.mutateAsync({
				name: daemonName,
				site_id: selectedSiteId,
				mode,
				url: isServerPoll ? constructDaemonUrl(daemonUrlBase, daemonPort) : null,
				seed_credential_refs: seedCredentialRefs,
				os: selectedOS
			});
			keyState = result.daemon_api_key;
			provisionedDaemonId = result.daemon.id;
			committedInstallParams = installCommandParams();
		} catch {
			// The API client reports the failure.
		} finally {
			isProvisioning = false;
		}
	}

	/**
	 * Push credential targeting onto an already-provisioned daemon's discovery row — for the user
	 * who reaches Install, goes back, and adds or removes a credential. Without this the seeded
	 * refs would silently keep the state they had at provision time.
	 */
	async function syncSeededCredentialRefs() {
		// Reachable without going through the wizard's validation — the Install tab can be
		// clicked directly after a credential's targets were edited. Refuse rather than write a
		// ref list that has quietly dropped it.
		if (untargetableCredentialNames.length > 0) {
			pushError(
				daemons_credentialWizardTargetRequired({
					credentials: untargetableCredentialNames.join(', ')
				})
			);
			return;
		}
		const refs = seedCredentialRefs;
		const discoveries = await discoveriesQuery.refetch();
		const discovery = (discoveries.data ?? []).find((d) => d.daemon_id === provisionedDaemonId);
		if (!discovery) return;

		const fingerprint = (targets: IntegrationTarget[]) =>
			JSON.stringify(
				targets
					.map((t) => `${t.scope}:${t.credential_id}:${t.scope === 'Hosts' ? t.ips.join('+') : ''}`)
					.sort()
			);
		if (fingerprint(discovery.integration_targets) === fingerprint(refs)) return;

		try {
			await updateDiscoveryMutation.mutateAsync({ ...discovery, integration_targets: refs });
		} catch {
			// The API client reports the failure. This is a background sync, so the
			// generic message loses the hint about checking discovery settings.
		}
	}

	/** The advanced settings the builder should fold into the install command. */
	function installCommandParams(values = formValues): InstallCommandParams {
		const cfg = buildInstallConfig(values);
		return {
			purpose: 'install',
			log_level: cfg.log_level ?? undefined,
			log_file: cfg.log_file ?? undefined,
			heartbeat_interval: cfg.heartbeat_interval ?? undefined,
			bind_address: cfg.bind_address ?? undefined,
			allow_self_signed_certs: cfg.allow_self_signed_certs ?? undefined,
			accept_invalid_scan_certs: cfg.accept_invalid_scan_certs ?? undefined,
			interfaces: cfg.interfaces?.length ? cfg.interfaces.join(',') : undefined
			// No credential_refs: the install command never carries targeting. That param is for
			// manual seeding only; UI installs get their targeting from the daemon's discovery row.
		};
	}

	/**
	 * Leave the Advanced panel, folding any changes into the emitted install command. The
	 * builder is a pure read, so this re-fetches the command — it never re-mints the key.
	 */
	function closeAdvanced() {
		showAdvanced = false;
		committedInstallParams = installCommandParams();
	}

	// --- Navigation handlers ---
	async function handleNext() {
		if (activeTab === 'configure') {
			// Provisioning would be refused; the notice above the form says why.
			if (provisionRefused) return;
			const fields = getVisibleFieldIds(formValues);
			const isValid = await validateForm(form, fields);

			if (!isValid) return;

			// ServerPoll: run reachability test from Next button (cloud only — self-hosted doesn't need port forwarding)
			if (formValues.mode === 'server_poll' && isCloudDeployment && serverPollReachable !== true) {
				const daemonUrlBase = String(formValues.daemonUrl ?? '');
				if (!daemonUrlBase) return;
				const port = Number(formValues.daemonPort) || 60073;
				const fullUrl = constructDaemonUrl(daemonUrlBase, port);
				isTestingReachability = true;
				try {
					const result = await testReachabilityMutation.mutateAsync({
						url: fullUrl,
						check_health: false
					});
					serverPollReachable = result.reachable;
					serverPollReachabilityResult = {
						reachable: result.reachable,
						error: result.error ?? undefined
					};
					if (!result.reachable) return; // stay on step, result shown inline
				} catch {
					serverPollReachable = false;
					serverPollReachabilityResult = {
						reachable: false,
						error: 'Failed to test reachability'
					};
					return;
				} finally {
					isTestingReachability = false;
				}
			}

			trackEvent('daemon_wizard_step_completed', { step: 'configure' });

			// Provisioning happens on the way into Install, not here: every daemon gets a
			// server-side record with a fresh 1:1 key, and the credentials created in the next
			// step are seeded onto its discovery row by that same call.
			configureCommitted = true;
			if (furthestReached < 1) furthestReached = 1;
			// Advance to the Credentials step (step 2). It's optional — the user can
			// Skip to Install (step 3), which is also unlocked.
			activeTab = 'credentials';
			if (!credentialsSeeded) {
				credentialsSeeded = true;
				await tick();
				await credentialsStep?.addExistingCredentials(await defaultDaemonHostCredentials());
			}
		}
	}

	// --- Connection waiting ---
	function startWaitingTimeout() {
		if (troubleTimeoutId) return; // already started
		troubleTimeoutId = setTimeout(() => {
			if (connectionStatus === 'waiting') {
				connectionStatus = 'trouble';
				daemonSetupState.set({ connectionStatus: 'trouble' });
				trackEvent('daemon_connection_timeout');
			}
		}, 45_000);
	}

	function getCopiedToastMessage(): string {
		if (selectedOS === 'linux' && linuxMethod === 'docker')
			return daemons_installCopiedToastDocker();
		if (selectedOS === 'macos') return daemons_installCopiedToastMac();
		if (selectedOS === 'windows') return daemons_installCopiedToastWindows();
		return daemons_installCopiedToastLinux();
	}

	async function handleCopyCommand() {
		if (!currentInstallCommand) return;
		try {
			await copyText(currentInstallCommand);
			hasCopied = true;
			trackEvent('daemon_install_command_copied', { os: selectedOS, context: 'footer-cta' });
			pushSuccess(getCopiedToastMessage());
		} catch (error) {
			pushWarning(common_failedToCopy({ error: String(error) }));
		}
	}

	function handleInstalled() {
		connectionStatus = 'waiting';
		daemonSetupState.set({ connectionStatus: 'waiting' });
		trackEvent('daemon_install_confirmed');

		// DaemonPoll: start timeout immediately. ServerPoll: wait for health check to pass.
		if (formValues.mode !== 'server_poll') {
			startWaitingTimeout();
		}
	}

	function handleReviewCommands() {
		connectionStatus = 'idle';
		// Don't update store — polling continues via org query
	}

	function handleViewDiscovery() {
		onNavigate?.('topology');
		handleOnClose();
	}

	function handleProgressComplete() {
		if (connectionStatus === 'waiting') {
			connectionStatus = 'trouble';
			daemonSetupState.set({ connectionStatus: 'trouble' });
			trackEvent('daemon_connection_timeout');
		}
	}

	function handleReviewCommandsFromTrouble() {
		connectionStatus = 'idle';
		trackEvent('daemon_trouble_review_commands');
	}

	function handleEnableSelfSigned() {
		form.setFieldValue('allowSelfSignedCerts', true);
		// Read from the form directly: `formValues` syncs through the store subscription.
		committedInstallParams = installCommandParams({
			...form.state.values,
			allowSelfSignedCerts: true
		} as Record<string, string | number | boolean>);
		connectionStatus = 'idle';
		trackEvent('daemon_trouble_enable_self_signed');
	}

	function markConnected() {
		connectionStatus = 'connected';
		daemonSetupState.set({ connectionStatus: 'connected' });
		// The connection poll refreshes only this daemon's own query. The Daemons table reads the list
		// pages, which still hold the daemon as never seen. Refetch rather than patch: the server
		// derives its status.
		queryClient.invalidateQueries({ queryKey: queryKeys.daemons.all });
		if (troubleTimeoutId) {
			clearTimeout(troubleTimeoutId);
			troubleTimeoutId = null;
		}
		confetti({ particleCount: 100, spread: 70, origin: { y: 0.6 } });
		trackEvent('daemon_connected');
	}

	// Poll the provisioned daemon every 5s when waiting/trouble
	$effect(() => {
		if ((connectionStatus === 'waiting' || connectionStatus === 'trouble') && provisionedDaemonId) {
			const interval = setInterval(() => {
				provisionedDaemonQuery.refetch();
			}, 5000);
			return () => clearInterval(interval);
		}
	});

	// Detect connection when the daemon first checks in. Every daemon is provisioned before the
	// Install step, and registration claims that record rather than creating a new one, so
	// `last_seen` becoming non-null is the arrival signal for both modes.
	$effect(() => {
		if (
			(connectionStatus === 'waiting' || connectionStatus === 'trouble') &&
			provisionedDaemonId &&
			provisionedDaemonQuery.data?.last_seen
		) {
			markConnected();
		}
	});

	// --- Close / Open ---
	function handleOnClose() {
		trackEvent('daemon_wizard_closed');

		daemonSetupState.set({ connectionStatus: 'idle' });

		if (troubleTimeoutId) {
			clearTimeout(troubleTimeoutId);
			troubleTimeoutId = null;
		}

		// The key and the daemon it is bound to go together: keeping the daemon id while dropping
		// the key left a reopened wizard skipping provisioning with no key to show.
		keyState = null;
		provisionedDaemonId = '';
		committedInstallParams = null;
		isProvisioning = false;
		configureCommitted = false;
		nameManuallyEdited = false;
		activeTab = 'configure';
		furthestReached = 0;
		showAdvanced = false;
		credentialsSeeded = false;
		pendingCredentials = [];
		credentialIds = [];
		connectionStatus = 'idle';
		serverPollReachable = null;
		isTestingReachability = false;
		serverPollReachabilityResult = null;

		// Reset form fields to defaults so advanced overrides don't persist
		const defaults = buildDefaultValues();
		for (const [key, value] of Object.entries(defaults)) {
			form.setFieldValue(key, value);
		}

		onClose();
	}

	function handleOpen() {
		trackEvent('daemon_wizard_opened');
		nameManuallyEdited = false;
		activeTab = 'configure';
		furthestReached = 0;
		showAdvanced = false;
		credentialsSeeded = false;
		connectionStatus = 'idle';
		startedAsFirstDaemon = isFirstDaemon;
		serverPollReachable = null;
		serverPollReachabilityResult = null;
		configureCommitted = false;
		hasCopied = false;
		if (siteId) selectedSiteId = siteId;
	}

	let colorHelper = entities.getColorHelper('Daemon');
	let title = daemons_createDaemon();

	async function handleCredentialsContinue() {
		const ids = await credentialsStep?.collectCredentialIds();
		if (ids === null || ids === undefined) return; // validation failed
		trackEvent('daemon_wizard_step_completed', {
			step: 'credentials',
			skipped: ids.length === 0,
			types_selected: new Set(pendingCredentials.map((p) => p.credential.credential_type.type))
				.size,
			credentials_attached: ids.length
		});
		// Provision after the credentials exist, so they're seeded onto the
		// daemon's discovery row in the same call.
		await ensureProvisioned();
		activeTab = 'install';
	}

	function handleEmailInstallCommand() {
		if (!committedInstallParams) return;
		// The server rebuilds the command from the same params the
		// on-screen command came from; it never takes rendered text.
		emailInstallMutation.mutate(
			{
				daemon_id: provisionedDaemonId,
				...committedInstallParams,
				os: selectedOS,
				docker: isDockerInstall,
				api_key: keyState
			},
			{
				onSuccess: () => pushSuccess(daemons_installCommandEmailed())
			}
		);
	}

	let actions = $derived.by<ModalAction[]>(() => {
		if (activeTab === 'credentials') {
			return [
				{
					role: 'primary',
					label:
						unsavedCredentialCount > 0
							? daemons_credentialWizardReturn({ count: unsavedCredentialCount })
							: daemons_credentialWizardReturnToInstall(),
					onClick: handleCredentialsContinue,
					disabled: credentialsStep?.busy || isProvisioning
				}
			];
		}
		if (showAdvanced) {
			return [
				{
					role: 'primary',
					label: daemons_installBackToInstall(),
					onClick: closeAdvanced,
					icon: ArrowLeft
				}
			];
		}
		if (activeTab === 'configure') {
			return [
				{
					role: 'primary',
					label: isTestingReachability
						? daemons_testingConnectionTo({
								url: String(formValues.daemonUrl ?? ''),
								port: String(formValues.daemonPort || 60073)
							})
						: common_next(),
					onClick: handleNext,
					disabled: isTestingReachability || provisionRefused,
					large: true,
					busy: isTestingReachability
				}
			];
		}
		if (activeTab !== 'install') return [];
		if (connectionStatus === 'connected') {
			return [{ role: 'primary', label: common_close(), onClick: handleOnClose }];
		}
		if (connectionStatus === 'waiting' || connectionStatus === 'trouble') {
			return [
				{
					role: 'secondary',
					label: daemons_installReturnToCommands(),
					onClick: handleReviewCommands
				},
				{ role: 'cancel', label: common_close(), onClick: handleOnClose }
			];
		}
		return [
			...(hasEmail && currentInstallCommand && committedInstallParams
				? [
						{
							role: 'secondary' as const,
							label: daemons_emailInstallCommand(),
							onClick: handleEmailInstallCommand,
							disabled: emailInstallMutation.isPending,
							icon: Mail
						}
					]
				: []),
			hasCopied
				? { role: 'primary', label: installCtaLabel, onClick: handleInstalled, icon: Check }
				: {
						role: 'primary',
						label: daemons_installCopyCommand(),
						onClick: handleCopyCommand,
						icon: Copy
					}
		];
	});
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	size="full"
	fixedHeight={true}
	onClose={handleOnClose}
	onOpen={handleOpen}
	{tabs}
	{activeTab}
	tabStyle="stepper"
	onTabChange={handleTabChange}
	{actions}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Daemon')} color={colorHelper.color} />
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col overflow-hidden">
		{#if showAdvanced}
			<div class="flex-1 overflow-auto p-4 sm:p-6">
				<AdvancedStep {form} {formValues} {selectedOS} {linuxMethod} />
			</div>
		{:else if activeTab === 'credentials'}
			<CredentialsStep
				bind:this={credentialsStep}
				siteId={selectedSiteId}
				bind:pendingCredentials
				bind:credentialIds
				daemonOs={selectedOS}
				daemonName={String(formValues.name ?? '')}
				assignedRowsRemovable
			/>
		{:else}
			<div class="flex-1 overflow-auto p-4 sm:p-6">
				{#key activeTab}
					{#if activeTab === 'configure'}
						{#if provisionGateState === 'unconnected' && provisionEligibilityQuery.data}
							<!-- The configure fields stay hidden: none of the three options uses them. -->
							<UnconnectedDaemonsNotice
								eligibility={provisionEligibilityQuery.data}
								onClose={() => onClose()}
							/>
						{:else}
							{#if provisionGateState === 'plan'}
								<div class="mb-4">
									<InlineWarning title="" body={daemons_planRequiredForMoreDaemons()}>
										<button
											type="button"
											class="text-sm text-link hover:underline"
											onclick={() =>
												triggerUpgrade({
													source: 'create_daemon_plan_required',
													surface: 'sidebar',
													gate_type: 'limit_hit',
													beforeModal: () => onClose()
												})}
										>
											{billing_lapsedBannerCta()}
										</button>
									</InlineWarning>
								</div>
							{:else if provisionGateState === 'verification'}
								<div class="mb-4">
									<EmailVerificationNotice body={daemons_verifyEmailForMoreDaemons()} />
								</div>
							{/if}
							<ConfigureStep
								{form}
								{selectedOS}
								onOsSelect={handleOsSelect}
								osLocked={credentialIds.length > 0}
								{formValues}
								{selectedSiteId}
								onSiteChange={(id) => (selectedSiteId = id)}
								onNameInput={() => (nameManuallyEdited = true)}
								identityLocked={configureCommitted}
								{isFirstDaemon}
								onReachabilityChange={(r) => {
									serverPollReachable = r;
									if (r === null) serverPollReachabilityResult = null;
								}}
								bind:reachabilityResult={serverPollReachabilityResult}
							/>
						{/if}
					{:else if activeTab === 'install' && currentInstallCommand}
						<InstallStep
							{selectedOS}
							onOsSelect={(os) => (selectedOS = os)}
							{linuxMethod}
							onLinuxMethodChange={(method) => (linuxMethod = method)}
							{windowsMethod}
							onWindowsMethodChange={(method) => (windowsMethod = method)}
							command={currentInstallCommand}
							{hasErrors}
							isFirstDaemon={startedAsFirstDaemon}
							{connectionStatus}
							onViewDiscovery={handleViewDiscovery}
							onAdvanced={() => (showAdvanced = true)}
							artifacts={installArtifacts}
							daemonMode={(formValues.mode as DaemonMode) ?? 'daemon_poll'}
							daemonName={String(formValues.name ?? 'scanopy-daemon')}
							logFilePath={String(formValues.logFile ?? '')}
							daemonUrl={constructDaemonUrl(
								String(formValues.daemonUrl ?? ''),
								Number(formValues.daemonPort) || 60073
							)}
							{provisionedDaemonId}
							onStartWaitingTimeout={startWaitingTimeout}
							onProgressComplete={handleProgressComplete}
							onReviewCommands={handleReviewCommandsFromTrouble}
							onEnableSelfSigned={handleEnableSelfSigned}
							onCopied={() => {
								hasCopied = true;
								pushSuccess(getCopiedToastMessage());
							}}
						/>
					{:else if activeTab === 'install'}
						<!-- Provisioning or the install-command fetch is in flight (reaching Install via
						     the tab strip doesn't await it). Show nothing until the server's command and
						     the key both exist. -->
						<div class="flex flex-1 items-center justify-center gap-3 p-6 text-muted">
							<Loader2 class="h-4 w-4 animate-spin" />
							{daemons_provisioningDaemon()}
						</div>
					{/if}
				{/key}
			</div>
		{/if}
	</div>
</GenericModal>
