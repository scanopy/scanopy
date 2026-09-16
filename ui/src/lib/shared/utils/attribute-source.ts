/**
 * Where one discovered value came from, as a label and a tag.
 *
 * The labels are backend metadata: `attribute-sources.json` names each source by its variant, and
 * `client-probes.json` names the probe that `Probe` and `Authored` carry. Every tag is the same
 * neutral grey: the label says where a value came from, and colour adds no ranking on top of it.
 */

import type { components } from '$lib/api/schema';
import type { TagProps } from '$lib/shared/components/data/types';
import { metaDescriptionWith, metaNameWith } from '$lib/i18n/metadata';
import { attributeSources, clientProbes } from '$lib/shared/stores/metadata';

/** Derived from the backend enum rather than restated, so a new source cannot drift out of sync. */
export type AttributeSource = components['schemas']['AttributeSource'];

/**
 * The variant, and the probe it carries.
 *
 * Externally tagged: a source with nothing to carry is its own bare name, and the two that carry a
 * probe are a single-entry object keyed by the variant, `{ Probe: 'Snmp' }`.
 */
function sourceParts(source: AttributeSource): { variant: string; probe: string | null } {
	if (typeof source === 'string') return { variant: source, probe: null };
	const [[variant, probe]] = Object.entries(source);
	return { variant, probe };
}

/** A stable key for a source, so a probe is compared along with the variant that carries it. */
export function sourceKey(source: AttributeSource): string {
	const { variant, probe } = sourceParts(source);
	return probe ? `${variant}:${probe}` : variant;
}

function slots(probe: string | null): Record<string, string> {
	return { probe: probe ? clientProbes.getName(probe) : '' };
}

/** What to call a source: "SNMP", "Reverse DNS", "Entered in Scanopy". */
export function attributeSourceLabel(source: AttributeSource): string {
	const { variant, probe } = sourceParts(source);
	const fallback = attributeSources.getItem(variant)?.name ?? variant;
	return metaNameWith('attribute_sources', variant, slots(probe), fallback);
}

/** One sentence on how a value from this source reached Scanopy. */
export function attributeSourceDescription(source: AttributeSource): string {
	const { variant, probe } = sourceParts(source);
	const fallback = attributeSources.getItem(variant)?.description ?? '';
	return metaDescriptionWith('attribute_sources', variant, slots(probe), fallback);
}

/** A neutral tag naming the source, with the source's description as its title. */
export function attributeSourceTag(source: AttributeSource): TagProps {
	return {
		label: attributeSourceLabel(source),
		color: 'Gray',
		title: attributeSourceDescription(source)
	};
}
