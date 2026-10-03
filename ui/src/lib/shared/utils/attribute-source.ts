/**
 * Where one discovered value came from, as a label and a tag.
 *
 * The labels are backend metadata: `attribute-sources.json` names each source by its variant, and
 * `client-probes.json` names the probe that `Probe` and `Authored` carry. Every tag is the same
 * neutral grey: the label says where a value came from, and colour adds no ranking on top of it.
 */

import type { components } from '$lib/api/schema';
import type { TagProps } from '$lib/shared/components/data/types';
import { metaDescriptionWith, metaName, metaNameWith } from '$lib/i18n/metadata';
import { attributeSources, clientProbes } from '$lib/shared/stores/metadata';
import attributeMethods from '$lib/data/attribute-methods.json';

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

/**
 * How a value from this source reached Scanopy, led by the tier's own name when the value was
 * inferred: "Assumed: Scanopy inferred this from …". The tier is what tells a reader to treat the
 * value with caution, so it is said in words wherever the source is explained.
 */
export function attributeSourceExplanation(source: AttributeSource): string {
	const description = attributeSourceDescription(source);
	if (!isInferredSource(source)) return description;
	const inferred = attributeMethods.find((method) => method.id === 'Inferred');
	return `${metaName('attribute_methods', 'Inferred', inferred?.name ?? '')}: ${description}`;
}

/** A neutral tag naming the source, with how the value reached Scanopy as its title. */
export function attributeSourceTag(source: AttributeSource): TagProps {
	return {
		label: attributeSourceLabel(source),
		color: 'Gray',
		title: attributeSourceExplanation(source)
	};
}

/**
 * Every source that sits at the `Inferred` tier, as the backend groups them.
 *
 * Read from the metadata fixture rather than matched against a variant name: the tier a source
 * belongs to is a backend decision, the same `AttributeSource::method()` the applier orders by.
 * Keyed on the whole source, probe included, because `Probe(Snmp)` and `Probe(Docker)` sit at
 * different tiers.
 */
const INFERRED_SOURCES: ReadonlySet<string> = new Set(
	(
		(attributeMethods.find((method) => method.id === 'Inferred')?.metadata?.sources ??
			[]) as AttributeSource[]
	).map(sourceKey)
);

/** Whether a value from this source was derived rather than read: a guess, not evidence. */
export function isInferredSource(source: AttributeSource | null | undefined): boolean {
	return source ? INFERRED_SOURCES.has(sourceKey(source)) : false;
}
