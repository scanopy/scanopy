import type { EntityDiscriminants } from '$lib/api/entities';
import { hostDisplayName } from '$lib/features/hosts/host-display-name';
import {
	buildWarningReport,
	credentialIdsOf,
	hostIdsOf,
	integrationWithoutCredentialSentence,
	integrationWithoutCredentialTitle,
	remedyOf
} from '$lib/features/discovery/utils/warnings';
import { actionFor } from '$lib/features/discovery/utils/warningActions';
import type { CredentialLookup } from '$lib/features/discovery/utils/snmpCheck';
import {
	createIntegrationCredentialLabel,
	openCredentialCreate
} from '$lib/features/credentials/utils/prefill';
import {
	activeView,
	showDependencyTutorial,
	showViewSwitcherHint
} from '$lib/features/topology/queries';
import { navigateToEntity, openModal } from '$lib/shared/stores/modal-registry';
import { openTab } from '$lib/shared/stores/url-state';
import { daemonHealths, setupItems } from '$lib/shared/stores/metadata';
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
	home_nudges_apiKeysAction,
	home_nudges_dependenciesAction,
	home_nudges_exploreViewsAction,
	home_nudges_inviteTeamAction,
	home_nudges_multiSiteAction,
	home_nudges_shareAction,
	home_nudges_tagsAction,
	home_nudges_targetlessCredentialAction,
	home_nudges_targetlessCredentialDescription,
	home_nudges_unconfirmedHostsDescription,
	home_nudges_unconfirmedReviewHosts,
	home_nudges_unconfirmedReviewSubnets,
	home_nudges_unconfirmedSubnetsDescription,
	home_setup_installDaemonOnSite,
	home_setup_openDaemon,
	home_setup_troubleshootConnection
} from '$lib/paraglide/messages';
import type { SetupItem, SetupItemType } from './setup-list';

/** A row's fix: what clicking the row does, and its label. */
export interface SetupFix {
	label: string;
	run: () => void;
}

/** The loaded entities a row names or opens. */
export interface SetupLookups {
	daemon: (id: string) => Record<string, unknown> | undefined;
	host: (
		id: string
	) => (Record<string, unknown> & Parameters<typeof hostDisplayName>[0]) | undefined;
	site: (id: string) => Record<string, unknown> | undefined;
	credential: CredentialLookup;
}

/**
 * One chip beside a row's title: the entity the item is about, which the title doesn't name,
 * with `+N` when it stands for several. Its own tooltip and click open that entity.
 */
export interface SetupSubject {
	type: EntityDiscriminants;
	id: string;
	label: string;
	data: object | undefined;
	badge?: string;
}

/** What a Setup list row says and does. */
export interface SetupRowView {
	title: string;
	sentence: string;
	subjects: SetupSubject[];
	fix: SetupFix;
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
}

function go(tab: string, modal?: string, opts?: Parameters<typeof openModal>[1]) {
	window.location.hash = tab;
	if (modal) openModal(modal, opts);
}

/** The first of `ids` as a chip, with `+N` for the rest. Empty when there are none. */
function firstOf(
	type: EntityDiscriminants,
	ids: string[],
	resolve: (id: string) => { label: string; data: object | undefined } | undefined
): SetupSubject[] {
	const [first] = ids;
	const resolved = first ? resolve(first) : undefined;
	if (!first || !resolved) return [];
	return [
		{
			type,
			id: first,
			...resolved,
			badge: ids.length > 1 ? `+${ids.length - 1}` : undefined
		}
	];
}

/** Opens a list filtered to the Inferred source, which is exactly its unconfirmed rows. */
function reviewInferred(tab: string) {
	return () => openTab(tab, new URLSearchParams({ sources: 'Inferred' }));
}

/** What the row for `item` says and does, by item type. */
export function setupRowView(item: SetupItem, ctx: SetupRowContext): SetupRowView {
	const { setup, lookups, onAnswer, onTroubleshoot } = ctx;
	const hostSubjects = (ids: string[]) =>
		firstOf('Host', ids, (id) => {
			const host = lookups.host(id);
			return host ? { label: hostDisplayName(host), data: host } : undefined;
		});
	const credentialSubjects = (ids: string[]) =>
		firstOf('Credential', ids, (id) => {
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
			// old release gets the upgrade; an unknown version opens the daemon itself.
			const fix: SetupFix =
				health === 'awaiting_connection' || health === 'unreachable' || health === 'standby'
					? { label: home_setup_troubleshootConnection(), run: onTroubleshoot }
					: health === 'outdated' || health === 'deprecated' || health === 'unsupported'
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
				item.daemon_name,
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
		case 'UnconfirmedHosts':
			return {
				title,
				sentence: home_nudges_unconfirmedHostsDescription({ count: item.count }),
				subjects: [],
				fix: { label: home_nudges_unconfirmedReviewHosts(), run: reviewInferred('hosts') }
			};
		case 'UnconfirmedSubnets':
			return {
				title,
				sentence: home_nudges_unconfirmedSubnetsDescription({ count: item.count }),
				subjects: [],
				fix: { label: home_nudges_unconfirmedReviewSubnets(), run: reviewInferred('subnets') }
			};
		case 'InstallDaemon':
			return plain({
				label: gettingStarted_stepDaemonLabel(),
				run: () => go('daemons', 'create-daemon')
			});
		case 'RunFirstScan':
			return plain({
				label: gettingStarted_stepDiscoveryLabel(),
				run: () => go('discovery-scans')
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
			return plain({ label: home_nudges_tagsAction(), run: () => go('tags', 'tag-editor') });
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
