/**
 * How a host's operating system reads: the product its source named, else the family, then the
 * release. Family labels are backend metadata (`host-os-families.json`), proper nouns that are not
 * translated, the same as service names.
 */

import type { components } from '#lib/api/schema';
import hostOsFamilies from '#lib/data/host-os-families.json';
import type { DaemonOS } from '#lib/features/daemons/utils.ts';
import {
	attributeSourceExplanation,
	type AttributeSource
} from '#lib/shared/utils/attribute-source.ts';
import { createIconComponent } from '#lib/shared/utils/styling.ts';
import { logoIcon } from '#lib/shared/utils/logos.ts';
import type { IconComponent } from '#lib/shared/utils/types.ts';
import type { TooltipContent, TooltipSection } from '#lib/shared/actions/tooltip.ts';
import { common_codename, common_edition, common_kernel } from '#lib/paraglide/messages.js';

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
 * The icon for a family, from `host-os-families.json`; nothing here names a family. The family's
 * logo when it has one (a daemon OS family's too), else its glyph.
 */
export function hostOsIcon(
	family: HostOsFamily,
	options: { framed?: boolean } = {}
): IconComponent {
	const entry = hostOsFamilies.find((item) => item.id === family);
	const meta = entry?.metadata as { logo?: string | null } | undefined;
	return logoIcon(meta?.logo, createIconComponent(entry?.icon ?? null), options);
}

/**
 * The tooltip for an OS tag: the OS as its source named it, each detail the source read, then how
 * the value reached Scanopy, which for an inferred source says it was assumed.
 */
export function hostOsTooltip(
	os: HostOs,
	source: AttributeSource | null | undefined
): TooltipContent {
	const details: [string, string | null | undefined][] = [
		[common_edition(), os.edition],
		[common_codename(), os.codename],
		[common_kernel(), os.kernel_version]
	];
	const rows = details.flatMap(([label, value]) => (value ? [{ label, value }] : []));
	const sections: TooltipSection[] = [];
	if (rows.length > 0) sections.push({ rows });
	if (source) sections.push({ text: attributeSourceExplanation(source) });
	return { title: hostOsLabel(os), sections };
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
