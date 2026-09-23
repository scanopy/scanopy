<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm, validateForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { ModalTab } from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { entities, credentialTypes } from '$lib/shared/stores/metadata';
	import {
		DAEMON_HOST_IP,
		hasExplicitTarget,
		integrationTargetFor,
		supportsTarget
	} from '$lib/features/credentials/utils/credentialTargets';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import DiscoveryDetailsForm from './DiscoveryDetailsForm.svelte';
	import DiscoveryTargetsForm from './DiscoveryTargetsForm.svelte';
	import DiscoveryDetectionForm from './DiscoveryDetectionForm.svelte';
	import DiscoveryScanSettingsForm from './DiscoveryScanSettingsForm.svelte';
	import DiscoveryScheduleForm from './DiscoveryScheduleForm.svelte';
	import type { Discovery } from '../../types/base';
	import DiscoveryHistoricalSummary from './DiscoveryHistoricalSummary.svelte';
	import WarningReport from './WarningReport.svelte';
	import { uuidv4Sentinel } from '$lib/shared/utils/formatting';
	import { createEmptyDiscoveryFormData, parseDayTimeCronSchedule } from '../../queries';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import { pushError, pushSuccess, pushWarning } from '$lib/shared/stores/feedback';
	import { copyText } from '$lib/shared/utils/clipboard';
	import { formatDiagnostics } from '../../utils/diagnostics';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import { isPreUnifiedDaemon } from '$lib/features/daemons/utils';
	import type { Host } from '$lib/features/hosts/types/base';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import {
		Info,
		Crosshair,
		ScanSearch,
		Gauge,
		Calendar,
		ArrowRight,
		KeyRound,
		TriangleAlert,
		Copy
	} from 'lucide-svelte';
	import CredentialsStep, {
		type PendingCredential
	} from '$lib/features/credentials/components/CredentialsStep.svelte';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import {
		common_back,
		common_cancel,
		common_close,
		common_copied,
		common_credentials,
		common_delete,
		common_deleting,
		common_details,
		common_failedToCopy,
		common_issues,
		common_next,
		common_saving,
		common_schedule,
		common_detection,
		common_performance,
		common_targets,
		daemons_credentialWizardTargetRequired,
		discovery_copyDiagnostics,
		discovery_copyWarningData,
		discovery_couldNotGetNetworkId,
		discovery_createDiscovery,
		discovery_createScheduled,
		discovery_credentialsDescription,
		discovery_edit,
		discovery_noDaemonSelected,
		discovery_editActiveInfo,
		discovery_updateDiscovery,
		discovery_viewRun
	} from '$lib/paraglide/messages';

	interface Props {
		discovery?: Discovery | null;
		hasActiveSession?: boolean;
		isOpen?: boolean;
		daemons?: Daemon[];
		hosts?: Host[];
		onCreate: (data: Discovery) => Promise<void> | void;
		onUpdate: (id: string, data: Discovery) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
	}

	let {
		discovery = null,
		hasActiveSession = false,
		isOpen = false,
		daemons = [],
		hosts = [],
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined
	}: Props = $props();

	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	const subnetsQuery = useSubnetsQuery();
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let hasScheduledDiscovery = $derived.by(() => {
		if (!org?.plan?.type) return true;
		return billingPlans.getMetadata(org.plan.type).features.scheduled_discovery;
	});

	let loading = $state(false);
	let deleting = $state(false);
	let rawCronMode = $state(false);
	let activeTab = $state('details');
	let furthestReached = $state(0);
	let pendingCredentials = $state<PendingCredential[]>([]);
	let credentialsStep: ReturnType<typeof CredentialsStep> | undefined = $state();
	// The discovery modal always opens on the credential wizard; the Integrations-grid
	// type picker is skipped (it couldn't advance in edit mode, and the wizard's own
	// type dropdown adds any credential type, sockets included).
	let credentialSubStep = $state<'typeSelect' | 'wizard'>('wizard');
	let credentialIds = $state<string[]>([]);
	const allCredentialsQuery = useCredentialsQuery();

	// Mutable form data that sub-components can update
	let formData = $state<Discovery>(createEmptyDiscoveryFormData(null));

	let isEditing = $derived(discovery !== null);
	let isHistoricalRun = $derived(discovery?.run_type.type === 'Historical');
	let historicalResults = $derived(
		discovery?.run_type.type === 'Historical' ? discovery.run_type.results : null
	);
	/** A run that failed or was cancelled: the Issues tab has its reason to show. */
	let endedBadly = $derived(
		historicalResults?.phase === 'Failed' || historicalResults?.phase === 'Cancelled'
	);
	let historicalWarnings = $derived(
		discovery?.run_type.type === 'Historical' ? (discovery.run_type.results.warnings ?? []) : []
	);
	let readOnly = $derived(formData.run_type.type == 'Historical');

	/**
	 * Put the run's warnings on the clipboard as recorded, for pasting into an issue or a thread.
	 *
	 * The rendered report is the wrong thing to share: its sentences are this build's copy in the
	 * reader's locale, its rows merge occurrences the backend deliberately kept apart, and its
	 * chips show names resolved live rather than what the scan recorded. The payload is the
	 * durable artefact — every code with its own fields, one object per occurrence.
	 *
	 * `copyText` falls back to a selection copy on plain-HTTP self-hosts, the deployment most
	 * likely to be sharing warnings with us.
	 */
	async function copyWarningData() {
		await copyToClipboard(JSON.stringify(historicalWarnings, null, 2));
	}

	/**
	 * Put the run's identifiers, versions and timings on the clipboard as plain text, for a
	 * support thread about a run that failed or was cancelled.
	 */
	async function copyDiagnostics() {
		if (historicalResults) await copyToClipboard(formatDiagnostics(historicalResults));
	}

	async function copyToClipboard(text: string) {
		try {
			await copyText(text);
			pushSuccess(common_copied());
		} catch (error) {
			pushWarning(common_failedToCopy({ error: String(error) }));
		}
	}

	let title = $derived(
		isEditing
			? isHistoricalRun
				? discovery_viewRun({ name: discovery?.name ?? '' })
				: discovery_edit({ name: discovery?.name ?? '' })
			: discovery_createScheduled()
	);

	let daemon = $derived(daemons.find((d) => d.id === formData.daemon_id) || null);
	let daemonHostId = $derived(
		(daemon ? hosts.find((h) => h.id === daemon.host_id)?.id : null) || null
	);

	function ipIsLoopback(ip: string): boolean {
		const s = ip.trim();
		return s === '127.0.0.1' || s === '::1' || s.startsWith('127.');
	}

	// Credentials targeting the daemon's own host. Daemon-host targeting lives in the
	// host_credentials junction (managed via the host/credential modals), so union:
	// (1) DaemonHost-scope integration targets, (2) loopback Hosts-scope targets, and
	// (3) credentials assigned to the daemon host via the junction (host_assignments).
	function computeDaemonHostCredentials(dHostId: string | null) {
		if (!dHostId) return [];
		const credMap = new Map((allCredentialsQuery.data ?? []).map((c) => [c.id, c]));
		const fromTargets = (discovery?.integration_targets ?? [])
			.filter(
				(t) =>
					t.scope === 'DaemonHost' ||
					(t.scope === 'Hosts' && t.ips.length > 0 && t.ips.every(ipIsLoopback))
			)
			.map((t) => credMap.get(t.credential_id));
		const fromAssignments = (allCredentialsQuery.data ?? []).filter((c) =>
			(c.host_assignments ?? []).some((a) => a.host_id === dHostId)
		);
		const all = [...fromTargets, ...fromAssignments].filter((c): c is NonNullable<typeof c> => !!c);
		return all.filter((c, i) => all.findIndex((x) => x.id === c.id) === i);
	}
	let daemonHostCredentials = $derived(computeDaemonHostCredentials(daemonHostId));

	// Hosts the credentials are assigned to through the `host_credentials` junction, fetched
	// by id. The `hosts` prop is whatever the parent happened to load (in practice only the
	// daemons' own hosts), which is why assignments to any other host went unseen — and
	// pulling every host just to resolve a handful of ids would be wasteful.
	let assignedHostIds = $derived([
		...new Set(
			(allCredentialsQuery.data ?? []).flatMap((c) =>
				(c.host_assignments ?? []).map((a) => a.host_id)
			)
		)
	]);
	const assignedHostsQuery = useHostsByIds(() => assignedHostIds);

	/**
	 * Credential id → the hosts on THIS discovery's network it is already assigned to.
	 *
	 * The scan already runs all of these (the server builds host-level mappings for every host
	 * on the network, independent of this discovery's targets), so omitting them made the step
	 * an incomplete picture of what a scan will do. They surface on the credential's own card.
	 *
	 * Deliberately separate from `computeDaemonHostCredentials`: that one feeds the daemon-host
	 * endpoint blocking, and generalizing it in place would let another host's credential type
	 * falsely claim the daemon host.
	 */
	let junctionHostsByCredential = $derived.by(() => {
		const onNetwork = new Map(
			(assignedHostsQuery.data ?? [])
				.filter((h) => h.network_id === formData.network_id)
				.map((h) => [h.id, h])
		);
		return new Map(
			(allCredentialsQuery.data ?? []).flatMap((c) => {
				// The junction holds a row per host+IP-scope, so a host id can repeat.
				const assigned = [...new Set((c.host_assignments ?? []).map((a) => a.host_id))]
					.map((id) => onNetwork.get(id))
					.filter((h): h is Host => !!h);
				return assigned.length > 0 ? [[c.id, assigned] as const] : [];
			})
		);
	});

	// Identity of the map above, so the effect that applies it runs once per real change
	// rather than on every re-derivation (it both reads and writes `pendingCredentials`).
	let junctionFingerprint = $derived(
		[...junctionHostsByCredential]
			.map(([id, hs]) => `${id}:${hs.map((h) => h.id).join(',')}`)
			.sort()
			.join('|')
	);
	let appliedJunctionFingerprint = $state('');

	// Applied reactively, not in handleOpen: the by-id host query resolves after the modal
	// opens, and handleOpen runs once. Only ever attaches hosts and appends rows, so it
	// cannot clobber anything the user has entered.
	$effect(() => {
		if (!isOpen || !allCredentialsQuery.data) return;
		if (junctionFingerprint === appliedJunctionFingerprint) return;
		appliedJunctionFingerprint = junctionFingerprint;

		const byCredential = junctionHostsByCredential;
		const withLocked = pendingCredentials.map((p) => {
			const assigned = byCredential.get(p.credential.id);
			return assigned ? { ...p, lockedHosts: assigned } : p;
		});
		const credentialById = new Map(allCredentialsQuery.data.map((c) => [c.id, c]));
		const lockedOnly: PendingCredential[] = [...byCredential]
			.filter(([id]) => !withLocked.some((p) => p.credential.id === id))
			.flatMap(([id, assigned]) => {
				const credential = credentialById.get(id);
				return credential
					? [
							{
								credential,
								targetIps: [],
								fieldValues: {},
								isExisting: true,
								lockedHosts: assigned
							}
						]
					: [];
			});
		pendingCredentials = [...withLocked, ...lockedOnly];
	});

	// Claimed integrations (credential types) on the daemon host — feeds the shared
	// CredentialsStep's bidirectional socket↔proxy blocking. Generic across
	// integrations (Docker, Podman, …); no per-integration capability flag.
	let daemonHostCredentialTypeIds = $derived.by(() => {
		const types = daemonHostCredentials.map((c) => c.credential_type.type);
		return types.filter((t, i) => types.indexOf(t) === i);
	});

	// User-chosen configurable integrations (the fixed socket card is shown checked
	// via the step's read-only handling, not via this selection).
	let selectedCredentialTypeIds = $state<string[]>([]);

	let hasTargetsTab = $derived(
		formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified'
	);
	let hasDetectionTab = $derived(
		formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified'
	);
	let hasPerformanceTab = $derived(
		formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified'
	);
	let daemonSupportsUnified = $derived(!daemon || !isPreUnifiedDaemon(daemon));
	let hasCredentialsTab = $derived(formData.discovery_type.type === 'Unified');
	let hasScheduleTab = $derived(formData.run_type.type === 'Scheduled');

	/**
	 * A run has issues and it has details, and they are read for different reasons — "did this
	 * need me" before "what did it do". Two tabs rather than the warnings stapled to the top of
	 * the details, which is what made a run with fourteen of them unreadable.
	 *
	 * Issues holds everything that went wrong: the warnings, and for a run that failed or was
	 * cancelled, why it ended. The tab carries no status dot. Landing on it already says the run
	 * has something, and the count that says how many belongs on the row in scan history, where
	 * runs are compared.
	 */
	let tabs: ModalTab[] = $derived(
		isHistoricalRun
			? [
					{
						id: 'issues',
						label: common_issues(),
						icon: TriangleAlert
					},
					{ id: 'details', label: common_details(), icon: Info }
				]
			: [
					{ id: 'details', label: common_details(), icon: Info },
					...(hasTargetsTab
						? [
								{
									id: 'targets',
									label: common_targets(),
									icon: Crosshair,
									disabled: !isEditing && furthestReached < 1
								}
							]
						: []),
					...(hasCredentialsTab
						? [
								{
									id: 'credentials',
									label: common_credentials(),
									icon: KeyRound,
									disabled: !isEditing && furthestReached < 2
								}
							]
						: []),
					...(hasDetectionTab
						? [
								{
									id: 'detection',
									label: common_detection(),
									icon: ScanSearch,
									disabled:
										!isEditing && furthestReached < (hasCredentialsTab ? 3 : hasTargetsTab ? 2 : 1)
								}
							]
						: []),
					...(hasPerformanceTab
						? [
								{
									id: 'performance',
									label: common_performance(),
									icon: Gauge,
									disabled:
										!isEditing &&
										furthestReached <
											(hasCredentialsTab ? 4 : hasDetectionTab ? 3 : hasTargetsTab ? 2 : 1)
								}
							]
						: []),
					...(hasScheduleTab
						? [
								{
									id: 'schedule',
									label: common_schedule(),
									icon: Calendar,
									disabled:
										!isEditing &&
										furthestReached <
											(hasCredentialsTab
												? hasPerformanceTab
													? 5
													: hasDetectionTab
														? 4
														: 3
												: hasPerformanceTab
													? 4
													: hasDetectionTab
														? 3
														: hasTargetsTab
															? 2
															: 1)
								}
							]
						: [])
				]
	);

	// Auto-navigate away from tabs that no longer exist
	$effect(() => {
		if (activeTab === 'schedule' && !hasScheduleTab) {
			activeTab = 'details';
		}
		if (activeTab === 'targets' && !hasTargetsTab) {
			activeTab = 'details';
		}
		if (activeTab === 'detection' && !hasDetectionTab) {
			activeTab = hasTargetsTab ? 'targets' : 'details';
		}
		if (activeTab === 'performance' && !hasPerformanceTab) {
			activeTab = hasDetectionTab ? 'detection' : hasTargetsTab ? 'targets' : 'details';
		}
		if (activeTab === 'credentials' && !hasCredentialsTab) {
			activeTab = 'details';
		}
	});

	function getFlow() {
		return [
			'details',
			...(hasTargetsTab ? ['targets'] : []),
			...(hasCredentialsTab ? ['credentials'] : []),
			...(hasDetectionTab ? ['detection'] : []),
			...(hasPerformanceTab ? ['performance'] : []),
			...(hasScheduleTab ? ['schedule'] : [])
		];
	}

	function nextTab() {
		const flow = getFlow();
		const idx = flow.indexOf(activeTab);
		if (idx >= 0 && idx < flow.length - 1) {
			activeTab = flow[idx + 1];
		}
	}

	function previousTab() {
		const flow = getFlow();
		const idx = flow.indexOf(activeTab);
		if (idx > 0) {
			activeTab = flow[idx - 1];
		}
	}

	async function handleNext() {
		if (activeTab === 'details') {
			const isValid = await validateForm(form);
			if (isValid) {
				if (furthestReached < 1) furthestReached = 1;
				nextTab();
			}
		} else if (activeTab === 'targets') {
			if (furthestReached < 2) furthestReached = 2;
			nextTab();
		} else if (activeTab === 'credentials') {
			// Credentials has a sub-flow: the Integrations grid → the wizard. Advance
			// within it before moving on to the next tab.
			if (credentialSubStep === 'typeSelect') {
				await credentialsStep?.continueToWizard();
				return;
			}
			if (furthestReached < 3) furthestReached = 3;
			nextTab();
		} else if (activeTab === 'detection') {
			const level = hasCredentialsTab ? 4 : 3;
			if (furthestReached < level) furthestReached = level;
			nextTab();
		} else if (activeTab === 'performance') {
			const level = hasCredentialsTab ? 5 : 4;
			if (furthestReached < level) furthestReached = level;
			nextTab();
		}
	}

	let isLastTab = $derived.by(() => {
		const flow = getFlow();
		return activeTab === flow[flow.length - 1];
	});

	let isFirstTab = $derived(activeTab === 'details');

	function getDefaultFormData(): Discovery {
		const defaultDaemon = daemons.length > 0 ? daemons[0] : null;
		if (discovery) {
			return { ...discovery };
		}
		const empty = createEmptyDiscoveryFormData(defaultDaemon);
		if (defaultDaemon) {
			empty.daemon_id = defaultDaemon.id;
			empty.network_id = defaultDaemon.network_id;
		}
		// Default to AdHoc for plans without scheduled discovery (e.g. Free)
		if (!hasScheduledDiscovery) {
			empty.run_type = { type: 'AdHoc', last_run: null };
		}
		return empty;
	}

	// TanStack Form for validation (only fields that need validation)
	// NOTE: defaultValues must NOT read from $state to avoid reactivity loops
	const form = createForm(() => ({
		defaultValues: {
			name: '',
			run_type_type: (hasScheduledDiscovery ? 'Scheduled' : 'AdHoc') as 'AdHoc' | 'Scheduled',
			discovery_type_type: 'Unified' as 'Network' | 'Docker' | 'SelfReport' | 'Unified',
			host_naming_fallback: 'BestService' as 'BestService' | 'Ip',
			schedule_days_of_week: '0',
			schedule_time: '00:00',
			schedule_timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
			schedule_cron: '0 0 0 * * 0'
		},
		onSubmit: async ({ value }) => {
			// Update formData with form values
			formData.name = value.name.trim();

			if (daemon) {
				loading = true;
				// Persist credentials via the shared step (validate + create/update).
				const ids = await credentialsStep?.collectCredentialIds();
				if (ids === null) {
					loading = false;
					return; // validation failed — stay on the form
				}
				try {
					// Per-credential targeting comes from the wizard and is delivered as per-daemon
					// integration targets on the Discovery. Parity with the daemon-create modal: the
					// scope comes from the type's `targets` metadata, so a type that excludes Network
					// (e.g. a UniFi controller) can never be emitted as a network-wide broadcast the
					// server would drop at dispatch.
					const persisted = new Set(ids ?? []);
					// Only rows the user actually pointed somewhere: a credential listed purely
					// because it is assigned elsewhere has no selection here, and an empty IP list
					// would otherwise read as "network-wide" for a broadcast-capable type —
					// inventing a scope nobody asked for.
					const targeted = pendingCredentials
						.filter(
							(p) => hasExplicitTarget(p.scope, p.targetIps) && persisted.has(p.credential.id)
						)
						.map((p) => ({
							name: p.credential.name,
							target: integrationTargetFor(
								p.credential.id,
								credentialTypes.getMetadata(p.credential.credential_type.type)?.targets,
								p.targetIps
							)
						}));
					// The wizard validates targets first, so a credential with no permitted scope
					// should be unreachable. Fail closed rather than saving a discovery that quietly
					// drops it — a credential targeted nowhere never runs, with nothing to show why.
					const untargetable = targeted.filter((t) => t.target === null).map((t) => t.name);
					if (untargetable.length > 0) {
						pushError(
							daemons_credentialWizardTargetRequired({ credentials: untargetable.join(', ') })
						);
						loading = false;
						return;
					}
					formData.integration_targets = targeted.flatMap((t) => (t.target ? [t.target] : []));
					if (isEditing && discovery) {
						await onUpdate(discovery.id, formData);
					} else {
						await onCreate(formData);
					}
					onClose();
				} catch {
					// The API client reports the failure.
				} finally {
					loading = false;
				}
			} else {
				pushError(discovery_couldNotGetNetworkId());
			}
		}
	}));

	function handleOpen() {
		// A run with something wrong opens on it: its warnings, or the reason it did not finish.
		// A clean one opens on its details. The Issues tab exists either way, so the modal does
		// not change shape between runs.
		activeTab = historicalWarnings.length > 0 || endedBadly ? 'issues' : 'details';
		appliedJunctionFingerprint = '';
		furthestReached = discovery ? Infinity : 0;
		formData = getDefaultFormData();
		pendingCredentials = [];
		credentialIds = [];
		if (allCredentialsQuery.data) {
			const credMap = new Map(allCredentialsQuery.data.map((c) => [c.id, c]));
			// Editable: every credentialed target that lives on THIS discovery, daemon-host ones
			// included. They were previously excluded as "junction-managed", but nothing writes a
			// junction row for them — a daemon-host target created here lives only on the
			// discovery, so rendering it read-only left it impossible to remove or retarget and
			// dropped it from the record on the next save. A daemon-host target round-trips as a
			// loopback IP row, which `integrationTargetFor` maps back to the DaemonHost scope.
			const editable: PendingCredential[] = (discovery?.integration_targets ?? []).flatMap((t) => {
				const c = credMap.get(t.credential_id);
				if (!c) return [];
				// Drop a target whose scope the credential's type doesn't permit — rows written
				// before the scope was gated. Such a target is already discarded at dispatch, and
				// seeding it would surface as a row with no valid selection that blocks the save.
				// Omitting it here lets the submit mapping rewrite the record without it.
				if (!supportsTarget(credentialTypes.getMetadata(c.credential_type.type)?.targets, t.scope))
					return [];
				// A daemon-host target is shown as the loopback row the picker already renders (a
				// disabled "daemon host" label with a remove button), so it reads back as the same
				// scope it was saved with. Network scope carries no IPs; an empty row is the
				// picker's "nothing chosen" placeholder.
				const ips = t.scope === 'Hosts' ? t.ips : t.scope === 'DaemonHost' ? [DAEMON_HOST_IP] : [];
				return [
					{
						credential: c,
						targetIps: ips.length ? ips : [''],
						fieldValues: {},
						isExisting: true,
						// Record the scope this target was saved with, so a Network-scope row
						// round-trips as broadcast rather than reading back as "nothing chosen".
						scope: t.scope === 'Network' ? ('broadcast' as const) : ('per_host' as const)
					}
				];
			});
			pendingCredentials = editable;
		}
		// Always open straight on the wizard (the Integrations-grid picker is skipped).
		credentialSubStep = 'wizard';

		// Parse schedule fields from cron
		let scheduleDaysOfWeek = '0';
		let scheduleTime = '00:00';
		let scheduleCron = '0 0 0 * * 0';
		let scheduleTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;

		if (formData.run_type.type === 'Scheduled') {
			scheduleCron = formData.run_type.cron_schedule;
			scheduleTimezone = formData.run_type.timezone || scheduleTimezone;

			// Sync computed timezone back to formData so submit sends the correct value
			// even if the user never touches the timezone dropdown
			formData.run_type = { ...formData.run_type, timezone: scheduleTimezone };

			const parsed = parseDayTimeCronSchedule(formData.run_type.cron_schedule);
			if (parsed) {
				scheduleDaysOfWeek = parsed.daysOfWeek.join(',');
				scheduleTime = `${String(parsed.hour).padStart(2, '0')}:${String(parsed.minute).padStart(2, '0')}`;
				rawCronMode = false;
			} else {
				// Unmappable cron — open in raw cron mode
				rawCronMode = true;
			}
		}

		// Compute host naming fallback
		const hostNamingFallback =
			formData.discovery_type.type === 'Network' ||
			formData.discovery_type.type === 'Docker' ||
			formData.discovery_type.type === 'Unified'
				? formData.discovery_type.host_naming_fallback
				: 'BestService';

		form.reset({
			name: formData.name,
			// This form only edits user-owned config. Server-managed rows —
			// Historical runs and Rescans — never open here, so coerce both
			// fields to the editable options rather than widening the form type.
			run_type_type: formData.run_type.type === 'Scheduled' ? 'Scheduled' : 'AdHoc',
			discovery_type_type:
				formData.discovery_type.type === 'Rescan' ? 'Unified' : formData.discovery_type.type,
			host_naming_fallback: hostNamingFallback,
			schedule_days_of_week: scheduleDaysOfWeek,
			schedule_time: scheduleTime,
			schedule_timezone: scheduleTimezone,
			schedule_cron: scheduleCron
		});
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && discovery) {
			deleting = true;
			try {
				await onDelete(discovery.id);
				onClose();
			} catch {
				// The API client reports the failure.
			} finally {
				deleting = false;
			}
		}
	}

	// Set default daemon when available and formData has sentinel
	$effect(() => {
		if (formData.daemon_id === uuidv4Sentinel && daemons.length > 0) {
			formData.daemon_id = daemons[0].id;
			formData.network_id = daemons[0].network_id;
		}
	});

	let saveLabel = $derived(isEditing ? discovery_updateDiscovery() : discovery_createDiscovery());
	let showSave = $derived(!isHistoricalRun);

	let colorHelper = entities.getColorHelper('Discovery');
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={discovery?.id}
	{onClose}
	onOpen={handleOpen}
	size="full"
	fixedHeight={true}
	showCloseButton={true}
	{tabs}
	bind:activeTab
	tabStyle={isEditing || isHistoricalRun ? 'tabs' : 'stepper'}
	onTabChange={(id) => (activeTab = id)}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Discovery')} color={colorHelper.color} />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			if (showSave) handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div
			class="min-h-0 flex-1"
			class:overflow-y-auto={activeTab !== 'credentials'}
			class:flex={activeTab === 'credentials'}
			class:flex-col={activeTab === 'credentials'}
		>
			{#if isHistoricalRun && discovery?.run_type.type === 'Historical'}
				<div class="space-y-8 p-6">
					{#if activeTab === 'issues'}
						<WarningReport payload={discovery.run_type.results} />
					{:else}
						<DiscoveryHistoricalSummary payload={discovery.run_type.results} />
					{/if}
				</div>
			{:else if activeTab === 'details'}
				<div class="space-y-8 p-6">
					{#if hasActiveSession && isEditing}
						<InlineInfo
							title=""
							body={discovery_editActiveInfo()}
							dismissableKey="discovery-edit-active-session"
						/>
					{/if}
					<DiscoveryDetailsForm
						{form}
						{daemons}
						{hosts}
						subnets={subnetsData}
						bind:formData
						{readOnly}
						{hasScheduledDiscovery}
						{daemon}
					/>
				</div>
			{:else if activeTab === 'targets'}
				<div class="space-y-8 p-6">
					{#if daemon}
						<DiscoveryTargetsForm bind:formData {daemonHostId} {daemon} />
					{:else}
						<InlineWarning body={discovery_noDaemonSelected()} />
					{/if}
				</div>
			{:else if activeTab === 'detection'}
				<div class="space-y-8 p-6">
					<DiscoveryDetectionForm bind:formData {readOnly} {isEditing} />
				</div>
			{:else if activeTab === 'performance'}
				<div class="space-y-8 p-6">
					<DiscoveryScanSettingsForm bind:formData {daemon} {readOnly} />
				</div>
			{:else if activeTab === 'schedule'}
				<div class="space-y-8 p-6">
					<DiscoveryScheduleForm
						{form}
						bind:formData
						{readOnly}
						bind:rawCronMode
						schedulePaused={!hasScheduledDiscovery}
					/>
				</div>
			{/if}
			{#if hasCredentialsTab}
				<div class="flex min-h-0 flex-1 flex-col" class:hidden={activeTab !== 'credentials'}>
					<CredentialsStep
						bind:this={credentialsStep}
						networkId={formData.network_id}
						description={discovery_credentialsDescription()}
						bind:pendingCredentials
						bind:credentialIds
						bind:subStep={credentialSubStep}
						bind:selectedTypeIds={selectedCredentialTypeIds}
						localAutoMode="fixed"
						fixedCapabilityTypeIds={daemonHostCredentialTypeIds}
						daemonVersion={daemon?.version ?? null}
						daemonName={daemon?.name ?? null}
					/>
				</div>
			{/if}
		</div>

		{#if isEditing}
			<EntityMetadataSection entities={[discovery]} />
		{/if}

		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if isEditing && !isHistoricalRun && onDelete}
						<button
							type="button"
							disabled={deleting || loading}
							onclick={handleDelete}
							class="btn-danger"
						>
							{deleting ? common_deleting() : common_delete()}
						</button>
					{/if}
				</div>
				<div class="flex items-center gap-3">
					<!-- Beside Close rather than above the report: it acts on the whole run, not on
					     any one row, and the footer is where a modal's whole-record actions live. -->
					{#if activeTab === 'issues' && historicalWarnings.length > 0}
						<button
							type="button"
							class="btn-secondary flex items-center gap-1"
							onclick={copyWarningData}
						>
							<Copy class="h-4 w-4" />
							<span>{discovery_copyWarningData()}</span>
						</button>
					{/if}
					{#if activeTab === 'issues' && endedBadly}
						<button
							type="button"
							class="btn-secondary flex items-center gap-1"
							onclick={copyDiagnostics}
						>
							<Copy class="h-4 w-4" />
							<span>{discovery_copyDiagnostics()}</span>
						</button>
					{/if}
					{#if isEditing || isHistoricalRun}
						<button
							type="button"
							disabled={loading || deleting}
							onclick={onClose}
							class="btn-secondary"
						>
							{isHistoricalRun ? common_close() : common_cancel()}
						</button>
						{#if showSave}
							<button type="submit" disabled={loading || deleting} class="btn-primary">
								{loading ? common_saving() : saveLabel}
							</button>
						{/if}
					{:else}
						{#if !isFirstTab}
							<button type="button" class="btn-secondary" onclick={previousTab}>
								{common_back()}
							</button>
						{:else}
							<button type="button" onclick={onClose} class="btn-secondary">
								{common_cancel()}
							</button>
						{/if}
						{#if isLastTab}
							<button
								type="submit"
								disabled={loading || (!isEditing && !daemonSupportsUnified)}
								class="btn-primary"
							>
								{loading ? common_saving() : saveLabel}
							</button>
						{:else}
							<button
								type="button"
								class="btn-primary btn-primary-lg"
								onclick={handleNext}
								disabled={!isEditing && !daemonSupportsUnified}
							>
								{common_next()}
								<ArrowRight class="h-4 w-4" />
							</button>
						{/if}
					{/if}
				</div>
			</div>
		</div>
	</form>
</GenericModal>
