import type { components } from '$lib/api/schema';
import { closeModal, navigateToEntity, openModal } from '$lib/shared/stores/modal-registry';
import { credentialIntegrations, entities } from '$lib/shared/stores/metadata';
import {
	createIntegrationCredentialLabel,
	openCredentialCreate
} from '$lib/features/credentials/utils/prefill';
import { createIconComponent } from '$lib/shared/utils/styling';
import type { IconComponent } from '$lib/shared/utils/types';
import {
	common_credentials,
	daemons_upgradeDaemon,
	discovery_scanSettings,
	gettingStarted_stepDaemonLabel,
	hosts_resolveUnconfirmed,
	subnets_resolveRange
} from '$lib/paraglide/messages';
import { credentialIdsOf, type WarningEntry } from './warnings';
import type { CredentialLookup } from './snmpCheck';

type CredentialIntegration = components['schemas']['CredentialIntegration'];

/** A warning row's hand-off: the button's label and icon, and what it opens. */
export interface WarningAction {
	label: string;
	icon: IconComponent;
	run: () => void;
}

/** The run a row's warnings came from, which the actions read their ids off. */
export interface WarningRunContext {
	/** The daemon that ran the scan. One session has exactly one daemon. */
	daemonId: string;
	siteId: string;
	/** The scan whose settings the run used. `null` for a rescan, whose discovery is gone. */
	scanId: string | null;
	/** The stored credential a warning names, when it has loaded. */
	credentialOf: CredentialLookup;
}

/** The codes whose answer is a scan setting, and the editor tab that holds it. */
const SCAN_SETTING_TABS = new Map<WarningEntry['code'], string>([
	// Max Discovery Duration lives on Detection, not Performance.
	['ScanTimeLimit', 'detection'],
	['ScanTimeLimitWithEstimate', 'detection'],
	['WarningsTruncated', 'detection'],
	// The guests' subnet is added to the scan's targets.
	['GuestOutsideScan', 'targets']
]);

/**
 * Codes whose fix is a new daemon: guests only a daemon beside them can reach, on the hypervisor
 * or on their bridge, and a daemon whose VM keeps it off the LAN.
 */
const NEEDS_A_DAEMON_CODES = new Set<WarningEntry['code']>([
	'GuestBehindHypervisor',
	'GuestIsolated',
	'DaemonInDesktopVm'
]);

/**
 * Where a row hands off to, when Scanopy owns the fix.
 *
 * A row without a destination renders no button rather than a dead one. The hypervisor guest
 * warnings come first: they carry an `integration` only to name the hypervisor, and what they
 * ask for is a daemon or a scan setting. Past them, an `integration` on the payload marks a
 * credential warning, recognised by the field rather than a list of code names so that one a
 * newer daemon sends (`Unknown` here) still gets its credential.
 */
export function actionFor(entry: WarningEntry, run: WarningRunContext): WarningAction | null {
	// A credential withheld for the daemon's version is fixed the same way: upgrade that daemon.
	if (entry.code === 'OutdatedDaemonFormat' || entry.code === 'CredentialNeedsNewerDaemon') {
		return {
			label: daemons_upgradeDaemon(),
			icon: createIconComponent('circle-arrow-up'),
			run: () => {
				// The daemon to upgrade is the one that ran this scan, so the id comes off the run
				// rather than the warning, which would let the two disagree.
				window.location.hash = 'daemons';
				openModal('upgrade-daemon', { id: run.daemonId });
			}
		};
	}

	if (entry.code === 'ProvisionalSubnetInferred') {
		// A range deleted since the scan has nothing left to resolve.
		const subnetIds = entry.warnings.flatMap((w) =>
			'subnet_id' in w && w.subnet_id ? [w.subnet_id] : []
		);
		if (subnetIds.length === 0) return null;
		return {
			label: subnets_resolveRange(),
			icon: createIconComponent('circle-dashed-check'),
			run: () => {
				window.location.hash = 'subnets';
				// One range resolves in place; several is a trip to the list, because the
				// four-way choice is made per subnet and there is no "all of them" answer.
				// Opening the resolve modal replaces this one; the list needs this one closed.
				if (subnetIds.length === 1) openModal('provisional-range', { id: subnetIds[0] });
				else closeModal();
			}
		};
	}

	if (entry.code === 'UnconfirmedHostInferred') {
		// The same hand-off as an assumed range: one host resolves in place, several is the list,
		// where each one is settled on its own.
		const hostIds = entry.warnings.flatMap((w) =>
			w.code === 'UnconfirmedHostInferred' && w.host_id ? [w.host_id] : []
		);
		if (hostIds.length === 0) return null;
		return {
			label: hosts_resolveUnconfirmed(),
			icon: createIconComponent('circle-dashed-check'),
			run: () => {
				window.location.hash = 'hosts';
				if (hostIds.length === 1) openModal('unconfirmed-host', { id: hostIds[0] });
				else closeModal();
			}
		};
	}

	if (entry.code === 'IntegrationWithoutCredential') {
		// The row's title names its integration, so every occurrence under it is that one.
		const found = entry.warnings.flatMap((w) =>
			w.code === 'IntegrationWithoutCredential' ? [w] : []
		);
		if (!found.length) return null;
		const integration: CredentialIntegration = found[0].integration;
		const hostIds = found.flatMap((w) => (w.host_id ? [w.host_id] : []));
		const siteIds = [run.siteId];
		return {
			label: createIntegrationCredentialLabel(integration),
			icon: credentialIntegrations.getIconComponent(integration),
			run: () => openCredentialCreate(integration, { hostIds, siteIds })
		};
	}

	if (NEEDS_A_DAEMON_CODES.has(entry.code)) {
		return {
			label: gettingStarted_stepDaemonLabel(),
			icon: entities.getIconComponent('Daemon'),
			run: () => {
				window.location.hash = 'daemons';
				openModal('create-daemon');
			}
		};
	}

	const scanId = run.scanId;
	const scanTab = SCAN_SETTING_TABS.get(entry.code);
	if (scanTab) {
		// A rescan's discovery is gone by now, so it has no settings to open.
		if (!scanId) return null;
		return {
			label: discovery_scanSettings(),
			icon: createIconComponent('sliders-horizontal'),
			run: () => {
				window.location.hash = 'discovery-scans';
				openModal('discovery-editor', { id: scanId, tab: scanTab });
			}
		};
	}

	if (entry.warnings.some((w) => 'integration' in w)) {
		const [credentialId] = credentialIdsOf(entry);
		return {
			label: common_credentials(),
			icon: createIconComponent('key-round'),
			run: () => {
				// Any known id opens that credential's editor. Every credential on this row needs
				// the same thing done to it, so landing on the first is a head start on the list,
				// and the row's chips reach the others directly. Only a row with no id at all
				// (written before ids were carried, or posted by a daemon that predates them)
				// still goes to the list.
				if (!credentialId) {
					window.location.hash = 'credentials';
					closeModal();
					return;
				}
				// The credential itself where it has loaded, so the editor opens on the id alone
				// rather than waiting for the credentials tab's own query to settle.
				navigateToEntity('Credential', credentialId, run.credentialOf(credentialId));
			}
		};
	}

	return null;
}
