/**
 * Rendering coded scan warnings into the sentences an operator reads.
 *
 * The daemon and the server both record one warning per occurrence — one device, one neighbour,
 * one credential attempt — because that is what makes them countable and what keeps each address's
 * own diagnostic attached to it. Grouping them back into readable prose is this module's job, and
 * it belongs here rather than on the backend for one reason: `Intl.ListFormat` joins a list in the
 * reader's own language, where the Rust helper it replaced only ever spoke English.
 *
 * Each code's sentence is a template with `{named}` slots (see `warning-codes.json`), and the
 * renderer below supplies exactly the slots that fixture declares. `WARNING_PARAMS` is typed
 * `satisfies Record<DiscoveryWarningCode, …>`, so TypeScript will not compile a new backend code
 * without a renderer for it.
 */

import type { components } from '$lib/api/schema';
import type { EntityDiscriminants } from '$lib/api/entities';
import claimSources from '$lib/data/claim-sources.json';
import discoveryIntegrations from '$lib/data/discovery-integrations.json';
import malformedNeighbourConsequences from '$lib/data/malformed-neighbour-consequences.json';
import osFamilies from '$lib/data/os-families.json';
import snmpWalkCauses from '$lib/data/snmp-walk-causes.json';
import snmpWalkGroups from '$lib/data/snmp-walk-groups.json';
import warningCodes from '$lib/data/warning-codes.json';
import warningRemedies from '$lib/data/warning-remedies.json';
import { metaDescription, metaDescriptionWith, metaName, metaNameWith } from '$lib/i18n/metadata';
import { joinList } from '$lib/i18n/list';
import { snmpCheckOf, type CredentialLookup, type SnmpCheckOutcome } from './snmpCheck';
import { toColor, type Color } from '$lib/shared/utils/styling';
import { osLabel } from '$lib/features/daemons/utils';
import { credentialIntegrations } from '$lib/shared/stores/metadata';
import {
	common_host,
	common_unknown,
	common_unknownEntity,
	discovery_warningNoFurtherDetail,
	discovery_warningUnknownDaemon,
	discovery_warningAtAddress,
	discovery_warningGapContainerBridge,
	discovery_warningGapCredentialType,
	discovery_warningGapSnmpV3Options,
	discovery_warningSnmpV3OptionAuthNoPriv,
	discovery_warningSnmpV3OptionMd5Auth,
	discovery_warningSnmpV3OptionNoAuthNoPriv,
	discovery_warningInferredFrom,
	discovery_warningSeenBy,
	discovery_warningNoPortId,
	discovery_warningNoWalkFailed,
	discovery_warningWalkFailures,
	discovery_warningWalkFailuresUnrecorded
} from '$lib/paraglide/messages';

export type DiscoveryWarning = components['schemas']['DiscoveryWarning'];
export type DiscoveryWarningCode = DiscoveryWarning['code'];

/**
 * What a row is about: a device, a credential, a range, or a bare address.
 *
 * `entity` is present only where the warning carried an id — the LLDP/CDP resolution family for a
 * host, the credential family for a credential. Those render as entity tags that navigate to the
 * record; an address on its own has no entity to point at and renders as plain text.
 *
 * Typed by entity rather than by a `hostId` field, which is what it was when only hosts could be
 * named. A parallel `credentialId` beside it would have meant every consumer growing a second
 * branch that does the same thing.
 */
export interface WarningSubject {
	label: string;
	entity?: { type: EntityDiscriminants; id: string };
}

/**
 * One of the individual cases behind a statement — an unresolved neighbour pair, an inferred range.
 *
 * Split into ends rather than pre-joined into a sentence for two reasons. It renders on a line of
 * its own, so eight of them stop being the last clause of a paragraph; and each end that resolved
 * to a device is a real entity, so it can be tagged and navigated to instead of printed.
 *
 * Either end can be empty — a host deleted since the scan, a far end nothing matched, a device that
 * reported no interface description. The arrow between them is drawn only when both survive:
 * `-> TAMMIERENEW` reads as a rendering fault rather than as missing data.
 */
export interface WarningExample {
	/** The device that reported this, when its name resolved. */
	near: WarningSubject | null;
	/** Its own port, as the device described it. */
	nearText: string;
	/** The device at the other end, when one resolved. */
	far: WarningSubject | null;
	/** What was advertised for the far end, or the identifier that matched nothing. */
	farText: string;
}

/**
 * One thing a row says, and the individual cases behind it.
 *
 * `examples` is a list because it *is* a list. Eight unresolved pairs joined into the last clause
 * of a sentence is the wall of text this report exists to undo, one level down — so the sentence
 * keeps the explanation and the count, and the pairs render underneath it, one per line.
 */
export interface WarningStatement {
	sentence: string;
	examples: WarningExample[];
	/** The occurrences behind the sentence, one per device, for what the row lists under it. */
	warnings: DiscoveryWarning[];
}

/** One device in a row that offers commands: what the row says about it, and how to check it. */
export interface DeviceCheck {
	address: string;
	credentialId: string | null;
	sentence: string;
	outcome: NonNullable<SnmpCheckOutcome>;
}

/** How many addresses, hosts or subnets a row concerns, and the one when there is only one. */
export interface WarningAffects {
	type: 'address' | 'host' | 'subnet';
	count: number;
	/** The single subject's label when `count` is 1 and it resolved. */
	only: string | null;
}

/** One warning code's worth of a run, as a row in the report. */
export interface WarningEntry {
	/** Unique within a run: the code and the title, since a code whose name is a template has a row
	 *  per title. */
	key: string;
	code: DiscoveryWarningCode;
	/** The code's short name — "Credential incomplete" — which is what the row is headed by. */
	title: string;
	/** Severity, as the fixture already publishes it. */
	color: Color;
	icon: string;
	/** What the row concerns, counted for its one-line header. `null` for a scan-level code. */
	affects: WarningAffects | null;
	/**
	 * One device at a time, for a row that offers commands: each device's own sentence and what
	 * checking it takes. Empty when no device in the row has commands, and the row lists its
	 * statements instead.
	 */
	devices: DeviceCheck[];
	/** What the row says, shown on disclosure. Usually one statement; more when they differ. */
	details: WarningStatement[];
	/** Every occurrence behind the row, so an action can read ids off the payload. */
	warnings: DiscoveryWarning[];
}

/** One rung of the report: what the reader has to do about everything under it. */
export interface WarningSection {
	/** The `WarningRemedy` id, which is what an action keys off. */
	remedy: string;
	title: string;
	description: string;
	icon: string;
	entries: WarningEntry[];
}

type Params = Record<string, string | number>;
type WarningOf<C extends DiscoveryWarningCode> = Extract<DiscoveryWarning, { code: C }>;

/**
 * Resolve a host id to its name.
 *
 * Passed in rather than queried here: the warnings carry ids, and the component already holds the
 * host query that turns them into names. Three outcomes, and the difference between the last two
 * is what the reader sees:
 *
 * - a **name** — the device is there and is named;
 * - **`null`** — the lookup ran and this id is not among the results, so the device is gone. A
 *   historical scan outlives the hosts it names, and deleting one used to make its chip vanish,
 *   which quietly changed what the line said it was about;
 * - **`undefined`** — nothing is known yet, because the query is still in flight or was never
 *   made. Distinct from `null` so a chip does not flash "unknown" on its way to a name.
 */
export type HostNameLookup = (hostId: string) => string | null | undefined;

/** What a sentence needs beyond its warnings: the run's daemon, named by the mismatch warning. */
type ParamContext = { daemonName: string };

/**
 * Resolve any entity a warning names to its display name, or `undefined` when it cannot be.
 *
 * The generalization of {@link HostNameLookup}, with the same three outcomes. Credentials
 * deliberately never return `null`: a viewer without permission to read them resolves nothing, and
 * labelling every credential "unknown" for that reader would be a worse lie than saying nothing.
 */
export type EntityNameLookup = (type: EntityDiscriminants, id: string) => string | null | undefined;

/**
 * The chip for a host a warning named.
 *
 * A device deleted since the scan is still part of what the line is about, so it is named rather
 * than dropped: eight unresolved pairs used to render as eight bare port descriptions with nothing
 * to say which devices they concerned. No `entity` on it, because there is nothing left to
 * navigate to — it reads as a label, the same as an address does.
 *
 * A `null` id is a host deleted since the scan: the stored warning lost the id with it, so it reads
 * as gone without a lookup.
 *
 * `null` back means the lookup has not run yet, and the caller omits the segment exactly as it did
 * before, so nothing flashes "unknown" on its way to a name.
 */
function hostSubject(hostId: string | null, hostName: HostNameLookup): WarningSubject | null {
	const gone = { label: common_unknownEntity({ entity: common_host() }) };
	if (hostId === null) return gone;
	const label = hostName(hostId);
	if (label) return { label, entity: { type: 'Host', id: hostId } };
	if (label === null) return gone;
	return null;
}

/** No names available: every named segment is omitted, which is how this rendered before names. */
const NO_ENTITY_NAMES: EntityNameLookup = () => undefined;

/** Metadata lookup for a slot value, falling back to the fixture's own English. */
function nameOf(fixture: { id: string; name: string | null }[], fixtureKey: string, id: string) {
	const entry = fixture.find((item) => item.id === id);
	return metaName(fixtureKey, id, entry?.name ?? id);
}

const group = (id: string) => nameOf(snmpWalkGroups, 'snmp_walk_groups', id);
const cause = (id: string) => nameOf(snmpWalkCauses, 'snmp_walk_causes', id);

/**
 * How each failed read on a device ended, as one clause per cause naming the groups that ended
 * that way. `null` is a daemon that recorded no causes; an empty list is a device whose every read
 * ended normally on an empty table.
 */
function failuresOf(failures: components['schemas']['SnmpWalkFailure'][] | null): string {
	if (failures === null) return discovery_warningWalkFailuresUnrecorded();
	if (failures.length === 0) return discovery_warningNoWalkFailed();

	const byCause = new Map<string, string[]>();
	for (const failure of failures) {
		const groups = byCause.get(failure.cause) ?? [];
		groups.push(group(failure.group));
		byCause.set(failure.cause, groups);
	}
	return [...byCause]
		.map(([id, groups]) =>
			discovery_warningWalkFailures({ groups: joinList(groups, 'conjunction'), cause: cause(id) })
		)
		.join(' ');
}
const claimSource = (id: string) => nameOf(claimSources, 'claim_sources', id);
const consequence = (id: string) =>
	nameOf(malformedNeighbourConsequences, 'malformed_neighbour_consequences', id);

/**
 * The integration's display name.
 *
 * Keyed by `CredentialQueryPayloadDiscriminants`, which is what the warning carries. Not
 * `credential-types.json`: that is keyed by `CredentialType` (SnmpCommunity/SnmpV3, UnifiApiKey, …), so
 * `DockerProxy` resolved there by coincidence while `Snmp`, `UnifiController` and `InstantOn` fell
 * through and rendered as their raw discriminants.
 */
export const integration = (id: string) =>
	nameOf(discoveryIntegrations, 'discovery_integrations', id);

/** An `OsFamily`'s display name ("Linux, macOS, BSD", "Windows"). */
const osFamily = (id: string) => nameOf(osFamilies, 'os_families', id);

const addressesOf = (group_: { address: string }[]) =>
	joinList(
		group_.map((w) => w.address),
		'conjunction'
	);

/** Each guest by its hypervisor's name for it and address: an address on an internal bridge names nothing. */
const guestsOf = (group_: { address: string; guest: string }[]) =>
	joinList(
		group_.map((w) => `${w.guest} (${w.address})`),
		'conjunction'
	);

type CapabilityGap = components['schemas']['CapabilityGap'];
type SnmpV3SecurityOption = components['schemas']['SnmpV3SecurityOption'];

const SNMP_V3_OPTION_LABELS: Record<SnmpV3SecurityOption, () => string> = {
	NoAuthNoPriv: () => discovery_warningSnmpV3OptionNoAuthNoPriv(),
	AuthNoPriv: () => discovery_warningSnmpV3OptionAuthNoPriv(),
	Md5Auth: () => discovery_warningSnmpV3OptionMd5Auth()
};

/**
 * What a daemon missed of one credential, as the clause after "The {credential} credential".
 * Exhaustive over the generated union, so a gap the backend adds does not compile until it reads.
 */
function capabilityGap(gap: CapabilityGap): string {
	switch (gap.type) {
		case 'ParsesCredentialType':
			return discovery_warningGapCredentialType();
		case 'UsesSnmpV3SecurityOptions':
			return discovery_warningGapSnmpV3Options({
				options: joinList(
					gap.details.options.map((o) => SNMP_V3_OPTION_LABELS[o]()),
					'conjunction'
				)
			});
		case 'TargetsContainerBridgeWithCredentials':
			return discovery_warningGapContainerBridge({
				addresses: joinList(gap.details.addresses, 'conjunction')
			});
	}
}

const groupsOf = (group_: { group: string }[]) =>
	joinList(
		group_.map((w) => group(w.group)),
		'disjunction'
	);

const sum = (values: number[]) => values.reduce((total, n) => total + n, 0);

/**
 * Every code's slot values, built from the warnings sharing that code.
 *
 * `satisfies` rather than a plain annotation so the object keeps its precise type while still being
 * checked for completeness — a backend code with no entry here is a compile error, which is the
 * TypeScript half of the agreement the Rust `slots()` test enforces on the other side.
 */
const WARNING_PARAMS = {
	InterfaceSetCutShort: (w) => ({ addresses: addressesOf(w) }),
	InterfaceDetailsCutShort: (w) => ({ addresses: addressesOf(w) }),

	SnmpWalkEntryCap: (w) => ({
		addresses: addressesOf(w),
		groups: groupsOf(w),
		limit: w[0].limit
	}),
	SnmpWalkUnsupported: (w) => ({ addresses: addressesOf(w), groups: groupsOf(w) }),
	SnmpWalkDesynchronised: (w) => ({ addresses: addressesOf(w), groups: groupsOf(w) }),
	SnmpWalkPartialDiscarded: (w) => ({ addresses: addressesOf(w), groups: groupsOf(w) }),
	SnmpWalkPartialRecorded: (w) => ({ addresses: addressesOf(w), groups: groupsOf(w) }),
	SnmpWalkBridgeMibAbsent: (w) => ({ addresses: addressesOf(w), groups: groupsOf(w) }),
	SnmpWalkNoAnswer: (w) => ({ addresses: addressesOf(w), groups: groupsOf(w) }),
	// The cause identifies the statement: reads that timed out and reads answered with genErr
	// are different sentences.
	SnmpWalkFailed: (w) => ({
		addresses: addressesOf(w),
		groups: groupsOf(w),
		cause: cause(w[0].cause)
	}),

	// The figures are the substance of a claim warning, so two switches disagreeing with themselves
	// by different amounts do not share a sentence — the bucketing below keeps them apart, and
	// merges the ones that do agree.
	ClaimedCountReadCutShort: (w) => ({
		addresses: addressesOf(w),
		source: claimSource(w[0].source),
		expected: w[0].expected,
		observed: w[0].observed
	}),
	ClaimedCountUnderRead: (w) => ({
		addresses: addressesOf(w),
		source: claimSource(w[0].source),
		expected: w[0].expected,
		observed: w[0].observed
	}),
	ClaimedCapabilityReadCutShort: (w) => ({
		addresses: addressesOf(w),
		source: claimSource(w[0].source),
		group: group(w[0].group)
	}),
	ClaimedCapabilityEmpty: (w) => ({
		addresses: addressesOf(w),
		source: claimSource(w[0].source),
		group: group(w[0].group)
	}),

	LldpLocalPortDropped: droppedPortParams,
	LldpLocalPortDroppedReadCutShort: droppedPortParams,
	LldpLocalPortMisplaced: (w) => ({
		addresses: addressesOf(w),
		misplaced: sum(w.map((x) => x.misplaced))
	}),

	MalformedNeighboursWalkCutShort: malformedParams,
	MalformedNeighboursGhostRows: malformedParams,
	MalformedNeighboursIncompleteRecords: malformedParams,
	MalformedNeighboursUnexpectedType: malformedParams,
	MalformedNeighboursUnreadableIndex: malformedParams,

	// Devices whose reads ended the same way share a sentence; any difference splits them.
	SnmpCollectedNothing: (w) => ({
		addresses: addressesOf(w),
		failures: failuresOf(w[0].failures)
	}),
	VlanRecordingFailed: (w) => ({ addresses: addressesOf(w) }),
	// One per host, like the credential codes. The two integrations identify the statement, so hosts
	// read by the same pair in the same order share a sentence and a different pair stays apart.
	EqualReachIntegrationsMerged: (w) => ({
		addresses: addressesOf(w),
		first: integration(w[0].first),
		second: integration(w[0].second)
	}),

	CredentialTargetNotScanned: credentialParams,
	CredentialTargetNotResponding: credentialParams,
	CredentialGateClosed: (w) => ({
		credential: integration(w[0].integration),
		addresses: addressesOf(w),
		ports: joinList(w[0].ports.map(String), 'conjunction')
	}),
	CredentialRejected: attemptParams,
	CredentialMalformed: attemptParams,
	CredentialTlsFailed: attemptParams,
	CredentialNotThisService: attemptParams,
	CredentialCollectionFailed: attemptParams,
	CredentialCollectionTimedOut: attemptParams,
	CredentialUnreachable: attemptParams,
	CredentialTimedOut: attemptParams,
	// The two OS slots identify the statement: a credential declared for Windows and skipped by a
	// Unix daemon is a different sentence from the reverse.
	CredentialDaemonOsMismatch: (w, { daemonName }) => ({
		credential: integration(w[0].integration),
		declared: osFamily(w[0].declared),
		daemon: daemonName,
		// The daemon's own OS where it sent one; runs recorded before that name its family.
		actual: w[0].actual_os ? osLabel(w[0].actual_os) : osFamily(w[0].actual)
	}),
	// Server-raised, for the one daemon that ran the scan, so its name is the session's. A daemon that
	// never reported a version still has to read as a sentence.
	// The gap identifies the statement: a credential withheld whole and one that lost only its
	// bridge addresses are different sentences.
	CredentialNeedsNewerDaemon: (w, { daemonName }) => ({
		credential: integration(w[0].integration),
		gap: capabilityGap(w[0].gap),
		required: w[0].last_unsupported_daemon_version,
		daemon: daemonName,
		version: w[0].daemon_version ?? common_unknown()
	}),
	// Server-raised, one per credential. The run's Credentials tab names each one.
	CredentialWithoutTarget: (w) => ({ credential: integration(w[0].integration) }),
	// Server-raised, one per credential whose SSH login is gone. The run's Credentials tab names it.
	CredentialLoginUnavailable: (w) => ({ credential: integration(w[0].integration) }),

	// One per host. The integration identifies the statement, so every host running one
	// integration shares a sentence, and `discovers` is that integration's own description.
	IntegrationWithoutCredential: (w) =>
		integrationWithoutCredentialParams(w[0].integration, w.length),

	// One per guest. The hypervisor integration, bridge and hypervisor host identify the statement,
	// so guests on one bridge share a sentence.
	GuestOutsideScan: (w) => ({
		addresses: guestsOf(w),
		integration: integration(w[0].integration)
	}),
	GuestBehindHypervisor: (w) => ({
		addresses: guestsOf(w),
		integration: integration(w[0].integration),
		bridge: w[0].bridge,
		hypervisor: w[0].hypervisor
	}),
	GuestIsolated: (w) => ({
		addresses: guestsOf(w),
		integration: integration(w[0].integration),
		bridge: w[0].bridge
	}),

	// One warning per subnet, so the first is the only one — unlike the address-scoped codes above,
	// which the daemon raises per host and the UI folds together here.
	ConnectionsWithoutProtocolResponse: (w) => ({
		cidr: w[0].cidr,
		declined: w[0].declined,
		ports: joinList(w[0].ports.map(String), 'conjunction')
	}),

	ScanTimeLimitWithEstimate: (w) => ({
		hours: w[0].hours,
		hosts_not_scanned: w[0].hosts_not_scanned,
		minutes_remaining: w[0].minutes_remaining
	}),
	ScanTimeLimit: (w) => ({ hours: w[0].hours, hosts_not_scanned: w[0].hosts_not_scanned }),

	// Once per session: each sweep runs once, and the lookup count is already summed daemon-side.
	DcpSweepTimedOut: (w) => ({ seconds: w[0].seconds }),
	IcmpSweepTimedOut: (w) => ({ seconds: w[0].seconds }),
	ReverseDnsTimedOut: (w) => ({ count: w[0].count }),

	LldpNeighbourNotFound: neighbourParams,
	LldpNeighbourAmbiguous: neighbourParams,
	LldpPortNoStrategy: portParams,
	LldpPortNotFound: portParams,
	LldpPortAmbiguous: portParams,

	ProvisionalSubnetInferred: (w) => ({ count: w.length }),
	UnconfirmedHostInferred: (w) => ({ count: w.length }),

	// One per session by construction — the pass runs once and is cut short at most once — so this
	// reads the single occurrence rather than summing.
	NeighbourResolutionIncomplete: (w) => ({
		budget_seconds: w[0].budget_seconds,
		neighbours: w[0].neighbours
	}),
	// Same reasoning as NeighbourResolutionIncomplete: the FDB pass also runs once per session and
	// is cut short at most once.
	FdbResolutionIncomplete: (w) => ({
		budget_seconds: w[0].budget_seconds,
		interfaces: w[0].interfaces
	}),

	// One per session: the server raises it once, on the terminal payload, for the one daemon that
	// ran the scan. A daemon too old to report a version at all still has to read as a sentence,
	// so the empty case says so rather than leaving a hole where the number goes.
	OutdatedDaemonFormat: (w) => ({
		daemon_version: w[0].daemon_version ?? common_unknown()
	}),

	WarningsTruncated: (w) => ({ elided: sum(w.map((x) => x.elided)) }),
	// The whole sentence *is* the detail: a warning from another version, or one written before
	// warnings were coded at all. Rendered one per occurrence, so this only ever sees one.
	Unknown: (w) => ({ detail: w[0].detail })
} satisfies {
	[C in DiscoveryWarningCode]: (warnings: WarningOf<C>[], context: ParamContext) => Params;
};

/**
 * The two codes carry the same three slots and differ only in the sentence they fill.
 *
 * Which sentence is the whole point: one says the device numbers its LLDP ports separately from
 * its interfaces, the other says we did not finish reading that numbering. The backend decides
 * which, from whether the reads completed — offering both at once left the operator to guess.
 */
function droppedPortParams(
	w: WarningOf<'LldpLocalPortDropped' | 'LldpLocalPortDroppedReadCutShort'>[]
): Params {
	return {
		addresses: addressesOf(w),
		dropped: sum(w.map((x) => x.dropped)),
		total: sum(w.map((x) => x.total))
	};
}

function malformedParams(
	w: WarningOf<
		| 'MalformedNeighboursWalkCutShort'
		| 'MalformedNeighboursGhostRows'
		| 'MalformedNeighboursIncompleteRecords'
		| 'MalformedNeighboursUnexpectedType'
		| 'MalformedNeighboursUnreadableIndex'
	>[]
): Params {
	return {
		addresses: addressesOf(w),
		discarded: sum(w.map((x) => x.discarded)),
		// Worst case across the group: a device that lost every link must not be described as
		// having lost only some of them because another device in the same sentence kept its.
		consequence: consequence(
			w.some((x) => x.consequence === 'AllLinksLost') ? 'AllLinksLost' : 'SomeLinksLost'
		)
	};
}

type CredentialIntegration = components['schemas']['CredentialIntegration'];

/**
 * The slots of `IntegrationWithoutCredential`'s sentence. Exported because the home nudge says the
 * same thing in the same words, from the latest scans rather than one run.
 */
export function integrationWithoutCredentialParams(
	id: CredentialIntegration,
	hosts: number
): Params {
	return {
		integration: credentialIntegrations.getName(id),
		count: hosts,
		discovers: credentialIntegrations.getDescription(id)
	};
}

/** `IntegrationWithoutCredential`'s title for one integration, which heads its run report row. */
export function integrationWithoutCredentialTitle(id: CredentialIntegration): string {
	const code: DiscoveryWarningCode = 'IntegrationWithoutCredential';
	const fallback = warningCodes.find((c) => c.id === code)?.name ?? code;
	return metaNameWith('warning_codes', code, integrationWithoutCredentialParams(id, 0), fallback);
}

/** `IntegrationWithoutCredential`'s sentence for one integration found on `hosts` hosts. */
export function integrationWithoutCredentialSentence(
	id: CredentialIntegration,
	hosts: number
): string {
	const code: DiscoveryWarningCode = 'IntegrationWithoutCredential';
	const fallback = warningCodes.find((c) => c.id === code)?.description ?? '';
	return metaDescriptionWith(
		'warning_codes',
		code,
		integrationWithoutCredentialParams(id, hosts),
		fallback
	);
}

function credentialParams(
	w: WarningOf<'CredentialTargetNotScanned' | 'CredentialTargetNotResponding'>[]
): Params {
	return { credential: integration(w[0].integration), addresses: addressesOf(w) };
}

function attemptParams(
	w: WarningOf<
		| 'CredentialRejected'
		| 'CredentialMalformed'
		| 'CredentialTlsFailed'
		| 'CredentialNotThisService'
		| 'CredentialCollectionFailed'
		| 'CredentialCollectionTimedOut'
		| 'CredentialUnreachable'
		| 'CredentialTimedOut'
	>[]
): Params {
	// The diagnostic the library returned, which is usually this address's alone — so the bucketing
	// below leaves these apart. Addresses that failed with the *same* message share a sentence,
	// which is the one case where merging loses nothing.
	return {
		credential: integration(w[0].integration),
		addresses: addressesOf(w),
		detail: w[0].detail || discovery_warningNoFurtherDetail()
	};
}

/**
 * `switch7 Gi1/0/1 -> 00:ad:24:89:cc:f0 (core-sw) at 10.20.30.11`.
 *
 * Which of our devices saw the neighbour leads the line, because it is the first thing an operator
 * needs in order to act — the identifier alone says a link is missing without saying where to go
 * and look. Every segment is dropped when it is absent rather than filled with a placeholder: a
 * host whose name has not loaded yet, or one deleted since the scan, then reads exactly as it did
 * before names were resolved at all.
 *
 * The address is the segment that says which type of gap this is. A far end that published one and
 * still matched nothing is a device on a range this site has not scanned, which an operator can
 * act on; one that published none cannot be placed however much gets scanned. Reading the two apart
 * used to cost a round trip to whoever reported the scan.
 */
function describeNeighbour(
	w: {
		host_id: string | null;
		if_descr: string;
		identifier: string;
		sys_name?: string | null;
		address?: string | null;
	},
	hostName: HostNameLookup
): WarningExample {
	const named = `${w.identifier}${w.sys_name ? ` (${w.sys_name})` : ''}`;
	return {
		near: hostSubject(w.host_id, hostName),
		nearText: w.if_descr,
		// The far end is the whole point of this warning: nothing on this site matched it, so
		// there is no device to tag, only the identifier it advertised.
		far: null,
		farText: w.address ? `${named} ${discovery_warningAtAddress({ address: w.address })}` : named
	};
}

function neighbourParams(
	w: WarningOf<'LldpNeighbourNotFound' | 'LldpNeighbourAmbiguous'>[]
): Params {
	return { count: w.length };
}

/**
 * `switch7 Gi0/1 -> core-sw via MacAddress("00:ad:…") (Port 9)`.
 *
 * Both ends are named here, unlike the unmatched case: the far end resolved to a device, and the
 * point of this warning is that the two are on the map and still not joined port-to-port. Both
 * halves of the advertised id are kept, because the subtype says which tier ran and the value says
 * what it looked for.
 */
function describePort(
	w: {
		host_id: string | null;
		remote_host_id: string | null;
		if_descr: string;
		port_id?: string | null;
		port_desc?: string | null;
	},
	hostName: HostNameLookup
): WarningExample {
	const desc = w.port_desc ? ` (${w.port_desc})` : '';
	// "via <id>" when the device advertised one, "with no port id" when it did not — the
	// distinction the tiers turn on, and the phrasing the prose these replaced used.
	const id = w.port_id ? `via ${w.port_id}${desc}` : `${discovery_warningNoPortId()}${desc}`;
	return {
		near: hostSubject(w.host_id, hostName),
		nearText: w.if_descr,
		far: hostSubject(w.remote_host_id, hostName),
		farText: id
	};
}

/**
 * `10.20.30.0/24, from offsite-core-01 (10.20.30.11) and offsite-edge-01 (10.20.30.24) seen by
 * switch-offsite-01`.
 *
 * Names the far ends rather than only the range, because "we invented a subnet" is not something an
 * operator can act on: the labels are what let them recognise a segment as real, spot a device with
 * a factory-default address, or decide the range needs correcting.
 */
function describeProvisionalSubnet(
	w: WarningOf<'ProvisionalSubnetInferred'>,
	hostName: HostNameLookup
): WarningExample {
	// A range is reported for as long as it is unconfirmed, so most of them arrive without the
	// far-end evidence that produced them — the pass that inferred it may have been scans ago, and
	// one inferred while placing a controller-reported address never had a neighbour at all. The
	// range alone is still the actionable part.
	// The range itself links to its subnet, which is where it is confirmed. A subnet deleted since
	// the scan lost its id and reads as the bare range.
	const range: WarningSubject = w.subnet_id
		? { label: w.cidr, entity: { type: 'Subnet', id: w.subnet_id } }
		: { label: w.cidr };
	if (!w.addresses.length) return { near: range, nearText: '', far: null, farText: '' };

	// Paired positionally where both are present: `sys_names` omits far ends that sent none, so a
	// group where only some did would otherwise misalign names against addresses.
	const named =
		w.sys_names.length === w.addresses.length
			? w.addresses.map((address, i) => `${w.sys_names[i]} (${address})`)
			: w.addresses;
	const seenBy = [...new Set(w.seen_by_host_ids.map(hostName).filter(Boolean))] as string[];
	const from = discovery_warningInferredFrom({ far_ends: joinList(named, 'conjunction') });
	const text = seenBy.length
		? `${from} ${discovery_warningSeenBy({ devices: joinList(seenBy, 'conjunction') })}`
		: from;
	return { near: range, nearText: text, far: null, farText: '' };
}

function portParams(
	w: WarningOf<'LldpPortNoStrategy' | 'LldpPortNotFound' | 'LldpPortAmbiguous'>[]
): Params {
	return { count: w.length };
}

/**
 * The slots whose value is computed from a whole group rather than from one occurrence.
 *
 * This is what makes the grouping below general. Two warnings of the same code say the *same
 * thing* when every slot outside this set matches: four switches that each advertised the bridge
 * bit and served no bridge-port numbering differ only in which device it was, so they are one
 * sentence naming four devices. Two credential attempts that failed with different diagnostics
 * differ in `detail`, so they stay two sentences — which is what the per-occurrence records the
 * backend keeps exist for.
 *
 * A new aggregating slot missing from this set only costs grouping: those occurrences render as
 * separate sentences, each correct. Listing an *identifying* slot here would be the harmful
 * direction — it would merge two statements and let one set of figures speak for both — which is
 * why the set is written this way round.
 */
const AGGREGATING_SLOTS = new Set([
	'addresses',
	'groups',
	'count',
	'dropped',
	'total',
	'misplaced',
	'discarded',
	'consequence',
	'elided'
]);

/**
 * Split one code's occurrences into the distinct statements they make.
 *
 * Insertion-ordered, so the buckets come out in the order the producers emitted them.
 */
function bucketByStatement(
	code: DiscoveryWarningCode,
	group_: DiscoveryWarning[],
	context: ParamContext
): DiscoveryWarning[][] {
	const build = WARNING_PARAMS[code] as (w: DiscoveryWarning[], context: ParamContext) => Params;
	const buckets = new Map<string, DiscoveryWarning[]>();

	for (const warning of group_) {
		const params = build([warning], context);
		const identity = Object.entries(params)
			.filter(([slot]) => !AGGREGATING_SLOTS.has(slot))
			.sort(([a], [b]) => a.localeCompare(b));
		const key = JSON.stringify(identity);

		const existing = buckets.get(key);
		if (existing) {
			existing.push(warning);
		} else {
			buckets.set(key, [warning]);
		}
	}

	return [...buckets.values()];
}

/**
 * The individual cases behind a statement, one per line.
 *
 * Only the codes that carry them — a neighbour pair, an unresolved port, an inferred range. The
 * rest name their devices in the sentence itself and have nothing to list.
 */
function examplesOf(group_: DiscoveryWarning[], hostName: HostNameLookup): WarningExample[] {
	return group_.flatMap((w) => {
		switch (w.code) {
			case 'LldpNeighbourNotFound':
			case 'LldpNeighbourAmbiguous':
				return [describeNeighbour(w, hostName)];
			case 'LldpPortNoStrategy':
			case 'LldpPortNotFound':
			case 'LldpPortAmbiguous':
				return [describePort(w, hostName)];
			case 'ProvisionalSubnetInferred':
				return [describeProvisionalSubnet(w, hostName)];
			// Each host, linking to it, beside what it is known by.
			case 'UnconfirmedHostInferred': {
				const host = hostSubject(w.host_id, hostName);
				return host ? [{ near: host, nearText: w.identifier, far: null, farText: '' }] : [];
			}
			// The hosts a credential would be added for, one per line, each linking to its host.
			case 'IntegrationWithoutCredential': {
				const host = hostSubject(w.host_id, hostName);
				return host ? [{ near: host, nearText: '', far: null, farText: '' }] : [];
			}
			default:
				return [];
		}
	});
}

/**
 * One statement per distinct thing a code's occurrences say, with the cases behind it.
 *
 * Usually one. Several when the occurrences genuinely differ — different credential diagnostics,
 * different claimed figures — and the devices that made the *same* statement are named together
 * inside a single sentence rather than repeating it once each.
 */
function renderStatements(
	code: DiscoveryWarningCode,
	group_: DiscoveryWarning[],
	hostName: HostNameLookup,
	context: ParamContext
): WarningStatement[] {
	const build = WARNING_PARAMS[code] as (w: DiscoveryWarning[], context: ParamContext) => Params;
	const fallback = warningCodes.find((c) => c.id === code)?.description;
	if (!fallback) return [];

	return bucketByStatement(code, group_, context).map((bucket) => ({
		sentence: metaDescriptionWith('warning_codes', code, build(bucket, context), fallback),
		examples: examplesOf(bucket, hostName),
		warnings: bucket
	}));
}

/**
 * Each device in a row, one at a time, when at least one of them has commands to offer.
 *
 * A device gets the sentence for itself alone, so the reader takes in one device's result and
 * its command together rather than a grouped sentence and a list to match it against. A row where
 * no device has commands returns none, and lists its statements, so the devices are still named.
 */
function deviceChecksOf(
	code: DiscoveryWarningCode,
	row: DiscoveryWarning[],
	credentialOf: CredentialLookup,
	context: ParamContext
): DeviceCheck[] {
	const build = WARNING_PARAMS[code] as (w: DiscoveryWarning[], context: ParamContext) => Params;
	const fallback = warningCodes.find((c) => c.id === code)?.description;
	if (!fallback) return [];

	const byAddress = new Map<string, DeviceCheck>();
	for (const warning of row) {
		if (!('address' in warning) || !warning.address || byAddress.has(warning.address)) continue;
		const outcome = snmpCheckOf(warning, credentialOf);
		if (!outcome) continue;
		byAddress.set(warning.address, {
			address: warning.address,
			credentialId: 'credential_id' in warning ? (warning.credential_id ?? null) : null,
			sentence: metaDescriptionWith('warning_codes', code, build([warning], context), fallback),
			outcome
		});
	}
	const devices = [...byAddress.values()];
	return devices.some((d) => 'commands' in d.outcome) ? devices : [];
}

/**
 * What a row concerns, counted for its header: addresses, hosts or subnets, read off what the
 * occurrences carry. A list of chips there told a reader nothing they could act on, and repeated
 * what the open row already says.
 *
 * The host id is preferred where a warning has both. An unmatched neighbour carries the address the
 * *far* end published, and the device an operator would go and look at is the near one that
 * reported it. Scan-level codes carry none of the three and get no count.
 */
function affectsOf(warnings: DiscoveryWarning[], hostName: HostNameLookup): WarningAffects | null {
	let type: WarningAffects['type'] | null = null;
	const labels = new Map<string, string | null>();
	for (const [i, w] of warnings.entries()) {
		if ('host_id' in w) {
			type = 'host';
			// A host deleted since the scan lost its id; each still counts as one.
			labels.set(w.host_id ?? `gone:${i}`, hostSubject(w.host_id, hostName)?.label ?? null);
		} else if ('cidr' in w) {
			type = 'subnet';
			labels.set(w.cidr, w.cidr);
		} else if ('address' in w && w.address) {
			type = 'address';
			labels.set(w.address, w.address);
		}
	}
	if (type === null) return null;
	const [only] = labels.values();
	return { type, count: labels.size, only: labels.size === 1 ? only : null };
}

/** The rung a code sits on, as the backend filed it. */
function remedyOf(code: DiscoveryWarningCode): string | null {
	return warningCodes.find((c) => c.id === code)?.category ?? null;
}

/**
 * Whether this run is asking the reader for something.
 *
 * The one bit the scan-history table and the modal's tab dot both need, and the reason they agree:
 * "needs you" means the same thing on every surface because it is the same question of the same
 * backend metadata.
 */
export function warningsNeedAttention(warnings: DiscoveryWarning[]): boolean {
	return warnings.some((w) => remedyOf(w.code) === NEEDS_ATTENTION_REMEDY);
}

/** The rung that means a person has to do something in Scanopy. */
const NEEDS_ATTENTION_REMEDY = 'FixInScanopy';

/**
 * The distinct stored credentials a row implicates, in first-seen order.
 *
 * What turns *Fix in Scanopy* from a signpost into a destination: with exactly one, the row can
 * open that credential; with several, the list is the honest answer, because the row covers all of
 * them and there is no single record to open — the same rule `ProvisionalSubnetInferred` already
 * applies to inferred ranges.
 *
 * Empty is the ordinary case for a warning recorded before ids were carried, or posted by a daemon
 * that predates them, so an empty result means "no better destination than the list", never "this
 * is not a credential row".
 *
 * Lives here rather than in the component so it can be tested: everything in `WarningReport.svelte`
 * is reachable only by mounting it, and there are no component tests in this suite.
 */
export function credentialIdsOf(entry: Pick<WarningEntry, 'warnings'>): string[] {
	return [...new Set(entry.warnings.flatMap(credentialIdOf))];
}

/** The stored credential one warning names, as a zero- or one-element list. */
function credentialIdOf(w: DiscoveryWarning): string[] {
	return 'credential_id' in w && w.credential_id ? [w.credential_id] : [];
}

/**
 * Whether a warning belongs on the run's Credentials tab rather than its Scan tab, as the
 * backend files its code (`concerns_credential`). Each warning appears on exactly one of the two.
 */
export function isCredentialWarning(w: DiscoveryWarning): boolean {
	return warningCodes.find((c) => c.id === w.code)?.metadata?.concerns_credential === true;
}

/**
 * How many of a run's warnings name each stored credential.
 *
 * Counts occurrences, not rows: a credential rejected on three hosts has three warnings. Warnings
 * without a credential id are not counted anywhere.
 */
export function warningCountsByCredential(warnings: DiscoveryWarning[]): Map<string, number> {
	const counts = new Map<string, number>();
	for (const id of warnings.flatMap(credentialIdOf)) {
		counts.set(id, (counts.get(id) ?? 0) + 1);
	}
	return counts;
}

/**
 * One code's occurrences, split by the title each fills its name in with. Insertion-ordered. A
 * fixed name fills in the same for all of them, so that is one group.
 */
function byTitle(
	code: DiscoveryWarningCode,
	group_: DiscoveryWarning[],
	fallback: string,
	context: ParamContext
): Map<string, DiscoveryWarning[]> {
	const build = WARNING_PARAMS[code] as (w: DiscoveryWarning[], context: ParamContext) => Params;
	const rows = new Map<string, DiscoveryWarning[]>();
	for (const warning of group_) {
		const title = metaNameWith('warning_codes', code, build([warning], context), fallback);
		rows.set(title, [...(rows.get(title) ?? []), warning]);
	}
	return rows;
}

/**
 * A run's warnings, grouped into the sections an operator reads them in.
 *
 * The sections are the rungs of `warning-remedies.json` — what the reader has to do — in fixture
 * order, most demanding first. That, and not severity, is the top-level cut: severity measures what
 * the scan lost, and the two come apart badly enough that a thirty-second credential fix and a
 * permanently malformed LLDP table are both `Lost`/Red. Severity stays on the row, as its icon and
 * colour, which is the job it was written for.
 *
 * Within a section a row is one *code*, not one occurrence, and inside a row the occurrences that
 * say the same thing become one sentence naming every device — see {@link AGGREGATING_SLOTS}. The
 * wall of text this replaced came from every occurrence carrying its own full explanation: four
 * devices restricting the same SNMP view produced four near-identical paragraphs, and are now one
 * row and one sentence. Occurrences that genuinely differ — a credential diagnostic, a device's own
 * figures — still get a sentence each, so nothing the backend kept apart is merged.
 *
 * A row's title may name only what every occurrence under it shares. So a code whose name is a
 * template ("{integration} found without a credential") becomes one row per distinct title, each
 * with its own action, and a code with a fixed name stays one row.
 */
export function buildWarningReport(
	warnings: DiscoveryWarning[],
	nameOfEntity: EntityNameLookup = NO_ENTITY_NAMES,
	daemonName: string | null = null,
	credentialOf: CredentialLookup = () => undefined
): WarningSection[] {
	const context: ParamContext = { daemonName: daemonName ?? discovery_warningUnknownDaemon() };
	// The example and sentence builders below only ever name hosts, so they keep taking the
	// narrower lookup rather than every one of them growing an entity-type argument it would
	// always pass the same value for.
	const hostName: HostNameLookup = (id) => nameOfEntity('Host', id);
	const byCode = new Map<DiscoveryWarningCode, DiscoveryWarning[]>();
	for (const warning of warnings) {
		const existing = byCode.get(warning.code);
		if (existing) {
			existing.push(warning);
		} else {
			byCode.set(warning.code, [warning]);
		}
	}

	const entries: WarningEntry[] = [];
	for (const [code, group_] of byCode) {
		const meta = warningCodes.find((c) => c.id === code);
		// A code this build has no fixture entry for renders nothing, as it did before: there is no
		// sentence to show and no rung to file it under.
		if (!meta) continue;

		for (const [title, row] of byTitle(code, group_, meta.name ?? code, context)) {
			entries.push({
				key: `${code}:${title}`,
				code,
				title,
				color: toColor(meta.color),
				icon: meta.icon,
				affects: affectsOf(row, hostName),
				devices: deviceChecksOf(code, row, credentialOf, context),
				details: renderStatements(code, row, hostName, context),
				warnings: row
			});
		}
	}

	return warningRemedies
		.map((remedy) => ({
			remedy: remedy.id,
			title: metaName('warning_remedies', remedy.id, remedy.name ?? remedy.id),
			description: metaDescription('warning_remedies', remedy.id, remedy.description ?? ''),
			icon: remedy.icon,
			entries: entries.filter((entry) => remedyOf(entry.code) === remedy.id)
		}))
		.filter((section) => section.entries.length > 0);
}
