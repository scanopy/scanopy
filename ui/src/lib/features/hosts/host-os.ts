/**
 * How a host's operating system reads: the product its source named, else the family, then the
 * release. Family labels are backend metadata (`host-os-families.json`), proper nouns that are not
 * translated, the same as service names.
 */

import type { components } from '$lib/api/schema';
import hostOsFamilies from '$lib/data/host-os-families.json';

export type HostOs = components['schemas']['HostOs'];
export type HostOsFamily = components['schemas']['HostOsFamily'];

/** A family's display name ("macOS", "Cisco IOS XE"). */
export function hostOsFamilyName(family: HostOsFamily): string {
	return hostOsFamilies.find((entry) => entry.id === family)?.name ?? family;
}

/** One line for an OS: "Ubuntu 24.04", "Windows Server 2008 R2", "macOS". */
export function hostOsLabel(os: HostOs): string {
	const name = os.name ?? hostOsFamilyName(os.family);
	return os.version ? `${name} ${os.version}` : name;
}

/** Every family the backend can report. Fixture ids are `HostOsFamily` names, emitted from that
 *  enum, so each is a valid filter value by construction. */
export const hostOsFamilyIds: HostOsFamily[] = hostOsFamilies.map(
	(entry) => entry.id as HostOsFamily
);
