import type { components } from '$lib/api/schema';
import {
	common_hostname,
	common_ipAddress,
	common_name,
	hosts_snmp_chassisId,
	hosts_snmp_sysName
} from '$lib/paraglide/messages';
import type { HostNameLadderEntry, HostNameRung } from './types/base';

type AttributeSource = components['schemas']['AttributeSource'];

/**
 * The host editor's naming model: a discovered name, and an optional override a person sets.
 *
 * Both live in the stored `name`. Discovery writes it with its own source, and a person's entry is
 * stored with source `Manual`, which nothing discovery reads displaces. The editor splits them back
 * apart: the discovered name is shown read-only, and the Name field edits only the override.
 *
 * The ladder behind the discovered name is `Host::name_ladder` on the server, sent as
 * `name_ladder`: the rungs in resolution order, their values with blanks dropped, and each value's
 * source. The server already places a guessed name below the identifiers (the placement rule is
 * documented in `backend/src/server/hosts/impl/name.rs`). Nothing here walks rungs of its own (see
 * `host-display-name.ts` for why a second copy is forbidden). The discovered name is the first
 * entry holding a value, skipping a name a person set: the server's own resolution rule, applied to
 * the server's list, with the override set aside.
 */
export interface DiscoveredName {
	value: string;
	/** The rung that supplied it. `Name` when discovery wrote the stored name itself. */
	rung: HostNameRung;
	/** Where the value came from. `null` for rungs that record no source. */
	source: AttributeSource | null;
}

/** What the host is called when no override is set. `null` when nothing identifies it. */
export function discoveredName(ladder: HostNameLadderEntry[]): DiscoveredName | null {
	const entry = ladder.find(
		(candidate) => candidate.value && !(candidate.rung === 'Name' && candidate.source === 'Manual')
	);
	return entry?.value
		? { value: entry.value, rung: entry.rung, source: entry.source ?? null }
		: null;
}

/** The override a saved host carries: its stored name when a person set it, otherwise none. */
export function overrideOf(savedName: string, nameSource: AttributeSource | undefined): string {
	return nameSource === 'Manual' ? savedName : '';
}

/**
 * The `name` to send when the editor saves.
 *
 * A typed override is sent as typed, and the server stores it as `Manual`. A blank override clears
 * a name a person set, handing naming back to discovery. A blank override on a host whose name
 * discovery supplied sends that name back unchanged, so the save leaves it and its source alone.
 */
export function nameToSubmit({
	override,
	savedName,
	nameSource
}: {
	override: string;
	savedName: string;
	nameSource: AttributeSource | undefined;
}): string {
	if (override.trim()) return override;
	return nameSource === 'Manual' ? '' : savedName;
}

/** Labels reuse the field names each rung already has elsewhere in the editor. */
const RUNG_LABELS: Record<HostNameRung, () => string> = {
	Name: () => common_name(),
	Hostname: () => common_hostname(),
	SysName: () => hosts_snmp_sysName(),
	ChassisId: () => hosts_snmp_chassisId(),
	Address: () => common_ipAddress()
};

export function rungLabel(rung: HostNameRung): string {
	return RUNG_LABELS[rung]();
}
