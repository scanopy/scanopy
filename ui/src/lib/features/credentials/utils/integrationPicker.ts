import type { CredentialTarget } from './credentialTargets';

/** The fields of a credential type the integration picker reads. */
export interface PickerType {
	id: string;
	metadata?: {
		integration?: string;
		targets?: CredentialTarget[];
	} | null;
}

/** One picker row: an integration and the credential types that are its transports. */
export interface IntegrationGroup<I extends { id: string }, T extends PickerType> {
	integration: I;
	types: T[];
}

// Rank a type by how far its applicable targets reach: daemon-only first (0), host (1),
// network-applicable last (2).
function targetRank(type: PickerType): number {
	const targets = type.metadata?.targets ?? [];
	if (targets.includes('Network')) return 2;
	if (targets.includes('Hosts')) return 1;
	return 0;
}

/**
 * Group credential types under their integration (the backend `metadata.integration` key, which
 * names an entry in `credential-integrations.json`).
 *
 * Within a group, daemon-only types precede network-applicable ones; groups are ordered the same
 * way by their nearest-reaching type, so daemon-only integrations come first. Sorts are stable, so
 * fixture order breaks ties. A type whose integration has no fixture entry is left out rather
 * than shown under a raw id.
 */
export function groupTypesByIntegration<I extends { id: string }, T extends PickerType>(
	types: T[],
	integrations: I[]
): IntegrationGroup<I, T>[] {
	const groups: IntegrationGroup<I, T>[] = [];
	for (const type of types) {
		const integration = integrations.find((i) => i.id === type.metadata?.integration);
		if (!integration) continue;
		let group = groups.find((g) => g.integration.id === integration.id);
		if (!group) {
			group = { integration, types: [] };
			groups.push(group);
		}
		group.types.push(type);
	}
	for (const group of groups) {
		group.types.sort((a, b) => targetRank(a) - targetRank(b));
	}
	const groupRank = (g: IntegrationGroup<I, T>) => Math.min(...g.types.map(targetRank));
	groups.sort((a, b) => groupRank(a) - groupRank(b));
	return groups;
}

/** How many of a group's types are selected (checked or forced on). */
export function selectedTypeCount(
	group: IntegrationGroup<{ id: string }, PickerType>,
	checkedTypeIds: string[]
): number {
	return group.types.filter((t) => checkedTypeIds.includes(t.id)).length;
}

/**
 * Integration ids whose row opens expanded: those with more than one type and at least one of
 * them already checked, so a selection made earlier (or a returning wizard's rows) is visible.
 * Single-type integrations have nothing to expand.
 */
export function initiallyExpandedIntegrationIds(
	groups: IntegrationGroup<{ id: string }, PickerType>[],
	checkedTypeIds: string[]
): string[] {
	return groups
		.filter((g) => g.types.length > 1 && selectedTypeCount(g, checkedTypeIds) > 0)
		.map((g) => g.integration.id);
}

/** The fields of a wizard row that decide whether the picker owns it. */
export interface PickerPendingRow {
	credential: { id: string; credential_type: { type: string } };
	isExisting?: boolean;
	lockedHosts?: unknown[];
}

/**
 * Whether the picker's selection owns this wizard row: a new credential not yet saved. Existing
 * credentials, rows listed because they are assigned elsewhere, and credentials already created
 * this session are managed in the wizard itself, never added or removed by the picker.
 */
function pickerOwnsRow(row: PickerPendingRow, savedIds: string[]): boolean {
	return !row.isExisting && !row.lockedHosts?.length && !savedIds.includes(row.credential.id);
}

/** The type ids the picker shows selected when the wizard returns to it. */
export function pickerSelectionFromPending(rows: PickerPendingRow[], savedIds: string[]): string[] {
	return [
		...new Set(
			rows.filter((r) => pickerOwnsRow(r, savedIds)).map((r) => r.credential.credential_type.type)
		)
	];
}

/**
 * Whether a wizard row survives continuing from the picker with `selectedTypeIds`: a row the
 * picker owns is dropped when its type was deselected; every other row is kept.
 */
export function keepPendingRow(
	row: PickerPendingRow,
	selectedTypeIds: string[],
	savedIds: string[]
): boolean {
	return (
		selectedTypeIds.includes(row.credential.credential_type.type) || !pickerOwnsRow(row, savedIds)
	);
}
