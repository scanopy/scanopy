import type { EntityDiscriminants } from '#lib/api/entities.ts';
import { hostDisplayName } from '#lib/features/hosts/host-display-name.ts';
import type { Daemon } from '#lib/features/daemons/types/base.ts';
import {
	buildWarningReport,
	credentialIdsOf,
	hostIdsOf,
	integrationWithoutCredentialSentence,
	integrationWithoutCredentialTitle,
	remedyOf
} from '#lib/features/discovery/utils/warnings.ts';
import { actionFor } from '#lib/features/discovery/utils/warningActions.ts';
import type { CredentialLookup } from '#lib/features/discovery/utils/snmpCheck.ts';
import {
	createIntegrationCredentialLabel,
	openCredentialCreate
} from '#lib/features/credentials/utils/prefill.ts';
import {
	activeView,
	showDependencyTutorial,
	showViewSwitcherHint
} from '#lib/features/topology/queries.ts';
import { navigateToEntity, openModal } from '#lib/shared/stores/modal-registry.ts';
import { openTab } from '#lib/shared/stores/url-state.ts';
import {
	daemonHealths,
	discoveryTerminalReasons,
	scanFailureFixes,
	setupItems
} from '#lib/shared/stores/metadata.ts';
import {
	common_answer,
	common_getStarted,
	common_openScanThatFoundIt,
	common_troubleshoot,
	common_viewTopology,
	credentials_createCredential,
	daemons_upgradeDaemon,
	dependencies_createDependency,
	gettingStarted_stepDaemonLabel,
	gettingStarted_stepDiscoveryLabel,
	home_failedScans_layoutGuide,
	home_failedScans_nextRunFull,
	home_nudges_apiKeysAction,
	home_nudges_dependenciesAction,
	home_nudges_exploreViewsAction,
	home_nudges_inviteTeamAction,
	home_nudges_multiSiteAction,
	home_nudges_shareAction,
	home_nudges_tagHostsAction,
	home_nudges_targetlessCredentialAction,
	home_nudges_targetlessCredentialDescription,
	home_nudges_unconfirmedHostsDescription,
	home_nudges_unconfirmedReviewHosts,
	home_nudges_unconfirmedReviewSubnets,
	home_nudges_unconfirmedSubnetsDescription,
	home_setup_installDaemonOnSite,
	home_setup_openDaemon,
	home_setup_runScan,
	home_setup_troubleshootConnection
} from '#lib/paraglide/messages.js';
import type { SetupItem, SetupItemType } from './setup-list';

/** A row's fix: what its button does, and its label. */
export interface SetupFix {
	label: string;
	run: () => void;
}

/** The loaded entities a row names or opens. */
export interface SetupLookups {
	daemon: (id: string) => Daemon | undefined;
	host: (
		id: string
	) => (Record<string, unknown> & Parameters<typeof hostDisplayName>[0]) | undefined;
	site: (id: string) => Record<string, unknown> | undefined;
	credential: CredentialLookup;
	scan: (id: string) => object | undefined;
}

/**
 * One chip beside a row's title: an entity the item is about, which the title doesn't name. A
 * chip names exactly one entity, so an item about several has one chip each. Its own tooltip
 * and click open that entity.
 */
export interface SetupSubject {
	type: EntityDiscriminants;
	id: string;
	label: string;
	data: object | undefined;
}

/** What a Setup list row says and does. */
export interface SetupRowView {
	title: string;
	sentence: string;
	subjects: SetupSubject[];
	fix: SetupFix;
	/** A docs section for a fix the button can't make, offered beside it. */
	guide?: SetupFix;
}

/** What a row reads and calls besides its item. */
export interface SetupRowContext {
	/** The org's done Setup item types. */
	setup: SetupItemType[];
	lookups: SetupLookups;
	/** Opens the question a question item asks. */
	onAnswer: (itemType: 'AnswerReferralSource' | 'AnswerProfile') => void;
	/** Opens the daemon troubleshooting help. */
	onTroubleshoot: () => void;
	/** Starts a run of each of these scans. */
	onRetryScans: (scanIds: string[]) => void;
}

function go(tab: string, modal?: string, opts?: Parameters<typeof openModal>[1]) {
	window.location.hash = tab;
	if (modal) openModal(modal, opts);
}

/** A chip for each of `ids` that resolves. The row truncates them for display. */
function allOf(
	type: EntityDiscriminants,
	ids: string[],
	resolve: (id: string) => { label: string; data: object | undefined } | undefined
): SetupSubject[] {
	return ids.flatMap((id) => {
		const resolved = resolve(id);
		return resolved ? [{ type, id, ...resolved }] : [];
	});
}

/**
 * Opens a list filtered to the Inferred source on one site, which is exactly that site's
 * unconfirmed rows.
 */
function reviewInferred(tab: string, siteId: string) {
	return () => openTab(tab, new URLSearchParams({ sources: 'Inferred', site_ids: siteId }));
}

/** Opens the scans list filtered to one daemon's scans. */
function daemonScans(daemonId: string) {
	return () => openTab('discovery-scans', new URLSearchParams({ daemon_ids: daemonId }));
}

/** What the row for `item` says and does, by item type. */
export function setupRowView(item: SetupItem, ctx: SetupRowContext): SetupRowView {
	const { setup, lookups, onAnswer, onTroubleshoot, onRetryScans } = ctx;
	const hostSubjects = (ids: string[]) =>
		allOf('Host', ids, (id) => {
			const host = lookups.host(id);
			return host ? { label: hostDisplayName(host), data: host } : undefined;
		});
	const credentialSubjects = (ids: string[]) =>
		allOf('Credential', ids, (id) => {
			const credential = lookups.credential(id);
			return credential ? { label: credential.name, data: credential } : undefined;
		});

	const title = setupItems.getName(item.type);
	const sentence = setupItems.getDescription(item.type);
	const plain = (fix: SetupFix) => ({ title, sentence, subjects: [], fix });
	switch (item.type) {
		case 'DaemonHealth': {
			const { daemon_id: id, name, health } = item;
			// A daemon that isn't talking to the server gets the connection help; one on an
			// old release, or one that never reported its version, gets the upgrade; one on
			// standby has had no scan in 30 days, and starting one of its scans wakes it, so it
			// gets its own scans, each with Run.
			const fix: SetupFix =
				health === 'awaiting_connection' || health === 'unreachable' || health === 'not_checking_in'
					? { label: home_setup_troubleshootConnection(), run: onTroubleshoot }
					: health === 'standby'
						? { label: home_setup_runScan(), run: daemonScans(id) }
						: health === 'outdated' ||
							  health === 'deprecated' ||
							  health === 'unsupported' ||
							  health === 'unknown'
							? {
									label: daemons_upgradeDaemon(),
									run: () => go('daemons', 'upgrade-daemon', { id })
								}
							: {
									label: home_setup_openDaemon(),
									run: () => navigateToEntity('Daemon', id, lookups.daemon(id))
								};
			return {
				title,
				sentence: daemonHealths.getDescription(health),
				subjects: [{ type: 'Daemon', id, label: name, data: lookups.daemon(id) }],
				fix
			};
		}
		case 'UncoveredIntegration': {
			const { integration, host_ids, site_ids } = item;
			return {
				title: integrationWithoutCredentialTitle(integration),
				sentence: integrationWithoutCredentialSentence(integration, host_ids.length),
				subjects: hostSubjects(host_ids),
				fix: {
					label: createIntegrationCredentialLabel(integration),
					run: () => openCredentialCreate(integration, { hostIds: host_ids, siteIds: site_ids })
				}
			};
		}
		case 'TargetlessCredential': {
			// The title stays generic; the chip names the credential, once.
			const { credential_id: id, name } = item;
			return {
				title,
				sentence: home_nudges_targetlessCredentialDescription(),
				subjects: [{ type: 'Credential', id, label: name, data: lookups.credential(id) }],
				fix: {
					label: home_nudges_targetlessCredentialAction(),
					run: () => go('credentials', 'credential-editor', { id, tab: 'assignments' })
				}
			};
		}
		case 'SiteWithoutDaemon': {
			const { site_id: id, name } = item;
			return {
				title,
				sentence,
				subjects: [{ type: 'Site', id, label: name, data: lookups.site(id) }],
				fix: {
					label: home_setup_installDaemonOnSite(),
					run: () => go('daemons', 'create-daemon', { prefill: { sites: id } })
				}
			};
		}
		case 'RunWarnings': {
			const report = buildWarningReport(
				item.warnings,
				(type, id) => (type === 'Credential' ? lookups.credential(id)?.name : undefined),
				{ name: item.daemon_name, os: lookups.daemon(item.daemon_id)?.os },
				lookups.credential
			)[0]?.entries[0];
			const action = report
				? actionFor(report, {
						daemonId: item.daemon_id,
						siteId: item.site_id,
						scanId: item.scan_id ?? null,
						credentialOf: lookups.credential
					})
				: null;
			const discoveryId = item.discovery_id;
			// The credentials and hosts the warning is about. A scan-level warning names
			// neither, and then the chip is the daemon whose scan it was.
			const named = report
				? [...credentialSubjects(credentialIdsOf(report)), ...hostSubjects(hostIdsOf(report))]
				: [];
			return {
				title: report?.title ?? title,
				sentence: report?.details[0]?.sentence ?? report?.devices[0]?.sentence ?? sentence,
				subjects:
					named.length > 0
						? named
						: [
								{
									type: 'Daemon',
									id: item.daemon_id,
									label: item.daemon_name,
									data: lookups.daemon(item.daemon_id)
								}
							],
				// Without a fix in Scanopy, the run's report is the destination: for a
				// device-side problem it holds the commands to check the device with.
				fix: action
					? { label: action.label, run: action.run }
					: {
							label:
								remedyOf(item.code) === 'CheckTheDevice'
									? common_troubleshoot()
									: common_openScanThatFoundIt(),
							run: () => go('discovery-history', 'discovery-history-detail', { id: discoveryId })
						}
			};
		}
		case 'FailedScans': {
			const { daemon_id: daemonId, daemon_name, reason, scans } = item;
			// Newest run first: the run to open, and the scan whose schedule to move.
			const newest = scans[0];
			const fixType = discoveryTerminalReasons.getMetadata(reason).issue_fix ?? 'OpenRun';
			const run: () => void = (() => {
				switch (fixType) {
					case 'RetryScan':
						return () => onRetryScans(scans.map((s) => s.scan_id));
					case 'TroubleshootConnection':
						return onTroubleshoot;
					case 'RescheduleScan':
						return () =>
							go('discovery-scans', 'discovery-editor', { id: newest?.scan_id, tab: 'schedule' });
					case 'RaiseMaxDuration':
						return () =>
							go('discovery-scans', 'discovery-editor', {
								id: newest?.scan_id,
								tab: 'performance'
							});
					case 'OpenRun':
						return () =>
							go('discovery-history', 'discovery-history-detail', { id: newest?.run_id });
				}
			})();
			// What the fix's docs section teaches: splitting a scan that runs too long across daemons.
			const guideLabel = ((): string | undefined => {
				switch (fixType) {
					case 'RaiseMaxDuration':
						return home_failedScans_layoutGuide();
					case 'TroubleshootConnection':
					case 'RetryScan':
					case 'RescheduleScan':
					case 'OpenRun':
						return undefined;
				}
			})();
			const guideUrl = scanFailureFixes.getMetadata(fixType).docs_url;
			return {
				title,
				// What ended the run, the newest run's error, that it drew no links, and whether a full
				// scan runs full again.
				sentence: [
					discoveryTerminalReasons.getDescription(reason),
					// The daemon's text, which often ends without a full stop.
					...(newest?.error
						? [/[.!?]$/.test(newest.error) ? newest.error : `${newest.error}.`]
						: []),
					sentence,
					...(scans.some((s) => s.next_run_full) ? [home_failedScans_nextRunFull()] : [])
				].join(' '),
				subjects: [
					...scans.map((s) => ({
						type: 'Discovery' as const,
						id: s.scan_id,
						label: s.name,
						data: lookups.scan(s.scan_id)
					})),
					{ type: 'Daemon', id: daemonId, label: daemon_name, data: lookups.daemon(daemonId) }
				],
				fix: { label: scanFailureFixes.getName(fixType), run },
				guide:
					guideLabel && guideUrl
						? {
								label: guideLabel,
								run: () => window.open(guideUrl, '_blank', 'noopener,noreferrer')
							}
						: undefined
			};
		}
		case 'UnconfirmedHosts':
			return {
				title,
				sentence: home_nudges_unconfirmedHostsDescription({ count: item.count }),
				subjects: [],
				fix: {
					label: home_nudges_unconfirmedReviewHosts(),
					run: reviewInferred('hosts', item.site_id)
				}
			};
		case 'UnconfirmedSubnets':
			return {
				title,
				sentence: home_nudges_unconfirmedSubnetsDescription({ count: item.count }),
				subjects: [],
				fix: {
					label: home_nudges_unconfirmedReviewSubnets(),
					run: reviewInferred('subnets', item.site_id)
				}
			};
		case 'InstallDaemon':
			return plain({
				label: gettingStarted_stepDaemonLabel(),
				run: () => go('daemons', 'create-daemon')
			});
		// Registration creates and starts the first daemon's scans, so its list shows the scan
		// running, or Run if it stopped.
		case 'RunFirstScan':
			return plain({
				label: gettingStarted_stepDiscoveryLabel(),
				run: item.daemon_id ? daemonScans(item.daemon_id) : () => go('discovery-scans')
			});
		case 'OpenTopology':
			return plain({ label: common_viewTopology(), run: () => go('topology') });
		case 'AnswerReferralSource':
		case 'AnswerProfile': {
			const type = item.type;
			return plain({ label: common_answer(), run: () => onAnswer(type) });
		}
		case 'ExploreViews':
			return plain({
				label: home_nudges_exploreViewsAction(),
				run: () => {
					showViewSwitcherHint.set(true);
					go('topology');
				}
			});
		case 'ApplicationTags':
			return plain({
				label: common_getStarted(),
				run: () => {
					activeView.set('Application');
					go('topology');
				}
			});
		// The same step to the user: the first opens the tutorial, the second (once the
		// tutorial is finished) the map where the dependency is made.
		case 'DependencyTutorial':
		case 'Dependencies': {
			const tutorial = item.type === 'DependencyTutorial';
			return plain({
				label: tutorial ? home_nudges_dependenciesAction() : dependencies_createDependency(),
				run: () => {
					activeView.set(setup.includes('ApplicationTags') ? 'Application' : 'Workloads');
					showDependencyTutorial.set(tutorial);
					go('topology');
				}
			});
		}
		case 'InterfaceCredential':
			return plain({
				label: credentials_createCredential(),
				run: () => go('credentials', 'credential-editor')
			});
		case 'Tags':
			// The step needs a tag on an entity. Each host's Tags cell assigns one, and creates it
			// from a typed name when the org has none.
			return plain({ label: home_nudges_tagHostsAction(), run: () => go('hosts') });
		case 'InviteTeam':
			return plain({
				label: home_nudges_inviteTeamAction(),
				run: () => go('users', 'invite-user')
			});
		case 'MultiSite':
			return plain({
				label: home_nudges_multiSiteAction(),
				run: () => go('sites', 'site-editor')
			});
		case 'ApiKeys':
			return plain({
				label: home_nudges_apiKeysAction(),
				run: () => go('api-keys', 'user-api-key')
			});
		case 'Share':
			return plain({
				label: home_nudges_shareAction(),
				run: () => go('topology', 'topology-share')
			});
	}
}
