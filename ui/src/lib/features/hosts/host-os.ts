/**
 * How a host's operating system reads: the product its source named, else the family, then the
 * release. Family labels are backend metadata (`host-os-families.json`), proper nouns that are not
 * translated, the same as service names.
 */

import type { components } from '$lib/api/schema';
import hostOsFamilies from '$lib/data/host-os-families.json';
import type { DaemonOS } from '$lib/features/daemons/utils';
import OsIcon from '$lib/features/daemons/components/OsIcon.svelte';
import {
	attributeSourceExplanation,
	type AttributeSource
} from '$lib/shared/utils/attribute-source';
import { createIconComponent, createLogoIconComponent } from '$lib/shared/utils/styling';
import type { IconComponent } from '$lib/shared/utils/types';
import { hosts_os_codename, hosts_os_edition, hosts_os_kernel } from '$lib/paraglide/messages';

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

/**
 * The icon for a family, from `host-os-families.json`; nothing here names a family. A family the
 * daemon also runs on draws the daemon OS icon, else the vendor's downloaded logo, else a glyph.
 */
export function hostOsIcon(family: HostOsFamily): IconComponent {
	const entry = hostOsFamilies.find((item) => item.id === family);
	const meta = entry?.metadata as
		| { daemon_os?: DaemonOS | null; logo_ext?: string; logo_needs_white_background?: boolean }
		| undefined;
	if (meta?.daemon_os) {
		const os = meta.daemon_os;
		// Pre-binds the OS, as `createLogoIconComponent` pre-binds the logo.
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		return ($$payload: any, $$props: any) => OsIcon($$payload, { os, ...$$props });
	}
	if (meta?.logo_ext) {
		return createLogoIconComponent(
			entry?.icon ?? null,
			`/logos/os-families/${family}.${meta.logo_ext}`,
			!!meta.logo_needs_white_background
		);
	}
	return createIconComponent(entry?.icon ?? null);
}

/**
 * The tooltip for an OS tag: the OS as its source named it, each detail the source read, then how
 * the value reached Scanopy, which for an inferred source says it was assumed.
 */
export function hostOsTooltip(os: HostOs, source: AttributeSource | null | undefined): string {
	return [
		hostOsLabel(os),
		os.edition ? hosts_os_edition({ value: os.edition }) : null,
		os.codename ? hosts_os_codename({ value: os.codename }) : null,
		os.kernel_version ? hosts_os_kernel({ value: os.kernel_version }) : null,
		source ? attributeSourceExplanation(source) : null
	]
		.filter((line): line is string => !!line)
		.join('\n');
}

/**
 * A daemon's OS as a host OS, so a daemon's tag is the same `OsTag`. The family is read from the
 * fixture's `daemon_os` link, which the backend derives from `From<DaemonOs> for HostOsFamily`.
 */
export function hostOsOfDaemonOs(os: DaemonOS): HostOs | null {
	const entry = hostOsFamilies.find(
		(item) => (item.metadata as { daemon_os?: string | null } | null)?.daemon_os === os
	);
	return entry ? { family: entry.id as HostOsFamily } : null;
}

/** Every family the backend can report. Fixture ids are `HostOsFamily` names, emitted from that
 *  enum, so each is a valid filter value by construction. */
export const hostOsFamilyIds: HostOsFamily[] = hostOsFamilies.map(
	(entry) => entry.id as HostOsFamily
);
