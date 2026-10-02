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

/** How many integrations have at least one type selected (checked or forced on). */
export function selectedIntegrationCount(
	groups: IntegrationGroup<{ id: string }, PickerType>[],
	checkedTypeIds: string[]
): number {
	return groups.filter((g) => selectedTypeCount(g, checkedTypeIds) > 0).length;
}

/**
 * Whether an integration row is expanded. The user's own toggle wins; untouched, a row is
 * expanded while it holds a selection, so a preselected type (a daemon-host socket, a fixed
 * capability) is visible whenever it arrives. Single-type integrations have nothing to expand.
 */
export function isIntegrationExpanded(
	group: IntegrationGroup<{ id: string }, PickerType>,
	checkedTypeIds: string[],
	userToggle: boolean | undefined
): boolean {
	if (group.types.length < 2) return false;
	return userToggle ?? selectedTypeCount(group, checkedTypeIds) > 0;
}

/** A picker section: the integrations sharing one credential category. */
export interface CategorySection<
	I extends { id: string; category?: string | null },
	T extends PickerType
> {
	category: string;
	groups: IntegrationGroup<I, T>[];
}

/**
 * Split integration groups into sections by the integration's `category` (the same field the
 * credential-type dropdown groups on). Sections follow the categories' order in `integrations`
 * (the fixture), and each keeps its groups' order.
 */
export function groupIntegrationsByCategory<
	I extends { id: string; category?: string | null },
	T extends PickerType
>(groups: IntegrationGroup<I, T>[], integrations: I[]): CategorySection<I, T>[] {
	const order = [...new Set(integrations.map((i) => i.category ?? ''))];
	return order
		.map((category) => ({
			category,
			groups: groups.filter((g) => (g.integration.category ?? '') === category)
		}))
		.filter((section) => section.groups.length > 0);
}
