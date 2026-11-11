/**
 * The net-snmp commands that check an SNMP scan warning against the device, from the daemon's
 * host.
 *
 * What each code checks comes from the backend: the code's `snmp_check` (and, for codes whose
 * warning names no group, `snmp_check_group`) in `warning-codes.json`. Everything that varies is
 * off the wire or a fixture: the address and group from the warning; each group's first column,
 * table, page size, timeout and retries from `snmp-walk-groups.json`, which the daemon's own
 * constants fill; the claim's object from `claim-sources.json`; and the credential options from
 * the stored credential the warning names, with every secret a placeholder.
 */

import type { components } from '#lib/api/schema';
import claimSources from '#lib/data/claim-sources.json';
import snmpAuthProtocols from '#lib/data/snmp-auth-protocols.json';
import snmpPrivProtocols from '#lib/data/snmp-priv-protocols.json';
import snmpWalkGroups from '#lib/data/snmp-walk-groups.json';
import warningCodes from '#lib/data/warning-codes.json';
import {
	common_snmpAuthPassphrasePlaceholder,
	discovery_snmpCheckNoCredential,
	discovery_snmpCheckNoFailedRead,
	discovery_snmpCheckUnrecorded,
	common_snmpCommunityPlaceholder,
	common_snmpPrivPassphrasePlaceholder
} from '#lib/paraglide/messages.js';
import type { DiscoveryWarning } from './warnings';

type SnmpCheck = components['schemas']['SnmpCheck'];
type SnmpVersion = components['schemas']['SnmpVersion'];
type Credential = components['schemas']['Credential'];

/** A stored credential by id, or `undefined` when it is gone or this viewer cannot read it. */
export type CredentialLookup = (id: string) => Credential | undefined;

/**
 * What a warning's row shows for checking it: the commands, or why there are none for this
 * device. `null` for a code nothing can check.
 */
export type SnmpCheckOutcome = { commands: string[] } | { reason: string } | null;

interface WalkParameters {
	first_column: string;
	table: string;
	max_repetitions: number;
	timeout_seconds: number;
	retries: number;
}

function parametersOf(groupId: string): WalkParameters | null {
	const entry = snmpWalkGroups.find((g) => g.id === groupId);
	return (entry?.metadata as WalkParameters | undefined) ?? null;
}

function checkOf(code: string): { check: SnmpCheck; group: string | null } | null {
	const metadata = warningCodes.find((c) => c.id === code)?.metadata as
		{ snmp_check?: SnmpCheck | null; snmp_check_group?: string | null } | undefined;
	if (!metadata?.snmp_check) return null;
	return { check: metadata.snmp_check, group: metadata.snmp_check_group ?? null };
}

function netSnmpName(fixture: { id: string; metadata: unknown }[], id: string): string {
	const metadata = fixture.find((p) => p.id === id)?.metadata as
		{ net_snmp_name?: string } | undefined;
	return metadata?.net_snmp_name ?? id;
}

/** `value` as one shell word: bare when it is plainly safe, single-quoted otherwise. */
function shellWord(value: string): string {
	return /^[A-Za-z0-9\-_.@]+$/.test(value) ? value : `'${value.replaceAll("'", `'\\''`)}'`;
}

/**
 * The net-snmp options that authenticate as a stored credential: its version, user, security
 * level, protocols and context, with each secret a placeholder. A community credential is tried
 * over v2c and v1, so its version is the one the device `answered` on, v2c when the warning does
 * not say. `null` for a credential that is gone, that this viewer cannot read, or that is not an
 * SNMP credential: a command with its authentication left blank checks nothing.
 */
export function snmpOptionsFor(
	credential: Credential | undefined,
	answered: SnmpVersion | null
): string | null {
	const type = credential?.credential_type;
	switch (type?.type) {
		case 'SnmpCommunity':
			return `${answered === 'V1' ? '-v1' : '-v2c'} -c ${common_snmpCommunityPlaceholder()}`;
		case 'SnmpV3': {
			const options = ['-v3', `-u ${shellWord(type.security_name)}`];
			switch (type.security_level) {
				case 'NoAuthNoPriv':
					options.push('-l noAuthNoPriv');
					break;
				case 'AuthNoPriv':
					options.push(
						'-l authNoPriv',
						`-a ${netSnmpName(snmpAuthProtocols, type.auth_protocol)}`,
						`-A ${common_snmpAuthPassphrasePlaceholder()}`
					);
					break;
				case 'AuthPriv':
					options.push(
						'-l authPriv',
						`-a ${netSnmpName(snmpAuthProtocols, type.auth_protocol)}`,
						`-A ${common_snmpAuthPassphrasePlaceholder()}`,
						`-x ${netSnmpName(snmpPrivProtocols, type.priv_protocol)}`,
						`-X ${common_snmpPrivPassphrasePlaceholder()}`
					);
					break;
			}
			if (type.context_name) options.push(`-n ${shellWord(type.context_name)}`);
			return options.join(' ');
		}
		default:
			return null;
	}
}

/** The groups a warning's check covers. A whole-device line checks one group per cause, since
 *  the reads that ended the same way repeat each other. */
function groupsOf(warning: DiscoveryWarning, impliedGroup: string | null): string[] {
	if (warning.code === 'SnmpCollectedNothing') {
		const firstPerCause = new Map<string, string>();
		for (const failure of warning.failures ?? []) {
			if (!firstPerCause.has(failure.cause)) firstPerCause.set(failure.cause, failure.group);
		}
		return [...firstPerCause.values()];
	}
	if ('group' in warning) return [warning.group];
	return impliedGroup ? [impliedGroup] : [];
}

/** The `snmpget` of what the device published about itself, for a claim warning. */
function claimObjectOf(warning: DiscoveryWarning): string | null {
	if (!('source' in warning)) return null;
	const metadata = claimSources.find((c) => c.id === warning.source)?.metadata as
		{ oid?: string } | undefined;
	return metadata?.oid ?? null;
}

/**
 * What checking one warning against its device takes, as its row shows it.
 *
 * A whole-device line with no failed read has no commands, and says why: either every read ended
 * normally on an empty table, or the daemon that ran the scan recorded no failures.
 */
export function snmpCheckOf(
	warning: DiscoveryWarning,
	credentialOf: CredentialLookup
): SnmpCheckOutcome {
	const declared = checkOf(warning.code);
	if (!declared || !('address' in warning) || !warning.address) return null;

	if (warning.code === 'SnmpCollectedNothing') {
		if (warning.failures == null) return { reason: discovery_snmpCheckUnrecorded() };
		if (warning.failures.length === 0) return { reason: discovery_snmpCheckNoFailedRead() };
	}

	const credentialId = 'credential_id' in warning ? warning.credential_id : null;
	const answered = 'snmp_version' in warning ? warning.snmp_version : null;
	const options = snmpOptionsFor(credentialId ? credentialOf(credentialId) : undefined, answered);
	if (options === null)
		return { reason: discovery_snmpCheckNoCredential({ address: warning.address }) };
	const claimObject = declared.check === 'Claim' ? claimObjectOf(warning) : null;

	const commands = groupsOf(warning, declared.group).flatMap((groupId) => {
		const walk = parametersOf(groupId);
		if (!walk) return [];
		const common = `${options} -t ${walk.timeout_seconds} -r ${walk.retries}`;
		const page = `-Cr${walk.max_repetitions}`;
		const column = `${warning.address} ${walk.first_column}`;
		// SNMPv1 has no GETBULK, so a device read over v1 is checked with `snmpwalk` alone.
		const bulkwalk = (args: string) =>
			answered === 'V1' ? [] : [`snmpbulkwalk ${common} ${args}`];

		switch (declared.check) {
			case 'Repeat':
				return [
					...bulkwalk(`${page} ${column}`),
					...bulkwalk(`-Cr1 ${column}`),
					`snmpwalk ${common} ${column}`
				];
			case 'Recount':
				return [...bulkwalk(`${page} ${column}`), `snmpwalk ${common} ${column}`];
			case 'Claim':
				return [
					...(claimObject ? [`snmpget ${common} ${warning.address} ${claimObject}`] : []),
					...bulkwalk(`${page} ${column}`),
					`snmpwalk ${common} ${column}`
				];
			case 'Unordered':
				return [...bulkwalk(`-Cc ${page} ${column}`), `snmpwalk ${common} -Cc ${column}`];
			case 'Table':
				return answered === 'V1'
					? [`snmpwalk ${common} ${warning.address} ${walk.table}`]
					: [`snmpbulkwalk ${common} ${page} ${warning.address} ${walk.table}`];
		}
	});

	return commands.length > 0 ? { commands } : null;
}
