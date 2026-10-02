import { coerce, lt } from 'semver';
import osFamilies from '$lib/data/os-families.json';
import { credentials_daemonOsMismatch } from '$lib/paraglide/messages';
import type { Credential } from '../types/base';
import { osFamilyOf, osLabel, type DaemonOS } from '$lib/features/daemons/utils';

/**
 * Whether a daemon at `daemonVersion` is too old to receive a credential type
 * whose minimum-daemon-version floor is `minVersion`.
 *
 * `coerce` tolerates partial/odd version strings; a missing daemon version or a
 * missing floor means there is nothing to gate on (returns false). Used by the
 * discovery credential picker and wizard to disable too-new credential types.
 */
export function daemonTooOldForCredential(
	minVersion: string | undefined | null,
	daemonVersion: string | undefined | null
): boolean {
	if (!minVersion || !daemonVersion) return false;
	const dv = coerce(daemonVersion);
	return dv ? lt(dv, minVersion) : false;
}

/**
 * Why a daemon on `daemonOs` cannot use `credential`, or null when it can: the credential reads
 * files on a daemon of another OS. Mirrors the server's `CredentialBase::daemon_os_refusal`; a
 * credential that reads nothing on the daemon has no `daemon_os` and is never refused.
 */
export function daemonOsRefusal(
	credential: Credential,
	daemonOs: DaemonOS | null | undefined,
	daemonName: string | null | undefined
): string | null {
	const declared = credential.daemon_os;
	if (!declared || !daemonOs || declared === osFamilyOf(daemonOs)) return null;
	return credentials_daemonOsMismatch({
		declared: osFamilies.find((f) => f.id === declared)?.name ?? declared,
		actual: osLabel(daemonOs),
		name: daemonName ?? ''
	});
}
