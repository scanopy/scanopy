import { persistedSet, toggleInSet } from '$lib/shared/stores/persisted-set';

/**
 * Inspector sections and subsections the viewer collapsed, keyed by section id (not by the
 * selected node): collapsing "IP addresses" once keeps it collapsed on every host. Sections
 * start expanded.
 */
export const collapsedInspectorSections = persistedSet('scanopy_topology_inspector_collapsed');

export function toggleInspectorSection(id: string): void {
	toggleInSet(collapsedInspectorSections, id);
}
